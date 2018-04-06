use serde_json;
use std::io;

use strategy::{Strategy, Command, TickData};
use config::{Config, init_config};

pub fn run() {
    init_config(read_config());
    let mut strategy = Strategy::new();
    let mut tick = 0;
    while let Some(data) = read_tick_data() {
        let command = strategy.tick(tick, data);
        print_command(command);
        tick += 1;
    }
}

fn read_config() -> Config {
    Config::from_json(read_json())
}

fn read_tick_data() -> Option<TickData> {
    let mut json = read_json();
    Some(TickData {})
}

fn read_json() -> serde_json::Value {
    serde_json::from_str(&read_line()).expect("parse JSON")
}

fn read_line() -> String {
    let mut line = String::new();
    io::stdin().read_line(&mut line).expect("read line");
    line
}

fn print_command(command: Command) {
    let r = Response::new(50., 50., "nothring");
    println!("{}", serde_json::to_string(&r).unwrap());
    //if cfg!(feature = "debug") {
    //    eprintln!("debug!");
    //} else {
    //    eprintln!("no debug!");
    //}
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Mine {
    id: String,
    x: f32,
    y: f32,
    r: f32,
    m: f32,
    s_x: f32,
    s_y: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Objects {
    x: f32,
    y: f32,
    t: String,
    id: Option<String>,
    m: Option<f32>,
    r: Option<f32>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Data {
    mine: Vec<Mine>,
    objects: Vec<Objects>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Response {
    x: f32,
    y: f32,
    debug: Option<String>,
    #[cfg(feature = "debug")]
    draw: String,
}

impl Response {
    fn new(x: f32, y: f32, d: &str) -> Response {
        Response {
            x: x,
            y: y,
            debug: Some(d.to_string()),
            #[cfg(feature = "debug")]
            draw: String::new(),
        }
    }
}
