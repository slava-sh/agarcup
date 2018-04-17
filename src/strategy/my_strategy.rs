use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::f64::consts::PI;
use std::rc::{Rc, Weak};
use std::time::{Instant, Duration};

use rand::{Rng, SeedableRng, XorShiftRng};

use config::config;
use strategy::*;
use strategy::mechanic::{Mechanic, State};
use version::VERSION;

const AVG_TICK_TIME_SECS: f64 = 600.0 / 25000.0;
const MIN_SKIPS: i64 = 5;
const SIMULATION_DEPTH: i64 = 7;
const COMMAND_DISTANCE_FACTOR: f64 = 2.0;

const GHOST_TICKS: i64 = 50;
const GHOST_VISIBILITY_FACTOR: f64 = 0.80;
const GHOST_TTF_FACTOR: f64 = 0.5;

const SPEED_REWARD_FACTOR: f64 = 0.01;
const DANGER_PENALTY_FACTOR: f64 = -300.0;
const SAFETY_MARGIN_FACTOR: f64 = 7.0;
const SAFETY_MARGIN_PENALTY: f64 = -5.0;
const SMALL_BLOB_PENALTY: f64 = -10.0;
const MAX_SMALL_BLOB_MASS: f64 = 85.0;

lazy_static! {
    static ref DISCOVERY_ANGLES: Vec<Angle> = {
        let n = 4 * 3;
        (0..n).map(|i| 2.0 * PI * i as f64 / n as f64).collect()
    };
}

#[derive(Debug)]
pub struct MyStrategy {
    root: SharedNode,
    next_root: SharedNode,
    commands: VecDeque<Command>,
    ghost_enemies: HashMap<PlayerBlobId, Ghost>,
    rng: XorShiftRng,

    state: State,
    food: Vec<Food>,
    ejections: Vec<Ejection>,
    viruses: Vec<Virus>,

    tick_start_time: Instant,
    skips: i64,
    target: SharedNode,

