use std::collections::{BTreeMap, HashSet};
use std::iter::FromIterator;
use std::mem;
use std::rc::Rc;

use models::*;
use command::Command;
use config::config;

#[derive(Debug)]
pub struct Mechanic {
    pub state: State,
    my_blobs: Vec<Player>,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub tick: i64,
    pub my_blobs: Rc<BTreeMap<PlayerBlobId, Player>>,
    pub eaten_food: HashSet<FoodId>,
    pub eaten_ejections: HashSet<EjectionId>,
    pub eaten_viruses: HashSet<VirusId>,
    pub eaten_enemies: HashSet<PlayerBlobId>,
}

impl State {
    pub fn new(tick: i64, my_blobs: Vec<Player>) -> State {
        State {
            tick,
            my_blobs: State::my_blobs_from_vec(my_blobs),
            eaten_food: Default::default(),
            eaten_ejections: Default::default(),
            eaten_viruses: Default::default(),
            eaten_enemies: Default::default(),
        }
    }

    fn my_blobs_from_vec(my_blobs: Vec<Player>) -> Rc<BTreeMap<PlayerBlobId, Player>> {
        Rc::new(BTreeMap::from_iter(
            my_blobs.into_iter().map(|me| (me.id().clone(), me)),
        ))
    }
}

impl Mechanic {
    pub fn new(state: &State) -> Mechanic {
        Mechanic {
            state: state.clone(),
            my_blobs: Default::default(),
        }
    }

    pub fn tick(
        &mut self,
        command: &Command,
        food: &[Food],
        ejections: &[Ejection],
        enemies: &[Player],
    ) {
        self.my_blobs = self.state.my_blobs.values().cloned().collect();

        self.apply_strategies(command);
        self.state.tick += 1;
        self.move_moveables();
        self.player_ejects();
        self.player_splits(command);

        if self.state.tick % config().shrink_every_tick == 0 {
            self.shrink_players();
        }
        self.eat_all(food, ejections, enemies);
        self.fuse_players();
        self.burst_on_viruses();

        self.update_players_radius();
        self.update_scores();
        self.split_viruses();

        let my_blobs = mem::replace(&mut self.my_blobs, Default::default());
        self.state.my_blobs = State::my_blobs_from_vec(my_blobs);
    }

    fn apply_strategies(&mut self, command: &Command) {
        // TODO: Move other players?
        for me in self.my_blobs.iter_mut() {
            apply_direct(me, command);
        }
    }

    fn move_moveables(&mut self) {
        // TODO: Move ejections?
        // TODO: Move viruses?

        for i in 0..self.my_blobs.len() {
            for j in (i + 1)..self.my_blobs.len() {
                let (left, right) = self.my_blobs.as_mut_slice().split_at_mut(j);
                collision_calc(&mut left[i], &mut right[0]);
            }
        }

        // TODO: Move other players?
        for me in self.my_blobs.iter_mut() {
            move_player(me);
        }
    }

    fn player_ejects(&mut self) {
        // TODO
    }

    fn player_splits(&mut self, command: &Command) {
        if command.split() {
            split_fragments(&mut self.my_blobs);
        }
    }

    fn shrink_players(&mut self) {
        for me in self.my_blobs.iter_mut() {
            if me.can_shrink() {
                shrink_now(me);
            }
        }
    }

    fn eat_all(&mut self, food: &[Food], ejections: &[Ejection], enemies: &[Player]) {
        eat(food, &mut self.state.eaten_food, &mut self.my_blobs);
        eat(
            ejections,
            &mut self.state.eaten_ejections,
            &mut self.my_blobs,
        );
        eat(enemies, &mut self.state.eaten_enemies, &mut self.my_blobs);

        // TODO: Other players eat food.

        for enemy in enemies.iter() {
            if self.state.eaten_enemies.contains(enemy.id()) {
                continue;
            }
            if let Some(i) = nearest_me(enemy, |me| enemy.can_eat(me), &self.my_blobs) {
                // TODO: Allow an enemy to eat multiple blobs.
                self.my_blobs.swap_remove(i);
            }
        }
    }

    fn fuse_players(&mut self) {}
    fn burst_on_viruses(&mut self) {}
    fn update_players_radius(&mut self) {}

    fn update_scores(&mut self) {}

    fn split_viruses(&mut self) {}

    //pub fn pastebin() {

    //    // TODO: who_need_fusion.

    //    // who_intersected_virus.
    //    if slow {
    //        for virus in self.viruses.iter() {
    //            if let Some(i) = find_nearest_me(virus, |me| me.can_burst(), my_blobs.as_ref()) {
    //                let me = my_blobs.swap_remove(i);
    //                my_blobs.extend(self.burst(me, virus));
    //            }
    //        }
    //    }

    //    // update_by_state: update r, limit v, split.
    //    for me in my_blobs.iter_mut() {
    //        me.update_r();
    //        me.limit_speed();
    //    }
    //    if command.split() {
    //        let mut max_fragment_id = my_blobs.iter().map(|me| me.id().fragment_id).max().expect(
    //            "max_fragment_id",
    //        );
    //        my_blobs = my_blobs
    //            .into_iter()
    //            .flat_map(|me| self.split(me, &mut max_fragment_id))
    //            .collect();
    //    }
    //}

    //fn burst(&self, me: Player, virus: &Virus) -> Vec<Player> {
    //    // TODO
    //    if virus.can_hurt(&me) {
    //        vec![] // Assume we die.
    //    } else {
    //        vec![me]
    //    }
    //}
}

