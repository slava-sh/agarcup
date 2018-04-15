#[macro_use]
extern crate criterion;

extern crate my_strategy;

use std::f64::consts::PI;

use criterion::Criterion;

use my_strategy::models::*;
use my_strategy::config::Config;

fn bench(c: &mut Criterion) {
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
    }.init_singleton();
    c.bench_function("Player::can_eat_blob", |b| {
        let m = 300.0;
        let player = Player {
            id_: PlayerBlobId {
                player_id: 1,
                fragment_id: 0,
            },
            point_: Point::new(100.0, 200.0),
            m_: m,
            r_: Player::mass_to_radius(m),
            v_: Default::default(),
            is_fast_: Default::default(),
            ttf_: Default::default(),
        };
        let m = 80.0;
        let other = Player {
            id_: PlayerBlobId {
                player_id: 2,
                fragment_id: 0,
            },
            point_: Point::new(100.0, 200.0),
            m_: m,
            r_: Player::mass_to_radius(m),
            v_: Default::default(),
            is_fast_: Default::default(),
            ttf_: Default::default(),
        };
        b.iter(|| player.can_eat_blob(&other))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
