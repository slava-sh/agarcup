pub use self::my_strategy::MyStrategy;
#[cfg(feature = "debug")]
pub use self::timing_wrapper::TimingWrapper;
pub use command::*;
pub use models::*;

mod mechanic;
mod my_strategy;
#[cfg(feature = "debug")]
mod timing_wrapper;

pub trait Strategy {
    fn tick(
        &mut self,
        tick: i64,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command;
}
