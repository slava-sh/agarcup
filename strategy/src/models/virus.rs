use models::*;

#[derive(Debug)]
pub struct Virus {
    id: String,
    point: Point,
    m: f64,
}

impl Virus {
    impl_getter!(point() -> Point);
    impl_getter!(m() -> f64);

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
