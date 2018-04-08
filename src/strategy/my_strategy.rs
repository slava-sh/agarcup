use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::f64::consts::PI;
use std::rc::{Rc, Weak};

use config::config;
use strategy::*;
use strategy::mechanic::{Mechanic, State};
use version::VERSION;

const MAX_LEADING_BLOBS: i64 = 3;
const MAX_DEPTH: i64 = 15;
const MIN_SKIPS: i64 = 5;
const COMMAND_DISTANCE: f64 = 100.0;

const SPEED_REWARD_FACTOR: f64 = 0.01;

const SAFETY_MARGIN_FACTOR: f64 = 2.5;
const SAFETY_MARGIN_PENALTY: f64 = -3.0;

const SMALL_BLOB_PENALTY: f64 = -10.0;
const MAX_SMALL_BLOB_MASS: f64 = 85.0;

lazy_static! {
    static ref DISCOVERY_ANGLES: Vec<f64> = {
        let n = 4 * 3;
        (0..n).map(|i| 2.0 * PI * i as f64 / n as f64).collect()
    };
}

#[derive(Debug, Default)]
pub struct MyStrategy {
    root: SharedNode,
    next_root: SharedNode,
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
    commands: Vec<Command>,
    parent: Weak<RefCell<Node>>,
    children: Vec<SharedNode>,
}

type SharedNode = Rc<RefCell<Node>>;

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
        self.state = State::new(tick, my_blobs);
        self.food = food;
        self.ejections = ejections;
        self.viruses = viruses;
        self.enemies = enemies;

        let mut command = Command::new();
        if tick == 0 {
            command.add_debug_message(format!("running my strategy version {}", VERSION));
        }

        if self.commands.is_empty() {
            let me = &self.state.my_blobs.values().next().unwrap(); // TODO: Move away from me.
            let speed = (me.speed() + me.max_speed()) / 2.0;
            self.skips = ((me.r() / speed).round() as i64).max(MIN_SKIPS);

            let mut child: Node = Default::default();
            child.state = self.state.clone();
            self.root = Rc::new(RefCell::new(child));
            self.add_nodes(&self.root);

            self.target = find_nodes(&self.root)
                .into_iter()
                .filter(|node| !Rc::ptr_eq(&node, &self.root))
                .max_by(|a, b| {
                    self.node_score(a).partial_cmp(&self.node_score(b)).expect(
                        "incomparable scores",
                    )
                })
                .expect("no nodes found");
            self.next_root = self.next_root();
            self.commands.extend(
                self.next_root
                    .borrow()
                    .commands
                    .iter()
                    .cloned(),
            );
        }

        command.set_point(self.commands.pop_front().expect("no commands left").point());

        #[cfg(feature = "debug")]
        {
            self.debug(&mut command);
        }

        command
    }
}

impl MyStrategy {
    pub fn new() -> MyStrategy {
        Default::default()
    }

    fn node_score(&self, node: &SharedNode) -> f64 {
        node.borrow()
            .state
            .my_blobs
            .values()
            .map(|me| self.blob_score(me))
            .sum()
    }

    fn blob_score(&self, me: &Player) -> f64 {
        let mut score = 0.0;
        score += me.m();

        score += me.speed() * SPEED_REWARD_FACTOR;

        if me.m() <= MAX_SMALL_BLOB_MASS {
            score += SMALL_BLOB_PENALTY;
        }

        // TODO: safety.
        // TODO: global goal.

        let safety_margin = me.r() * SAFETY_MARGIN_FACTOR;
        if me.x() < safety_margin || me.x() > config().game_width as f64 - safety_margin {
            score += SAFETY_MARGIN_PENALTY;
        }
        if me.y() < safety_margin || me.y() > config().game_height as f64 - safety_margin {
            score += SAFETY_MARGIN_PENALTY;
        }

        score
    }

    fn add_nodes(&self, root: &SharedNode) {
        let leading_blobs: Vec<_> = root.borrow()
            .state
            .my_blobs
            .values()
            .take(MAX_LEADING_BLOBS as usize)
            .cloned()
            .collect(); // TODO: Sort by mass.
        let fragment_count = root.borrow().state.my_blobs.len() as i64;
        for me in leading_blobs {
            for angle in DISCOVERY_ANGLES.iter() {
                let v = Point::from_polar(COMMAND_DISTANCE, me.angle() + angle);
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
                        .map(|i| {
                            let mut command = Command::from_point(node_me.point() + v);
                            command
                        })
                        .collect();
                    let child = Rc::new(RefCell::new(Node {
                        state: self.predict_states(&node.borrow().state, commands.as_ref()),
                        commands: commands,
                        parent: Rc::downgrade(&node),
                        children: Default::default(),
                    }));
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

        for me in self.next_root.borrow().state.my_blobs.values() {
            command.add_debug_circle(DebugCircle {
                center: me.point(),
                radius: me.r(),
                color: String::from("green"),
                opacity: 0.1,
            });
        }

        let command_point = command.point();
        command.add_debug_circle(DebugCircle {
            center: command_point,
            radius: 4.0,
            color: String::from("pink"),
            opacity: 1.0,
        });

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
