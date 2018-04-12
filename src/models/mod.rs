pub use self::common::{Blob, Circle, Speed, Mass, Angle};
pub use self::ejection::{Ejection, EjectionId};
pub use self::food::{Food, FoodId};
pub use self::player::{Player, PlayerBlobId, PlayerId, FragmentId};
pub use self::point::{Point, HasPoint};
pub use self::virus::{Virus, VirusId};

use config::config;

mod common;
mod ejection;
mod food;
mod player;
mod point;
mod virus;
