use std::ops::{Mul, Add, Div, Sub};
use config::Config;

pub struct Strategy {
    config: Config,
}

pub struct TickData {}

pub struct Command {
    pub point: Point,
}

impl Strategy {
    pub fn new(config: Config) -> Strategy {
        Strategy { config }
    }

    pub fn tick(&self, tick: i64, data: TickData) -> Command {
        Command { point: Point::zero() }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn from_polar(r: f64, angle: f64) -> Point {
        Point::new(r * angle.cos(), r * angle.sin())
    }

    pub fn zero() -> Point {
        Point::new(0.0, 0.0)
    }

    pub fn dist(self, other: Point) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    pub fn qdist(self, other: Point) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn with_length(self, new_length: f64) -> Point {
        let current_length = self.length();
        if current_length == 0.0 {
            Point::zero()
        } else {
            self * (new_length / current_length)
        }
    }

    pub fn unit(self) -> Point {
        self.with_length(1.0)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}

impl Div<f64> for Point {
    type Output = Point;
    fn div(self, k: f64) -> Point {
        Point::new(self.x / k, self.y / k)
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, other: Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }
}

pub trait Circle {
    fn point(&self) -> Point;
    fn r(&self) -> f64;
}

pub trait Blob: Circle {
    fn id(&self) -> String;
    fn m(&self) -> f64;
}

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

#[derive(Debug)]
pub struct Food {
    id: String,
    point: Point,
}

macro_rules! impl_getter {
    ($name:ident() -> $type:ty) => {
        pub fn $name(&self) -> $type {
            self.$name()
        }
    };
    ($name:ident() -> &$type:ty) => {
        pub fn $name(&self) -> &$type {
            &self.$name()
        }
    };
    ($name:ident() -> &mut $type:ty) => {
        pub fn $name(&mut self) -> &mut $type {
            &mut self.$name()
        }
    };
}

impl Command {
    impl_getter!(point() -> Point);
}

impl Food {
    pub fn r(&self) -> f64 {
        config().food_radius
    }

    pub fn m(&self) -> f64 {
        config().food_mass
    }
}

#[derive(Debug)]
pub struct Ejection {
    id: String,
    point: Point,
}

impl Ejection {
    pub fn r(&self) -> f64 {
        config().ejection_radius
    }

    pub fn m(&self) -> f64 {
        config().ejection_mass
    }
}

#[derive(Debug)]
struct Virus {
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

static CONFIG: Option<Config> = None;

fn config() -> Config {
    unimplemented!();
}

impl Player {
    impl_getter!(point() -> Point);
    impl_getter!(point_mut() -> &mut Point);
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
