use models::*;

#[derive(Debug)]
pub struct Ejection {
    pub id_: EjectionId,
    pub point_: Point,
}

pub type EjectionId = u64;

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
    type Id = EjectionId;

    fn id(&self) -> EjectionId {
        self.id_
    }

    fn m(&self) -> Mass {
        config().ejection_mass
    }
}
