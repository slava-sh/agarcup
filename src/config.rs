use std::f64::consts::PI;
use std::sync::{Mutex, MutexGuard};

use lazy_static;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub burst_angle_spectrum: f64,
    pub burst_bonus: f64,
    pub burst_start_speed: f64,
    pub collision_power: f64,
    pub diam_eat_factor: f64,
    pub ejection_mass: f64,
    pub ejection_radius: f64,
    pub food_mass: f64,
    pub food_radius: f64,
    pub game_height: i64,
    pub game_width: i64,
    pub inertion_factor: f64,
    pub mass_eat_factor: f64,
    pub max_frags_cnt: i64,
    pub min_burst_mass: f64,
    pub min_shrink_mass: f64,
    pub min_split_mass: f64,
    pub rad_hurt_factor: f64,
    pub radius_factor: f64,
    pub shrink_every_tick: i64,
    pub shrink_factor: f64,
    pub speed_factor: f64,
    pub split_start_speed: f64,
    pub ticks_til_fusion: i64,
    pub virus_radius: f64,
    pub vis_factor: f64,
    pub vis_factor_fr: f64,
    pub vis_shift: f64,
    pub viscosity: f64,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            burst_angle_spectrum: PI,
            burst_bonus: 5.0,
            burst_start_speed: 8.0,
            collision_power: 20.0,
            diam_eat_factor: 2.0 / 3.0,
            ejection_mass: 15.0,
            ejection_radius: 4.0,
            food_mass: 1.0,
            food_radius: 2.5,
            game_height: 990,
            game_width: 990,
            inertion_factor: 10.0,
            mass_eat_factor: 1.2,
            max_frags_cnt: 10,
            min_burst_mass: 60.0,
            min_shrink_mass: 100.0,
            min_split_mass: 120.0,
            rad_hurt_factor: 0.66,
            radius_factor: 2.0,
            shrink_every_tick: 50,
            shrink_factor: 0.01,
            speed_factor: 25.0,
            split_start_speed: 9.0,
            ticks_til_fusion: 250,
            virus_radius: 22.0,
            vis_factor: 4.0,
            vis_factor_fr: 2.5,
            vis_shift: 10.0,
            viscosity: 0.25,
        }
    }
}

lazy_static! {
    static ref INITIALIZER: Mutex<Option<Config>> = Mutex::new(None);
    static ref SINGLETON: Config = {
        lock_initializer().take().expect("config::INITIALIZER is None")
    };
}

pub fn config() -> &'static Config {
    &*SINGLETON
}

impl Config {
    pub fn init_singleton(self) {
        *lock_initializer() = Some(self);
        lazy_static::initialize(&SINGLETON);
    }
}

fn lock_initializer<'mutex>() -> MutexGuard<'mutex, Option<Config>> {
    INITIALIZER.lock().expect(
        "config::INITIALIZER.lock() failed",
    )
}
