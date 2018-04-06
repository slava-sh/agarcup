pub use self::point::{Point, HasPoint};
pub use self::food::Food;
pub use self::ejection::Ejection;
pub use self::virus::Virus;
pub use self::player::Player;
use config::config;

mod point;
mod food;
mod ejection;
mod virus;
mod player;

pub type BlobId = String;

pub trait Circle: HasPoint {
    fn r(&self) -> f64;
}

pub trait Blob: Circle {
    fn id(&self) -> BlobId;
    fn m(&self) -> f64;
}
