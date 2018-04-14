use models::*;

#[derive(Debug)]
pub struct Virus {
    pub id_: VirusId,
    pub point_: Point,
    pub m_: Mass,
}

pub type VirusId = u64;

impl HasPoint for Virus {
    fn point(&self) -> Point {
        self.point_
    }
}

impl Circle for Virus {
    fn r(&self) -> f64 {
        config().virus_radius
    }
}

impl Blob for Virus {
    type Id = VirusId;

    fn id(&self) -> VirusId {
        self.id_
    }

    fn m(&self) -> Mass {
        self.m_
    }
}

impl Virus {
    pub fn can_hurt(&self, other: &Player) -> bool {
        if other.r() < self.r() {
            return false;
        }
        let max_dist = self.r() * config().rad_hurt_factor + other.r();
        self.point().qdist(other.point()) < max_dist.powi(2)
    }
}
