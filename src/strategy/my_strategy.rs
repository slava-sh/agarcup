use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::f64::consts::PI;
use std::rc::{Rc, Weak};

use config::config;
use strategy::*;
use strategy::mechanic::{Mechanic, State};
use version::VERSION;

const COMMAND_DISTANCE_FACTOR: f64 = 2.0;
const MAX_LEADING_BLOBS: i64 = 3;
const MAX_DEPTH: i64 = 5;
const MIN_SKIPS: i64 = 5;

const SPEED_REWARD_FACTOR: f64 = 0.01;
const DANGER_PENALTY_FACTOR: f64 = -100.0;
const SAFETY_MARGIN_FACTOR: f64 = 2.5;
const SAFETY_MARGIN_PENALTY: f64 = -3.0;
const SMALL_BLOB_PENALTY: f64 = -10.0;
const MAX_SMALL_BLOB_MASS: f64 = 85.0;

lazy_static! {
    static ref DISCOVERY_ANGLES: Vec<Angle> = {
        let n = 4 * 3;
        (0..n).map(|i| 2.0 * PI * i as f64 / n as f64).collect()
    };
}

#[derive(Debug, Default)]
pub struct MyStrategy {
    root: SharedNode,
    next_root: SharedNode,
    commands: VecDeque<Command>,
    enemy_pos: HashMap<PlayerBlobId, Point>,

    state: State,
    food: Vec<Food>,
    ejections: Vec<Ejection>,
    viruses: Vec<Virus>,

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
type Score = f64;

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
        self.state = State::new(tick, my_blobs, enemies);
        self.food = food;
        self.ejections = ejections;
        self.viruses = viruses;
        self.infer_speeds();
        if self.commands.is_empty() {
            self.add_commands();
        }
        let mut command = self.commands.pop_front().expect("no commands left");
        if self.state.tick == 0 {
            command.add_debug_message(format!("running my strategy version {}", VERSION));
        }
        #[cfg(feature = "debug")] self.debug(&mut command);
        command
    }
}

impl MyStrategy {
    pub fn new() -> MyStrategy {
        Default::default()
    }

    fn node_score(&self, node: &SharedNode) -> Score {
        let ref state = node.borrow().state;
        state
            .my_blobs
            .iter()
            .map(|me| self.blob_score(me, state))
            .sum()
    }

