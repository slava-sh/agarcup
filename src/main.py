import json
import logging
import math
import os
import random
import numpy as np
import collections
import time

SKIPPER_INTERVAL = 50
BIG_V = 100
NUM_DIRECTIONS = 4 * 5
NUM_EXPANSIONS = 30
ROOT_EPS = 1
SKIPS = 10
SAFETY_MARGIN_FACTOR = 2.5
SAFETY_MARGIN_PENALTY = -5
AVG_TICK_TIME_SECS = 150 / 7500 * SKIPS
DEBUG_TAIL_SIZE = 50


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def distance_to(self, other):
        return math.hypot(self.x - other.x, self.y - other.y)

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
    def __init__(self, id, x, y, r, m, vision_radius, config):
        super().__init__(x, y, r, m)
        self.config = config
        self.id = id
        self.vision_radius = vision_radius

    def can_eat(self, other):
        return self.m > other.m * self.config.MASS_EAT_FACTOR and self.distance_to(
            other) < self.r - other.r * self.config.RAD_EAT_FACTOR

    def can_see(self, other):
        angle = self.v.angle()
        x = self.x + math.cos(angle) * self.config.VIS_SHIFT
        y = self.y + math.sin(angle) * self.config.VIS_SHIFT
        dist = other.distance_to(Point(x, y))
        return dist < self.vision_radius + other.r


class Enemy(Player):
    pass


class Me(Player):
    def __init__(self, id, x, y, r, m, v, vision_radius, config, ttf=None):
        super().__init__(id, x, y, r, m, vision_radius, config)
        self.v = v
        self.ttf = ttf


class Food(Blob):
    def __init__(self, x, y, config):
        super().__init__(x, y, r=config.FOOD_RADIUS, m=config.FOOD_MASS)


class Ejection(Food):
    def __init__(self, x, y, config):
        super().__init__(
            x, y, r=config.EJECTION_RADIUS, m=config.EJECTION_MASS)


class Virus(Blob):
    def __init__(self, id, x, y, m, config):
        super().__init__(x, y, r=config.VIRUS_RADIUS, m=m)
        self.id = id


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

    def add_debug_message(self, debug_message):
        self.debug_message = debug_message
        return self

    def add_debug_line(self, points, color=None):
        self.debug_lines.append((points, color))
        return self

    def add_debug_circle(self, circle, color=None):
        self.debug_circles.append((circle, color))
        return self


class GoTo(Command):
    def __init__(self, point, debug_message=None):
        super().__init__(point.x, point.y, debug_message)


class Skipper:
    def __init__(self, config, interval=SKIPPER_INTERVAL):
        self.config = config
        self.tick = 0
        self.interval = interval

    def skip(self, debug_message=None):
        if self.tick % self.interval == 0:
            self.target = Point(
                random.randint(1, self.config.GAME_WIDTH - 1),
                random.randint(1, self.config.GAME_HEIGHT - 1))
        self.tick += 1
        return GoTo(self.target, debug_message)


class Config:
    def __init__(self, config):
        self.GAME_WIDTH = config['GAME_WIDTH']
        self.GAME_HEIGHT = config['GAME_HEIGHT']
        self.FOOD_RADIUS = config.get('FOOD_RADIUS', 2.5)
        self.FOOD_MASS = config['FOOD_MASS']
        self.EJECTION_RADIUS = config.get('EJECTION_RADIUS', 4.0)
        self.EJECTION_MASS = config.get('EJECTION_MASS', 15.0)
        self.VIRUS_RADIUS = config['VIRUS_RADIUS']
        self.SPEED_FACTOR = config['SPEED_FACTOR']
        self.INERTION_FACTOR = config['INERTION_FACTOR']
        self.MASS_EAT_FACTOR = config.get('MASS_EAT_FACTOR', 1.2)
        self.RAD_EAT_FACTOR = config.get('RAD_EAT_FACTOR', 0.66)
        self.VIS_FACTOR = config.get('VIS_FACTOR', 4.0)
        self.VIS_FACTOR_FR = config.get('VIS_FACTOR_FR', 2.5)
        self.VIS_SHIFT = config.get('VIS_SHIFT', 10.0)


class PathTree:
    def __init__(self, state, config, v=None, parent=None, children=None):
        self.state = state
        self.v = v
        self.parent = parent
        self.children = children or []
        self.visits = 0

        me = state.me
        self.score = me.m + me.v.length()

        SAFETY_MARGIN = me.r * SAFETY_MARGIN_FACTOR
        if me.x < SAFETY_MARGIN or me.x > config.GAME_WIDTH - SAFETY_MARGIN:
            self.score += SAFETY_MARGIN_PENALTY
        if me.y < SAFETY_MARGIN or me.y > config.GAME_HEIGHT - SAFETY_MARGIN:
            self.score += SAFETY_MARGIN_PENALTY

    def __repr__(self):
        return '{}{!r}'.format(id(self), self.state.me)


class State:
    def __init__(self, me, foods):
        self.me = me
        self.foods = foods


