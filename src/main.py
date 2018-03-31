import json
import logging
import math
import os
import random
import numpy as np
import collections

SKIPPER_INTERVAL = 50
BIG_V = 100
NUM_DIRECTIONS = 4 * 5
ROOT_EPS = 3
NUM_EXPANSIONS = 2


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
    def __init__(self, id, x, y, r, m):
        super().__init__(x, y, r, m)
        self.id = id


class Enemy(Player):
    pass


class Me(Player):
    def __init__(self, id, x, y, r, m, v, config, ttf=None):
        super().__init__(id, x, y, r, m)
        self.config = config
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


class PathTree:
    def __init__(self, me, v=None, parent=None, children=None):
        self.me = me
        self.v = v
        self.parent = parent
        self.children = children or []

    def score(self, target):
        return -self.me.distance_to(target)

    def __repr__(self):
        return '{}{!r}'.format(id(self), self.me)


class Planner:
    def __init__(self, config, logger):
        self.config = config
        self.logger = logger
        self.vs = [
            Point(math.cos(angle), math.sin(angle)) * BIG_V
            for angle in np.linspace(0, math.pi * 2, NUM_DIRECTIONS + 1)[:-1]
        ]
        self.roots = []
        self.tips = {}
        self.skipper = Skipper(config)

    def update(self, me):
        self.skipper.skip()
        self.update_roots(me)

        for _ in range(NUM_EXPANSIONS):
            node = self.select_node(me)
            self.expand(node)
        tip = max(self.tips.values(), key=lambda node: node.score(self.skipper.target))
        return self.get_v(tip), tip

    def select_node(self, me):
        score = lambda node: node.score(self.skipper.target)
        return max(self.tips.values(), key=score)
        node = max(self.roots, key=score)
        while node.children:
            node = max(node.children, key=score)
        return node

    def update_roots(self, me):
        new_roots = []
        for root in self.roots:
            better_roots = []
            not_roots = []
            for child in root.children:
                if child.me.distance_to(me) < root.me.distance_to(me):
                    child.parent = None
                    better_roots.append(child)
                else:
                    not_roots.append(child)
            if better_roots:
                new_roots.extend(better_roots)
                for node in self.discover_nodes(not_roots):
                    self.remove_tip(node)
            else:
                new_roots.append(root)

        self.roots = []
        not_roots = []
        for root in new_roots:
            if root.me.distance_to(me) < ROOT_EPS:
                self.roots.append(root)
            else:
                not_roots.append(root)
        for node in self.discover_nodes(not_roots):
            self.remove_tip(node)

        if not self.roots:
            self.roots.append(self.new_tip(me))

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
        tip = PathTree(*args, **kwargs)
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
            self.new_tip(self.predict_move(node.me, v), v=v, parent=node)
            for v in self.vs
        ]
        self.remove_tip(node)
        return node.children

    def get_v(self, node):
        v = None
        while node is not None and node.v is not None:
            v = node.v
            node = node.parent
        return v

    def predict_move(self, me, v):
        max_speed = self.config.SPEED_FACTOR / math.sqrt(me.m)
        new_v = me.v + (v.unit() * max_speed - me.v) * (
            self.config.INERTION_FACTOR / me.m)
        new_v = new_v.with_length(min(max_speed, new_v.length()))
        new_pos = me + new_v
        SAFETY_MARGIN = me.r
        new_pos.x = max(SAFETY_MARGIN,
                        min(self.config.GAME_WIDTH - SAFETY_MARGIN, new_pos.x))
        new_pos.y = max(SAFETY_MARGIN,
                        min(self.config.GAME_HEIGHT - SAFETY_MARGIN,
                            new_pos.y))
        return Me(
            id=me.id,
            x=new_pos.x,
            y=new_pos.y,
            r=me.r,
            m=me.m,
            v=new_v,
            config=self.config)


class Strategy:
    def __init__(self, logger):
        self.logger = logger
        TAIL_SIZE = 50
        self.tail = collections.deque([], TAIL_SIZE)

    def on_tick(self):
        if not self.my_blobs:
            assert False
            return GoTo(0, 0, 'DEAD')

        # Find my biggest blob.
        self.my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = self.my_blobs[0]

        v, tip = self.planner.update(me)
        v = v or Point(0, 0)
        command = GoTo(me + v)

        #for root in self.planner.roots:
        #    self.logger.debug('%s', repr(root))

        self.tail.append(me)
        command.add_debug_line(self.tail, 'gray')

        tips = 0
        nodes = 0
        lines = 0
        def dfs(node):
            nonlocal tips, lines, nodes
            nodes += 1
            if node.children:
                for child in node.children:
                    command.add_debug_line([node.me, child.me])
                    lines += 1
                    dfs(child)
            else:
                tips += 1
        for root in self.planner.roots:
            dfs(root)
        command.add_debug_message(
            'roots={} n={} t={} nt={} lines={} v=({:.2f} {:.2f})'.format(
                len(self.planner.roots), nodes, tips, len(self.planner.tips),
                lines, v.x, v.y))

        t = self.planner.skipper.target
        command.add_debug_circle(Circle(t.x, t.y, 4), 'red')

        t = tip.me
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

            if self.my_blobs:
                me = self.my_blobs[0]
                self.logger.debug(
                    '%d me(%.3f %.3f) m=%f r=%f cmd(%.3f %.3f) %s',
                    len(self.my_blobs), me.x, me.y, me.m, me.r, command.x,
                    command.y, command.debug_message)

    def parse_blobs(self, data):
        self.my_blobs = [
            Me(id=blob.get('Id'),
               x=blob.get('X'),
               y=blob.get('Y'),
               r=blob.get('R'),
               m=blob.get('M'),
               v=Point(blob.get('SX'), blob.get('SY')),
               ttf=blob.get('TTF'),
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
                self.enemies.append(
                    Enemy(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M'),
                        r=obj.get('R')))
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
