import json
import logging
import math
import os
import numpy as np
import time
import collections
import copy

ROOT_EPS = 1
EXPANSIONS_PER_TICK = 1
MIN_EXPANSION_DEPTH = 3
EXPAND_ANGLES = [0, math.pi / 2, -math.pi / 2, math.pi]
DISCOVERY_ANGLES = np.linspace(0, 2 * math.pi, 4 * 3)[:-1]
MAX_POWER_BLOBS = 1

SKIP_DISTANCE = 20

SPEED_REWARD_FACTOR = 0.01

SAFETY_MARGIN_FACTOR = 2.5
SAFETY_MARGIN_PENALTY = -3


class Config:
    GAME_WIDTH = None
    GAME_HEIGHT = None
    VISCOSITY = None
    TICKS_TIL_FUSION = None
    VIRUS_RADIUS = None
    SPEED_FACTOR = None
    INERTION_FACTOR = None
    FOOD_MASS = None
    FOOD_RADIUS = 2.5
    EJECTION_RADIUS = 4.0
    EJECTION_MASS = 15.0
    MASS_EAT_FACTOR = 1.2
    RADIUS_FACTOR = 2.0
    DIAM_EAT_FACTOR = 2 / 3
    VIS_FACTOR = 4.0
    VIS_FACTOR_FR = 2.5
    VIS_SHIFT = 10.0
    RAD_HURT_FACTOR = 0.66
    MIN_SPLIT_MASS = 120.0
    SPLIT_START_SPEED = 9.0
    SHRINK_EVERY_TICK = 50
    MIN_SHRINK_MASS = 100
    SHRINK_FACTOR = 0.01
    MIN_BURST_MASS = 60.0


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    @staticmethod
    def from_polar(r, angle):
        x = r * math.cos(angle)
        y = r * math.sin(angle)
        return Point(x, y)

    def dist(self, other):
        return math.hypot(self.x - other.x, self.y - other.y)

    def qdist(self, other):
        return (self.x - other.x)**2 + (self.y - other.y)**2

    def angle(self):
        return math.atan2(self.y, self.x)

    def length(self):
        return math.hypot(self.x, self.y)

    def with_length(self, length):
        self_length = self.length()
        if self_length == 0:
            return Point(0, 0)
        return self * (length / self_length)

    def unit(self):
        return self.with_length(1)

    def __mul__(self, k):
        return Point(self.x * k, self.y * k)

    def __truediv__(self, k):
        return Point(self.x / k, self.y / k)

    def __add__(self, other):
        return Point(self.x + other.x, self.y + other.y)

    def __sub__(self, other):
        return Point(self.x - other.x, self.y - other.y)

    def __repr__(self):
        return '({:.3f}, {:.3f})'.format(self.x, self.y)


class Circle(Point):
    def __init__(self, x, y, r):
        super().__init__(x, y)
        self.r = r


class Blob(Circle):
    def __init__(self, id, x, y, r, m):
        super().__init__(x, y, r)
        self.id = id
        self.m = m


class Player(Blob):
    def can_eat(self, other):
        if not (self.m > other.m * Config.MASS_EAT_FACTOR):
            return False
        dist = self.dist(other)
        min_r = dist - other.r + other.r * 2 * Config.DIAM_EAT_FACTOR
        return min_r < self.r

    def can_see(self, other):
        p = self + Point.from_polar(Config.VIS_SHIFT, self.angle())
        vision_radius = self.r * Config.VIS_FACTOR  # TODO: Not always true.
        max_dist = vision_radius + other.r
        return other.qdist(p) < max_dist**2

    def can_burst(self):
        if self.m < Config.MIN_BURST_MASS * 2:
            return False
        # TODO: Consider Config.MAX_FRAGS_CNT.
        frags_cnt = int(self.m / Config.MIN_BURST_MASS)
        return frags_cnt > 1

    def can_hurt(self, other):
        return self.can_eat(other)

    def can_split(self):
        # TODO: Consider Config.MAX_FRAGS_CNT.
        return self.m > Config.MIN_SPLIT_MASS

    def max_speed(self):
        return Config.SPEED_FACTOR / math.sqrt(self.m)


