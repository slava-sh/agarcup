pub use self::ejection::{Ejection, EjectionId};
pub use self::food::{Food, FoodId};
pub use self::player::{Player, PlayerBlobId, PlayerId, FragmentId};
pub use self::point::{Point, HasPoint};
pub use self::virus::{Virus, VirusId};

use std::hash::Hash;

use config::config;

mod ejection;
mod food;
mod player;
mod point;
mod virus;

pub type Speed = f64;
pub type Mass = f64;
pub type Angle = f64;

pub trait Circle: HasPoint {
    fn r(&self) -> f64;
}

pub trait Blob: Circle {
    type Id: Clone + Eq + Hash;
    fn id(&self) -> &Self::Id;
    fn m(&self) -> Mass;
}
