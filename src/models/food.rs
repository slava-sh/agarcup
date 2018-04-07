use models::*;

#[derive(Debug)]
pub struct Food {
    pub id_: FoodId,
    pub point_: Point,
}

pub type FoodId = String;

impl HasPoint for Food {
    fn point(&self) -> Point {
        self.point_
    }
}

impl Circle for Food {
    fn r(&self) -> f64 {
        config().food_radius
    }
}

impl Blob for Food {
    type Id = FoodId;

    fn id(&self) -> &FoodId {
        &self.id_
    }

    fn m(&self) -> f64 {
        config().food_mass
    }
}
