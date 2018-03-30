import json
import logging
import math
import os
import random


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
        logging.getLogger('Strategy').debug('v %f %f d %f', v.x, v.y, dist)
        pos = self + v
        pos.x = max(self.r, min(self.config.GAME_WIDTH - self.r, pos.x))
        pos.y = max(self.r, min(self.config.GAME_HEIGHT - self.r, pos.y))
        return pos


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
    def __init__(self, x, y, debug=None):
        self.x = x
        self.y = y
        self.debug = debug


class GoTo(Command):
    def __init__(self, point, debug=None):
        super().__init__(point.x, point.y, debug)


class Skipper:
    def __init__(self, config, interval=100):
        self.config = config
        self.tick = 0
        self.interval = interval

    def skip(self, debug=None):
        if self.tick % self.interval == 0:
            self.target = Point(
                random.randint(1, self.config.GAME_WIDTH - 1),
                random.randint(1, self.config.GAME_HEIGHT - 1))
        self.tick += 1
        return GoTo(self.target, debug)


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


class Strategy:
    def __init__(self, logger):
        self.logger = logger

    def on_tick(self):
        if not self.my_blobs:
            return self.skipper.skip('Died')
        if not self.food:
            return self.skipper.skip('No food')

        # Find my biggest blob.
        self.my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = self.my_blobs[0]

        # Go to the closest food.
        food = min(self.food, key=lambda b: b.distance_to(me))
        return GoTo(food, 'EAT')

    def run(self):
        self.expected = Point(0, 0)
        self.logger.debug('hello')
        self.config = Config(self.read_json())
        self.skipper = Skipper(self.config)
        while True:
            try:
                data = self.read_json()
            except EOFError:
                break
            self.parse_blobs(data)
            command = self.on_tick()
            print(
                json.dumps(
                    dict(X=command.x, Y=command.y, Debug=command.debug)))
            if self.my_blobs:
                me = self.my_blobs[0]
                self.logger.debug('pos %d %f %f e %f %f %s %f %f',
                            len(self.my_blobs), me.x, me.y,
                            me.x - self.expected.x, me.y - self.expected.y,
                            command.debug, command.x, command.y)
                self.expected = me.predict_move(command)

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
        return json.loads(self.read_line())

    def read_line(self):
        line = input()
        # For testing with local runner output.
        if line[0] == '"':
            line = json.loads(line)
        return line


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
