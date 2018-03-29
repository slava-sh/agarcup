import json

class Strategy:
    def run(self):
        config = json.loads(input())
        while True:
            data = json.loads(input())
            command = self.on_tick(data, config)
            print(json.dumps(command))

    def on_tick(self, data, config):
        mine = data.get('Mine')
        if not mine:
            return dict(X=0, Y=0, Debug='Died')
        mine = mine[0]
        objects = data.get('Objects')
        food = self.find_food(objects)
        if not food:
            return dict(X=0, Y=0, Debug='No food')
        return dict(X=food.get('X'),
                    Y=food.get('Y'))

    def find_food(self, objects):
        for obj in objects:
            if obj.get('T') == 'F':
                return obj
        return None

if __name__ == '__main__':
    Strategy().run()
