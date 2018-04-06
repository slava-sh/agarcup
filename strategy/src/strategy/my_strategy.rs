use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use strategy::*;
use config::config;

const ROOT_EPS: f64 = 1.0;
//const DISCOVERY_ANGLES = np.linspace(0, 2 * math.pi, 4 * 3)[:-1]
const MAX_POWER_BLOBS: i64 = 1;
const MAX_DEPTH: i64 = 7;
const MIN_SKIPS: i64 = 5;

const SPEED_REWARD_FACTOR: f64 = 0.01;

const SAFETY_MARGIN_FACTOR: f64 = 2.5;
const SAFETY_MARGIN_PENALTY: f64 = -3.0;

pub struct MyStrategy {
    root: Option<Rc<Node>>,
    commands: VecDeque<Command>,
}

struct Node {
    state: State,
    parent: Rc<Node>,
    commands: Vec<Command>,
    chilren: Vec<Rc<Node>>,
    score: f64,
}

impl Node {
    fn recompute_tip_score(&mut self) {
        self.score = self.state
            .my_blobs
            .iter()
            .map(|me| self.compute_blob_score(me))
            .sum::<f64>()
            .max(0.0);
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

struct State {
    tick: i64,
    my_blobs: Vec<Player>,
    eaten: HashSet<BlobId>,
}

impl State {
    fn me(&self) -> Option<&Player> {
        self.my_blobs.first()
    }
}

impl MyStrategy {
    pub fn new() -> MyStrategy {
        MyStrategy {
            root: None,
            commands: VecDeque::new(),
        }
    }
}

impl Strategy for MyStrategy {
    fn tick(
        &mut self,
        tick: i64,
        mut my_blobs: Vec<Player>,
        mut food: Vec<Food>,
        mut ejections: Vec<Ejection>,
        mut viruses: Vec<Virus>,
        mut enemies: Vec<Player>,
    ) -> Command {
        let mut command = Command::new();
        my_blobs.sort_by(|a, b| {
            a.m().partial_cmp(&b.m()).unwrap_or_else(
                || a.id().cmp(&b.id()),
            )
        });
        let me = &my_blobs[0];
        let speed = (me.speed() + me.max_speed()) / 2.0;
        let skips = ((me.r() / speed).round() as i64).max(MIN_SKIPS);
        command.set_point(Point::new(500.0, 500.0));
        config();
        command
    }
}

//        if (self.root is None or self.root.state.me() is None
//                or (not self.commands
//                    and self.root.state.me().qdist(me) > ROOT_EPS**2)):
//            if (self.debug and self.root is not None
//                    and self.root.state.me() is not None):
//                self.debug_messages.append('RESET')
//                self.debug_messages.append('dist = {:.2f}'.format(
//                    self.root.state.me().dist(me)))
//            self.root = self.new_tip(State(tick, my_blobs))
//            self.add_nodes(self.root, skips)
//
//        if not self.commands:
//            nodes = find_nodes(self.root)
//            target = max(
//                (node for node in nodes if node is not self.root),
//                key=lambda node: node.score)
//            if self.debug:
//                self.debug_tip = target
//            self.root = self.get_next_root(target)
//            self.root.parent = None
//            for command in self.root.commands:
//                self.commands.append(
//                    Command(x=command.x, y=command.y, split=command.split))
//            self.add_nodes(self.root, skips)
//
//        command = self.commands.popleft()
//        if self.debug:
//            tree_size = 0
//
//            def go(node):
//                nonlocal tree_size
//                tree_size += 1
//                for me in node.state.my_blobs:
//                    command.add_debug_circle(
//                        Circle(me.x, me.y, 1), 'black', 0.3)
//                for child in node.children:
//                    for n, c in zip(node.state.my_blobs, child.state.my_blobs):
//                        command.add_debug_line([n, c], 'black', 0.3)
//                    go(child)
//
//            go(self.root)
//
//            for me in self.root.state.my_blobs:
//                command.add_debug_circle(
//                    Circle(me.x, me.y, me.r), 'green', 0.1)
//            for me in self.debug_tip.state.my_blobs:
//                command.add_debug_circle(Circle(me.x, me.y, 2), 'red')
//            node = self.debug_tip
//            while node.parent is not None:
//                for n, p in zip(node.state.my_blobs,
//                                node.parent.state.my_blobs):
//                    command.add_debug_line([n, p], 'black')
//                node = node.parent
//
//            for food in food + ejections + viruses + enemies:
//                if food.id in self.debug_tip.state.eaten:
//                    command.add_debug_circle(
//                        Circle(food.x, food.y, food.r + 2), 'green', 0.5)
//            for danger in viruses + enemies:
//                command.add_debug_circle(
//                    Circle(danger.x, danger.y, danger.r + 2), 'red', 0.1)
//
//            command.add_debug_message('skips: {}'.format(skips))
//            command.add_debug_message('queue: {}'.format(len(self.commands)))
//            command.add_debug_message('tree: {}'.format(tree_size))
//
//            if command.split:
//                command.add_debug_message('SPLIT')
//
//            for message in self.debug_messages:
//                command.add_debug_message(message)
//        return command
//
//    def add_nodes(self, root, skips):
//        for me in root.state.my_blobs[:MAX_POWER_BLOBS]:
//            for angle in DISCOVERY_ANGLES:
//                v = Point.from_polar(config().speed_factor, me.angle() + angle)
//                node = root
//                for _ in range(MAX_DEPTH):
//                    if node.state.me() is None or not me.can_see(node.state.me()):
//                        break
//                    commands = [
//                        Command.go_to(node.state.me() + v)
//                        for i in range(skips)
//                    ]
//                    child = self.new_tip(
//                        state=self.predict_states(node.state, commands, skips),
//                        parent=node,
//                        commands=commands)
//                    node.children.append(child)
//                    node = child
//
//    def get_next_root(self, target):
//        node = target
//        while node.parent is not None and node.parent.parent is not None:
//            node = node.parent
//        return node
//
//    def new_tip(self, *args, **kwargs):
//        tip = Node(*args, **kwargs)
//        tip.compute_tip_score()
//        return tip
//
//    def predict_states(self, state, commands, skips):
//        for i, command in enumerate(commands):
//            slow = len(commands) - 1
//            state = self.predict_state(state, command, slow)
//        return state
//
//    def predict_state(self, state, command, slow):
//        # TODO: Fix precision error when eating food at max speed.
//        my_blobs = [copy.copy(me) for me in state.my_blobs]
//
//        # Following the oringial mechanic.
//        # apply_strategies: update v.
//        for me in my_blobs:
//            me.update_v(command)
//
//        # shrink_players.
//        tick = state.tick + 1
//        if tick % config().shrink_every_tick == 0:
//            for me in my_blobs:
//                me.shrink()
//
//        # who_is_eaten: update m.
//        if slow:
//            eaten = state.eaten.copy()
//            for food in self.food + self.ejections + self.enemies:
//                if food.id in eaten:
//                    continue
//                i, me = find_nearest_me(food, lambda me: me.can_eat(food),
//                                        my_blobs)
//                if me is not None:
//                    eaten.add(food.id)
//                    me.m += food.m
//            for enemy in self.enemies:
//                if enemy.id in eaten:
//                    continue
//                i, me = find_nearest_me(enemy, lambda me: enemy.can_eat(me),
//                                        my_blobs)
//                if me is not None:
//                    del my_blobs[i]  # Die.
//        else:
//            eaten = state.eaten
//
//        # TODO: who_need_fusion.
//
//        # who_intersected_virus.
//        if slow:
//            for virus in self.viruses:
//                i, me = find_nearest_me(virus, lambda me: me.can_burst(), my_blobs)
//                if me is not None:
//                    my_blobs[i:i + 1] = self.burst(me, virus)
//
//        # update_by_state: update r, limit v, split.
//        for me in my_blobs:
//            me.update_r()
//            me.limit_speed()
//        if command.split:
//            my_blobs = [new_me for me in my_blobs for new_me in self.split(me)]
//
//        # move_moveables: collide (TODO), move, apply viscosity, update ttf.
//        for me in my_blobs:
//            me.apply_v()
//            me.apply_viscosity()
//            if me.ttf > 0:
//                me.ttf -= 1
//
//        my_blobs.sort(key=lambda me: (me.m, me.id), reverse=True)
//        return State(tick, my_blobs, eaten)
//
//    def split(self, me):
//        if not me.can_split():
//            return [me]
//        m = me.m / 2
//        v = Point.from_polar(config().split_start_speed, me.angle())
//        return [
//            Me(
//                id=me.id + '+1',  # TODO: Compute correct ids.
//                x=me.x,
//                y=me.y,
//                m=m,
//                v=v,
//                is_fast=True,
//                ttf=config().ticks_til_fusion),
//            Me(id=me.id + '+2',
//               x=me.x,
//               y=me.y,
//               m=m,
//               v=me.v,
//               is_fast=me.is_fast,
//               ttf=config().ticks_til_fusion)
//        ]
//
//    def burst(self, me, virus):
//        if virus.can_hurt(me):
//            # Assume we die. TODO.
//            return []
//        return [me]
//
//    def debug_prediction_error(self, tick, my_blobs, command):
//        if hasattr(self, 'next_blobs'):
//            for me, next_me in zip(my_blobs, self.next_blobs):
//                e = next_me.dist(me)
//                command.add_debug_message(
//                    'prediction error: {:.8f} {} {}'.format(
//                        e, next_me.is_fast, me.is_fast))
//                command.add_debug_message('me.m = {:.8f}'.format(me.m))
//                command.add_debug_message('ne.m = {:.8f}'.format(next_me.m))
//                command.add_debug_message('me.speed = {:.8f}'.format(
//                    me.speed()))
//                command.add_debug_message('ne.speed = {:.8f}'.format(
//                    next_me.speed()))
//                command.add_debug_message('me.max_speed = {:.8f}'.format(
//                    me.max_speed()))
//                command.add_debug_message('ne.max_speed = {:.8f}'.format(
//                    next_me.max_speed()))
//                if e > 1e-6 and len(my_blobs) > 1:
//                    command.pause = True
//                    command.add_debug_message('cmd: {!r}'.format(
//                        self.last_command))
//        self.next_blobs = self.predict_state(State(tick, my_blobs),
//                                             command).my_blobs
//        for nb in self.next_blobs:
//            command.add_debug_circle(Circle(nb.x, nb.y, nb.r), 'cyan', 0.5)
//        self.last_command = command
//
//
//def find_nearest_me(target, predicate, my_blobs):
//    return min(
//        ((i, me) for i, me in enumerate(my_blobs) if predicate(me)),
//        key=lambda i_me: i_me[1].qdist(target),
//        default=(None, None))
//
//
//def find_nodes(roots):
//    if not isinstance(roots, collections.Iterable):
//        roots = [roots]
//
//    def go(node, nodes):
//        nodes.append(node)
//        for child in node.children:
//            go(child, nodes)
//
//    nodes = []
//    for root in roots:
//        go(root, nodes)
//    return nodes
//
//
//        self.viruses = [v for v in viruses if me.can_see(v)]
