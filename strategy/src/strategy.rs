use models::*;
use config::config;
use command::Command;

pub struct Strategy {}

impl Strategy {
    pub fn new() -> Strategy {
        Strategy {}
    }

    pub fn tick(
        &self,
        tick: i64,
        my_blobs: Vec<Player>,
        food: Vec<Food>,
        ejections: Vec<Ejection>,
        viruses: Vec<Virus>,
        enemies: Vec<Player>,
    ) -> Command {
        let mut command = Command::new();
        command.set_point(Point::new(500.0, 500.0));
        command
    }
}
