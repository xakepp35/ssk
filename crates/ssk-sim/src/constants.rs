//! Authoritative timing and game tuning. Never derive logic timing from render FPS.
pub const TICK_PER_SEC: u64 = 360;
pub const TICK_PER_PACKET: u64 = 6;
pub const PACKET_PER_SEC: u64 = TICK_PER_SEC / TICK_PER_PACKET;
pub const DT: f32 = 1.0 / TICK_PER_SEC as f32;
pub const TICK_DURATION: std::time::Duration =
    std::time::Duration::from_nanos(1_000_000_000 / TICK_PER_SEC);
pub const PACKET_DURATION: std::time::Duration =
    std::time::Duration::from_nanos(1_000_000_000 * TICK_PER_PACKET / TICK_PER_SEC);
pub const SECTOR_SIZE: f32 = 4096.;
pub const MAX_CATCH_UP_TICKS: usize = TICK_PER_PACKET as usize * 4;
pub const MAX_LOGIC_LAG: std::time::Duration = std::time::Duration::from_millis(100);
pub const HISTORY_SECONDS: f64 = 2.;
pub const HISTORY_CAPACITY: usize = PACKET_PER_SEC as usize * HISTORY_SECONDS as usize + 2;
pub const LAG_COMPENSATION_SECONDS: f32 = 0.2;
pub const SHOT_HISTORY_TICKS: u64 = (TICK_PER_SEC as f32 * LAG_COMPENSATION_SECONDS) as u64;
pub const INTERPOLATION_SECONDS: f64 = TICK_PER_PACKET as f64 / TICK_PER_SEC as f64 * 3.;
pub const EXTRAPOLATION_SECONDS: f64 = 0.1;
const _: () =
    assert!(TICK_PER_SEC > 0 && TICK_PER_PACKET > 0 && TICK_PER_SEC % TICK_PER_PACKET == 0);

