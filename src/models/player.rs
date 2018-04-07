use models::*;

#[derive(Debug, Clone)]
pub struct Player {
    pub id_: PlayerBlobId,
    pub point_: Point,
    pub m_: f64,
    pub r_: f64,
    pub v_: Option<Point>,
    pub is_fast_: Option<bool>,
    pub ttf_: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerBlobId {
    pub player_id: u32,
    pub fragment_id: u32,
}

impl HasPoint for Player {
    fn point(&self) -> Point {
        self.point_
    }
}

impl Circle for Player {
    fn r(&self) -> f64 {
        self.r_
    }
}

impl Blob for Player {
    type Id = PlayerBlobId;

    fn id(&self) -> &PlayerBlobId {
        &self.id_
    }

    fn m(&self) -> f64 {
        self.m_
    }
}

impl Player {
    pub fn can_eat<Other: Blob>(&self, other: &Other) -> bool {
        if !(self.m() > other.m() * config().mass_eat_factor) {
            return false;
        }
        let dist = self.point().dist(other.point());
        let min_r = dist - other.r() + other.r() * 2.0 * config().diam_eat_factor;
        min_r < self.r()
    }

    pub fn can_see<Other: Circle>(&self, other: &Other) -> bool {
        let p = self.point() + Point::from_polar(config().vis_shift, self.angle());
        let vision_radius = self.r() * config().vis_factor; // TODO: Not always true.
        let max_dist = vision_radius + other.r();
        other.point().qdist(p) < max_dist.powi(2)
    }

    pub fn can_fuse(&self, other: &Player) -> bool {
        self.ttf() == 0 && other.ttf() == 0 &&
            self.point().qdist(other.point()) <= (self.r() + other.r()).powi(2)
    }

    pub fn can_burst(&self, yet_cnt: i64) -> bool {
        if self.m() < config().min_burst_mass * 2.0 || Player::rest_fragment_count(yet_cnt) <= 0 {
            return false;
        }
        let frags_cnt = (self.m() / config().min_burst_mass).floor() as i64;
        frags_cnt > 1
    }

    pub fn can_split(&self, yet_cnt: i64) -> bool {
        Player::rest_fragment_count(yet_cnt) > 0 && self.m() > config().min_split_mass
    }

    pub fn max_speed(&self) -> f64 {
        config().speed_factor / self.m().sqrt()
    }

    pub fn can_shrink(&self) -> bool {
        self.m() > config().min_shrink_mass
    }

    pub fn speed(&self) -> f64 {
        self.v().length()
    }

    pub fn angle(&self) -> f64 {
        self.v().angle()
    }

    pub fn set_point(&mut self, point: Point) {
        self.point_ = point;
    }

    pub fn v(&self) -> Point {
        self.v_.expect("v not set")
    }

    pub fn set_v(&mut self, v: Point) {
        self.v_ = Some(v);
    }

    pub fn is_fast(&self) -> bool {
        self.is_fast_.expect("is_fast not set")
    }

    pub fn set_fast(&mut self, is_fast: bool) {
        self.is_fast_ = Some(is_fast);
    }

    pub fn set_m(&mut self, m: f64) {
        self.m_ = m;
    }

    pub fn set_r(&mut self, r: f64) {
        self.r_ = r;
    }

    pub fn ttf(&self) -> i64 {
        self.ttf_
    }

    pub fn set_ttf(&mut self, ttf: i64) {
        self.ttf_ = ttf;
    }

    pub fn player_id(&self) -> u32 {
        self.id_.player_id
    }

    pub fn set_player_id(&mut self, player_id: u32) {
        self.id_.player_id = player_id;
    }

    pub fn fragment_id(&self) -> u32 {
        self.id_.fragment_id
    }

    pub fn set_fragment_id(&mut self, fragment_id: u32) {
        self.id_.fragment_id = fragment_id;
    }

    pub fn rest_fragment_count(existing_fragment_count: i64) -> i64 {
        config().max_frags_cnt - existing_fragment_count
    }
}
