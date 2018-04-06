use models::*;
use config::config;

pub struct Strategy {}

pub struct TickData {}

pub struct Command {
    point: Point,
}

impl Command {
    impl_getter!(point() -> Point);
    impl_setter!(set_point(point: Point));
}

impl Strategy {
    pub fn new() -> Strategy {
        Strategy {}
    }

    pub fn tick(&self, tick: i64, data: TickData) -> Command {
        Command { point: Point::zero() }
    }
}
