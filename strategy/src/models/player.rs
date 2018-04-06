use models::*;
use strategy::Command;

#[derive(Debug)]
pub struct Player {
    id: String,
    point: Point,
    m: f64,
    r: f64,
    v: Option<Point>,
    is_fast: Option<bool>,
    ttf: Option<i64>,
}

impl Player {
    impl_getter!(point() -> Point);
    impl_getter!(point_mut() -> point: &mut Point);
    impl_getter!(m() -> f64);
    impl_getter!(r() -> f64);

    pub fn can_eat<Other: Blob>(&self, other: &Other) -> bool {
        if !(self.m() > other.m() * config().mass_eat_factor) {
            return false;
        }
        let dist = self.point().dist(other.point());
        let min_r = dist - other.r() + other.r() * 2.0 * config().diam_eat_factor;
        min_r < self.r()
    }

    pub fn can_see<Other: Blob>(&self, other: &Other) -> bool {
        let p = self.point() + Point::from_polar(config().vis_shift, self.angle());
        let vision_radius = self.r() * config().vis_factor; // TODO: Not always true.
        let max_dist = vision_radius + other.r();
        other.point().qdist(p) < max_dist.powi(2)
    }

    pub fn can_burst(&self) -> bool {
        if self.m() < config().min_burst_mass * 2.0 {
            return false;
        }
        // TODO: Consider config().max_frags_cnt.
        let frags_cnt = (self.m() / config().min_burst_mass).floor() as i64;
        frags_cnt > 1
    }

    pub fn can_hurt<Other: Blob>(&self, other: &Other) -> bool {
        self.can_eat(other)
    }

    pub fn can_split(&self) -> bool {
        // TODO: Consider config().max_frags_cnt.
        self.m() > config().min_split_mass
    }

    pub fn max_speed(&self) -> f64 {
        config().speed_factor / self.m().sqrt()
    }

    pub fn v(&self) -> Point {
        self.v.expect("v not set")
    }

    pub fn is_fast(&self) -> bool {
        self.is_fast.expect("is_fast not set")
    }

    pub fn speed(&self) -> f64 {
        self.v().length()
    }

    pub fn angle(&self) -> f64 {
        self.v().angle()
    }

    pub fn update_r(&mut self) {
        self.r = config().radius_factor * self.m().sqrt();
    }

    pub fn limit_speed(&mut self) {
        if !self.is_fast() {
            return;
        }
        self.v = Some(self.v().with_length(self.speed().min(self.max_speed())))
    }

    pub fn update_v(&mut self, command: Command) {
        if self.is_fast() {
            return;
        }
        let target_v = (command.point() - self.point()).with_length(self.max_speed());
        self.v = Some(
            self.v() + (target_v - self.v()) * (config().inertion_factor / self.m()),
        );
        self.limit_speed();
    }

    pub fn apply_v(&mut self) {
        self.point_mut().x = (self.point().x + self.v().x)
            .min(config().game_width as f64 - self.r())
            .max(self.r());
        self.point_mut().y = (self.point().y + self.v().y)
            .min(config().game_height as f64 - self.r())
            .max(self.r());
    }

    pub fn apply_viscosity(&mut self) {
        if !self.is_fast() {
            return;
        }
        let mut speed = self.speed();
        let max_speed = self.max_speed();
        if speed > max_speed {
            speed = (speed - config().viscosity).max(max_speed);
        }
        if speed <= max_speed {
            self.is_fast = Some(false);
            speed = max_speed;
        }
        self.v = Some(self.v().with_length(speed));
    }

    pub fn can_shrink(&self) -> bool {
        self.m() > config().min_shrink_mass
    }

    pub fn shrink(&mut self) {
        if !self.can_shrink() {
            return;
        }
        self.m -= (self.m() - config().min_shrink_mass) * config().shrink_factor;
        self.update_r();
    }
}
