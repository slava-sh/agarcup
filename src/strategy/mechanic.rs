use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::iter::{self, FromIterator};

use models::*;
use command::Command;
use config::config;

#[derive(Debug)]
pub struct Mechanic {
    pub state: State,
    players: Vec<Player>,
    my_player_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub tick: i64,
    pub my_blobs: HashMap<PlayerBlobId, Player>,
    pub enemies: HashMap<PlayerBlobId, Player>,
    pub eaten_food: HashSet<FoodId>,
    pub eaten_ejections: HashSet<EjectionId>,
    pub eaten_viruses: HashSet<VirusId>,
}

impl State {
    pub fn new(tick: i64, my_blobs: Vec<Player>, enemies: Vec<Player>) -> State {
        State {
            tick,
            my_blobs: State::players_from_vec(my_blobs),
            enemies: State::players_from_vec(enemies),
            eaten_food: Default::default(),
            eaten_ejections: Default::default(),
            eaten_viruses: Default::default(),
        }
    }

    fn players_from_vec(players: Vec<Player>) -> HashMap<PlayerBlobId, Player> {
        HashMap::from_iter(players.into_iter().map(
            |player| (player.id().clone(), player),
        ))
    }
}

impl Mechanic {
    pub fn new(state: &State) -> Mechanic {
        let my_player_id = state.my_blobs.values().next().map_or(
            <PlayerId>::max_value(),
            |me| me.player_id(),
        );
        Mechanic {
            state: state.clone(),
            players: Default::default(),
            my_player_id,
        }
    }

    pub fn tick(
        &mut self,
        command: &Command,
        food: &[Food],
        ejections: &[Ejection],
        viruses: &[Virus],
    ) {
        self.players = iter::empty()
            .chain(self.state.my_blobs.values())
            .chain(self.state.enemies.values())
            .cloned()
            .collect();

        // Following the vendor mechanic.h.
        self.apply_strategies(command);
        self.state.tick += 1;
        self.move_moveables();
        self.player_ejects();
        self.player_splits(command);

        if self.state.tick % config().shrink_every_tick == 0 {
            self.shrink_players();
        }
        self.eat_all(food, ejections);
        self.fuse_players();
        self.burst_on_viruses(viruses);

        self.update_players_radius();
        self.update_scores();
        self.split_viruses();

        let my_player_id = self.my_player_id;
        let (my_blobs, enemies) = self.players.drain(..).partition(|player| {
            player.player_id() == my_player_id
        });
        self.state.my_blobs = State::players_from_vec(my_blobs);
        self.state.enemies = State::players_from_vec(enemies);
    }

    fn apply_strategies(&mut self, command: &Command) {
        for player in self.players.iter_mut() {
            if player.player_id() == self.my_player_id {
                apply_direct(player, command);
            }
        }
    }

    fn move_moveables(&mut self) {
        // TODO: Move ejections?
        // TODO: Move viruses?

        for i in 0..self.players.len() {
            for j in (i + 1)..self.players.len() {
                let (left, right) = self.players.as_mut_slice().split_at_mut(j);
                let ref mut player = left[i];
                let ref mut other = right[0];
                if player.player_id() == other.player_id() {
                    collision_calc(player, other);
                }
            }
        }

        for player in self.players.iter_mut() {
            move_player(player);
        }
    }

    fn player_ejects(&mut self) {
        // TODO: Implement if adding command.eject.
    }

    fn player_splits(&mut self, command: &Command) {
        if command.split() {
            let new_blobs = {
                let my_player_id = self.my_player_id;
                let ref mut my_blobs: Vec<_> = self.players
                    .iter_mut()
                    .filter(|player| player.player_id() == my_player_id)
                    .collect();
                split_fragments(my_blobs)
            };
            self.players.extend(new_blobs);
        }
    }

    fn shrink_players(&mut self) {
        for player in self.players.iter_mut() {
            if player.can_shrink() {
                shrink_now(player);
            }
        }
    }

    fn eat_all(&mut self, food: &[Food], ejections: &[Ejection]) {
        eat_food(food, &mut self.state.eaten_food, &mut self.players);
        eat_food(
            ejections,
            &mut self.state.eaten_ejections,
            &mut self.players,
        );
        eat_players(&mut self.players);
    }

