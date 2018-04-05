use config::Config;

pub struct Strategy {
    config: Config,
}

pub struct TickData {}

pub struct Command {}

impl Strategy {
    pub fn new(config: Config) -> Self {
        Strategy { config }
    }

    pub fn tick(&self, tick: isize, data: TickData) -> Command {
        Command {}
    }
}
