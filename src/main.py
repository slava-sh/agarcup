import json
import logging
import math
import os
import numpy as np
import time

MAX_EXPANSIONS = 5
BIG_SPEED = 1000
NUM_DIRECTIONS = 4 * 2
SAFETY_MARGIN_FACTOR = 2.5
SAFETY_MARGIN_PENALTY = -3
MIN_SKIPS = 5
MIN_SKIPS_MASS = 40
MAX_SKIPS = 50
MAX_SKIPS_MASS = 500


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
    RADIUS_FACTOR = 2.0
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
        # max_dist = self.r + other.r - other.r * 2 * Config.DIAM_EAT_FACTOR
        # return self.qdist(other) < max_dist**2
        dist = self.dist(other)
        min_r = dist - other.r + other.r * 2 * Config.DIAM_EAT_FACTOR
        return min_r < self.r

    def can_see(self, other):
        angle = self.v.angle()
        x = self.x + math.cos(angle) * Config.VIS_SHIFT
        y = self.y + math.sin(angle) * Config.VIS_SHIFT
        vision_radius = self.r * Config.VIS_FACTOR  # TODO: Not always true.
        max_dist = vision_radius + other.r
        return other.qdist(Point(x, y)) < max_dist**2

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
        return self.qdist(other) < (
            self.r * Config.RAD_HURT_FACTOR + other.r)**2


class Command(Point):
    def __init__(self,
                 x,
                 y,
                 debug_messages=None,
                 debug_lines=None,
                 debug_circles=None):
        super().__init__(x, y)
        self.debug_messages = debug_messages or []
        self.debug_lines = debug_lines or []
        self.debug_circles = debug_circles or []

    @staticmethod
    def go_to(target):
        return Command(target.x, target.y)

    def add_debug_message(self, debug_message):
        self.debug_messages.append(debug_message)

    def add_debug_line(self, points, color=None, alpha=None):
        self.debug_lines.append((points, color, alpha))

    def add_debug_circle(self, circle, color=None, alpha=None):
        self.debug_circles.append((circle, color, alpha))


class Node:
    def __init__(self, state, parent=None, command=None, children=None):
        self.state = state
        self.parent = parent
        self.command = command
        self.children = children or []
        self.score = 0
        self.subtree_score_sum = 0
        self.subtree_size = 0

    def compute_tip_score(self, dangers):
        me = self.state.me
        score = me.m

        #score += math.sqrt(me.v.length()) / 10

        SAFETY_MARGIN = me.r * SAFETY_MARGIN_FACTOR
        if me.x < SAFETY_MARGIN or me.x > Config.GAME_WIDTH - SAFETY_MARGIN:
            score += SAFETY_MARGIN_PENALTY
        if me.y < SAFETY_MARGIN or me.y > Config.GAME_HEIGHT - SAFETY_MARGIN:
            score += SAFETY_MARGIN_PENALTY

        for danger in dangers:
            if danger.id not in self.state.eaten and danger.can_hurt(me):
                score = 0
                break

        score = max(0, score)

        self.score = score
        self.subtree_score_sum = self.score
        self.subtree_size = 1

    def subtree_score(self):
        return self.subtree_score_sum / self.subtree_size

    def __repr__(self):
        return '{}{!r}'.format(id(self), self.state.me)


class State:
    def __init__(self, me, eaten=None):
        self.me = me
        self.eaten = eaten or set()