    fn blob_score(&self, me: &Player, state: &State) -> Score {
        let mut score = 0.0;
        score += me.m();

        score += me.speed() * SPEED_REWARD_FACTOR;

        if me.m() <= MAX_SMALL_BLOB_MASS {
            score += SMALL_BLOB_PENALTY;
        }

        for enemy in state.enemies.iter() {
            if enemy.m() > me.m() && enemy.can_see(me) {
                let mut speed = enemy.max_speed();
                if enemy.m() > me.m() * 2.0 {
                    speed = speed.max(config().split_start_speed);
                }
                let dist = me.point().dist(enemy.point());
                score += DANGER_PENALTY_FACTOR / (dist / speed).max(1.0);
            }
        }

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

    fn add_commands(&mut self) {
        let biggest_me = &self.state
            .my_blobs
            .iter()
            .max_by(|a, b| a.m().partial_cmp(&b.m()).expect("incomparable mass"))
            .expect("add_commands with no blobs");
        let speed = (biggest_me.speed() + biggest_me.max_speed()) / 2.0;
        self.skips = ((biggest_me.r() / speed).round() as i64).max(MIN_SKIPS);

        self.root = Default::default();
        self.root.borrow_mut().state = self.state.clone();
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

    fn add_nodes(&self, root: &SharedNode) {
        let mut leading_blobs: Vec<_> = root.borrow().state.my_blobs.iter().cloned().collect();
        leading_blobs.sort_by(|a, b| {
            a.m()
                .partial_cmp(&b.m())
                .expect("incomparable mass")
                .reverse()
                .then_with(|| a.fragment_id().cmp(&b.fragment_id()))
        });
        for me in leading_blobs.into_iter().take(MAX_LEADING_BLOBS as usize) {
            let splits = if me.can_split(1) {
                vec![false, true]
            } else {
                vec![false]
            };
            for split in splits {
                for angle in DISCOVERY_ANGLES.iter() {
                    let v = Point::from_polar(
                        me.base_vision_radius() * COMMAND_DISTANCE_FACTOR,
                        me.angle() + angle,
                    );
                    let mut node = Rc::clone(&root);
                    for depth in 0..MAX_DEPTH {
                        //let node_me: Player = match node.borrow().state.my_blobs.get(me.id()) {
                        //    Some(node_me) => node_me.clone(),
                        //    None => break,
                        //};
                        //if false && !me.can_see(&node_me) {
                        //    break;
                        //}
                        let commands: Vec<_> = (0..self.skips)
                            .map(|i| {
                                let mut command = Command::from_point(me.point() + v);
                                if split && depth == 0 && i == 0 {
                                    command.set_split();
                                }
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
        let mut mechanic = Mechanic::new(state);
        for command in commands.iter() {
            mechanic.tick(command, &self.food, &self.ejections, &self.viruses);
        }
        mechanic.state
    }

    fn infer_speeds(&mut self) {
        for enemy in self.state.enemies.iter_mut() {
            let v = if let Some(last_pos) = self.enemy_pos.get(enemy.id()) {
                enemy.point() - *last_pos
            } else {
                Point::zero()
            };
            enemy.set_v(v);
            enemy.update_is_fast();
        }
        self.enemy_pos.clear();
        for enemy in self.state.enemies.iter() {
            self.enemy_pos.insert(enemy.id().clone(), enemy.point());
        }
    }

    #[cfg(feature = "debug")]
    fn debug(&self, command: &mut Command) {
        fn zip<'a>(
            parents: &'a [Player],
            children: &'a [Player],
        ) -> Box<Iterator<Item = (&'a Player, &'a Player)> + 'a> {
            Box::new(parents.iter().cycle().zip(children.iter()))
        }

        fn go(node: &SharedNode, tree_size: &mut i64, command: &mut Command, depth: i64) {
            let node = node.borrow();
            let verbose = node.state.my_blobs.len() <= 9999 || depth > 0;
            *tree_size = *tree_size + 1;
            if verbose {
                for me in node.state.my_blobs.iter() {
                    command.add_debug_circle(DebugCircle {
                        center: me.point(),
                        radius: 1.0,
                        color: String::from("black"),
                        opacity: 0.3,
                    });
                }
            }
            for child in node.children.iter() {
                if verbose {
                    let color = if node.state.my_blobs.len() == 1 {
                        String::from("lightGray")
                    } else {
                        String::from("pink")
                    };
                    let child = child.borrow();
                    for (n, c) in zip(&node.state.my_blobs, &child.state.my_blobs) {
                        command.add_debug_line(DebugLine {
                            a: n.point(),
                            b: c.point(),
                            color: color.clone(),
                            opacity: 0.5,
                        });
                    }
                }
                go(child, tree_size, command, depth - 1);
            }
        }

        let mut tree_size = 0;
        go(&self.root, &mut tree_size, command, self.skips * 9999);

        for enemy in self.state.enemies.iter() {
            command.add_debug_circle(DebugCircle {
                center: enemy.point() + enemy.v() * self.skips as f64,
                radius: enemy.r(),
                color: String::from("red"),
                opacity: 0.1,
            });
        }

        for me in self.next_root.borrow().state.my_blobs.iter() {
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

        for me in self.target.borrow().state.my_blobs.iter() {
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
            {
                let parent = parent.borrow();
                let node = node.borrow();
                for (p, n) in zip(&parent.state.my_blobs, &node.state.my_blobs) {
                    command.add_debug_line(DebugLine {
                        a: n.point(),
                        b: p.point(),
                        color: String::from("black"),
                        opacity: 1.0,
                    });
                }
            }
            node = parent;
        }

        use std::collections::HashSet;
        fn mark_eaten<B: Blob>(blobs: &[B], eaten: &HashSet<B::Id>, command: &mut Command) {
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
        // TODO: mark_eaten(&self.enemies, &target_state.eaten_enemies, command);

        command.add_debug_message(format!("skips: {}", self.skips));
        command.add_debug_message(format!("queue: {}", self.commands.len()));
        command.add_debug_message(format!("tree: {}", tree_size));
        command.add_debug_message(format!("enemies: {}", self.state.enemies.len()));
        if self.target.borrow().state.my_blobs.is_empty() {
            command.add_debug_message(format!("ABOUT TO DIE"));
        }
        if self.target.borrow().state.my_blobs.len() > self.state.my_blobs.len() {
            command.add_debug_message(format!("ABOUT TO SPLIT"));
        }
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
