#[macro_use]
extern crate criterion;

extern crate my_strategy;

use criterion::Criterion;

use my_strategy::models::*;
use my_strategy::config::Config;

fn bench(c: &mut Criterion) {
    Config::default().init_singleton();
    c.bench_function("Player::can_eat_blob", |b| {
        b.iter(|| {
            (0..100)
                .map(|i| {
                    let i = i as f64;
                    let m = 300.0 + i;
                    let player = Player {
                        id_: PlayerBlobId {
                            player_id: 1,
                            fragment_id: 0,
                        },
                        point_: Point::new(100.0 + i, 200.0 + i),
                        m_: m,
                        r_: Player::mass_to_radius(m),
                        v_: Default::default(),
                        is_fast_: Default::default(),
                        ttf_: Default::default(),
                    };
                    let m = 80.0 - i;
                    let other = Player {
                        id_: PlayerBlobId {
                            player_id: 2,
                            fragment_id: 0,
                        },
                        point_: Point::new(100.0 - i, 200.0 - i),
                        m_: m,
                        r_: Player::mass_to_radius(m),
                        v_: Default::default(),
                        is_fast_: Default::default(),
                        ttf_: Default::default(),
                    };
                    player.can_eat_blob(&other) as i64
                })
                .sum::<i64>()
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