class Enemy(Player):
    pass


class Me(Player):
    def __init__(self, id, x, y, m, v, is_fast=None, r=None, ttf=None):
        super().__init__(id, x, y, r, m)
        self.v = v
        self.ttf = ttf or 0
        if r is None:
            self.update_r()
        if is_fast is None:
            self.is_fast = self.speed() > self.max_speed()
        else:
            self.is_fast = is_fast

    def speed(self):
        return self.v.length()

    def angle(self):
        return self.v.angle()

    def update_r(self):
        self.r = Config.RADIUS_FACTOR * math.sqrt(self.m)

    def limit_speed(self):
        if not self.is_fast:
            self.v = self.v.with_length(min(self.max_speed(), self.speed()))

    def update_v(self, command):
        self.v = self.v + ((command - self).unit() * self.max_speed() - self.v
                           ) * (Config.INERTION_FACTOR / self.m)
        self.limit_speed()

    def apply_v(self):
        self.x = max(self.r, min(Config.GAME_WIDTH - self.r,
                                 self.x + self.v.x))
        self.y = max(self.r, min(Config.GAME_HEIGHT - self.r,
                                 self.y + self.v.y))

    def apply_viscosity(self):
        if not self.is_fast:
            return
        speed = self.speed()
        max_speed = self.max_speed()
        if speed > max_speed:
            speed = max(max_speed, speed - Config.VISCOSITY)
        if speed <= max_speed:
            self.is_fast = False
            speed = max_speed
        self.v = self.v.with_length(speed)

    def can_shrink(self):
        return self.m > Config.MIN_SHRINK_MASS

    def shrink(self):
        if not self.can_shrink():
            return
        self.m -= (self.m - Config.MIN_SHRINK_MASS) * Config.SHRINK_FACTOR
        self.update_r()


class Food(Blob):
    def __init__(self, id, x, y):
        super().__init__(id, x, y, r=Config.FOOD_RADIUS, m=Config.FOOD_MASS)


class Ejection(Food):
    def __init__(self, id, x, y):
        super().__init__(
            id, x, y, r=Config.EJECTION_RADIUS, m=Config.EJECTION_MASS)


class Virus(Blob):
    def __init__(self, id, x, y, m):
        super().__init__(id, x, y, r=Config.VIRUS_RADIUS, m=m)

    def can_hurt(self, other):
        if other.r < self.r or not other.can_burst():
            return False
        max_dist = self.r * Config.RAD_HURT_FACTOR + other.r
        return self.qdist(other) < max_dist**2


class Command(Point):
    def __init__(self,
                 x,
                 y,
                 split=False,
                 pause=False,
                 debug_messages=None,
                 debug_lines=None,
                 debug_circles=None):
        x = max(0, min(Config.GAME_WIDTH, x))
        y = max(0, min(Config.GAME_HEIGHT, y))
        super().__init__(x, y)
        self.split = split
        self.pause = pause
        self.debug_messages = debug_messages or []
        self.debug_lines = debug_lines or []
        self.debug_circles = debug_circles or []

    @staticmethod
    def go_to(target, **kwargs):
        return Command(target.x, target.y, **kwargs)

    def add_debug_message(self, debug_message):
        self.debug_messages.append(debug_message)

    def add_debug_line(self, points, color=None, alpha=None):
        self.debug_lines.append((points, color, alpha))

    def add_debug_circle(self, circle, color=None, alpha=None):
        self.debug_circles.append((circle, color, alpha))