class Planner:
    def __init__(self, config, logger):
        self.config = config
        self.logger = logger
        self.vs = [
            Point(math.cos(angle), math.sin(angle)) * BIG_V
            for angle in np.linspace(0, math.pi * 2, NUM_DIRECTIONS + 1)[:-1]
        ]
        self.root = None
        self.skipper = Skipper(config)

    def plan(self, me, foods):
        self.skipper.skip()

        self.tips = {}
        self.root = self.new_tip(State(me, foods))
        for _ in range(NUM_EXPANSIONS):
            node = self.select_node(me)
            if not node:
                break
            self.expand(node)

        tip = max(self.tips.values(), key=lambda node: node.score)
        next_root = self.get_next_root(tip)
        return next_root.v, tip

    def get_next_root(self, tip):
        node = tip
        while node.parent is not self.root:
            node = node.parent
        return node

    def select_node(self, me):
        score = lambda node: node.score
        return max(
            (tip for tip in self.tips.values() if me.can_see(tip.state.me)),
            key=score,
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
        tip = PathTree(*args, **kwargs, config=self.config)
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
        node.children = [
            self.new_tip(
                self.predict_moves(node.state, [v] * SKIPS), v=v, parent=node)
            for v in self.vs
        ]
        self.remove_tip(node)
        return node.children

    def predict_moves(self, state, vs):
        for v in vs:
            state = self.predict_move(state, v)
        return state

    def predict_move(self, state, v):
        me = state.me
        foods = state.foods

        new_m = me.m
        new_foods = []
        for food in foods:
            if me.can_eat(food):
                new_m += food.m
            else:
                new_foods.append(food)

        max_speed = self.config.SPEED_FACTOR / math.sqrt(new_m)
        new_v = me.v + (v.unit() * max_speed - me.v) * (
            self.config.INERTION_FACTOR / new_m)
        new_v = new_v.with_length(min(max_speed, new_v.length()))
        new_pos = me + new_v
        new_pos.x = max(me.r, min(self.config.GAME_WIDTH - me.r, new_pos.x))
        new_pos.y = max(me.r, min(self.config.GAME_HEIGHT - me.r, new_pos.y))
        return State(
            Me(id=me.id,
               x=new_pos.x,
               y=new_pos.y,
               r=me.r,
               m=new_m,
               v=new_v,
               vision_radius=me.vision_radius,
               config=self.config), new_foods)


class Strategy:
    def __init__(self, logger):
        self.logger = logger
        self.tail = collections.deque([], DEBUG_TAIL_SIZE)
        self.tick = None

    def on_tick(self):
        start = time.time()

        if self.tick is None:
            self.tick = 0
        else:
            self.tick += 1

        if self.tick % SKIPS == 0:
            command = self.get_command()
            self.last_command = command
        else:
            command = self.last_command

        elapsed = time.time() - start
        if elapsed > AVG_TICK_TIME_SECS:
            command.add_debug_message('SLOW: {:.2f}s'.format(elapsed))
        return command

    def get_command(self):
        # Find my biggest blob.
        self.my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = self.my_blobs[0]

        foods = self.food + self.enemies

        v, tip = self.planner.plan(me, foods)
        command = GoTo(me + v)

        self.tail.append(me)
        command.add_debug_line(self.tail, 'gray')

        def dfs(node):
            for child in node.children:
                command.add_debug_line([node.state.me, child.state.me])
                dfs(child)
        dfs(self.planner.root)

        command.add_debug_message('t={}'.format(len(self.planner.tips)))

        t = tip.state.me
        command.add_debug_circle(Circle(t.x, t.y, 2), 'red')

        return command

    def run(self):
        self.logger.debug('hello')
        self.config = Config(self.read_json())
        self.planner = Planner(self.config, self.logger)
        while True:
            try:
                data = self.read_json()
            except EOFError:
                break
            self.parse_blobs(data)
            command = self.on_tick()
            print(
                json.dumps(
                    dict(
                        X=command.x,
                        Y=command.y,
                        Debug=command.debug_message,
                        Draw=dict(
                            Lines=[
                                dict(
                                    P=[dict(X=p.x, Y=p.y) for p in points],
                                    C=color)
                                for points, color in command.debug_lines
                            ],
                            Circles=[
                                dict(X=c.x, Y=c.y, R=c.r, C=color)
                                for c, color in command.debug_circles
                            ]))))

    def parse_blobs(self, data):
        self.my_blobs = [
            Me(id=blob.get('Id'),
               x=blob.get('X'),
               y=blob.get('Y'),
               r=blob.get('R'),
               m=blob.get('M'),
               v=Point(blob.get('SX'), blob.get('SY')),
               ttf=blob.get('TTF'),
               vision_radius=blob.get('R') * self.config.VIS_FACTOR, # TODO: Not always true.
               config=self.config) for blob in data.get('Mine', [])
        ]
        self.food = []
        self.viruses = []
        self.enemies = []
        for obj in data.get('Objects', []):
            t = obj.get('T')
            if t == 'F':
                self.food.append(Food(obj.get('X'), obj.get('Y'), self.config))
            elif t == 'E':
                self.food.append(
                    Ejection(obj.get('X'), obj.get('Y'), self.config))
            elif t == 'V':
                self.viruses.append(
                    Virus(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M'),
                        config=self.config))
            elif t == 'P':
                # TODO: Not always true.
                vision_radius = obj.get('R') * self.config.VIS_FACTOR
                self.enemies.append(
                    Enemy(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M'),
                        r=obj.get('R'),
                        vision_radius=vision_radius,
                        config=self.config))
            else:
                raise ValueError('unknown object type')

    def read_json(self):
        return json.loads(input())


def get_logger():
    logger = logging.getLogger('Strategy')
    logger.setLevel(logging.DEBUG)
    formatter = logging.Formatter(
        '%(asctime)s %(name)s %(levelname)-8s %(message)s')

    ch = logging.StreamHandler()
    ch.setLevel(logging.WARN)
    ch.setFormatter(formatter)
    logger.addHandler(ch)

    if os.getenv('DEBUG_STRATEGY'):
        fh = logging.FileHandler('/tmp/log.txt', 'w')
        fh.setLevel(logging.DEBUG)
        fh.setFormatter(formatter)
        logger.addHandler(fh)

    return logger


if __name__ == '__main__':
    Strategy(logger=get_logger()).run()
