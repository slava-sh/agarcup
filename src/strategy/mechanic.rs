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
    pub eaten_food: Rc<HashSet<FoodId>>,
    pub eaten_ejections: Rc<HashSet<EjectionId>>,
    pub eaten_viruses: Rc<HashSet<VirusId>>,
    pub eaten_enemies: Rc<HashSet<PlayerBlobId>>,
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

    pub fn tick(&mut self, command: &Command) {
        self.my_blobs = self.state.my_blobs.values().cloned().collect();

        self.apply_strategies(command);
        self.state.tick += 1;
        self.move_moveables();
        self.player_ejects();
        self.player_splits();

        if self.state.tick % config().shrink_every_tick == 0 {
            self.shrink_players();
        }
        self.eat_all();
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

    fn player_ejects(&mut self) {}

    fn player_splits(&mut self) {}
    fn shrink_players(&mut self) {}
    fn eat_all(&mut self) {}
    fn fuse_players(&mut self) {}
    fn burst_on_viruses(&mut self) {}
    fn update_players_radius(&mut self) {}
    fn update_scores(&mut self) {}
    fn split_viruses(&mut self) {}

    //pub fn pastebin() {
    //    // shrink_players.
    //    let tick = state.tick + 1;
    //    if tick % config().shrink_every_tick == 0 {
    //        for me in my_blobs.iter_mut() {
    //            me.shrink();
    //        }
    //    }

    //    // who_is_eaten: update m.
    //    let (eaten_food, eaten_ejections, eaten_enemies) = if slow {
    //        fn eat<B: Blob>(
    //            blobs: &Vec<B>,
    //            eaten: &Rc<HashSet<B::Id>>,
    //            my_blobs: &mut [Player],
    //        ) -> HashSet<B::Id> {
    //            let mut eaten = eaten.as_ref().clone();
    //            for blob in blobs.iter() {
    //                if eaten.contains(blob.id()) {
    //                    continue;
    //                }
    //                if let Some(i) = find_nearest_me(blob, |me| me.can_eat(blob), my_blobs) {
    //                    eaten.insert(blob.id().clone());
    //                    my_blobs[i].m_ += blob.m();
    //                }
    //            }
    //            eaten
    //        }

    //        let eaten_food = eat(&self.food, &state.eaten_food, my_blobs.as_mut());
    //        let eaten_ejections = eat(&self.ejections, &state.eaten_ejections, my_blobs.as_mut());
    //        let eaten_enemies = eat(&self.enemies, &state.eaten_enemies, my_blobs.as_mut());

    //        for enemy in self.enemies.iter() {
    //            if eaten_enemies.contains(enemy.id()) {
    //                continue;
    //            }
    //            if let Some(i) = find_nearest_me(enemy, |me| enemy.can_eat(me), my_blobs.as_ref()) {
    //                my_blobs.swap_remove(i); // Die.
    //            }
    //        }

    //        (
    //            Rc::new(eaten_food),
    //            Rc::new(eaten_ejections),
    //            Rc::new(eaten_enemies),
    //        )
    //    } else {
    //        (
    //            Rc::clone(&state.eaten_food),
    //            Rc::clone(&state.eaten_ejections),
    //            Rc::clone(&state.eaten_enemies),
    //        )
    //    };

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

    //fn split(&self, me: Player, max_fragment_id: &mut u32) -> Vec<Player> {
    //    if !me.can_split() {
    //        return vec![me];
    //    }
    //    let m = me.m() / 2.0;
    //    let v = Point::from_polar(config().split_start_speed, me.angle());

    //    let mut me1 = Player {
    //        id_: PlayerBlobId {
    //            player_id: me.id().player_id,
    //            fragment_id: *max_fragment_id + 1,
    //        },
    //        point_: me.point(),
    //        m_: m,
    //        r_: 0.0,
    //        v_: Some(v),
    //        is_fast_: Some(true),
    //        ttf_: Some(config().ticks_til_fusion),
    //    };
    //    me1.update_r();

    //    let mut me2 = me;
    //    me2.id_.fragment_id = *max_fragment_id + 2;
    //    me2.m_ = m;
    //    me2.update_r();
    //    me2.ttf_ = Some(config().ticks_til_fusion);

    //    *max_fragment_id += 2;
    //    vec![me1, me2]
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
