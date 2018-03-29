import json
import math

FOOD_RADIUS = 2.5
FOOD_MASS = 1.0
EJECTION_RADIUS = 4.0
EJECTION_MASS = 15.0
VIRUS_RADIUS = 22.0


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def distance_to(self, other):
        return math.hypot(self.x - other.x, self.y - other.y)


class Circle(Point):
    def __init__(self, x, y, r):
        super().__init__(x, y)
        self.r = r


class Blob(Circle):
    def __init__(self, x, y, r, m):
        super().__init__(x, y, r)
        self.m = m


class PlayerBlob(Blob):
    def __init__(self, id, x, y, r, m):
        super().__init__(x, y, r, m)
        self.id = id


class EnemyBlob(PlayerBlob):
    pass


class MyBlob(PlayerBlob):
    def __init__(self, id, x, y, r, m, v, ttf=None):
        super().__init__(id, x, y, r, m)
        self.v = v
        self.ttf = ttf


class FoodBlob(Blob):
    def __init__(self, x, y):
        super().__init__(x, y, r=FOOD_RADIUS, m=FOOD_MASS)


class EjectionBlob(FoodBlob):
    def __init__(self, x, y):
        super().__init__(x, y, r=EJECTION_RADIUS, m=EJECTION_MASS)


class VirusBlob(Blob):
    def __init__(self, id, x, y, m):
        super().__init__(x, y, r=VIRUS_RADIUS, m=m)
        self.id = id


class Command:
    def __init__(self, x, y, debug=None):
        self.x = x
        self.y = y
        self.debug = debug


class GoTo(Command):
    def __init__(self, point, debug=None):
        super().__init__(point.x, point.y, debug)


class Skip(Command):
    def __init__(self, debug=None):
        super().__init__(0, 0, debug)


class Strategy:
    def read_json(self):
        return json.loads(self.read_line())

    def read_line(self):
        line = input()
        # For testing with local runner output.
        if line[0] == '"':
            line = json.loads(line)
        return line

    def run(self):
        self.config = self.read_json()
        # Does not work in local runner.
        #assert self.config.get('FOOD_RADIUS') == FOOD_RADIUS
        assert self.config.get('FOOD_MASS') == FOOD_MASS
        assert self.config.get('VIRUS_RADIUS') == VIRUS_RADIUS
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

    def on_tick(self):
        if not self.my_blobs:
            return Skip('Died')
        if not self.food:
            return Skip('No food')

        # Find my biggest blob.
        self.my_blobs.sort(key=lambda b: b.m, reverse=True)
        me = self.my_blobs[0]

        # Go to the closest food.
        food = min(self.food, key=lambda b: b.distance_to(me))
        return GoTo(food)

    def parse_blobs(self, data):
        self.my_blobs = [
            MyBlob(
                id=blob.get('Id'),
                x=blob.get('X'),
                y=blob.get('Y'),
                r=blob.get('R'),
                m=blob.get('M'),
                v=Point(blob.get('SX'), blob.get('SY')),
                ttf=blob.get('TTF')) for blob in data.get('Mine', [])
        ]
        self.food = []
        self.viruses = []
        self.enemies = []
        for obj in data.get('Objects', []):
            t = obj.get('T')
            if t == 'F':
                self.food.append(FoodBlob(obj.get('X'), obj.get('Y')))
            elif t == 'E':
                self.food.append(EjectionBlob(obj.get('X'), obj.get('Y')))
            elif t == 'V':
                self.viruses.append(
                    VirusBlob(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M')))
            elif t == 'P':
                self.enemies.append(
                    EnemyBlob(
                        id=obj.get('Id'),
                        x=obj.get('X'),
                        y=obj.get('Y'),
                        m=obj.get('M'),
                        r=obj.get('R')))
            else:
                raise ValueError('unknown object type')

    def find_food(self, objects):
        for obj in objects:
            if obj.get('T') == 'F':
                return obj
        return None


if __name__ == '__main__':
    Strategy().run()
