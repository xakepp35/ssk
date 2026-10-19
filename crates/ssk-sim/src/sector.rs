use crate::*;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SectorState {
    pub tick: u64,
    pub next_id: u64,
    pub next_spawn: f32,
    pub rng: u64,
    pub asteroids: Vec<Asteroid>,
    pub npcs: Vec<Npc>,
    pub pickups: Vec<Pickup>,
}
impl SectorState {
    pub fn capture(w: &World) -> Self {
        Self {
            tick: w.tick,
            next_id: w.next_id,
            next_spawn: w.next_spawn,
            rng: w.rng.0,
            asteroids: w.asteroids.clone(),
            npcs: w.npcs.clone(),
            pickups: w.pickups.clone(),
        }
    }
    pub fn valid(&self) -> bool {
        let pos = |p: V2| {
            p.finite() && p.x.abs() <= SECTOR_MAX_POSITION && p.y.abs() <= SECTOR_MAX_POSITION
        };
        let vel = |v: V2| {
            v.finite() && v.x.abs() <= SECTOR_MAX_VELOCITY && v.y.abs() <= SECTOR_MAX_VELOCITY
        };
        self.asteroids.len() <= SECTOR_MAX_ROCKS
            && self.npcs.len() <= MAX_NPCS
            && self.pickups.len() <= MAX_PICKUPS
            && self.next_spawn.is_finite()
            && self.asteroids.iter().all(|a| {
                pos(a.pos)
                    && vel(a.velocity)
                    && a.radius.is_finite()
                    && (1. ..=SECTOR_MAX_ROCK_RADIUS).contains(&a.radius)
                    && a.hp.is_finite()
                    && a.angle.is_finite()
                    && a.spin.is_finite()
            })
            && self.npcs.iter().all(|n| {
                pos(n.ship.pos)
                    && vel(n.ship.velocity)
                    && n.ship.hp.is_finite()
                    && n.ship.energy.is_finite()
                    && n.ship.angle.is_finite()
                    && n.telegraph.is_finite()
                    && n.think.is_finite()
                    && n.age.is_finite()
                    && n.input.movement.finite()
                    && n.input.aim.is_finite()
                    && n.ship.cooldowns.iter().all(|x| x.is_finite())
            })
            && self.pickups.iter().all(|p| {
                pos(p.pos)
                    && p.amount.is_finite()
                    && (0. ..=PICKUP_MAX_AMOUNT).contains(&p.amount)
                    && p.life.is_finite()
            })
    }
    pub fn restore(&self, w: &mut World) {
        w.tick = self.tick;
        w.next_id = self.next_id;
        w.next_spawn = self.next_spawn;
        w.rng.0 = self.rng;
        w.asteroids = self.asteroids.clone();
        w.npcs = self.npcs.clone();
        w.pickups = self.pickups.clone();
        w.grid.rebuild(&w.asteroids);
    }
}
impl World {
    pub fn sector(seed: u64, sector: [i64; 2]) -> Self {
        let mut w = Self::new(seed, ShipClass::Cadet);
        w.origin = sector;
        w.player.pos = V2::new(SECTOR_SIZE / 2., SECTOR_SIZE / 2.);
        w.player.previous = w.player.pos;
        w.player.id = 0;
        // Generate only this sector, with stable object IDs scoped by its epoch.
        for x in -1..=1 {
            for y in -1..=1 {
                if x != 0 || y != 0 {
                    w.loaded
                        .insert([sector[0].saturating_add(x), sector[1].saturating_add(y)]);
                }
            }
        }
        w.next_id = ((seed
            ^ (sector[0] as u64).wrapping_mul(SECTOR_HASH_X)
            ^ (sector[1] as u64).wrapping_mul(SECTOR_HASH_Y))
            & SECTOR_ID_MASK)
            << SECTOR_ID_SHIFT
            | 2;
        w.field_step();
        w.shared_authority = true;
        w.spawn_radius = SECTOR_SPAWN_RADIUS;
        w
    }
    pub fn sector_step(&mut self, targets: Vec<Ship>) {
        self.targets = targets;
        if let Some(target) = self.targets.first() {
            self.player.pos = target.pos;
            self.player.previous = target.pos;
            self.player.velocity = target.velocity;
        }
        self.player.hp = SECTOR_PROXY_HEALTH;
        self.player.invulnerable = SECTOR_PROXY_PROTECTION;
        self.step(InputFrame::default());
        for a in &mut self.asteroids {
            for (pos, v) in [
                (&mut a.pos.x, &mut a.velocity.x),
                (&mut a.pos.y, &mut a.velocity.y),
            ] {
                if *pos < 0. || *pos > SECTOR_SIZE {
                    *v = -*v;
                    *pos = pos.clamp(0., SECTOR_SIZE);
                }
            }
        }
        for n in &mut self.npcs {
            n.ship.pos.x = n.ship.pos.x.clamp(0., SECTOR_SIZE);
            n.ship.pos.y = n.ship.pos.y.clamp(0., SECTOR_SIZE);
        }
        for p in &mut self.pickups {
            if let Some(target) = self.targets.iter().filter(|t| t.hp > 0.).min_by(|a, b| {
                (a.pos - p.pos)
                    .length_squared()
                    .total_cmp(&(b.pos - p.pos).length_squared())
            }) {
                let delta = target.pos - p.pos;
                if delta.length() < PICKUP_MAGNET_RADIUS {
                    p.pos += delta.normalized() * SHARED_MAGNET_SPEED * DT;
                }
            }
        }
    }
}
