use strategy::*;
use config::Config;

use serde_json;
use std::io;

pub struct Interactor {}

impl Interactor {
    pub fn new() -> Self {
        Interactor {}
    }

    pub fn run(self) {
        let config = self.read_config();
        let strategy = Strategy::new(config);
        let mut tick = 0;
        while let Some(data) = self.read_tick_data() {
            let command = strategy.tick(tick, data);
            self.print_command(command);
            tick += 1;
        }
    }

    fn read_config(&self) -> Config {
        Config::from_json(self.read_json())
    }

    fn read_tick_data(&self) -> Option<TickData> {
        let mut json = self.read_json();
        Some(TickData {})
    }

    fn read_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.read_line()).expect("parse JSON")
    }

    fn read_line(&self) -> String {
        let mut line = String::new();
        io::stdin().read_line(&mut line).expect("read line");
        line
    }

    fn print_command(&self, command: Command) {
        let r = Response::new(50., 50., "nothring");
        println!("{}", serde_json::to_string(&r).unwrap());
        //if cfg!(feature = "debug") {
        //    eprintln!("debug!");
        //} else {
        //    eprintln!("no debug!");
        //}
    }
}

#[derive(Serialize, Deserialize)]
struct Mine {
    Id: String,
    X: f32,
    Y: f32,
    R: f32,
    M: f32,
    SX: f32,
    SY: f32,
}

#[derive(Serialize, Deserialize)]
struct Objects {
    X: f32,
    Y: f32,
    T: String,
    Id: Option<String>,
    M: Option<f32>,
    R: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct Data {
    Mine: Vec<Mine>,
    Objects: Vec<Objects>,
}

#[derive(Serialize, Deserialize)]
struct Response {
    X: f32,
    Y: f32,
    Debug: Option<String>,
    #[cfg(feature = "debug")]
    Draw: String,
}

impl Response {
    fn new(x: f32, y: f32, d: &str) -> Response {
        Response {
            X: x,
            Y: y,
            Debug: Some(d.to_string()),
            #[cfg(feature = "debug")]
            Draw: String::new(),
        }
    }
}