    fn fuse_players(&mut self) {
        // TODO: Fuse other players.
        const FUSED: PlayerId = <PlayerId>::max_value();
        self.players.sort_by(|a, b| {
            a.player_id().cmp(&b.player_id()).then_with(|| {
                a.m()
                    .partial_cmp(&b.m())
                    .expect("incomparable mass")
                    .reverse()
                    .then_with(|| a.fragment_id().cmp(&b.fragment_id()))
            })
        });
        let mut fused_count = 0;
        {
            let fragments = {
                let mut i = 0;
                while i < self.players.len() && self.players[i].player_id() != self.my_player_id {
                    i += 1;
                }
                let mut j = i;
                while j < self.players.len() && self.players[j].player_id() == self.my_player_id {
                    j += 1;
                }
                &mut self.players[i..j]
            };
            loop {
                let mut idle = true;
                for i in 0..fragments.len() {
                    let mut fused = false;
                    for j in (i + 1)..fragments.len() {
                        let (left, right) = fragments.split_at_mut(j);
                        let ref mut player = left[i];
                        let ref mut other = right[0];
                        if player.player_id() != FUSED && other.player_id() != FUSED &&
                            player.can_fuse(other)
                        {
                            fusion(player, other);
                            other.set_player_id(FUSED);
                            fused_count += 1;
                            fused = true;
                        }
                    }
                    if fused {
                        idle = false;
                        update_by_mass(&mut fragments[i]);
                    }
                }
                if idle {
                    break;
                }
            }
            if fragments.len() == 1 {
                fragments[0].set_fragment_id(0);
            }
        }
        if fused_count != 0 {
            self.players.retain(|player| player.player_id() != FUSED);
        }
    }

    fn burst_on_viruses(&mut self, viruses: &[Virus]) {
        // TODO: Burst other players.
        let my_fragment_ids: Vec<_> = self.players
            .iter()
            .filter(|me| me.player_id() == self.my_player_id)
            .map(|me| me.fragment_id())
            .collect();
        let mut fragment_count = my_fragment_ids.len() as i64;
        let mut max_fragment_id = my_fragment_ids.into_iter().max().unwrap_or(0);
        for virus in viruses.iter() {
            if let Some(i) = nearest_player(
                virus,
                |me| {
                    me.player_id() == self.my_player_id && me.can_burst(fragment_count) &&
                        virus.can_hurt(me)
                },
                &self.players,
            )
            {
                let new_blobs = {
                    let ref mut me = self.players[i];
                    // TODO: targets.removeAll(player);
                    burst_on(me, virus);
                    burst_now(me, fragment_count, &mut max_fragment_id)
                };
                fragment_count += new_blobs.len() as i64;
                // TODO: Don't burst new_blobs on this tick.
                self.players.extend(new_blobs);
                self.state.eaten_viruses.insert(virus.id().clone());
            }
        }
    }

    fn update_players_radius(&mut self) {
        for player in self.players.iter_mut() {
            update_by_mass(player);
        }
    }

    fn update_scores(&mut self) {
        // Not relevant.
    }

    fn split_viruses(&mut self) {
        // TODO: Implement this if ejections are implemented.
    }
}

fn apply_direct(player: &mut Player, command: &Command) {
    if player.is_fast() {
        return;
    }
    let max_speed = player.max_speed();
    let target_v = (command.point() - player.point()).with_length(max_speed);
    let v = player.v() + (target_v - player.v()) * (config().inertion_factor / player.m());
    let v = v.limit_length(max_speed);
    player.set_v(v);
}

fn move_player(player: &mut Player) {
    let mut v = player.v();

    let min_x = player.r();
    let max_x = config().game_width as f64 - player.r();
    let mut new_x = player.point().x + player.v().x;
    if !(min_x <= new_x && new_x <= max_x) {
        v.x = 0.0;
        new_x = new_x.max(min_x).min(max_x);
    }

    let min_y = player.r();
    let max_y = config().game_height as f64 - player.r();
    let mut new_y = player.point().y + player.v().y;
    if !(min_y <= new_y && new_y <= max_y) {
        v.y = 0.0;
        new_y = new_y.max(min_y).min(max_y);
    }

    player.set_point(Point::new(new_x, new_y));
    player.set_v(v);

    if player.is_fast() {
        apply_viscosity(player);
    }

    if player.ttf() > 0 {
        let ttf = player.ttf() - 1;
        player.set_ttf(ttf);
    }
}