class Strategy:
    def __init__(self, logger, debug):
        self.logger = logger
        self.debug = debug
        self.angles = [
            angle
            for angle in np.linspace(0, math.pi * 2, NUM_DIRECTIONS + 1)[:-1]
        ]
        self.root = None
        self.next_root = None
        self.skips = MIN_SKIPS

    def tick(self, tick, data):
        my_blobs, food, viruses, enemies = data
        my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = my_blobs[0]
        self.foods = food + enemies
        self.dangers = viruses + enemies

        if self.root is not None and self.next_root is not None:
            distance_to_next_root = self.next_root.state.me.qdist(me)
            distance_to_root = self.root.state.me.qdist(me)
            if distance_to_next_root < distance_to_root:
                self.advance_root()

        if self.root is None or self.root.state.me.qdist(me) > me.r**2:
            self.tips = {}
            self.root = self.new_tip(State(me))

        for _ in range(MAX_EXPANSIONS):
            tip = self.select_tip(self.root)
            self.expand_tip(tip)

        tip = max(self.tips.values(), key=lambda node: node.score)
        self.next_root = self.get_next_root(tip)
        command = Command(self.next_root.command.x, self.next_root.command.y)

        if self.debug:
            tips = list(self.tips.values())
            tips.sort(key=lambda node: node.score)
            min_score = tips[0].score
            max_score = tips[-1].score
            self.logger.debug('tips')
            for node in tips:
                self.logger.debug('tip %d@%r %.6f', id(node), node.state.me,
                                  node.score)
                if max_score == min_score:
                    command.add_debug_circle(
                        Circle(node.state.me.x, node.state.me.y, 1), 'black')
                else:
                    alpha = (node.score - min_score) / (max_score - min_score)
                    command.add_debug_circle(
                        Circle(node.state.me.x, node.state.me.y, 1), 'blue',
                        alpha)
            command.add_debug_circle(
                Circle(tip.state.me.x, tip.state.me.y, 2), 'red')
            command.add_debug_message('t={}'.format(len(self.tips)))
            for food in food:
                command.add_debug_circle(
                    Circle(food.x, food.y, food.r + 2), 'green', 0.5)
            for danger in viruses + enemies:
                command.add_debug_circle(
                    Circle(danger.x, danger.y, danger.r + 2), 'red', 0.1)
        return command

    def select_tip(self, node):
        while node.children:
            p = np.array([child.subtree_score() for child in node.children])
            p /= np.sum(p)
            i = np.random.choice(np.arange(len(node.children)), p=p)
            node = node.children[i]
        return node

    def expand_tip(self, tip):
        if tip.children:
            raise Exception('not a tip')

        tip.children = []
        for angle in self.angles:
            me = tip.state.me
            v = Point.from_polar(BIG_SPEED, me.v.angle() + angle)
            command = Command.go_to(me + v)
            child = self.new_tip(
                state=self.predict_states(tip.state, [command] * self.skips),
                parent=tip,
                command=command)
            tip.children.append(child)
        self.remove_tip(tip)

        delta_score_sum = sum(child.score for child in tip.children)
        delta_size = len(tip.children)
        node = tip
        while node is not None:
            node.subtree_score_sum += delta_score_sum
            node.subtree_size += delta_size
            node = node.parent

    def get_next_root(self, tip):
        node = tip
        while node.parent is not None and node.parent.parent is not None:
            node = node.parent
        return node

    def advance_root(self):
        not_roots = [
            node for node in self.root.children if node is not self.next_root
        ]
        for tip in find_tips(not_roots):
            self.remove_tip(tip)
        self.root = self.next_root
        self.root.parent = None

    def new_tip(self, *args, **kwargs):
        tip = Node(*args, **kwargs)
        tip.compute_tip_score(self.dangers)
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
        me = state.me
        if me.id is None:  # Dead.
            return state

        for danger in self.dangers:
            if danger.id not in state.eaten and danger.can_hurt(me):
                # Assume we die.
                return State(
                    Me(id=None, x=me.x, y=me.y, r=0, m=0, v=Point(0, 0)))

        new_m = me.m
        new_eaten = set()
        for food in self.foods:
            if food.id not in state.eaten and me.can_eat(food):
                new_m += food.m
                new_eaten.add(food.id)
        new_r = Config.RADIUS_FACTOR * new_m

        max_speed = Config.SPEED_FACTOR / math.sqrt(new_m)
        v = Point(command.x, command.y) - me
        new_v = me.v + (v.unit() * max_speed - me.v) * (
            Config.INERTION_FACTOR / new_m)
        new_v = new_v.with_length(min(max_speed, new_v.length()))
        new_pos = me + new_v
        new_pos.x = max(new_r, min(Config.GAME_WIDTH - new_r, new_pos.x))
        new_pos.y = max(new_r, min(Config.GAME_HEIGHT - new_r, new_pos.y))
        return State(
            Me(id=me.id, x=new_pos.x, y=new_pos.y, r=new_r, m=new_m, v=new_v),
            state.eaten.union(new_eaten))


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
            X=command.x, Y=command.y, Debug='; '.join(command.debug_messages))
        if self.debug:
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
        viruses = []
        enemies = []
        for obj in data.get('Objects', []):
            t = obj.get('T')
            x = obj.get('X')
            y = obj.get('Y')
            if t == 'F':
                food.append(Food(id='F{:.1f}{:.1f}'.format(x, y), x=x, y=y))
            elif t == 'E':
                food.append(
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
