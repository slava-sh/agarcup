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

        self.apply_strategies();
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

        let mut my_blobs = Default::default();
        mem::swap(&mut my_blobs, &mut self.my_blobs);
        self.state.my_blobs = State::my_blobs_from_vec(my_blobs);
    }

    fn apply_strategies(&mut self) {}
    fn move_moveables(&mut self) {}
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
    //    // Following the oringial mechanic.
    //    // apply_strategies: update v.
    //    for me in my_blobs.iter_mut() {
    //        me.update_v(command);
    //    }

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

    //    // move_moveables: collide (TODO), move, apply viscosity, update ttf.
    //    for me in my_blobs.iter_mut() {
    //        me.apply_v();
    //        me.apply_viscosity();
    //        if let Some(ttf) = me.ttf_ {
    //            if ttf > 0 {
    //                me.ttf_ = Some(ttf - 1);
    //            }
    //        }
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
