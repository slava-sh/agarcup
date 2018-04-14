use models::*;
use command::Command;

pub type Tick = i64;

pub trait Strategy {
    fn tick(
        &mut self,
        tick: Tick,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command;
}