class Node:
    def __init__(self, state, parent=None, commands=None, children=None):
        self.state = state
        self.parent = parent
        self.commands = commands
        self.children = children or []
        self.score = 0
        self.subtree_score_sum = 0
        self.subtree_size = 1
        self.expandable = True

    def compute_tip_score(self):
        self.score = max(
            0, sum(self.compute_blob_score(me) for me in self.state.my_blobs))
        self.subtree_score_sum = self.score

    def compute_blob_score(self, me):
        score = me.m
        score += me.speed() * SPEED_REWARD_FACTOR

        SAFETY_MARGIN = me.r * SAFETY_MARGIN_FACTOR
        if me.x < SAFETY_MARGIN or me.x > Config.GAME_WIDTH - SAFETY_MARGIN:
            score += SAFETY_MARGIN_PENALTY
        if me.y < SAFETY_MARGIN or me.y > Config.GAME_HEIGHT - SAFETY_MARGIN:
            score += SAFETY_MARGIN_PENALTY

        score = max(0, score)
        return score

    def subtree_score(self):
        return self.subtree_score_sum / self.subtree_size


class State:
    def __init__(self, tick, my_blobs, eaten=None):
        self.tick = tick
        self.my_blobs = my_blobs
        self.eaten = eaten or set()

    def me(self):
        try:
            return self.my_blobs[0]
        except IndexError:
            return None


