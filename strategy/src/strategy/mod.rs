pub use self::my_strategy::MyStrategy;
pub use self::timing_wrapper::TimingWrapper;
pub use models::*;
pub use command::Command;

mod my_strategy;
mod timing_wrapper;

pub trait Strategy {
    fn tick(
        &self,
        tick: i64,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command;
}
