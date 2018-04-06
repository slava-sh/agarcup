use config::config;

pub trait Circle {
    fn point(&self) -> Point;
    fn r(&self) -> f64;
}

pub trait Blob: Circle {
    fn id(&self) -> String;
    fn m(&self) -> f64;
}

mod point;
mod food;
mod ejection;
mod virus;
mod player;

pub use self::point::Point;
pub use self::food::Food;
pub use self::ejection::Ejection;
pub use self::virus::Virus;
pub use self::player::Player;
