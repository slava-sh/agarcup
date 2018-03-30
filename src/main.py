import json
import logging
import math
import os
import random
import numpy as np


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

    def predict_move(self, target):
        #if (is_fast) return
        max_speed = self.config.SPEED_FACTOR / math.sqrt(self.m)
        d = target - self
        dist = self.distance_to(target)
        n = d / dist if dist > 0 else Point(0, 0)
        v = self.v + (n * max_speed - self.v) * (
            self.config.INERTION_FACTOR / self.m)
        v = v.with_length(min(max_speed, v.length()))
        pos = self + v
        pos.x = max(self.r, min(self.config.GAME_WIDTH - self.r, pos.x))
        pos.y = max(self.r, min(self.config.GAME_HEIGHT - self.r, pos.y))
        return pos, v


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
    def __init__(self, x, y, debug_message=None, debug_lines=None):
        self.x = x
        self.y = y
        self.debug_message = debug_message
        self.debug_lines = debug_lines or []

    def add_debug_message(self, debug_message):
        self.debug_message = debug_message
        return self

    def add_debug_line(self, line):
        self.debug_lines.append(line)
        return self


class GoTo(Command):
    def __init__(self, point, debug_message=None):
        super().__init__(point.x, point.y, debug_message)


class Skipper:
    def __init__(self, config, interval=100):
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


class Planner:
    def __init__(self, config, logger, skips=3):
        self.config = config
        self.logger = logger
        self.skips = skips
        self.paths = []
        self.vs = [
            Point(math.cos(angle), math.sin(angle)) * 100
            for angle in np.linspace(0, math.pi * 2, 4 + 1)[:-1]
        ]

    def update(self, me):
        new_paths = [[me]]
        ends = set()
        for path in self.paths:
            if path[0].distance_to(me) > me.r:
                continue
            for v in self.vs:
                new_path = path.copy()
                new_path.append(
                    self.predict_moves(new_path[-1], [v] * self.skips))
                end = (int(new_path[-1].x), int(new_path[-1].y))
                if end not in ends:
                    ends.add(end)
                    new_paths.append(new_path)
        new_paths.sort(key=lambda p: p[-1].distance_to(me), reverse=True)
        self.paths = new_paths[:10]

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
        new_pos.x = max(me.r, min(self.config.GAME_WIDTH - me.r, new_pos.x))
        new_pos.y = max(me.r, min(self.config.GAME_HEIGHT - me.r, new_pos.y))
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

        self.planner.update(me)
        v = self.planner.paths[0][-1] - me
        command = GoTo(me + v * 100)
        for path in self.planner.paths:
            command.add_debug_line(path)

        points = sum([len(line) for line in command.debug_lines])
        command.add_debug_message('{} ({:8.2f} {:8.2f})'.format(points, v.x, v.y))
        return command

    def run(self):
        self.last = None
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
                        Draw=dict(Lines=[[dict(X=p.x, Y=p.y) for p in line]
                                         for line in command.debug_lines]))))


            if self.my_blobs:
                me = self.my_blobs[0]
                ep, ev = me.predict_move(command)
                cur = (me, ep, ev)
                if False and self.last:
                    (pme, pep, pev) = self.last
                    v = me - pme
                    good = math.isclose(me.x, pep.x) and math.isclose(
                        me.y, pep.y)
                    self.logger.debug(
                        '%5r d(%.3f %.3f) %d me(%.3f %.3f) pep(%.3f %.3f) v(%.3f %.3f) pev(%.3f %.3f) %s %f %f',
                        good, me.x - pep.x, me.y - pep.y, len(
                            self.my_blobs), me.x, me.y, pep.x, pep.y, v.x, v.y,
                        pev.x, pev.y, command.debug_message, me.m, me.r)
                self.last = cur

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
        fh = logging.FileHandler('log.txt', 'w')
        fh.setLevel(logging.DEBUG)
        fh.setFormatter(formatter)
        logger.addHandler(fh)

    return logger


if __name__ == '__main__':
    Strategy(logger=get_logger()).run()
