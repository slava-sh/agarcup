pub use self::ejection::Ejection;
pub use self::food::Food;
pub use self::player::Player;
pub use self::point::{Point, HasPoint};
pub use self::virus::Virus;

use config::config;

mod ejection;
mod food;
mod player;
mod point;
mod virus;

pub type BlobId = String;

pub trait Circle: HasPoint {
    fn r(&self) -> f64;
}

pub trait Blob: Circle {
    fn id(&self) -> &BlobId;
    fn m(&self) -> f64;
}
