import json
import logging
import math
import os
import random
import numpy as np

NUM_TIPS_TO_LEAVE = 10
SKIPPER_INTERVAL = 100
BIG_V = 100
NUM_DIRECTIONS = 8
ROOT_EPS = 3


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

    def add_debug_line(self, line):
        self.debug_lines.append(line)
        return self

    def add_debug_circle(self, circle):
        self.debug_circles.append(circle)
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

    def __repr__(self):
        me = self.me
        return 'Node(me=({:.2f}, {:.2f}) v=({:.2f}, {:.2f}) children={})'.format(
            me.x, me.y, me.v.x, me.v.y, repr(self.children))


class Planner:
    def __init__(self, config, logger):
        self.config = config
        self.logger = logger
        self.vs = [
            Point(math.cos(angle), math.sin(angle)) * BIG_V
            for angle in np.linspace(0, math.pi * 2, NUM_DIRECTIONS + 1)[:-1]
        ]
        self.roots = []
        self.skipper = Skipper(config)

    def update(self, me):
        self.logger.debug('before update')
        self.skipper.skip()
        if not self.roots:
            self.roots.append(PathTree(me))
        self.logger.debug('before expand')
        for root in self.roots:
            self.expand(root)
        self.logger.debug('before trim')
        self.trim(me)
        self.logger.debug('after update / before get best v')
        return self.get_best_v()

    def expand(self, node):
        if node.children:
            for child in node.children:
                self.expand(child)
        else:
            for v in self.vs:
                new_me = self.predict_move(node.me, v)
                node.children.append(PathTree(new_me, v=v, parent=node))

    def trim(self, me):
        self.nodes = []
        self.tips = []

        def dfs(node):
            self.nodes.append(node)
            node.distance_to_me = me.distance_to(node.me)
            if node.children:
                for child in node.children:
                    dfs(child)
            else:
                self.tips.append(node)

        for root in self.roots:
            dfs(root)

        self.logger.debug('got %d nodes', len(self.nodes))
        self.tips.sort(key=lambda t: t.me.distance_to(self.skipper.target))
        self.tips = self.tips[:NUM_TIPS_TO_LEAVE]
        assert len(self.tips) <= NUM_TIPS_TO_LEAVE
        for node in self.nodes:
            node.should_keep = False
        for tip in self.tips:
            node = tip
            while node is not None:
                node.should_keep = True
                node = node.parent

        self.nodes = [node for node in self.nodes if node.should_keep]
        for node in self.nodes:
            node.children = [
                child for child in node.children if child.should_keep
            ]

        self.roots = [
            node for node in self.nodes if self.node_can_be_root(node)
        ]
        for root in self.roots:
            root.parent = None
        assert len(self.tips) <= NUM_TIPS_TO_LEAVE

    def node_can_be_root(self, node):
        if node.distance_to_me > ROOT_EPS:
            return False
        return node.parent is None or node.distance_to_me < node.parent.distance_to_me

    def get_best_v(self):
        assert len(self.tips) <= NUM_TIPS_TO_LEAVE
        v = None
        node = self.tips[0]
        while node is not None and node.v is not None:
            v = node.v
            me = node.me
            self.logger.debug('best path: %.3f %.3f to %.3f %.3f', v.x, v.y, me.x, me.y)
            node = node.parent
        return v

    def predict_moves(self, me, vs):
        for v in vs:
            me = self.predict_move(me, v)
        return me

    def predict_move(self, me, v):
        max_speed = self.config.SPEED_FACTOR / math.sqrt(me.m)
        new_v = me.v + (v.unit() * max_speed - me.v) * (
            self.config.INERTION_FACTOR / me.m)
        new_v = new_v.with_length(min(max_speed, new_v.length()))
        new_pos = me + new_v
        SAFETY_MARGIN = 8 * me.r
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

    def on_tick(self):
        if not self.my_blobs:
            return self.skipper.skip('Died')

        # Find my biggest blob.
        self.my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = self.my_blobs[0]

        #if not self.food:
        #    command = self.skipper.skip('No food')
        #else:
        #    # Go to the closest food.
        #    food = min(self.food, key=lambda b: b.distance_to(me))
        #    command = GoTo(food, 'EAT')

        v = self.planner.update(me)
        command = GoTo(me + v)

        #for root in self.planner.roots:
        #    self.logger.debug('%s', repr(root))

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

        #for root in self.planner.roots:
        #    dfs(root)
        #command.add_debug_message(
        #    'roots={} nodes={} tips={} t={} n={} lines={} v=({:.2f} {:.2f})'.format(
        #        len(self.planner.roots), len(self.planner.nodes),
        #        len(self.planner.tips), tips, nodes, lines, v.x, v.y))
        #t = self.planner.skipper.target
        #command.add_debug_circle(Circle(t.x, t.y, 4))
        #for tip in self.planner.tips:
        #    t = tip.me
        #    command.add_debug_circle(Circle(t.x, t.y, 2))

        return command

    def run(self):
        self.logger.debug('hello')
        self.config = Config(self.read_json())
        self.skipper = Skipper(self.config)
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
                            Lines=[[dict(X=p.x, Y=p.y) for p in line]
                                   for line in command.debug_lines],
                            Circles=[
                                dict(X=c.x, Y=c.y, R=c.r)
                                for c in command.debug_circles
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
