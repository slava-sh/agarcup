import json
import logging
import math
import os
import random
import numpy as np
import collections
import time

MAX_PROMISING_EXPANSIONS = 10
BIG_SPEED = 1000
NUM_DIRECTIONS = 4 * 2
SAFETY_MARGIN_FACTOR = 2.5
SAFETY_MARGIN_PENALTY = -3
MIN_SKIPS = 5
MIN_SKIPS_MASS = 40
MAX_SKIPS = 50
MAX_SKIPS_MASS = 500
DANGER_PENALTY = -1000


class Config:
    GAME_WIDTH = None
    GAME_HEIGHT = None
    VIRUS_RADIUS = None
    SPEED_FACTOR = None
    INERTION_FACTOR = None
    FOOD_MASS = None
    FOOD_RADIUS = 2.5
    EJECTION_RADIUS = 4.0
    EJECTION_MASS = 15.0
    MASS_EAT_FACTOR = 1.2
    DIAM_EAT_FACTOR = 2 / 3
    VIS_FACTOR = 4.0
    VIS_FACTOR_FR = 2.5
    VIS_SHIFT = 10.0
    RAD_HURT_FACTOR = 0.66


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    @staticmethod
    def from_polar(r, angle):
        x = r * math.cos(angle)
        y = r * math.sin(angle)
        return Point(x, y)

    def distance_to(self, other):
        return math.hypot(self.x - other.x, self.y - other.y)

    def qdistance_to(self, other):
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
    def __init__(self, x, y, r, m):
        super().__init__(x, y, r)
        self.m = m


class Player(Blob):
    def __init__(self, id, x, y, r, m):
        super().__init__(x, y, r, m)
        self.id = id

    def can_eat(self, other):
        if not (self.m > other.m * Config.MASS_EAT_FACTOR):
            return False
        max_dist = self.r + other.r - other.r * 2 * Config.DIAM_EAT_FACTOR
        return self.qdistance_to(other) < max_dist**2

    def can_see(self, other):
        angle = self.v.angle()
        x = self.x + math.cos(angle) * Config.VIS_SHIFT
        y = self.y + math.sin(angle) * Config.VIS_SHIFT
        vision_radius = self.r * Config.VIS_FACTOR  # TODO: Not always true.
        max_dist = vision_radius + other.r
        return other.qdistance_to(Point(x, y)) < max_dist**2

    def can_burst(self):
        # TODO
        return True

    def can_hurt(self, other):
        return self.can_eat(other)


class Enemy(Player):
    pass


class Me(Player):
    def __init__(self, id, x, y, r, m, v, ttf=None):
        super().__init__(id, x, y, r, m)
        self.v = v
        self.ttf = ttf


class Food(Blob):
    def __init__(self, x, y):
        super().__init__(x, y, r=Config.FOOD_RADIUS, m=Config.FOOD_MASS)


class Ejection(Food):
    def __init__(self, x, y):
        super().__init__(
            x, y, r=Config.EJECTION_RADIUS, m=Config.EJECTION_MASS)


class Virus(Blob):
    def __init__(self, id, x, y, m):
        super().__init__(x, y, r=Config.VIRUS_RADIUS, m=m)
        self.id = id

    def can_hurt(self, other):
        if other.r < self.r or not other.can_burst():
            return False
        return self.qdistance_to(other) < (
            self.r * Config.RAD_HURT_FACTOR + other.r)**2


class Command(Point):
    def __init__(self,
                 x,
                 y,
                 debug_message=None,
                 debug_lines=None,
                 debug_circles=None):
        self.x = x
        self.y = y
        self.debug_message = debug_message
        self.debug_lines = debug_lines or []
        self.debug_circles = debug_circles or []
        self.debug = bool(os.getenv('DEBUG_STRATEGY'))

    def add_debug_message(self, debug_message):
        self.debug_message = debug_message
        return self

    def add_debug_line(self, points, color=None):
        if self.debug:
            self.debug_lines.append((points, color))
        return self

    def add_debug_circle(self, circle, color=None):
        if self.debug:
            self.debug_circles.append((circle, color))
        return self


class GoTo(Command):
    def __init__(self, point, debug_message=None):
        super().__init__(point.x, point.y, debug_message)


class Node:
    def __init__(self,
                 state,
                 root=None,
                 command=None,
                 parent=None,
                 children=None):
        self.state = state
        self.command = command
        self.parent = parent
        self.children = children or []

        me = state.me
        self.score = me.m

        self.score += math.sqrt(me.v.length())

        for danger in state.dangers:
            if danger.can_hurt(me):
                self.score += DANGER_PENALTY

        SAFETY_MARGIN = me.r * SAFETY_MARGIN_FACTOR
        if me.x < SAFETY_MARGIN or me.x > Config.GAME_WIDTH - SAFETY_MARGIN:
            self.score += SAFETY_MARGIN_PENALTY
        if me.y < SAFETY_MARGIN or me.y > Config.GAME_HEIGHT - SAFETY_MARGIN:
            self.score += SAFETY_MARGIN_PENALTY

    def __repr__(self):
        return '{}{!r}'.format(id(self), self.state.me)


