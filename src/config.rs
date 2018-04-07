use std::sync::{Mutex, MutexGuard};

use serde_json;

pub fn config() -> &'static Config {
    &*SINGLETON
}

lazy_static! {
    static ref INITIALIZER: Mutex<Option<Config>> = Mutex::new(None);
    static ref SINGLETON: Config = {
        lock_initializer().take().expect("config::INITIALIZER is None")
    };
}

pub fn init_config(config: Config) {
    *lock_initializer() = Some(config);
    &*SINGLETON;
}

fn lock_initializer<'mutex>() -> MutexGuard<'mutex, Option<Config>> {
    INITIALIZER.lock().expect(
        "config::INITIALIZER.lock() failed",
    )
}

macro_rules! impl_config {
    ($($name:ident: $type:ty $(= $value:expr)*),* $(,)*) => {
        #[derive(Debug)]
        pub struct Config {
            $(
                pub $name: $type
            ),*
        }

        impl Config {
            pub fn from_json(json: serde_json::Value) -> Config {
                Config {
                    $(
                        $name: get_or_default!(json,
                                               stringify!($name).to_string().to_uppercase()
                                               $(, $value)*)
                    ),*
                }
            }
        }
    };
}

macro_rules! get_or_default {
    ($json:ident, $key:expr, $default_value:expr) => {
        ValueWrapper($json.get($key).unwrap_or(&json!($default_value))).into()
    };
    ($json:ident, $key:expr) => {
        ValueWrapper($json.get($key).expect("no key found")).into()
    };
}

struct ValueWrapper<'a>(&'a serde_json::Value);

macro_rules! impl_into {
    ($type:ty, $method:ident) => {
        impl<'a> Into<$type> for ValueWrapper<'a> {
            fn into(self) -> $type {
                (self.0).$method().expect("conversion failed")
            }
        }
    };
}

impl_into!(i64, as_i64);
impl_into!(f64, as_f64);

impl_config! {
    game_width: i64,
    game_height: i64,
    viscosity: f64,
    ticks_til_fusion: i64,
    virus_radius: f64,
    speed_factor: f64,
    inertion_factor: f64,
    food_mass: f64,
    food_radius: f64 = 2.5,
    ejection_radius: f64 = 4.0,
    ejection_mass: f64 = 15.0,
    mass_eat_factor: f64 = 1.2,
    radius_factor: f64 = 2.0,
    diam_eat_factor: f64 = 2.0 / 3.0,
    vis_factor: f64 = 4.0,
    vis_factor_fr: f64 = 2.5,
    vis_shift: f64 = 10.0,
    rad_hurt_factor: f64 = 0.66,
    min_split_mass: f64 = 120.0,
    split_start_speed: f64 = 9.0,
    shrink_every_tick: i64 = 50,
    min_shrink_mass: f64 = 100,
    shrink_factor: f64 = 0.01,
    min_burst_mass: f64 = 60.0,
    collision_power: f64 = 20.0,
}