// Ship balance: health, top speed, acceleration, collision radius.
pub const SHIP_STATS: [(f32, f32, f32, f32); 3] = [
    (750., 520., 2800., 13.),
    (1000., 420., 1900., 17.),
    (1400., 320., 1200., 22.),
];
pub const WEAPON_COUNT: usize = 3;
pub const ENERGY_BASE: f32 = 1000.;
pub const ENERGY_MAX: f32 = ENERGY_BASE * 2.;
pub const ENERGY_REGEN: f32 = 80.;
pub const ENERGY_REGEN_DELAY: f32 = 0.6;
pub const HEALTH_OVERCAP: f32 = 2.;
pub const HEALTH_DECAY: f32 = 0.025;
pub const ABILITY_MIN_ENERGY: f32 = 160.;
pub const ABILITY_START_COST: f32 = 128.;
pub const ABILITY_DRAIN: [f32; 3] = [64., 16., 8.];
pub const BOOST_SPEED: f32 = 1.6;
pub const BOOST_ACCELERATION: f32 = 2.4;
pub const MAX_SHIP_SPEED: f32 = SHIP_STATS[0].1 * BOOST_SPEED;
pub const QUAD_DAMAGE: f32 = 4.;
pub const SHIELD_ABSORPTION: f32 = 0.5;
pub const MOVEMENT_EPSILON_SQ: f32 = 0.001;
pub const DRAG: f32 = 5.5;
pub const MAX_BULLETS: usize = 4096;
pub const MAX_WORLD_EVENTS: usize = 512;
pub const ASTEROID_RESERVE: usize = 512;
pub const MAX_NPCS: usize = 24;
pub const MAX_PICKUPS: usize = 256;
pub const DEFAULT_SPAWN_RADIUS: f32 = 1200.;
pub const DUEL_SPAWN_RADIUS: f32 = 240.;
pub const DUEL_SPAWN_HASH_MODULUS: u64 = 100003;
pub const SPAWN_PROTECTION_SECONDS: f32 = 1.;
pub const RESPAWN_SECONDS: f32 = 2.;
pub const GRID_CELL_SIZE: f32 = 128.;
pub const MUZZLE_OFFSET: f32 = 5.;
pub const COLLISION_EPSILON: f32 = 1e-8;
pub const NORMAL_EPSILON: f32 = 0.0001;
pub const DRY_FIRE_TICKS: u64 = TICK_PER_SEC * 150 / 1000;
pub const PELLET_JITTER_MIN: f32 = 0.1;
pub const PELLET_JITTER_MAX: f32 = 0.9;
pub const SHIELD_IMPACT_DAMAGE: f32 = 20.;
pub const COMBO_SECONDS: f32 = 4.;
pub const COMBO_INCREMENT: f32 = 0.25;
pub const COMBO_MAX: f32 = 2.;
pub const PICKUP_LIFETIME: f32 = 15.;
pub const PICKUP_MAGNET_RADIUS: f32 = 180.;
pub const PICKUP_MAGNET_SPEED: f32 = 100.;
pub const PICKUP_MAGNET_ACCELERATION: f32 = 2.;
pub const PICKUP_REACH: f32 = 12.;
pub const PICKUP_CLAIM_REACH: f32 = 60.;
pub const PICKUP_REQUEST_REACH: f32 = 18.;
pub const PICKUP_MAX_AMOUNT: f32 = 300.;
pub const NPC_LOOT_ENERGY_CHANCE: f32 = 0.7;
pub const NPC_LOOT_HEALTH_THRESHOLD: f32 = 0.85;
pub const ROCK_LOOT_ENERGY_CHANCE: f32 = 0.1;
pub const ROCK_LOOT_HEALTH_THRESHOLD: f32 = 0.13;
pub const NPC_ENERGY_PICKUP: f32 = 300.;
pub const NPC_HEALTH_PICKUP: f32 = 250.;
pub const ROCK_ENERGY_PICKUP: f32 = 150.;
pub const ROCK_HEALTH_PICKUP: f32 = 100.;
pub const NPC_INITIAL_COUNT: usize = 2;
pub const NPC_COUNT_INTERVAL: f32 = 20.;
pub const NPC_DIRECTOR_LIMIT: usize = 12;
pub const NPC_HEAVY_AFTER: f32 = 60.;
pub const NPC_HEAVY_THRESHOLD: f32 = 0.7;
pub const NPC_MEDIUM_THRESHOLD: f32 = 0.45;
pub const NPC_HEALTH_SCALE: f32 = 0.6;
pub const NPC_HEALTH_RAMP_SECONDS: f32 = 1200.;
pub const NPC_HEALTH_RAMP_MAX: f32 = 1.5;
pub const NPC_REACTION_SECONDS: f32 = 0.25;
pub const NPC_REACTION_JITTER: f32 = 0.15;
pub const NPC_TELEGRAPH_SECONDS: f32 = 0.8;
pub const NPC_SPAWN_SECONDS: f32 = 3.;
pub const NPC_INITIAL_SPAWN_SECONDS: f32 = 2.;
pub const NPC_MAX_SHOOTERS: usize = 3;
pub const NPC_TACTICS: [(f32, usize, f32); 3] = [(500., 0, 0.85), (250., 1, 0.2), (750., 2, 0.25)];
pub const NPC_RETREAT_HEALTH: f32 = 0.25;
pub const NPC_RETREAT_RANGE: f32 = 500.;
pub const NPC_PHASE_ID_SCALE: f32 = 0.37;
pub const NPC_PHASE_SECONDS: f32 = 3.;
pub const NPC_BOOST_RANGE: f32 = 900.;
pub const NPC_QUAD_RANGE: f32 = 950.;
pub const NPC_QUAD_SECONDS: f32 = 0.75;
pub const NPC_AIM_LEAD: f32 = 0.6;
pub const NPC_MOVEMENT_RANGE: f32 = 200.;
pub const NPC_AIM_ERROR: f32 = 0.10;
pub const NPC_FIRE_MARGIN: f32 = 350.;
pub const NPC_FIRE_SECONDS: f32 = 1.1;
pub const NPC_AVOID_QUERY: f32 = 150.;
pub const NPC_AVOID_MARGIN: f32 = 140.;
pub const NPC_AVOID_FORCE: f32 = 70.;
pub const NPC_BUMP_SPEED: f32 = 80.;
pub const NPC_DAMAGE_SCALE: f32 = 0.6;
pub const NPC_SCORE: [u64; 3] = [100, 150, 250];
pub const NPC_DESTRUCTION_SIZE: f32 = 30.;
pub const NPC_DESPAWN_RANGE: f32 = 3500.;
pub const DUEL_FRAG_SCORE: u64 = 500;
pub const VALIDATION_SPEED_MARGIN: f32 = 3.;
pub const VALIDATION_POSITION_MARGIN: f32 = 12.;
pub const VALIDATION_MUZZLE_MARGIN: f32 = 8.;
pub const VALIDATION_ENERGY_MARGIN: f32 = 0.1;
pub const VALIDATION_MAX_GAP_TICKS: u64 = TICK_PER_SEC * 10;
pub const VALIDATION_SHOT_MEMORY: usize = 128;
pub const MAX_REMOTE_SHOT_RANGE: f32 = 5000.;
pub const REPLICA_STALE_SECONDS: f64 = 5.;
pub const SHARED_MEMBER_SECONDS: f64 = 2.;
pub const SHARED_IMAGE_SECONDS: f64 = 5.;
pub const LEASE_SECONDS: f64 = 2.;
pub const LEASE_RENEW_SECONDS: f64 = 0.5;
pub const SHARED_SNAPSHOT_SECONDS: f64 = 0.2;
pub const SHARED_PREDICTION_SECONDS: f64 = SHARED_SNAPSHOT_SECONDS * 2.;
pub const SHARED_CORRECTION_SECONDS: f64 = SHARED_SNAPSHOT_SECONDS / 2.;
pub const SHARED_VISUAL_SNAP_DISTANCE: f32 = 512.;
pub const CLAIM_RETRY_SECONDS: f64 = 1.;
pub const CLAIM_MEMORY_SECONDS: f64 = 3.;
pub const SHARED_EVENT_MEMORY: usize = 8192;
pub const SHARED_STEP_MAX_SECONDS: f32 = 0.1;

