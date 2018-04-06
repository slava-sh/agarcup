use models::*;

#[derive(Debug)]
pub struct Food {
    pub id_: BlobId,
    pub point_: Point,
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
    fn id(&self) -> &BlobId {
        &self.id_
    }

    fn m(&self) -> f64 {
        config().food_mass
    }
}
