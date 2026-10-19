//! Platform independent, fixed frequency game simulation.
pub const TICK_HZ: u64 = 240;
pub const DT: f32 = 1.0 / TICK_HZ as f32;
pub const SECTOR_SIZE: f32 = 4096.;
mod math;
pub use math::*;
mod combat;
pub use combat::*;
mod field;
pub use field::*;
mod enemy;
pub use enemy::*;

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorldPos { pub sector: [i64; 2], pub local: V2 }
impl WorldPos {
    pub fn relative_to(self, origin: [i64; 2]) -> V2 {
        V2::new((self.sector[0] - origin[0]) as f32 * SECTOR_SIZE, (self.sector[1] - origin[1]) as f32 * SECTOR_SIZE) + self.local
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct InputFrame { pub movement: V2, pub aim: f32, pub fire: bool, pub ability: bool, pub weapon: usize }

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum ShipClass {
    Spark,
    #[default]
    Cadet,
    Thunder,
}

impl ShipClass {
    pub const ALL: [Self; 3] = [Self::Spark, Self::Cadet, Self::Thunder];
    pub fn index(self) -> usize { self as usize }
    pub fn name(self) -> &'static str { match self { Self::Spark => "SPARK", Self::Cadet => "CADET", Self::Thunder => "THUNDER" } }
    pub fn stats(self) -> (f32, f32, f32, f32) {
        match self { Self::Spark => (750., 520., 2800., 13.), Self::Cadet => (1000., 420., 1900., 17.), Self::Thunder => (1400., 320., 1200., 22.) }
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ship {
    pub id: u64, pub class: ShipClass, pub pos: V2, pub previous: V2,
    pub velocity: V2, pub angle: f32, pub hp: f32, pub energy: f32,
    pub cooldowns: [f32;3], pub quiet:f32, pub weapon:WeaponKind,
    pub bump_cooldown:f32,
}
impl Ship {
    pub fn new(id: u64, class: ShipClass, pos: V2) -> Self {
        Self { id, class, pos, previous: pos, velocity: V2::ZERO, angle: 0., hp: class.stats().0, energy: 1000., cooldowns:[0.;3], quiet:0., weapon:WeaponKind::MachineGun, bump_cooldown:0. }
    }
    pub fn move_step(&mut self, input: InputFrame) {
        self.previous = self.pos;
        let (_, speed, acceleration, _) = self.class.stats();
        let movement = input.movement.clamp_length(1.);
        if movement.length_squared() > 0.001 { self.velocity += movement * (acceleration * DT); }
        else { self.velocity *= (-5.5 * DT).exp(); }
        self.velocity = self.velocity.clamp_length(speed);
        self.pos += self.velocity * DT;
        self.angle = input.aim;
    }
}
pub struct World {
    pub player: Ship, pub origin: [i64; 2], pub tick: u64, pub seed: u64,
    pub bullets:Vec<Bullet>, pub asteroids:Vec<Asteroid>, pub events:Vec<WorldEvent>,
    pub rng:Rng, pub next_id:u64,
    pub grid:SpatialGrid, pub loaded:std::collections::HashSet<[i64;2]>, pub origin_shift:V2,
    pub npcs:Vec<Npc>, pub score:u64, pub next_spawn:f32,
}
impl World {
    pub fn new(seed: u64, class: ShipClass) -> Self {
        Self { player: Ship::new(1, class, V2::ZERO), origin: [0, 0], tick: 0, seed,
            bullets:Vec::with_capacity(4096), asteroids:Vec::with_capacity(512), events:Vec::with_capacity(512), rng:Rng(seed), next_id:2,
            grid:SpatialGrid::default(),loaded:field::empty_loaded(),origin_shift:V2::ZERO,
            npcs:Vec::with_capacity(24),score:0,next_spawn:2. }
    }
    pub fn step(&mut self, input: InputFrame) {
        if self.player.hp<=0. { return; }
        self.tick += 1; self.player.move_step(input); self.field_step(); self.enemies_step(); self.combat(input);
    }
}
