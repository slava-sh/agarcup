use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::f64::consts::PI;
use std::rc::{Rc, Weak};

use strategy::*;
use strategy::mechanic::{Mechanic, State};
use config::config;

const ROOT_EPS: f64 = 1.0;
const MASS_EPS: f64 = 0.5;
const MAX_POWER_BLOBS: i64 = 1;
const MAX_DEPTH: i64 = 15;
const MIN_SKIPS: i64 = 5;

const SPEED_REWARD_FACTOR: f64 = 0.01;

const SAFETY_MARGIN_FACTOR: f64 = 2.5;
const SAFETY_MARGIN_PENALTY: f64 = -3.0;

lazy_static! {
    static ref DISCOVERY_ANGLES: Vec<f64> = {
        let n = 4 * 3;
        (0..n).map(|i| 2.0 * PI * i as f64 / n as f64).collect()
    };
}

#[derive(Debug, Default)]
pub struct MyStrategy {
    root: SharedNode,
    commands: VecDeque<Command>,

    state: State,
    food: Vec<Food>,
    ejections: Vec<Ejection>,
    viruses: Vec<Virus>,
    enemies: Vec<Player>,

    skips: i64,
    target: SharedNode,
}

#[derive(Debug, Default)]
struct Node {
    state: State,
    parent: Weak<RefCell<Node>>,
    commands: Vec<Command>,
    children: Vec<SharedNode>,
    score: f64,
}

type SharedNode = Rc<RefCell<Node>>;

impl Node {
    fn new(state: State) -> Node {
        let mut node: Node = Default::default();
        node.state = state;
        node.score = node.state
            .my_blobs
            .values()
            .map(|me| node.compute_blob_score(me))
            .sum::<f64>()
            .max(0.0);
        node
    }

    fn compute_blob_score(&self, me: &Player) -> f64 {
        let mut score = 0.0;
        score += me.m();
        score += me.speed() * SPEED_REWARD_FACTOR;

        // TODO: safety.
        // TODO: global goal.

        let safety_margin = me.r() * SAFETY_MARGIN_FACTOR;
        if me.x() < safety_margin || me.x() > config().game_width as f64 - safety_margin {
            score += SAFETY_MARGIN_PENALTY;
        }
        if me.y() < safety_margin || me.y() > config().game_height as f64 - safety_margin {
            score += SAFETY_MARGIN_PENALTY;
        }

        score.max(0.0)
    }
}

impl Strategy for MyStrategy {
    fn tick(
        &mut self,
        tick: i64,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command {
        let mut command = Command::new();

        self.state = State::new(tick, my_blobs);
        self.food = food;
        self.ejections = ejections;
        self.viruses = viruses;
        self.enemies = enemies;

        if self.commands.is_empty() {
            let me = &self.state.my_blobs.values().next().unwrap(); // TODO: Move away from me.
            let speed = (me.speed() + me.max_speed()) / 2.0;
            self.skips = ((me.r() / speed).round() as i64).max(MIN_SKIPS);

            if self.state.my_blobs.len() != self.root.borrow().state.my_blobs.len() ||
                self.state
                    .my_blobs
                    .values()
                    .zip(self.root.borrow().state.my_blobs.values())
                    .any(|(a, b)| {
                        a.id() != b.id() || (a.m() - b.m()).abs() > MASS_EPS ||
                            a.point().qdist(b.point()) > ROOT_EPS.powi(2)
                    })
            {
                //#[cfg(feature = "debug")] self.debug_reset_root(&mut command);
                self.root = Rc::new(RefCell::new(Node::new(self.state.clone())));
                self.add_nodes(&self.root);
            }

            self.target = find_nodes(&self.root)
                .into_iter()
                .filter(|node| !Rc::ptr_eq(&node, &self.root))
                .max_by(|a, b| {
                    a.borrow().score.partial_cmp(&b.borrow().score).expect(
                        "incomparable scores",
                    )
                })
                .expect("no nodes found");
            self.root = self.next_root();
            self.root.borrow_mut().parent = Weak::new();
            self.commands.extend(
                self.root.borrow().commands.iter().map(
                    |command| {
                        Command::from_point(command.point())
                    },
                ),
            );
            self.add_nodes(&self.root);
        }

        command.set_point(self.commands.pop_front().expect("no commands left").point());
        if tick % 500 > 300 && self.state.my_blobs.values().next().unwrap().can_burst(1) {
            command.set_point(self.viruses[0].point());
        }

        #[cfg(feature = "debug")] self.debug(&mut command);

        command
    }
}

impl MyStrategy {
    pub fn new() -> MyStrategy {
        Default::default()
    }

    fn add_nodes(&self, root: &SharedNode) {
        let power_blobs: Vec<_> = root.borrow()
            .state
            .my_blobs
            .values()
            .take(MAX_POWER_BLOBS as usize)
            .cloned()
            .collect(); // TODO: Sort by mass.
        for me in power_blobs {
            for angle in DISCOVERY_ANGLES.iter() {
                let v = Point::from_polar(config().speed_factor, me.angle() + angle);
                let mut node = Rc::clone(&root);
                for _depth in 0..MAX_DEPTH {
                    // TODO: Move away from me.
                    let node_me: Player = match node.borrow().state.my_blobs.values().next() {
                        Some(node_me) => node_me.clone(),
                        None => break,
                    };
                    if !me.can_see(&node_me) {
                        break;
                    }
                    let commands: Vec<_> = (0..self.skips)
                        .map(|_i| Command::from_point(node_me.point() + v))
                        .collect();
                    let state = self.predict_states(&node.borrow().state, commands.as_ref());
                    let mut child = Node::new(state);
                    child.parent = Rc::downgrade(&node);
                    child.commands = commands;
                    let child = Rc::new(RefCell::new(child));
                    node.borrow_mut().children.push(Rc::clone(&child));
                    node = child;
                }
            }
        }
    }

