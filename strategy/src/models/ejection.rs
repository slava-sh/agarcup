use models::*;

#[derive(Debug)]
pub struct Ejection {
    id: String,
    point: Point,
}

impl Ejection {
    pub fn r(&self) -> f64 {
        config().ejection_radius
    }

    pub fn m(&self) -> f64 {
        config().ejection_mass
    }
}