fn apply_viscosity(player: &mut Player) {
    let usual_speed = player.max_speed();
    let mut speed = player.speed();
    if speed - config().viscosity > usual_speed {
        speed -= config().viscosity;
    } else {
        speed = usual_speed;
        player.set_fast(false);
    }
    let v = player.v().with_length(speed);
    player.set_v(v);
}

fn collision_calc(a: &mut Player, b: &mut Player) {
    if a.is_fast() || b.is_fast() {
        return;
    }

    let qdist = a.point().qdist(b.point());
    let sum_r = a.r() + b.r();
    if qdist >= sum_r.powi(2) {
        return;
    }

    let collision_vector = a.point() - b.point();
    const MIN_COLLISION_VECTOR_LENGTH: f64 = 1e-9;
    if collision_vector.length() < MIN_COLLISION_VECTOR_LENGTH {
        return;
    }
    let collision_vector = collision_vector.unit();
    let collision_force = (1.0 - qdist.sqrt() / sum_r).powi(2) * config().collision_power;

    let sum_m = a.m() + b.m();
    let v = a.v() + collision_vector * (collision_force * b.m() / sum_m);
    a.set_v(v);
    let v = b.v() - collision_vector * (collision_force * a.m() / sum_m);
    b.set_v(v);
}

fn split_fragments(fragments: &mut [&mut Player]) -> Vec<Player> {
    fragments.sort_by(|a, b| {
        a.m()
            .partial_cmp(&b.m())
            .expect("incomparable mass")
            .reverse()
            .then_with(|| a.fragment_id().cmp(&b.fragment_id()).reverse())
    });
    let mut fragment_count = fragments.len() as i64;
    let mut max_fragment_id = fragments
        .iter()
        .map(|player| player.fragment_id())
        .max()
        .unwrap_or(0);
    let mut new_blobs = vec![];
    for player in fragments.iter_mut() {
        if player.can_split(fragment_count) {
            new_blobs.push(split_now(player, &mut max_fragment_id));
            fragment_count += 1;
        }
    }
    new_blobs
}

fn split_now(player: &mut Player, max_fragment_id: &mut FragmentId) -> Player {
    let new_m = player.m() / 2.0;
    let new_r = mass_to_radius(new_m);

    let new_blob = Player {
        id_: PlayerBlobId {
            player_id: player.player_id(),
            fragment_id: *max_fragment_id + 1,
        },
        point_: player.point(),
        m_: new_m,
        r_: new_r,
        v_: Point::from_polar(config().split_start_speed, player.angle()),
        is_fast_: true,
        ttf_: config().ticks_til_fusion,
    };

    player.set_fragment_id(*max_fragment_id + 2);
    player.set_m(new_m);
    player.set_r(new_r);
    player.set_ttf(config().ticks_til_fusion);

    *max_fragment_id = player.fragment_id();
    new_blob
}

fn shrink_now(player: &mut Player) {
    let new_m = player.m() - (player.m() - config().min_shrink_mass) * config().shrink_factor;
    player.set_m(new_m);
    player.set_r(mass_to_radius(new_m));
}

fn eat_food<F: Blob>(food: &[F], eaten: &mut HashSet<F::Id>, players: &mut [Player]) {
    for blob in food.iter() {
        if eaten.contains(blob.id()) {
            continue;
        }
        if let Some(i) = nearest_player(blob, |player| player.can_eat_blob(blob), players.iter()) {
            player_eat(&mut players[i], blob);
            eaten.insert(blob.id().clone());
        }
    }
}

