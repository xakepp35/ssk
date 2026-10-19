use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeaponKind {
    #[default]
    MachineGun,
    Shotgun,
    Plasma,
}
impl WeaponKind {
    pub fn from_index(i: usize) -> Self {
        match i % WEAPON_COUNT {
            1 => Self::Shotgun,
            2 => Self::Plasma,
            _ => Self::MachineGun,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::MachineGun => "MACHINE GUN",
            Self::Shotgun => "SHOTGUN",
            Self::Plasma => "PLASMA",
        }
    }
    pub fn stats(self) -> WeaponStats {
        weapon_stats(self)
    }
}
pub struct WeaponStats {
    pub damage: f32,
    pub interval: f32,
    pub energy: f32,
    pub speed: f32,
    pub radius: f32,
    pub life: f32,
    pub pellets: usize,
    pub spread: f32,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub id: u64,
    pub owner: u64,
    pub pos: V2,
    pub angle: f32,
    pub weapon: WeaponKind,
    pub seed: u64,
    pub damage_scale: f32,
    pub blocked: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Bullet {
    pub shot: Shot,
    pub pos: V2,
    pub previous: V2,
    pub velocity: V2,
    pub life: f32,
    pub damage: f32,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug)]
pub enum WorldEvent {
    Shot(Shot),
    Dry,
    Impact { pos: V2, damage: f32 },
    Destroyed { pos: V2, size: f32 },
    Pickup { pos: V2, kind: PickupKind },
    Reward { id: u64, player: u64, amount: u64 },
}

impl Ship {
    pub fn combat_step(&mut self, input: InputFrame, rng: &mut Rng, shot_id: u64) -> Option<Shot> {
        for c in &mut self.cooldowns {
            *c = (*c - DT).max(0.);
        }
        self.quiet += DT;
        self.weapon = WeaponKind::from_index(input.weapon);
        let index = self.weapon as usize;
        let stats = self.weapon.stats();
        let scale = if matches!(self.class, ShipClass::Thunder) && self.ability_active {
            QUAD_DAMAGE
        } else {
            1.
        };
        if input.fire && self.cooldowns[index] <= 0. && self.energy >= stats.energy * scale {
            self.energy -= stats.energy * scale;
            self.quiet = 0.;
            self.cooldowns[index] = stats.interval;
            Some(Shot {
                id: shot_id,
                owner: self.id,
                pos: self.pos + V2::from_angle(self.angle) * (self.class.stats().3 + MUZZLE_OFFSET),
                angle: self.angle,
                weapon: self.weapon,
                seed: rng.next_u64(),
                damage_scale: scale,
                blocked: false,
            })
        } else {
            if self.quiet > ENERGY_REGEN_DELAY && !self.ability_active && self.energy < ENERGY_BASE
            {
                self.energy = (self.energy + ENERGY_REGEN * DT).min(ENERGY_BASE);
            }
            None
        }
    }
}

/// Fraction of the first intersection, including starting inside the circle.
pub fn swept_circle(a: V2, b: V2, center: V2, radius: f32) -> Option<f32> {
    let m = a - center;
    let d = b - a;
    let c = m.dot(m) - radius * radius;
    if c <= 0. {
        return Some(0.);
    }
    let aa = d.dot(d);
    let bb = m.dot(d);
    let disc = bb * bb - aa * c;
    if aa < COLLISION_EPSILON || bb >= 0. || disc < 0. {
        return None;
    }
    let t = (-bb - disc.sqrt()) / aa;
    (t <= 1.).then_some(t)
}

