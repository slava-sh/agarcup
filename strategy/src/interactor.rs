use std::io;

use serde_json;

use strategy::*;
use config::{Config, init_config};

pub fn run() {
    init_config(read_config());
    let mut strategy = get_strategy();
    let mut tick = 0;
    while let Some(data) = read_tick_data() {
        let command = strategy.tick(
            tick,
            data.my_blobs,
            data.food,
            data.ejections,
            data.viruses,
            data.enemies,
        );
        print_command(command);
        tick += 1;
    }
}

#[cfg(not(feature = "debug"))]
fn get_strategy() -> MyStrategy {
    MyStrategy::new()
}

#[cfg(feature = "debug")]
fn get_strategy() -> TimingWrapper<MyStrategy> {
    TimingWrapper::new(MyStrategy::new())
}

fn read_config() -> Config {
    Config::from_json(read_json().expect("EOF"))
}

#[derive(Default)]
struct Entities {
    my_blobs: Vec<Player>,
    food: Vec<Food>,
    ejections: Vec<Ejection>,
    viruses: Vec<Virus>,
    enemies: Vec<Player>,
}

fn read_tick_data() -> Option<Entities> {
    let data: TickData = serde_json::from_value(read_json()?).expect("TickData parsing failed");
    let mut entities = Entities::default();
    for mine in data.mine {
        let mut me = Player {
            id_: mine.id,
            point_: Point::new(mine.x, mine.y),
            m_: mine.m,
            r_: mine.r,
            v_: Some(Point::new(mine.s_x, mine.s_y)),
            is_fast_: None,
            ttf_: mine.ttf,
        };
        me.is_fast_ = Some(me.speed() > me.max_speed());
        entities.my_blobs.push(me);
    }
    for obj in data.objects {
        let point = Point::new(obj.x, obj.y);
        match obj.t.as_ref() {
            "F" => {
                entities.food.push(Food {
                    id_: format!("F{:.1}{:.1}", point.x, point.y),
                    point_: point,
                });
            }
            "E" => {
                entities.ejections.push(Ejection {
                    id_: format!("E{:.1}{:.1}", point.x, point.y),
                    point_: point,
                });
            }
            "V" => {
                entities.viruses.push(Virus {
                    id_: obj.id.expect("virus has no id"),
                    point_: point,
                    m_: obj.m.expect("virus has no mass"),
                });
            }
            "P" => {
                entities.enemies.push(Player {
                    id_: obj.id.expect("enemy has no id"),
                    point_: point,
                    m_: obj.m.expect("enemy has no mass"),
                    r_: obj.r.expect("enemy has no radius"),
                    v_: None,
                    is_fast_: None,
                    ttf_: None,
                });
            }
            _ => {
                panic!("unknown object type");
            }
        }
    }
    Some(entities)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TickData {
    mine: Vec<Mine>,
    objects: Vec<Objects>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Mine {
    id: String,
    x: f64,
    y: f64,
    r: f64,
    m: f64,
    s_x: f64,
    s_y: f64,
    #[serde(rename = "TTF")]
    ttf: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Objects {
    id: Option<String>,
    t: String,
    x: f64,
    y: f64,
    m: Option<f64>,
    r: Option<f64>,
}

fn read_json() -> Option<serde_json::Value> {
    serde_json::from_str(&read_line()?).expect("JSON parsing failed")
}

fn read_line() -> Option<String> {
    let mut line = String::new();
    let n = io::stdin().read_line(&mut line).expect("read line failed");
    if n == 0 { None } else { Some(line) }
}

fn print_command(command: Command) {
    let response = Response {
        x: command.point().x,
        y: command.point().y,
        debug: command.debug_messages().join("; "),
        #[cfg(feature = "debug")]
        draw: Draw {
            lines: command
                .debug_lines()
                .iter()
                .map(|line| {
                    DrawLine {
                        p: vec![XY::from(line.a), XY::from(line.b)],
                        c: line.color.clone(),
                        a: line.opacity,
                    }
                })
                .collect(),
            circles: command
                .debug_circles()
                .iter()
                .map(|circle| {
                    DrawCircle {
                        x: circle.center.x,
                        y: circle.center.y,
                        r: circle.radius,
                        c: circle.color.clone(),
                        a: circle.opacity,
                    }
                })
                .collect(),
        },
    };
    println!(
        "{}",
        serde_json::to_string(&response).expect("response serialization failed")
    );
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Response {
    x: f64,
    y: f64,
    debug: String,
    #[cfg(feature = "debug")]
    draw: Draw,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Draw {
    lines: Vec<DrawLine>,
    circles: Vec<DrawCircle>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct DrawLine {
    p: Vec<XY>,
    c: String,
    a: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct XY {
    x: f64,
    y: f64,
}

impl XY {
    fn from(point: Point) -> XY {
        XY {
            x: point.x,
            y: point.y,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct DrawCircle {
    x: f64,
    y: f64,
    r: f64,
    c: String,
    a: f64,
}
