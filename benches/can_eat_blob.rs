#[macro_use]
extern crate criterion;

extern crate my_strategy;

use criterion::Criterion;

use my_strategy::models::*;
use my_strategy::config::Config;

fn bench(c: &mut Criterion) {
    Config::default().init_singleton();
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