impl World {
    /// Bounded lag compensation against linear target history, capped at 200 ms.
    pub fn receive_shot(&mut self, shot: Shot, lag: f32, past_target: V2) {
        let start = self.bullets.len();
        self.spawn_shot(shot);
        let lag = lag.clamp(0., LAG_COMPENSATION_SECONDS);
        let current = self.player.pos;
        let mut impacts = Vec::new();
        for bullet in &mut self.bullets[start..] {
            let end = bullet.pos + bullet.velocity * lag;
            let hit = swept_circle(
                bullet.pos - past_target,
                end - current,
                V2::ZERO,
                self.player.class.stats().3 + bullet.radius,
            );
            let obstacle = self
                .asteroids
                .iter()
                .filter(|a| a.hp > 0.)
                .filter_map(|a| swept_circle(bullet.pos, end, a.pos, a.radius + bullet.radius))
                .min_by(f32::total_cmp);
            if let Some(t) = hit.filter(|t| obstacle.is_none_or(|o| *t < o)) {
                if self.player.damage(bullet.damage) > 0. {
                    self.last_attacker = shot.owner;
                    self.last_shot = shot.id;
                }
                impacts.push(WorldEvent::Impact {
                    pos: bullet.pos.lerp(end, t),
                    damage: bullet.damage,
                });
                bullet.life = 0.;
            } else if obstacle.is_some() {
                bullet.life = 0.;
            } else {
                bullet.pos = end;
                bullet.previous = end;
                bullet.life -= lag;
            }
        }
        self.bullets.retain(|b| b.life > 0.);
        for impact in impacts {
            self.emit(impact);
        }
    }
    pub fn emit(&mut self, event: WorldEvent) {
        if self.events.len() < MAX_WORLD_EVENTS {
            self.events.push(event);
        }
    }
    pub fn spawn_shot(&mut self, mut shot: Shot) {
        if shot.blocked {
            self.emit(WorldEvent::Shot(shot));
            return;
        }
        let stats = shot.weapon.stats();
        let mut rng = Rng(shot.seed);
        let firing = if shot.owner == self.player.id {
            Some(&mut self.player)
        } else {
            self.npcs
                .iter_mut()
                .find(|n| n.ship.id == shot.owner)
                .map(|n| &mut n.ship)
        };
        if let Some(ship) = firing {
            if matches!(ship.class, ShipClass::Cadet) && ship.ability_active {
                ship.energy = (ship.energy
                    - stats.damage * shot.damage_scale * stats.pellets as f32 * SHIELD_ABSORPTION)
                    .max(0.);
                if ship.energy <= 0. {
                    ship.ability_active = false;
                }
                shot.blocked = true;
                self.emit(WorldEvent::Shot(shot));
                self.emit(WorldEvent::Impact {
                    pos: shot.pos,
                    damage: SHIELD_IMPACT_DAMAGE,
                });
                return;
            }
        }
        for pellet in 0..stats.pellets {
            if self.bullets.len() >= MAX_BULLETS {
                break;
            }
            let spread = if stats.pellets > 1 {
                -stats.spread
                    + 2. * stats.spread
                        * (pellet as f32 + rng.range(PELLET_JITTER_MIN, PELLET_JITTER_MAX))
                        / stats.pellets as f32
            } else {
                rng.range(-stats.spread, stats.spread)
            };
            let direction = V2::from_angle(shot.angle + spread);
            self.bullets.push(Bullet {
                shot,
                pos: shot.pos,
                previous: shot.pos,
                velocity: direction * stats.speed,
                life: stats.life,
                damage: stats.damage * shot.damage_scale,
                radius: stats.radius,
            });
        }
        self.emit(WorldEvent::Shot(shot));
    }
    pub(crate) fn combat(&mut self, input: InputFrame) {
        let shot_id = self.next_id;
        if let Some(shot) = self.player.combat_step(input, &mut self.rng, shot_id) {
            self.next_id += 1;
            self.spawn_shot(shot);
        } else if input.fire
            && self.player.cooldowns[self.player.weapon as usize] <= 0.
            && self.tick % DRY_FIRE_TICKS == 0
        {
            self.emit(WorldEvent::Dry);
        }
        let mut impacts = Vec::new();
        let mut nearby = Vec::new();
        for bullet in &mut self.bullets {
            bullet.previous = bullet.pos;
            bullet.pos += bullet.velocity * DT;
            bullet.life -= DT;
            let mut first: Option<(f32, u8, usize)> = None;
            self.grid
                .query(bullet.previous, bullet.pos, bullet.radius, &mut nearby);
            for &index in &nearby {
                let target = &self.asteroids[index];
                if target.hp <= 0. {
                    continue;
                }
                if let Some(t) = swept_circle(
                    bullet.previous,
                    bullet.pos,
                    target.pos,
                    target.radius + bullet.radius,
                ) {
                    if first.is_none_or(|(best, _, _)| t < best) {
                        first = Some((t, 0, index));
                    }
                }
            }
            if bullet.shot.owner == self.player.id
                || (self.shared_authority && bullet.shot.owner & (1 << 63) != 0)
            {
                for (index, npc) in self.npcs.iter().enumerate() {
                    if npc.telegraph > 0. || npc.ship.hp <= 0. {
                        continue;
                    }
                    if let Some(t) = swept_circle(
                        bullet.previous,
                        bullet.pos,
                        npc.ship.pos,
                        npc.ship.class.stats().3 + bullet.radius,
                    ) {
                        if first.is_none_or(|(best, _, _)| t < best) {
                            first = Some((t, 1, index));
                        }
                    }
                }
            } else if let Some(t) = swept_circle(
                bullet.previous,
                bullet.pos,
                self.player.pos,
                self.player.class.stats().3 + bullet.radius,
            ) {
                if first.is_none_or(|(best, _, _)| t < best) {
                    first = Some((t, 2, 0));
                }
            }
            if let Some((t, kind, index)) = first {
                match kind {
                    0 => {
                        if !self.shared_replica {
                            self.asteroids[index].hp -= bullet.damage;
                            self.asteroids[index].last_hit = bullet.shot.owner;
                        }
                    }
                    1 => {
                        if !self.shared_replica {
                            self.npcs[index].ship.damage(bullet.damage);
                            self.npcs[index].last_hit = bullet.shot.owner;
                        }
                    }
                    _ => {
                        self.player.damage(bullet.damage);
                        self.last_attacker = bullet.shot.owner;
                        self.last_shot = bullet.shot.id;
                        self.combo = 1.;
                        self.last_kill = None;
                    }
                }
                bullet.life = 0.;
                impacts.push(WorldEvent::Impact {
                    pos: bullet.previous.lerp(bullet.pos, t),
                    damage: bullet.damage,
                });
            }
        }
        self.bullets.retain(|b| b.life > 0.);
        if !self.shared_replica {
            self.destroy_asteroids();
            self.destroy_enemies();
        }
        for event in impacts {
            self.emit(event);
        }
    }
}

impl WorldEvent {
    pub fn shift(&mut self, offset: V2) {
        match self {
            Self::Shot(s) => s.pos -= offset,
            Self::Impact { pos, .. } | Self::Destroyed { pos, .. } | Self::Pickup { pos, .. } => {
                *pos -= offset
            }
            _ => {}
        }
    }
}