fn apply_direct(me: &mut Player, command: &Command) {
    if me.is_fast() {
        return;
    }
    let max_speed = me.max_speed();
    let target_v = (command.point() - me.point()).with_length(max_speed);
    let v = me.v() + (target_v - me.v()) * (config().inertion_factor / me.m());
    let v = v.limit_length(max_speed);
    me.set_v(v);
}

fn move_player(me: &mut Player) {
    let mut v = me.v();

    let min_x = me.r();
    let max_x = config().game_width as f64 - me.r();
    let mut new_x = me.point().x + me.v().x;
    if !(min_x <= new_x && new_x <= max_x) {
        v.x = 0.0;
        new_x = new_x.max(min_x).min(max_x);
    }

    let min_y = me.r();
    let max_y = config().game_height as f64 - me.r();
    let mut new_y = me.point().y + me.v().y;
    if !(min_y <= new_y && new_y <= max_y) {
        v.y = 0.0;
        new_y = new_y.max(min_y).min(max_y);
    }

    me.set_point(Point::new(new_x, new_y));
    me.set_v(v);

    if me.is_fast() {
        apply_viscosity(me);
    }

    if me.ttf_ > 0 {
        me.ttf_ = me.ttf_ - 1;
    }
}

fn apply_viscosity(me: &mut Player) {
    let usual_speed = me.max_speed();
    let mut speed = me.speed();
    if speed - config().viscosity > usual_speed {
        speed -= config().viscosity;
    } else {
        speed = usual_speed;
        me.set_fast(false);
    }
    let v = me.v().with_length(speed);
    me.set_v(v);
}

fn collision_calc(me: &mut Player, other: &mut Player) {
    if me.is_fast() || other.is_fast() {
        return;
    }

    let qdist = me.point().qdist(other.point());
    let sum_r = me.r() + other.r();
    if qdist >= sum_r.powi(2) {
        return;
    }

    let collision_vector = me.point() - other.point();
    const MIN_COLLISION_VECTOR_LENGTH: f64 = 1e-9;
    if collision_vector.length() < MIN_COLLISION_VECTOR_LENGTH {
        return;
    }
    let collision_vector = collision_vector.unit();
    let collision_force = (1.0 - qdist.sqrt() / sum_r).powi(2) * config().collision_power;

    let sum_m = me.m() + other.m();
    let v = me.v() + collision_vector * (collision_force * other.m() / sum_m);
    me.set_v(v);
    let v = other.v() - collision_vector * (collision_force * me.m() / sum_m);
    other.set_v(v);
}

fn split_fragments(my_blobs: &mut Vec<Player>) {
    my_blobs.sort_by(|a, b| {
        a.m()
            .partial_cmp(&b.m())
            .expect("incomparable mass")
            .reverse()
            .then_with(|| a.id().fragment_id.cmp(&b.id().fragment_id).reverse())
    });
    let mut new_blobs = vec![];
    let mut fragment_count = my_blobs.len() as i64;
    let mut max_fragment_id = my_blobs.iter().map(|me| me.id().fragment_id).max().expect(
        "no blobs to split",
    );
    for me in my_blobs.iter_mut() {
        if me.can_split(fragment_count) {
            new_blobs.push(split_now(me, &mut max_fragment_id));
            fragment_count += 1;
        }
    }
    my_blobs.extend(new_blobs);
}

fn split_now(me: &mut Player, max_fragment_id: &mut u32) -> Player {
    let new_m = me.m() / 2.0;
    let new_r = mass_to_radius(new_m);

    let new_blob = Player {
        id_: PlayerBlobId {
            player_id: me.id().player_id,
            fragment_id: *max_fragment_id + 1,
        },
        point_: me.point(),
        m_: new_m,
        r_: new_r,
        v_: Some(Point::from_polar(config().split_start_speed, me.angle())),
        is_fast_: Some(true),
        ttf_: config().ticks_til_fusion,
    };

    me.id_.fragment_id = *max_fragment_id + 2;
    me.ttf_ = config().ticks_til_fusion;
    me.m_ = new_m;
    me.r_ = new_r;

    *max_fragment_id += 2;
    new_blob
}

fn shrink_now(me: &mut Player) {
    let new_m = me.m() - (me.m() - config().min_shrink_mass) * config().shrink_factor;
    me.set_m(new_m);
    me.set_r(mass_to_radius(new_m));
}

fn eat<F: Blob>(food: &[F], eaten: &mut HashSet<F::Id>, my_blobs: &mut [Player]) {
    for blob in food.iter() {
        if eaten.contains(blob.id()) {
            continue;
        }
        if let Some(i) = nearest_me(blob, |me| me.can_eat(blob), my_blobs) {
            eaten.insert(blob.id().clone());
            let ref mut me = my_blobs[i];
            let new_m = me.m() + blob.m();
            me.set_m(new_m);
        }
    }
}

fn nearest_me<T, P>(target: &T, predicate: P, my_blobs: &[Player]) -> Option<usize>
where
    T: HasPoint,
    P: Fn(&Player) -> bool,
{
    my_blobs
        .iter()
        .enumerate()
        .filter(|&(_, me)| predicate(me))
        .min_by(|&(_, a), &(_, b)| {
            // TODO: Incorporate depth calculation.
            a.point()
                .qdist(target.point())
                .partial_cmp(&b.point().qdist(target.point()))
                .expect("incomparable distances")
        })
        .map(|(i, _)| i)
}

fn mass_to_radius(mass: f64) -> f64 {
    config().radius_factor * mass.sqrt()
}
