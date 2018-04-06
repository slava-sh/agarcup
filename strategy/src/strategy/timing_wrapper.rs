use time::precise_time_s;

use strategy::*;

const AVG_TICK_TIME_SECS: f64 = 150.0 / 7500.0;

pub struct TimingWrapper<S: Strategy> {
    strategy: S,
    total: f64,
}

impl<S: Strategy> TimingWrapper<S> {
    pub fn new(strategy: S) -> TimingWrapper<S> {
        TimingWrapper {
            strategy,
            total: 0.0,
        }
    }
}

impl<S: Strategy> Strategy for TimingWrapper<S> {
    fn tick(
        &mut self,
        tick: i64,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command {
        let start = precise_time_s();
        let mut command = self.strategy.tick(
            tick,
            my_blobs,
            food,
            ejections,
            viruses,
            enemies,
        );
        let mut tick = tick;
        self.total += precise_time_s() - start;
        let expected = AVG_TICK_TIME_SECS * (tick + 1) as f64;
        command.add_debug_message(format!(
            "total: {:.2}\tbudget: {:.2}",
            self.total,
            expected - self.total
        ));
        command
    }
}