pub fn weapon_stats(kind: crate::WeaponKind) -> crate::WeaponStats {
    use crate::WeaponStats;
    match kind {
        crate::WeaponKind::MachineGun => WeaponStats {
            damage: 64.0,
            interval: 0.0625,
            energy: 8.0,
            speed: 2048.,
            radius: 2.0,
            life: 2.0,
            pellets: 1,
            spread: 2f32.to_radians(),
        },
        crate::WeaponKind::Shotgun => WeaponStats {
            damage: 48.0,
            interval: 0.5,
            energy: 64.0,
            speed: 2048.,
            radius: 5.0,
            life: 0.5,
            pellets: 16,
            spread: 8f32.to_radians(),
        },
        crate::WeaponKind::Plasma => WeaponStats {
            damage: 256.,
            interval: 0.75,
            energy: 96.0,
            speed: 1024.,
            radius: 16.,
            life: 1.6,
            pellets: 1,
            spread: 1f32.to_radians(),
        },
    }
}

pub const ROCK_KEEP_SECTORS: f32 = 2.5;
pub const ROCKS_PER_SECTOR: usize = 40;
pub const ROCK_SPAWN_CLEARANCE: f32 = 500.;
pub const ROCK_RADIUS: std::ops::Range<f32> = 30. ..64.;
pub const ROCK_HEALTH: f32 = 360.;
pub const ROCK_SPEED: std::ops::Range<f32> = 3. ..20.;
pub const ROCK_SPIN: f32 = 0.3;
pub const ROCK_CHILD_OFFSET: f32 = 0.45;
pub const ROCK_CHILD_RADIUS: f32 = 0.48;
pub const ROCK_CHILD_HEALTH: f32 = 100.;
pub const ROCK_CHILD_SPEED: f32 = 45.;
pub const ROCK_CHILD_SPIN: f32 = 0.8;
pub const ROCK_SCORE: u64 = 30;
pub const ROCK_CHILD_SCORE: u64 = 10;
pub const CONTACT_EPSILON: f32 = 0.001;
pub const CONTACT_SEPARATION: f32 = 0.1;
pub const CONTACT_RESTITUTION: f32 = 1.6;
pub const CONTACT_DAMAGE_SPEED: f32 = 80.;
pub const CONTACT_DAMAGE_SCALE: f32 = 0.2;
pub const CONTACT_DAMAGE_MAX: f32 = 100.;
pub const CONTACT_COOLDOWN: f32 = 0.35;
pub const PLAYER_ID_BIT: u64 = 1 << 63;
pub const SECTOR_HASH_X: u64 = 0x517cc1b727220a95;
pub const SECTOR_HASH_Y: u64 = 0x9e3779b97f4a7c15;
pub const SECTOR_ID_MASK: u64 = 0x7fffffff;
pub const SECTOR_ID_SHIFT: u32 = 32;
pub const SECTOR_MAX_ROCKS: usize = 160;
pub const SECTOR_MAX_POSITION: f32 = SECTOR_SIZE * 2.;
pub const SECTOR_MAX_VELOCITY: f32 = 5000.;
pub const SECTOR_MAX_ROCK_RADIUS: f32 = 100.;
pub const SECTOR_SPAWN_RADIUS: f32 = 1000.;
pub const SECTOR_PROXY_HEALTH: f32 = 1e9;
pub const SECTOR_PROXY_PROTECTION: f32 = 1000.;
pub const SHARED_MAGNET_SPEED: f32 = 200.;
pub const SHARED_SHOT_MARGIN: f32 = 1600.;
pub const LEASE_CLOCK_SKEW_MILLIS: u64 = 500;
pub const LEASE_MAX_FUTURE_MILLIS: u64 = (LEASE_SECONDS * 1000.) as u64 + LEASE_CLOCK_SKEW_MILLIS;
pub const INTERPOLATION_EPSILON: f64 = 0.00001;
pub const RNG_INCREMENT: u64 = 0x9e3779b97f4a7c15;
pub const RNG_MULTIPLIERS: [u64; 2] = [0xbf58476d1ce4e5b9, 0x94d049bb133111eb];
pub const RNG_MIX_SHIFTS: [u32; 3] = [30, 27, 31];
pub const RNG_FLOAT_SHIFT: u32 = 40;
pub const RNG_FLOAT_SCALE: f32 = 16777216.;

pub const COMBAT_EVENT_QUEUE: usize = 256;
pub const SHOT_EVENT_MEMORY: usize = 4096;
pub const DEATH_EVENT_MEMORY: usize = 1024;
