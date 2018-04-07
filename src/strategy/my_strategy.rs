use std::collections::{HashSet, VecDeque};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::f64::consts::PI;

use strategy::*;
use config::config;

const ROOT_EPS: f64 = 1.0;
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
    food: Vec<Food>,
    ejections: Vec<Ejection>,
    viruses: Vec<Virus>,
    enemies: Vec<Player>,
    skips: i64,
    #[cfg(feature = "debug")]
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

#[derive(Debug, Default)]
struct State {
    tick: i64,
    my_blobs: Vec<Player>,
    eaten: Rc<HashSet<BlobId>>,
}

impl State {
    fn me(&self) -> Option<&Player> {
        self.my_blobs.first()
    }
}

impl Node {
    fn new(state: State) -> Node {
        let mut node: Node = Default::default();
        node.state = state;
        node.score = node.state
            .my_blobs
            .iter()
            .map(|me| node.compute_blob_score(me))
            .sum::<f64>()
            .max(0.0);
        node
    }

    fn compute_blob_score(&self, me: &Player) -> f64 {
        let mut score = 0.0;
        score += me.m();
        score += me.speed() * SPEED_REWARD_FACTOR;

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

        self.food = food;
        self.ejections = ejections;
        self.viruses = viruses;
        self.enemies = enemies;

        let mut my_blobs = my_blobs;
        my_blobs.sort_by(|a, b| a.m().partial_cmp(&b.m()).expect("incomparable mass"));
        let me = &my_blobs[0];

        let speed = (me.speed() + me.max_speed()) / 2.0;
        self.skips = ((me.r() / speed).round() as i64).max(MIN_SKIPS);

        let mut should_reset_root = true;
        if let Some(ref root_me) = self.root.borrow().state.me() {
            should_reset_root = self.commands.is_empty() &&
                root_me.point().qdist(me.point()) > ROOT_EPS.powi(2)
        }
        if should_reset_root {
            #[cfg(feature = "debug")]
            {
                if let Some(ref root_me) = self.root.borrow().state.me() {
                    command.add_debug_message(format!("RESET"));
                    command.add_debug_message(
                        format!("dist = {:.2}", root_me.point().dist(me.point())),
                    );
                }
            }
            self.root = Rc::new(RefCell::new(Node::new(State {
                tick,
                my_blobs: my_blobs.to_vec(),
                eaten: Rc::new(HashSet::new()),
            })));
            self.add_nodes(&self.root);
        }

        if self.commands.is_empty() {
            let target = find_nodes(&self.root)
                .into_iter()
                .filter(|node| !Rc::ptr_eq(&node, &self.root))
                .max_by(|a, b| {
                    a.borrow().score.partial_cmp(&b.borrow().score).expect(
                        "incomparable scores",
                    )
                })
                .expect("no nodes found");
            #[cfg(feature = "debug")]
            {
                self.target = Rc::clone(&target);
            }
            self.root = self.get_next_root(target);
            self.root.borrow_mut().parent = Weak::new();
            for command in self.root.borrow().commands.iter() {
                self.commands.push_back(
                    Command::from_point(command.point()),
                );
            }
            self.add_nodes(&self.root);
        }

        command.set_point(self.commands.pop_front().expect("no commands left").point());

        #[cfg(feature = "debug")]
        {
            fn go(node: &SharedNode, tree_size: &mut i64, command: &mut Command) {
                *tree_size = *tree_size + 1;
                for me in node.borrow().state.my_blobs.iter() {
                    command.add_debug_circle(DebugCircle {
                        center: me.point(),
                        radius: 1.0,
                        color: String::from("black"),
                        opacity: 0.3,
                    });
                }
                for child in node.borrow().children.iter() {
                    for (n, c) in node.borrow().state.my_blobs.iter().zip(
                        child
                            .borrow()
                            .state
                            .my_blobs
                            .iter(),
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
            go(&self.root, &mut tree_size, &mut command);

            for me in self.root.borrow().state.my_blobs.iter() {
                command.add_debug_circle(DebugCircle {
                    center: me.point(),
                    radius: me.r(),
                    color: String::from("green"),
                    opacity: 0.1,
                });
            }

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
                for (n, p) in node.borrow().state.my_blobs.iter().zip(
                    parent
                        .borrow()
                        .state
                        .my_blobs
                        .iter(),
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

            fn as_blob<B: Blob>(b: &B) -> &Blob {
                b as &Blob
            }
            use std::iter;
            for blob in iter::empty()
                .chain(self.food.iter().map(as_blob))
                .chain(self.ejections.iter().map(as_blob))
                .chain(self.viruses.iter().map(as_blob))
                .chain(self.enemies.iter().map(as_blob))
            {
                if self.target.borrow().state.eaten.contains(blob.id()) {
                    command.add_debug_circle(DebugCircle {
                        center: blob.point(),
                        radius: blob.r() + 2.0,
                        color: String::from("green"),
                        opacity: 0.5,
                    });
                }
            }

            for blob in iter::empty()
                .chain(self.viruses.iter().map(as_blob))
                .chain(self.enemies.iter().map(as_blob))
            {
                command.add_debug_circle(DebugCircle {
                    center: blob.point(),
                    radius: blob.r() + 2.0,
                    color: String::from("red"),
                    opacity: 0.1,
                });
            }

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
            .iter()
            .take(MAX_POWER_BLOBS as usize)
            .cloned()
            .collect();
        for me in power_blobs {
            for angle in DISCOVERY_ANGLES.iter() {
                let v = Point::from_polar(config().speed_factor, me.angle() + angle);
                let mut node = Rc::clone(&root);
                for _depth in 0..MAX_DEPTH {
                    if node.borrow().state.me().is_none() ||
                        !me.can_see(node.borrow().state.me().unwrap())
                    {
                        break;
                    }
                    let commands: Vec<_> = (0..self.skips)
                        .map(|_i| {
                            Command::from_point(node.borrow().state.me().unwrap().point() + v)
                        })
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

    fn get_next_root(&self, target: SharedNode) -> SharedNode {
        let mut node = target;
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

    fn predict_state(&self, state: &State, command: &Command, slow: bool) -> State {
        let mut my_blobs = state.my_blobs.clone();

        // Following the oringial mechanic.
        // apply_strategies: update v.
        for me in my_blobs.iter_mut() {
            me.update_v(command);
        }

        // shrink_players.
        let tick = state.tick + 1;
        if tick % config().shrink_every_tick == 0 {
            for me in my_blobs.iter_mut() {
                me.shrink();
            }
        }

        // who_is_eaten: update m.
        let eaten = if slow {
            fn maybe_eat<B: Blob>(blob: &B, my_blobs: &mut [Player], eaten: &mut HashSet<BlobId>) {
                if eaten.contains(blob.id()) {
                    return;
                }
                if let Some(i) = find_nearest_me(blob, |me| me.can_eat(blob), my_blobs) {
                    eaten.insert(blob.id().to_string());
                    my_blobs[i].m_ += blob.m();
                }
            }

            let mut eaten = state.eaten.as_ref().clone();
            for blob in self.food.iter() {
                maybe_eat(blob, my_blobs.as_mut(), &mut eaten);
            }
            for blob in self.ejections.iter() {
                maybe_eat(blob, my_blobs.as_mut(), &mut eaten);
            }
            for blob in self.enemies.iter() {
                maybe_eat(blob, my_blobs.as_mut(), &mut eaten);
            }

            for enemy in self.enemies.iter() {
                if eaten.contains(enemy.id()) {
                    continue;
                }
                if let Some(i) = find_nearest_me(enemy, |me| enemy.can_eat(me), my_blobs.as_ref()) {
                    my_blobs.swap_remove(i); // Die.
                }
            }
            Rc::new(eaten)
        } else {
            Rc::clone(&state.eaten)
        };

        // TODO: who_need_fusion.

        // who_intersected_virus.
        if slow {
            for virus in self.viruses.iter() {
                if let Some(i) = find_nearest_me(virus, |me| me.can_burst(), my_blobs.as_ref()) {
                    let me = my_blobs.swap_remove(i);
                    my_blobs.extend(self.burst(me, virus));
                }
            }
        }

        // update_by_state: update r, limit v, split.
        for me in my_blobs.iter_mut() {
            me.update_r();
            me.limit_speed();
        }
        if command.split() {
            my_blobs = my_blobs.into_iter().flat_map(|me| self.split(me)).collect();
        }

        // move_moveables: collide (TODO), move, apply viscosity, update ttf.
        for me in my_blobs.iter_mut() {
            me.apply_v();
            me.apply_viscosity();
            if let Some(ttf) = me.ttf_ {
                if ttf > 0 {
                    me.ttf_ = Some(ttf - 1);
                }
            }
        }

        my_blobs.sort_by(|a, b| a.m().partial_cmp(&b.m()).expect("incomparable mass"));
        State {
            tick,
            my_blobs,
            eaten,
        }
    }

    fn split(&self, me: Player) -> Vec<Player> {
        // TODO
        if !me.can_split() {
            return vec![me];
        }
        let m = me.m() / 2.0;
        let v = Point::from_polar(config().split_start_speed, me.angle());

        let mut me1 = Player {
            id_: format!("{}+1", me.id()), // TODO: Compute correct ids.
            point_: me.point(),
            m_: m,
            r_: 0.0,
            v_: Some(v),
            is_fast_: Some(true),
            ttf_: Some(config().ticks_til_fusion),
        };
        me1.update_r();

        let mut me2 = me;
        me2.id_ = format!("{}+2", me2.id());
        me2.m_ = m;
        me2.update_r();
        me2.ttf_ = Some(config().ticks_til_fusion);

        vec![me1, me2]
    }

    fn burst(&self, me: Player, virus: &Virus) -> Vec<Player> {
        // TODO
        if virus.can_hurt(&me) {
            vec![] // Assume we die.
        } else {
            vec![me]
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

fn find_nearest_me<T, P>(target: &T, predicate: P, my_blobs: &[Player]) -> Option<usize>
where
    T: HasPoint,
    P: Fn(&Player) -> bool,
{
    my_blobs
        .iter()
        .enumerate()
        .filter(|&(_, me)| predicate(me))
        .min_by(|&(_, a), &(_, b)| {
            a.point()
                .qdist(target.point())
                .partial_cmp(&b.point().qdist(target.point()))
                .expect("incomparable distances")
        })
        .map(|(i, _)| i)
}
