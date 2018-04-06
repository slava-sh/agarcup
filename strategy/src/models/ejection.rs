use models::*;

#[derive(Debug)]
pub struct Ejection {
    pub id_: String,
    pub point_: Point,
}

impl Ejection {
    pub fn r(&self) -> f64 {
        config().ejection_radius
    }

    pub fn m(&self) -> f64 {
        config().ejection_mass
    }
}
