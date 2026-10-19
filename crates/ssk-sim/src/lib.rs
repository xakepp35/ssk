//! Platform independent, fixed frequency game simulation.
pub mod constants;
pub use constants::*;
mod math;
pub use math::*;
mod combat;
pub use combat::*;
mod field;
pub use field::*;
mod enemy;
pub use enemy::*;
mod loot;
mod sector;
pub use loot::*;
pub use sector::*;

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorldPos {
    pub sector: [i64; 2],
    pub local: V2,
}
impl WorldPos {
    pub fn relative_to(self, origin: [i64; 2]) -> V2 {
        V2::new(
            (self.sector[0] as i128 - origin[0] as i128) as f32 * SECTOR_SIZE,
            (self.sector[1] as i128 - origin[1] as i128) as f32 * SECTOR_SIZE,
        ) + self.local
    }
    pub fn normalized(mut self) -> Self {
        for axis in 0..2 {
            let v = if axis == 0 {
                &mut self.local.x
            } else {
                &mut self.local.y
            };
            let shift = (*v / SECTOR_SIZE).floor() as i64;
            self.sector[axis] = self.sector[axis].saturating_add(shift);
            *v = v.rem_euclid(SECTOR_SIZE);
        }
        self
    }
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct InputFrame {
    pub movement: V2,
    pub aim: f32,
    pub fire: bool,
    pub ability: bool,
    pub weapon: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShipClass {
    Spark,
    #[default]
    Cadet,
    Thunder,
}

impl ShipClass {
    pub const ALL: [Self; 3] = [Self::Spark, Self::Cadet, Self::Thunder];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Spark => "SPARK",
            Self::Cadet => "CADET",
            Self::Thunder => "THUNDER",
        }
    }
    pub fn stats(self) -> (f32, f32, f32, f32) {
        SHIP_STATS[self.index()]
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ship {
    pub id: u64,
    pub class: ShipClass,
    pub pos: V2,
    pub previous: V2,
    pub velocity: V2,
    pub angle: f32,
    pub hp: f32,
    pub energy: f32,
    pub cooldowns: [f32; WEAPON_COUNT],
    pub quiet: f32,
    pub weapon: WeaponKind,
    pub bump_cooldown: f32,
    pub ability_active: bool,
    pub ability_latched: bool,
    pub invulnerable: f32,
}
impl Ship {
    pub fn new(id: u64, class: ShipClass, pos: V2) -> Self {
        Self {
            id,
            class,
            pos,
            previous: pos,
            velocity: V2::ZERO,
            angle: 0.,
            hp: class.stats().0,
            energy: ENERGY_BASE,
            cooldowns: [0.; WEAPON_COUNT],
            quiet: 0.,
            weapon: WeaponKind::MachineGun,
            bump_cooldown: 0.,
            ability_active: false,
            ability_latched: false,
            invulnerable: 0.,
        }
    }
    pub fn move_step(&mut self, input: InputFrame) {
        self.invulnerable = (self.invulnerable - DT).max(0.);
        if input.fire || input.ability {
            self.invulnerable = 0.;
        }
        self.previous = self.pos;
        self.ability_step(input.ability);
        let (_, mut speed, mut acceleration, _) = self.class.stats();
        if matches!(self.class, ShipClass::Spark) && self.ability_active {
            speed *= BOOST_SPEED;
            acceleration *= BOOST_ACCELERATION;
        }
        let movement = input.movement.clamp_length(1.);
        if movement.length_squared() > MOVEMENT_EPSILON_SQ {
            self.velocity += movement * (acceleration * DT);
        } else {
            self.velocity *= (-DRAG * DT).exp();
        }
        self.velocity = self.velocity.clamp_length(speed);
        self.pos += self.velocity * DT;
        self.angle = input.aim;
    }
    pub fn ability_step(&mut self, held: bool) {
        if !held {
            self.ability_active = false;
            self.ability_latched = false;
        } else if !self.ability_latched {
            self.ability_latched = true;
            if self.energy >= ABILITY_MIN_ENERGY {
                self.energy -= ABILITY_START_COST;
                self.ability_active = true;
            }
        }
        if self.ability_active {
            self.quiet = 0.;
            let rate = ABILITY_DRAIN[self.class.index()];
            self.energy = (self.energy - rate * DT).max(0.);
            if self.energy <= 0. {
                self.ability_active = false;
            }
        }
    }
    pub fn damage(&mut self, amount: f32) -> f32 {
        if self.invulnerable > 0. {
            return 0.;
        }
        let absorbed = if matches!(self.class, ShipClass::Cadet) && self.ability_active {
            self.energy.min(amount * SHIELD_ABSORPTION)
        } else {
            0.
        };
        self.energy -= absorbed;
        if self.energy <= 0. {
            self.ability_active = false;
        }
        let hull = amount - absorbed;
        self.hp = (self.hp - hull).max(0.);
        hull
    }
}
#[derive(Clone)]
pub struct World {
    pub player: Ship,
    pub origin: [i64; 2],
    pub tick: u64,
    pub seed: u64,
    pub bullets: Vec<Bullet>,
    pub asteroids: Vec<Asteroid>,
    pub events: Vec<WorldEvent>,
    pub rng: Rng,
    pub next_id: u64,
    pub grid: SpatialGrid,
    pub loaded: std::collections::HashSet<[i64; 2]>,
    pub origin_shift: V2,
    pub npcs: Vec<Npc>,
    pub score: u64,
    pub next_spawn: f32,
    pub pickups: Vec<Pickup>,
    pub combo: f32,
    pub last_kill: Option<f32>,
    pub spawn_radius: f32,
    pub duel: bool,
    pub last_attacker: u64,
    pub last_shot: u64,
    pub shared_authority: bool,
    pub shared_replica: bool,
    pub targets: Vec<Ship>,
}
impl World {
    pub fn new(seed: u64, class: ShipClass) -> Self {
        Self {
            player: Ship::new(1, class, V2::ZERO),
            origin: [0, 0],
            tick: 0,
            seed,
            bullets: Vec::with_capacity(MAX_BULLETS),
            asteroids: Vec::with_capacity(ASTEROID_RESERVE),
            events: Vec::with_capacity(MAX_WORLD_EVENTS),
            rng: Rng(seed),
            next_id: 2,
            grid: SpatialGrid::default(),
            loaded: field::empty_loaded(),
            origin_shift: V2::ZERO,
            npcs: Vec::with_capacity(MAX_NPCS),
            score: 0,
            next_spawn: NPC_INITIAL_SPAWN_SECONDS,
            pickups: Vec::with_capacity(MAX_PICKUPS),
            combo: 1.,
            last_kill: None,
            spawn_radius: DEFAULT_SPAWN_RADIUS,
            duel: false,
            last_attacker: 0,
            last_shot: 0,
            shared_authority: false,
            shared_replica: false,
            targets: Vec::new(),
        }
    }
    pub fn step(&mut self, input: InputFrame) {
        if self.player.hp <= 0. {
            return;
        }
        self.tick += 1;
        if !self.shared_authority {
            self.player.move_step(input);
        }
        self.field_step();
        if !self.duel {
            self.enemies_step();
        }
        self.combat(input);
        self.resources_step();
    }
    pub fn duel(seed: u64, class: ShipClass, id: u64) -> Self {
        let mut world = Self::new(seed, class);
        world.duel = true;
        world.player.id = id;
        world.player.pos = V2::new(SECTOR_SIZE / 2., SECTOR_SIZE / 2.)
            + V2::from_angle(
                (id % DUEL_SPAWN_HASH_MODULUS) as f32 / DUEL_SPAWN_HASH_MODULUS as f32
                    * std::f32::consts::TAU,
            ) * DUEL_SPAWN_RADIUS;
        world.player.previous = world.player.pos;
        world.player.invulnerable = SPAWN_PROTECTION_SECONDS;
        world
    }
}

mod events;
pub use events::*;
mod replication;
pub use replication::*;
mod validation;
pub use validation::*;

mod motion;
mod shared;
pub use shared::*;

mod multiplayer;
pub use multiplayer::*;