fn eat_players(players: &mut [Player]) {
    let mut i = 0;
    while i < players.len() {
        if let Some(j) = nearest_player(
            &players[i],
            |eater| eater.can_eat_player(&players[i]),
            players.iter(),
        )
        {
            let (player, eater) = if i < j {
                let (left, right) = players.split_at_mut(j);
                (&left[i], &mut right[0])
            } else {
                let (left, right) = players.split_at_mut(i);
                (&right[0], &mut left[j])
            };
            player_eat(eater, player);
        } else {
            i += 1;
        }
    }
}

fn player_eat<F: Blob>(player: &mut Player, food: &F) {
    let new_m = player.m() + food.m();
    player.set_m(new_m);
}

fn nearest_player<'a, T, P, U>(target: &T, predicate: P, players: U) -> Option<usize>
where
    T: HasPoint,
    P: Fn(&Player) -> bool,
    U: IntoIterator<Item = &'a Player>,
{
    players
        .into_iter()
        .enumerate()
        .filter(|&(_, player)| predicate(player))
        .min_by(|&(_, a), &(_, b)| {
            // TODO: Incorporate depth calculation.
            a.point()
                .qdist(target.point())
                .partial_cmp(&b.point().qdist(target.point()))
                .expect("incomparable distances")
        })
        .map(|(i, _)| i)
}

fn update_by_mass(player: &mut Player) {
    let r = mass_to_radius(player.m());
    player.set_r(r);

    if !player.is_fast() {
        let v = player.v().limit_length(player.max_speed());
        player.set_v(v);
    }

    let x = player.point().x.max(player.r()).min(
        config().game_width as
            f64 - player.r(),
    );
    let y = player.point().y.max(player.r()).min(
        config().game_height as
            f64 - player.r(),
    );
    player.set_point(Point::new(x, y));
}

fn fusion(player: &mut Player, other: &Player) {
    let sum_m = player.m() + other.m();
    let player_influence = player.m() / sum_m;
    let other_influence = other.m() / sum_m;

    let point = player.point() * player_influence + other.point() * other_influence;
    player.set_point(point);

    let v = player.v() * player_influence + other.v() * other_influence;
    player.set_v(v);

    let m = player.m() + other.m();
    player.set_m(m);
}

fn burst_on(player: &mut Player, virus: &Virus) {
    let mut angle = 0.0;
    let dist = player.point().dist(virus.point());
    if dist > 0.0 {
        angle = ((player.point().y - virus.point().y) / dist).asin();
        if player.point().x < virus.point().x {
            angle = PI - angle;
        }
    }

    let speed = player.speed().min(player.max_speed());
    let v = Point::from_polar(speed, angle);
    player.set_v(v);

    let m = player.m() + config().burst_bonus;
    player.set_m(m);
}

fn burst_now(
    player: &mut Player,
    fragment_count: i64,
    max_fragment_id: &mut FragmentId,
) -> Vec<Player> {
    let new_fragment_count = ((player.m() / config().min_burst_mass).floor() as i64 - 1)
        .min(Player::rest_fragment_count(fragment_count));

    let new_m = player.m() / (new_fragment_count + 1) as Mass;
    let new_r = mass_to_radius(new_m);

    let new_blobs = (0..new_fragment_count)
        .map(|i| {
            let angle = player.angle() - config().burst_angle_spectrum / 2.0 +
                i as f64 * config().burst_angle_spectrum / new_fragment_count as f64;
            Player {
                id_: PlayerBlobId {
                    player_id: player.player_id(),
                    fragment_id: *max_fragment_id + 1 + i as FragmentId,
                },
                point_: player.point(),
                m_: new_m,
                r_: new_r,
                v_: Point::from_polar(config().burst_start_speed, angle),
                is_fast_: true,
                ttf_: config().ticks_til_fusion,
            }
        })
        .collect();

    let v = Point::from_polar(
        config().burst_start_speed,
        player.angle() + config().burst_angle_spectrum / 2.0,
    );
    player.set_v(v);
    player.set_fast(true);

    player.set_fragment_id(*max_fragment_id + 1 + new_fragment_count as FragmentId);
    player.set_m(new_m);
    player.set_r(new_r);
    player.set_ttf(config().ticks_til_fusion);

    *max_fragment_id = player.fragment_id();
    new_blobs
}

fn mass_to_radius(mass: Mass) -> f64 {
    config().radius_factor * mass.sqrt()
}
