use config::Config;
use models::*;

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

impl Command {
    impl_getter!(point() -> Point);
}