class Strategy:
    def __init__(self, logger, debug):
        self.logger = logger
        self.debug = debug
        self.root = None
        self.tips = {}
        self.commands = collections.deque([])

    def tick(self, tick, data):
        if self.debug:
            self.debug_messages = []

        my_blobs, food, ejections, viruses, enemies = data
        my_blobs.sort(key=lambda me: (me.m, me.is_fast), reverse=True)
        me = my_blobs[0]
        self.food = food
        self.ejections = ejections
        self.viruses = viruses
        self.enemies = enemies

        if (self.root is None or self.root.state.me() is None
                or (not self.commands
                    and self.root.state.me().qdist(me) > ROOT_EPS**2)):
            if (self.debug and self.root is not None
                    and self.root.state.me() is not None):
                self.debug_messages.append('RESET')
                self.debug_messages.append('dist = {:.2f}'.format(
                    self.root.state.me().dist(me)))
            self.root = self.new_tip(State(tick, my_blobs))
            self.tips = {}

        skips = max(1, int(SKIP_DISTANCE / me.max_speed()))
        for _ in range(EXPANSIONS_PER_TICK):
            self.expand_child(self.root, skips)

        if not self.commands:
            tip = max(self.tips.values(), key=lambda node: node.score)
            self.advance_root(self.get_next_root(tip))
            for command in self.root.commands:
                self.commands.append(
                    Command(x=command.x, y=command.y, split=command.split))
            self.debug_tip = tip
            self.add_expandable_nodes(skips)

        command = self.commands.popleft()
        if self.debug:

            def go(node):
                for child in node.children:
                    if True or child.children:
                        for n, c in zip(node.state.my_blobs,
                                        child.state.my_blobs):
                            command.add_debug_line([n, c], 'black', 0.3)
                    go(child)

            go(self.root)

            for tip in self.tips.values():
                for me in tip.state.my_blobs:
                    command.add_debug_circle(
                        Circle(me.x, me.y, 1), 'black', 0.3)

            for me in self.root.state.my_blobs:
                command.add_debug_circle(
                    Circle(me.x, me.y, me.r), 'green', 0.1)
            for me in self.debug_tip.state.my_blobs:
                command.add_debug_circle(Circle(me.x, me.y, 2), 'red')
            node = self.debug_tip
            while node.parent is not None:
                for n, p in zip(node.state.my_blobs,
                                node.parent.state.my_blobs):
                    command.add_debug_line([n, p], 'black')
                node = node.parent

            for food in food:
                command.add_debug_circle(
                    Circle(food.x, food.y, food.r + 2), 'green', 0.5)
            for danger in viruses + enemies:
                command.add_debug_circle(
                    Circle(danger.x, danger.y, danger.r + 2), 'red', 0.1)

            command.add_debug_message('skips: {}'.format(skips))
            command.add_debug_message('queue: {}'.format(len(self.commands)))
            command.add_debug_message('tips: {}'.format(len(self.tips)))
            command.add_debug_message('tree: {}'.format(
                self.root.subtree_size))
            command.add_debug_message('avg: {:.2f}'.format(
                self.root.subtree_score()))

            if command.split:
                command.add_debug_message('SPLIT')

            for message in self.debug_messages:
                command.add_debug_message(message)
        return command

    def expand_child(self, node, skips):
        while not node.expandable:
            node = self.select_child(node)
            if node is None:
                return
        self.expand_node(node, skips)

    def select_child(self, node):
        if not node.children:
            return None
        p = np.array([child.subtree_score() for child in node.children])
        p_sum = np.sum(p)
        if p_sum == 0:
            p[0] = 1.0
        else:
            p /= p_sum
        i = np.random.choice(np.arange(len(node.children)), p=p)
        return node.children[i]

    def expand_node(self, node, skips):
        self.remove_tip(node)
        node.expandable = False
        delta_score_sum = 0
        delta_size = 0
        for angle in EXPAND_ANGLES:
            me = node.state.me()
            if me is None:
                continue
            v = Point.from_polar(Config.SPEED_FACTOR, me.angle() + angle)
            command = Command.go_to(me + v)
            commands = [command] * skips
            child = self.new_tip(
                state=self.predict_states(node.state, commands),
                parent=node,
                commands=commands)
            node.children.append(child)
            delta_score_sum += child.score
            delta_size += 1
        while node is not None:
            node.subtree_score_sum += delta_score_sum
            node.subtree_size += delta_size
            node = node.parent

    def add_expandable_nodes(self, skips):
        for me in self.root.state.my_blobs[:MAX_POWER_BLOBS]:
            for angle in DISCOVERY_ANGLES:
                split = False
                v = Point.from_polar(Config.SPEED_FACTOR, me.angle() + angle)
                node = self.root
                depth = 0
                while (node.state.me() is not None
                       and me.can_see(node.state.me()) and
                       (node.parent is None or node.parent.state.me().qdist(
                           node.state.me()) > ROOT_EPS**2)):
                    commands = [
                        Command.go_to(
                            node.state.me() + v,
                            split=split and depth == 0 and i == 0)
                        for i in range(skips)
                    ]
                    child = self.new_tip(
                        state=self.predict_states(node.state, commands),
                        parent=node,
                        commands=commands)
                    node.children.append(
                        child)  # last_node is still expandable.
                    self.remove_tip(node)
                    depth += 1
                    if depth < MIN_EXPANSION_DEPTH:
                        child.expandable = False
                    node = child
                node.expandable = True

    def get_next_root(self, tip):
        node = tip
        while node.parent is not None and node.parent.parent is not None:
            node = node.parent
        return node

    def advance_root(self, next_root):
        not_roots = [
            node for node in self.root.children if node is not next_root
        ]
        for tip in find_tips(not_roots):
            self.remove_tip(tip)
        self.root = next_root
        self.root.parent = None

    def new_tip(self, *args, **kwargs):
        tip = Node(*args, **kwargs)
        tip.compute_tip_score()
        self.tips[id(tip)] = tip
        return tip

    def remove_tip(self, tip):
        try:
            del self.tips[id(tip)]
        except KeyError:
            pass

    def predict_states(self, state, commands):
        for command in commands:
            state = self.predict_state(state, command)
        return state

    def predict_state(self, state, command):
        # TODO: Fix precision error when eating food at max speed.
        my_blobs = [copy.copy(me) for me in state.my_blobs]

        # Following the oringial mechanic.
        # apply_strategies: update v.
        for me in my_blobs:
            if not me.is_fast:
                me.update_v(command)

        # shrink_players.
        tick = state.tick + 1
        if tick % Config.SHRINK_EVERY_TICK == 0:
            for me in my_blobs:
                me.shrink()

        # who_is_eaten: update m.
        eaten = state.eaten.copy()
        for food in self.food + self.ejections + self.enemies:
            if food.id in eaten:
                continue
            i, me = find_nearest_me(food, lambda me: me.can_eat(food),
                                    my_blobs)
            if me is not None:
                eaten.add(food.id)
                me.m += food.m
        for enemy in self.enemies:
            if enemy.id in eaten:
                continue
            i, me = find_nearest_me(enemy, lambda me: enemy.can_eat(me),
                                    my_blobs)
            if me is not None:
                del my_blobs[i]  # Die.

        # TODO: who_need_fusion.

        # who_intersected_virus.
        for virus in self.viruses:
            i, me = find_nearest_me(virus, lambda me: me.can_burst(), my_blobs)
            if me is not None:
                my_blobs[i:i + 1] = self.burst(me, virus)

        # update_by_state: update r, limit v, split.
        for me in my_blobs:
            me.update_r()
            me.limit_speed()
        if command.split:
            my_blobs = [new_me for me in my_blobs for new_me in self.split(me)]

        # move_moveables: collide (TODO), move, apply viscosity, update ttf.
        for me in my_blobs:
            me.apply_v()
            me.apply_viscosity()
            if me.ttf > 0:
                me.ttf -= 1

        my_blobs.sort(key=lambda me: (me.m, me.is_fast), reverse=True)
        return State(tick, my_blobs, eaten)

    def split(self, me):
        if not me.can_split():
            return [me]
        m = me.m / 2
        v = Point.from_polar(Config.SPLIT_START_SPEED, me.angle())
        return [
            Me(
                id=me.id + '+1',  # TODO: Compute correct ids.
                x=me.x,
                y=me.y,
                m=m,
                v=v,
                is_fast=True,
                ttf=Config.TICKS_TIL_FUSION),
            Me(id=me.id + '+2',
               x=me.x,
               y=me.y,
               m=m,
               v=me.v,
               is_fast=me.is_fast,
               ttf=Config.TICKS_TIL_FUSION)
        ]

    def burst(self, me, virus):
        if virus.can_hurt(me):
            # Assume we die. TODO.
            return []
        return [me]

    def debug_prediction_error(self, tick, my_blobs, command):
        if hasattr(self, 'next_blobs'):
            for me, next_me in zip(my_blobs, self.next_blobs):
                e = next_me.dist(me)
                command.add_debug_message(
                    'prediction error: {:.8f} {} {}'.format(
                        e, next_me.is_fast, me.is_fast))
                command.add_debug_message('me.m = {:.8f}'.format(me.m))
                command.add_debug_message('ne.m = {:.8f}'.format(next_me.m))
                command.add_debug_message('me.speed = {:.8f}'.format(
                    me.speed()))
                command.add_debug_message('ne.speed = {:.8f}'.format(
                    next_me.speed()))
                command.add_debug_message('me.max_speed = {:.8f}'.format(
                    me.max_speed()))
                command.add_debug_message('ne.max_speed = {:.8f}'.format(
                    next_me.max_speed()))
                if e > 1e-6 and len(my_blobs) > 1:
                    command.pause = True
                    command.add_debug_message('cmd: {!r}'.format(
                        self.last_command))
        self.next_blobs = self.predict_state(State(tick, my_blobs),
                                             command).my_blobs
        for nb in self.next_blobs:
            command.add_debug_circle(Circle(nb.x, nb.y, nb.r), 'cyan', 0.5)
        self.last_command = command


