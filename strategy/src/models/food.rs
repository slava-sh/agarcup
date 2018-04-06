use models::*;

#[derive(Debug)]
pub struct Food {
    id: String,
    point: Point,
}

impl Food {
    pub fn r(&self) -> f64 {
        config().food_radius
    }

    pub fn m(&self) -> f64 {
        config().food_mass
    }
}
