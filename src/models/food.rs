use models::*;

#[derive(Debug)]
pub struct Food {
    pub id_: FoodId,
    pub point_: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoodId {
    pub x10: u32,
    pub y10: u32,
}

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

    fn id(&self) -> FoodId {
        self.id_
    }

    fn m(&self) -> Mass {
        config().food_mass
    }
}