class State:
    def __init__(self, me, foods, dangers):
        self.me = me
        self.foods = foods
        self.dangers = dangers


class Strategy:
    def __init__(self, logger, debug):
        self.logger = logger
        self.debug = debug
        self.angles = [
            angle
            for angle in np.linspace(0, math.pi * 2, NUM_DIRECTIONS + 1)[:-1]
        ]
        self.root = None
        self.skips = MIN_SKIPS

    def tick(self, tick, data):
        my_blobs, food, viruses, enemies = data
        my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = my_blobs[0]
        foods = food + enemies
        dangers = viruses + enemies

        self.skips = int(
            max(MIN_SKIPS,
                min(MAX_SKIPS, MIN_SKIPS + (me.m - MIN_SKIPS_MASS) *
                    (MAX_SKIPS - MIN_SKIPS) /
                    (MAX_SKIPS_MASS - MIN_SKIPS_MASS))))

        self.tips = {}
        self.root = self.new_tip(State(me, foods, dangers))

        seen = set()
        frontier = collections.deque([self.root])
        while frontier:
            node = frontier.popleft()
            for tip in self.expand(node):
                xy = (int(tip.state.me.x / me.r), int(tip.state.me.y / me.r))
                if xy in seen or not me.can_see(tip.state.me) or tip.score < 0:
                    continue
                seen.add(xy)
                frontier.append(tip)

        for _ in range(MAX_PROMISING_EXPANSIONS):
            node = self.select_node(me)
            if not node:
                break
            self.expand(node)

        tip = max(self.tips.values(), key=lambda node: node.score)
        self.next_root = self.get_next_root(tip)
        return self.next_root.command

    def get_next_root(self, tip):
        node = tip
        while node.parent is not self.root:
            node = node.parent
        return node

    def select_node(self, me):
        return max(
            (tip for tip in self.tips.values() if me.can_see(tip.state.me)),
            key=lambda node: node.score,
            default=None)

    def discover_nodes(self, roots):
        def go(node, nodes):
            nodes.append(node)
            for child in node.children:
                go(child, nodes)

        nodes = []
        for root in roots:
            go(root, nodes)
        return nodes

    def new_tip(self, *args, **kwargs):
        tip = Node(*args, **kwargs, root=self.root)
        self.tips[id(tip)] = tip
        return tip

    def remove_tip(self, tip):
        try:
            del self.tips[id(tip)]
        except KeyError:
            pass

    def expand(self, node):
        if node.children:
            raise ValueError('node already expanded')
        node.children = []
        for angle in self.angles:
            me = node.state.me
            v = Point.from_polar(BIG_SPEED, me.v.angle() + angle)
            command = GoTo(me + v)
            node.children.append(
                self.new_tip(
                    self.predict_moves(node.state, [command] * self.skips),
                    command=command,
                    parent=node))
        self.remove_tip(node)
        return node.children

    def predict_moves(self, state, commands):
        for command in commands:
            state = self.predict_move(state, command)
        return state

    def predict_move(self, state, command):
        me = state.me
        if me.id is None:  # Dead.
            return state

        v = Point(command.x, command.y) - me

        for danger in state.dangers:
            if danger.can_hurt(me):
                return State(
                    Me(id=None, x=me.x, y=me.y, r=0, m=0, v=Point(0, 0)), [],
                    [])

        new_m = me.m
        new_foods = []
        for food in state.foods:
            if me.can_eat(food):
                new_m += food.m
            else:
                new_foods.append(food)

        max_speed = Config.SPEED_FACTOR / math.sqrt(new_m)
        new_v = me.v + (v.unit() * max_speed - me.v) * (
            Config.INERTION_FACTOR / new_m)
        new_v = new_v.with_length(min(max_speed, new_v.length()))
        new_pos = me + new_v
        new_pos.x = max(me.r, min(Config.GAME_WIDTH - me.r, new_pos.x))
        new_pos.y = max(me.r, min(Config.GAME_HEIGHT - me.r, new_pos.y))
        return State(
            Me(id=me.id, x=new_pos.x, y=new_pos.y, r=me.r, m=new_m, v=new_v),
            new_foods, state.dangers)


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
        output = dict(X=command.x, Y=command.y, Debug=command.debug_message)
        if self.debug:
            output['Draw'] = dict(
                Lines=[
                    dict(P=[dict(X=p.x, Y=p.y) for p in points], C=color)
                    for points, color in command.debug_lines
                ],
                Circles=[
                    dict(X=c.x, Y=c.y, R=c.r, C=color)
                    for c, color in command.debug_circles
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
        viruses = []
        enemies = []
        for obj in data.get('Objects', []):
            t = obj.get('T')
            if t == 'F':
                food.append(Food(obj.get('X'), obj.get('Y')))
            elif t == 'E':
                food.append(Ejection(obj.get('X'), obj.get('Y')))
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
                raise ValueError('unknown object type')
        return (my_blobs, food, viruses, enemies)


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
