use models::*;

#[derive(Debug)]
pub struct Ejection {
    pub id_: BlobId,
    pub point_: Point,
}

impl HasPoint for Ejection {
    fn point(&self) -> Point {
        self.point_
    }
}

impl Circle for Ejection {
    fn r(&self) -> f64 {
        config().ejection_radius
    }
}

impl Blob for Ejection {
    fn id(&self) -> &BlobId {
        &self.id_
    }

    fn m(&self) -> f64 {
        config().ejection_mass
    }
}
