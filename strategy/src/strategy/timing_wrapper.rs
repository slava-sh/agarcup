use time::precise_time_s;

use strategy::*;

const AVG_TICK_TIME_SECS: f64 = 150.0 / 7500.0;

pub struct TimingWrapper<S: Strategy> {
    strategy: S,
    start: f64,
}

impl<S: Strategy> TimingWrapper<S> {
    pub fn new(strategy: S) -> TimingWrapper<S> {
        TimingWrapper {
            strategy,
            start: precise_time_s(),
        }
    }
}

impl<S: Strategy> Strategy for TimingWrapper<S> {
    fn tick(
        &self,
        tick: i64,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command {
        let mut command = self.strategy.tick(
            tick,
            my_blobs,
            food,
            ejections,
            viruses,
            enemies,
        );
        let elapsed = precise_time_s() - self.start;
        let expected = AVG_TICK_TIME_SECS * (tick + 1) as f64;
        if elapsed > expected {
            command.add_debug_message(format!("SLOW: {:.2}s", 1.0))
        } else {
            command.add_debug_message(format!("OK: {:.2}s", elapsed))
        }
        command
    }
}