def find_nearest_me(target, predicate, my_blobs):
    return min(
        ((i, me) for i, me in enumerate(my_blobs) if predicate(me)),
        key=lambda i_me: i_me[1].qdist(target),
        default=(None, None))


def find_tips(roots):
    def go(node, tips):
        if node.children:
            for child in node.children:
                go(child, tips)
        else:
            tips.append(node)

    tips = []
    for root in roots:
        go(root, tips)
    return tips


class TimingStrategy:
    AVG_TICK_TIME_SECS = 150 / 7500

    def __init__(self, strategy):
        self.strategy = strategy

    def tick(self, *args, **kwargs):
        start = time.time()
        command = self.strategy.tick(*args, **kwargs)
        elapsed = time.time() - start
        if elapsed > self.AVG_TICK_TIME_SECS:
            command.add_debug_message('SLOW: {:.2f}s'.format(elapsed))
        return command


class Interactor:
    def __init__(self, strategy, logger, debug):
        self.strategy = strategy
        self.logger = logger
        self.debug = debug

    def run(self):
        self.logger.debug('hello')
        self.read_config()
        tick = 0
        while True:
            data = self.read_tick_data()
            if not data:
                break
            command = self.strategy.tick(tick, data)
            self.print_command(command)
            tick += 1

    def read_config(self):
        config = self.read_json()
        for key, value in config.items():
            setattr(Config, key, value)

    def read_tick_data(self):
        try:
            data = self.read_json()
        except EOFError:
            return False
        return self.parse_tick_data(data)

    def read_json(self):
        return json.loads(input())

    def print_command(self, command):
        output = dict(
            X=command.x,
            Y=command.y,
            Split=command.split,
            Debug='; '.join(command.debug_messages))
        if self.debug:
            output['Pause'] = command.pause
            output['Draw'] = dict(
                Lines=[
                    dict(
                        P=[dict(X=p.x, Y=p.y) for p in points],
                        C=color,
                        A=alpha)
                    for points, color, alpha in command.debug_lines
                ],
                Circles=[
                    dict(X=c.x, Y=c.y, R=c.r, C=color, A=alpha)
                    for c, color, alpha in command.debug_circles
                ])
        print(json.dumps(output))

    def parse_tick_data(self, data):
        my_blobs = [
            Me(id=blob.get('Id'),
               x=blob.get('X'),
               y=blob.get('Y'),
               r=blob.get('R'),
               m=blob.get('M'),
               v=Point(blob.get('SX'), blob.get('SY')),
               ttf=blob.get('TTF')) for blob in data.get('Mine', [])
        ]
        food = []
        ejections = []
        viruses = []
        enemies = []
        for obj in data.get('Objects', []):
            t = obj.get('T')
            x = obj.get('X')
            y = obj.get('Y')
            if t == 'F':
                food.append(Food(id='F{:.1f}{:.1f}'.format(x, y), x=x, y=y))
            elif t == 'E':
                ejections.append(
                    Ejection(id='E{:.1f}{:.1f}'.format(x, y), x=x, y=y))
            elif t == 'V':
                viruses.append(
                    Virus(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M')))
            elif t == 'P':
                # TODO: Not always true.
                enemies.append(
                    Enemy(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M'),
                        r=obj.get('R')))
            else:
                raise Exception('unknown object type')
        return (my_blobs, food, ejections, viruses, enemies)


def get_logger(debug):
    logger = logging.getLogger('Strategy')
    logger.setLevel(logging.DEBUG)
    formatter = logging.Formatter(
        '%(asctime)s %(name)s %(levelname)-8s %(message)s')

    ch = logging.StreamHandler()
    ch.setLevel(logging.WARN)
    ch.setFormatter(formatter)
    logger.addHandler(ch)

    if debug:
        fh = logging.FileHandler('/tmp/log.txt', 'w')
        fh.setLevel(logging.DEBUG)
        fh.setFormatter(formatter)
        logger.addHandler(fh)

    return logger


def main():
    debug = bool(os.getenv('DEBUG_STRATEGY'))
    logger = get_logger(debug)
    strategy = Strategy(logger, debug)
    if debug:
        strategy = TimingStrategy(strategy)
    interactor = Interactor(strategy, logger, debug)
    interactor.run()


if __name__ == '__main__':
    main()