    fn next_root(&self) -> SharedNode {
        let mut node = Rc::clone(&self.target);
        loop {
            let mut next = None;
            if let Some(parent) = node.borrow().parent.upgrade() {
                if parent.borrow().parent.upgrade().is_some() {
                    next = Some(parent);
                }
            }
            match next {
                Some(next) => node = next,
                None => break,
            }
        }
        node
    }

    fn predict_states(&self, state: &State, commands: &[Command]) -> State {
        let mut state = self.predict_state(state, &commands[0], true);
        for command in commands.iter().skip(1) {
            state = self.predict_state(&state, command, true);
        }
        state
    }

    fn predict_state(&self, state: &State, command: &Command, _slow: bool) -> State {
        let mut mechanic = Mechanic::new(state);
        mechanic.tick(
            command,
            &self.food,
            &self.ejections,
            &self.viruses,
            &self.enemies,
        );
        mechanic.state
    }

    #[cfg(feature = "debug")]
    fn debug_reset_root(&self, command: &mut Command) {
        let ref root = self.root.borrow();
        command.set_pause();
        command.add_debug_message(format!("RESET"));
        if self.state.my_blobs.len() != root.state.my_blobs.len() {
            command.add_debug_message(format!(
                "len: {} vs {}",
                self.state.my_blobs.len(),
                root.state.my_blobs.len()
            ));
        }
        self.state
            .my_blobs
            .values()
            .zip(root.state.my_blobs.values())
            .for_each(|(a, b)| {
                if a.id() != b.id() {
                    command.add_debug_message(format!("id: {:?} vs {:?}", a.id(), b.id()));
                }
                if (a.m() - b.m()).abs() > MASS_EPS {
                    command.add_debug_message(format!("m: {} vs {}", a.m(), b.m()));
                }
                command.add_debug_message(format!("dist: {:.2}", a.point().dist(b.point())));
            });
    }

    #[cfg(feature = "debug")]
    fn debug(&self, command: &mut Command) {
        fn go(node: &SharedNode, tree_size: &mut i64, command: &mut Command) {
            *tree_size = *tree_size + 1;
            for me in node.borrow().state.my_blobs.values() {
                command.add_debug_circle(DebugCircle {
                    center: me.point(),
                    radius: 1.0,
                    color: String::from("black"),
                    opacity: 0.3,
                });
            }
            for child in node.borrow().children.iter() {
                for (n, c) in node.borrow().state.my_blobs.values().zip(
                    child
                        .borrow()
                        .state
                        .my_blobs
                        .values(),
                )
                {
                    command.add_debug_line(DebugLine {
                        a: n.point(),
                        b: c.point(),
                        color: String::from("black"),
                        opacity: 0.3,
                    });
                }
                go(child, tree_size, command);
            }
        }

        let mut tree_size = 0;
        go(&self.root, &mut tree_size, command);

        for me in self.root.borrow().state.my_blobs.values() {
            command.add_debug_circle(DebugCircle {
                center: me.point(),
                radius: me.r(),
                color: String::from("green"),
                opacity: 0.1,
            });
        }

        for me in self.target.borrow().state.my_blobs.values() {
            command.add_debug_circle(DebugCircle {
                center: me.point(),
                radius: 2.0,
                color: String::from("red"),
                opacity: 1.0,
            });
        }
        let mut node = Rc::clone(&self.target);
        loop {
            let parent = match node.borrow().parent.upgrade() {
                Some(parent) => parent,
                None => break,
            };
            for (n, p) in node.borrow().state.my_blobs.values().zip(
                parent
                    .borrow()
                    .state
                    .my_blobs
                    .values(),
            )
            {
                command.add_debug_line(DebugLine {
                    a: n.point(),
                    b: p.point(),
                    color: String::from("black"),
                    opacity: 1.0,
                });
            }
            node = parent;
        }

        fn mark_eaten<B: Blob>(blobs: &Vec<B>, eaten: &HashSet<B::Id>, command: &mut Command) {
            for blob in blobs.iter() {
                if eaten.contains(blob.id()) {
                    command.add_debug_circle(DebugCircle {
                        center: blob.point(),
                        radius: blob.r() + 2.0,
                        color: String::from("green"),
                        opacity: 0.5,
                    });
                }
            }
        }

        let target_state = &self.target.borrow().state;
        mark_eaten(&self.food, &target_state.eaten_food, command);
        mark_eaten(&self.ejections, &target_state.eaten_ejections, command);
        mark_eaten(&self.viruses, &target_state.eaten_viruses, command);
        mark_eaten(&self.enemies, &target_state.eaten_enemies, command);

        command.add_debug_message(format!("skips: {}", self.skips));
        command.add_debug_message(format!("queue: {}", self.commands.len()));
        command.add_debug_message(format!("tree: {}", tree_size));
        if command.split() {
            command.add_debug_message(format!("SPLIT"));
        }
        if command.pause() {
            command.add_debug_message(format!("PAUSE"));
        }
    }
}

fn find_nodes(root: &SharedNode) -> Vec<SharedNode> {
    fn go(node: &SharedNode, nodes: &mut Vec<SharedNode>) {
        nodes.push(Rc::clone(node));
        for child in node.borrow().children.iter() {
            go(child, nodes);
        }
    }

    let mut nodes = vec![];
    go(root, &mut nodes);
    nodes
}
