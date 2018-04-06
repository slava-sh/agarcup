use models::*;

#[derive(Debug)]
pub struct Virus {
    pub id_: BlobId,
    pub point_: Point,
    pub m_: f64,
}

impl Virus {
    impl_getter!(point() -> point_: Point);
    impl_getter!(m() -> m_: f64);

    pub fn r(&self) -> f64 {
        config().virus_radius
    }

    pub fn can_hurt(&self, other: &Player) -> bool {
        if other.r() < self.r() || !other.can_burst() {
            return false;
        }
        let max_dist = self.r() * config().rad_hurt_factor + other.r();
        self.point().qdist(other.point()) < max_dist.powi(2)
    }
}