    paths_seen: i64,
    num_paths: i64,
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

#[derive(Debug)]
struct Ghost {
    player: Player,
    last_seen: Tick,
}

impl MyStrategy {
    pub fn new() -> MyStrategy {
        MyStrategy {
            root: Default::default(),
            next_root: Default::default(),
            commands: Default::default(),
            ghost_enemies: Default::default(),
            rng: XorShiftRng::from_seed([0x1337_5EED; 4]),

            state: Default::default(),
            food: Default::default(),
            ejections: Default::default(),
            viruses: Default::default(),

            tick_start_time: Instant::now(),
            skips: Default::default(),
            target: Default::default(),

            paths_seen: Default::default(),
            num_paths: Default::default(),
        }
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
            if enemy.m() > me.m() {
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

    fn tick_impl(
        &mut self,
        tick: Tick,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command {
        self.tick_start_time = Instant::now();
        self.food = food;
        self.ejections = ejections;
        self.viruses = viruses;
        self.state.tick = tick;
        self.state.my_blobs = my_blobs;
        self.state.eaten_food = Default::default();
        self.state.eaten_ejections = Default::default();
        self.state.eaten_viruses = Default::default();
        self.update_enemies(enemies);
        if self.commands.is_empty() ||
            self.state.my_blobs.len() != self.target.borrow().state.my_blobs.len() ||
            self.state.enemies.len() != self.target.borrow().state.enemies.len()
        {
            self.commands.clear();
            self.update_skips();
            self.add_commands();
        }
        let mut command = self.commands.pop_front().expect("no commands left");
        if self.state.tick == 0 {
            command.add_debug_message(format!("running my strategy version {}", VERSION));
        }
        #[cfg(feature = "debug")] self.debug(&mut command);
        command
    }

    fn add_commands(&mut self) {
        self.root = Default::default();
        self.root.borrow_mut().state = self.state.clone();
        self.add_nodes();

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
                .take(self.skips as usize)
                .cloned(),
        );
    }

    fn add_nodes(&mut self) {
        let mut paths = self.generate_paths();
        self.rng.shuffle(&mut paths);
        #[cfg(feature = "debug")]
        {
            self.paths_seen = 0;
            self.num_paths = paths.len() as i64;
        }
        for (i, path) in paths.into_iter().enumerate() {
            let time_budget = AVG_TICK_TIME_SECS * self.skips as f64;
            let elapsed = duration_to_secs(self.tick_start_time.elapsed());
            if i != 0 && elapsed * (i + 1) as f64 / i as f64 > time_budget {
                break;
            }
            #[cfg(feature = "debug")]
            {
                self.paths_seen += 1;
            }
            let mut node = Rc::clone(&self.root);
            let mut depth = 0;
            for _ in 0..SIMULATION_DEPTH {
                let commands: Vec<_> = (0..self.skips)
                    .map(|_| {
                        let command = path[depth.min(path.len() - 1)].clone();
                        depth += 1;
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

    fn generate_paths(&self) -> Vec<Vec<Command>> {
        let mut paths: Vec<Vec<Command>> = Vec::new();
        for me in self.state.my_blobs.iter() {
            let splits = if me.can_split(1) {
                vec![false, true]
            } else {
                vec![false]
            };
            for split in splits {
                for angle in DISCOVERY_ANGLES.iter() {
                    let target = me.point() +
                        Point::from_polar(
                            me.vision_radius(self.state.my_blobs.len()) * COMMAND_DISTANCE_FACTOR,
                            me.angle() + angle,
                        );
                    paths.push(
                        (0..2)
                            .map(|i| {
                                let mut command = Command::from_point(target);
                                if split && i == 0 {
                                    command.set_split();
                                }
                                command
                            })
                            .collect(),
                    );
                }
            }
        }
        paths
    }

    fn update_skips(&mut self) {
        let me = &self.state
            .my_blobs
            .iter()
            .max_by(|a, b| a.m().partial_cmp(&b.m()).expect("incomparable mass"))
            .expect("add_commands with no blobs");
        let speed = (me.speed() + me.max_speed()) / 2.0;
        self.skips = ((me.r() / speed).round() as i64).max(MIN_SKIPS);
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

    fn update_enemies(&mut self, enemies: Vec<Player>) {
        let tick = self.state.tick;
        for mut enemy in enemies {
            if let Some(ghost) = self.ghost_enemies
                .get(&enemy.id())
                .into_iter()
                .filter(|ghost| ghost.last_seen == tick - 1)
                .next()
            {
                let v = enemy.point() - ghost.player.point();
                enemy.set_v(v);
                enemy.update_is_fast();
                enemy.set_ttf((ghost.player.ttf() - 1).max(0));
            } else {
                enemy.set_ttf((config().ticks_til_fusion as f64 * GHOST_TTF_FACTOR) as i64);
            }
            self.ghost_enemies.insert(
                enemy.id(),
                Ghost {
                    player: enemy,
                    last_seen: tick,
                },
            );
        }
        let ref my_blobs = self.state.my_blobs;
        self.ghost_enemies.retain(|_, ghost| {
            ghost.last_seen >= tick - GHOST_TICKS &&
                (ghost.last_seen == tick ||
                     !my_blobs.iter().any(|me| {
                        me.can_see_safe(&ghost.player, my_blobs.len(), GHOST_VISIBILITY_FACTOR)
                    }))
        });
        self.state.enemies = self.ghost_enemies
            .values()
            .map(|enemy| enemy.player.clone())
            .collect();
    }

    #[cfg(feature = "debug")]
    fn debug(&self, command: &mut Command) {
        fn zip<'a>(
            parents: &'a [Player],
            children: &'a [Player],
        ) -> Box<Iterator<Item = (&'a Player, &'a Player)> + 'a> {
            Box::new(parents.iter().cycle().zip(children.iter()))
        }

        fn go(node: &SharedNode, tree_size: &mut i64, command: &mut Command, num_blobs: usize) {
            let node = node.borrow();
            *tree_size = *tree_size + 1;
            let debug_skips = false;
            if debug_skips {
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
                let debug_simulation_depth = true;
                if debug_simulation_depth {
                    let color = if node.state.my_blobs.len() > num_blobs {
                        String::from("pink")
                    } else {
                        String::from("lightGray")
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
                go(child, tree_size, command, num_blobs);
            }
        }

        let mut tree_size = 0;
        go(
            &self.root,
            &mut tree_size,
            command,
            self.state.my_blobs.len(),
        );

        for enemy in self.next_root.borrow().state.enemies.iter() {
            command.add_debug_circle(DebugCircle {
                center: enemy.point(),
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
                if eaten.contains(&blob.id()) {
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
        for enemy in self.state.enemies.iter() {
            if !target_state.enemies.iter().any(|player| {
                player.id() == enemy.id()
            })
            {
                command.add_debug_circle(DebugCircle {
                    center: enemy.point(),
                    radius: enemy.r() + 2.0,
                    color: String::from("green"),
                    opacity: 0.5,
                });
            }
        }
        for me in self.state.my_blobs.iter() {
            command.add_debug_circle(DebugCircle {
                center: me.point() + Point::from_polar(config().vis_shift, me.angle()),
                radius: me.vision_radius(self.state.my_blobs.len()) * GHOST_VISIBILITY_FACTOR,
                color: String::from("blue"),
                opacity: 0.05,
            });
        }
        for ghost in self.ghost_enemies.values() {
            command.add_debug_circle(DebugCircle {
                center: ghost.player.point(),
                radius: ghost.player.r(),
                color: String::from("blue"),
                opacity: 0.5,
            });
        }

        command.add_debug_message(format!("skips:\t{}", self.skips));
        command.add_debug_message(format!("queue:\t{}", self.commands.len()));
        command.add_debug_message(format!("paths:\t{} / {}", self.paths_seen, self.num_paths));
        command.add_debug_message(format!("tree:\t{}", tree_size));
        command.add_debug_message(format!("enemies:\t{}", self.state.enemies.len()));
        command.add_debug_message(format!("food:\t{}", self.food.len()));
        command.add_debug_message(format!("viruses:\t{}", self.viruses.len()));
        command.add_debug_message(format!(
            "goal:\t{:.4}",
            AVG_TICK_TIME_SECS * self.skips as f64
        ));
        command.add_debug_message(format!(
            "spent:\t{:.4}",
            duration_to_secs(self.tick_start_time.elapsed())
        ));
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

impl Strategy for MyStrategy {
    fn tick(
        &mut self,
        tick: Tick,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command {
        if my_blobs.is_empty() {
            return Default::default();
        }
        self.tick_impl(tick, my_blobs, food, ejections, viruses, enemies)
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

fn duration_to_secs(duration: Duration) -> f64 {
    duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9
}
