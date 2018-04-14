use std::hash::Hash;

use models::point::HasPoint;

pub type Speed = f64;
pub type Mass = f64;
pub type Angle = f64;

pub trait Circle: HasPoint {
    fn r(&self) -> f64;
}

pub trait Blob: Circle {
    type Id: Copy + Eq + Hash;
    fn id(&self) -> Self::Id;
    fn m(&self) -> Mass;
}
