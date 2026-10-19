use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Asteroid {
    pub id: u64,
    pub pos: V2,
    pub radius: f32,
    pub hp: f32,
    pub velocity: V2,
    pub angle: f32,
    pub spin: f32,
    pub small: bool,
    pub last_hit: u64,
}
#[derive(Clone, Default)]
pub struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<usize>>,
}
impl SpatialGrid {
    pub fn rebuild(&mut self, asteroids: &[Asteroid]) {
        for bucket in self.cells.values_mut() {
            bucket.clear();
        }
        for (i, a) in asteroids.iter().enumerate() {
            for x in ((a.pos.x - a.radius) / GRID_CELL_SIZE).floor() as i32
                ..=((a.pos.x + a.radius) / GRID_CELL_SIZE).floor() as i32
            {
                for y in ((a.pos.y - a.radius) / GRID_CELL_SIZE).floor() as i32
                    ..=((a.pos.y + a.radius) / GRID_CELL_SIZE).floor() as i32
                {
                    self.cells.entry((x, y)).or_default().push(i);
                }
            }
        }
        self.cells.retain(|_, v| !v.is_empty());
    }
    pub fn query(&self, a: V2, b: V2, r: f32, result: &mut Vec<usize>) {
        result.clear();
        for x in ((a.x.min(b.x) - r) / GRID_CELL_SIZE).floor() as i32
            ..=((a.x.max(b.x) + r) / GRID_CELL_SIZE).floor() as i32
        {
            for y in ((a.y.min(b.y) - r) / GRID_CELL_SIZE).floor() as i32
                ..=((a.y.max(b.y) + r) / GRID_CELL_SIZE).floor() as i32
            {
                if let Some(bucket) = self.cells.get(&(x, y)) {
                    for &i in bucket {
                        if !result.contains(&i) {
                            result.push(i);
                        }
                    }
                }
            }
        }
    }
}

impl World {
    pub(crate) fn field_step(&mut self) {
        self.rebase();
        for a in &mut self.asteroids {
            a.pos += a.velocity * DT;
            a.angle += a.spin * DT;
        }
        self.asteroids
            .retain(|a| (a.pos - self.player.pos).length() < SECTOR_SIZE * ROCK_KEEP_SECTORS);
        self.grid.rebuild(&self.asteroids);
        self.player.bump_cooldown = (self.player.bump_cooldown - DT).max(0.);
        let mut nearby = Vec::new();
        self.grid.query(
            self.player.pos,
            self.player.pos,
            self.player.class.stats().3,
            &mut nearby,
        );
        for i in nearby {
            let a = &self.asteroids[i];
            let delta = self.player.pos - a.pos;
            let distance = delta.length();
            let min = a.radius + self.player.class.stats().3;
            if distance < min {
                let normal = if distance > CONTACT_EPSILON {
                    delta / distance
                } else {
                    V2::new(1., 0.)
                };
                self.player.pos = a.pos + normal * (min + CONTACT_SEPARATION);
                let incoming = (self.player.velocity - a.velocity).dot(normal);
                if incoming < 0. {
                    self.player.velocity -= normal * (incoming * CONTACT_RESTITUTION);
                    if self.player.bump_cooldown <= 0. && incoming < -CONTACT_DAMAGE_SPEED {
                        let damage = (-incoming * CONTACT_DAMAGE_SCALE).min(CONTACT_DAMAGE_MAX);
                        self.player.damage(damage);
                        self.player.bump_cooldown = CONTACT_COOLDOWN;
                        self.emit(WorldEvent::Impact {
                            pos: self.player.pos,
                            damage,
                        });
                    }
                }
            }
        }
    }
    fn rebase(&mut self) {
        if self.shared_authority {
            return;
        }
        let shift = [
            (self.player.pos.x / SECTOR_SIZE).floor() as i64,
            (self.player.pos.y / SECTOR_SIZE).floor() as i64,
        ];
        if shift != [0, 0] {
            self.origin[0] += shift[0];
            self.origin[1] += shift[1];
            let offset = V2::new(shift[0] as f32 * SECTOR_SIZE, shift[1] as f32 * SECTOR_SIZE);
            self.player.pos -= offset;
            self.player.previous -= offset;
            for a in &mut self.asteroids {
                a.pos -= offset;
            }
            for npc in &mut self.npcs {
                npc.ship.pos -= offset;
                npc.ship.previous -= offset;
            }
            for pickup in &mut self.pickups {
                pickup.pos -= offset;
            }
            for b in &mut self.bullets {
                b.pos -= offset;
                b.previous -= offset;
                b.shot.pos -= offset;
            }
            self.origin_shift += offset;
            for event in &mut self.events {
                match event {
                    WorldEvent::Shot(s) => s.pos -= offset,
                    WorldEvent::Impact { pos, .. }
                    | WorldEvent::Destroyed { pos, .. }
                    | WorldEvent::Pickup { pos, .. } => *pos -= offset,
                    _ => {}
                }
            }
        }
        if self.duel {
            return;
        }
        self.loaded
            .retain(|s| (s[0] - self.origin[0]).abs() <= 2 && (s[1] - self.origin[1]).abs() <= 2);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let sector = [self.origin[0] + dx, self.origin[1] + dy];
                if !self.loaded.insert(sector) {
                    continue;
                }
                let mut rng = Rng(self.seed
                    ^ (sector[0] as u64).wrapping_mul(SECTOR_HASH_X)
                    ^ (sector[1] as u64).wrapping_mul(SECTOR_HASH_Y));
                for _ in 0..ROCKS_PER_SECTOR {
                    let local = V2::new(rng.range(0., SECTOR_SIZE), rng.range(0., SECTOR_SIZE));
                    let pos = WorldPos { sector, local }.relative_to(self.origin);
                    if (pos - self.player.pos).length() < ROCK_SPAWN_CLEARANCE {
                        continue;
                    }
                    let a = Asteroid {
                        id: self.next_id,
                        pos,
                        radius: rng.range(ROCK_RADIUS.start, ROCK_RADIUS.end),
                        hp: ROCK_HEALTH,
                        velocity: V2::from_angle(rng.range(0., std::f32::consts::TAU))
                            * rng.range(ROCK_SPEED.start, ROCK_SPEED.end),
                        angle: rng.range(0., std::f32::consts::TAU),
                        spin: rng.range(-ROCK_SPIN, ROCK_SPIN),
                        small: false,
                        last_hit: 0,
                    };
                    self.next_id += 1;
                    self.asteroids.push(a);
                }
            }
        }
    }
    pub(crate) fn destroy_asteroids(&mut self) {
        let mut children = Vec::new();
        let mut events = Vec::new();
        let mut rewards = Vec::new();
        for a in &self.asteroids {
            if a.hp <= 0. {
                events.push(WorldEvent::Destroyed {
                    pos: a.pos,
                    size: a.radius,
                });
                rewards.push((
                    a.pos,
                    a.small,
                    a.last_hit == self.player.id,
                    a.id,
                    a.last_hit,
                ));
                if !a.small {
                    for sign in [-1., 1.] {
                        let normal = V2::from_angle(a.angle) * sign;
                        children.push(Asteroid {
                            id: self.next_id,
                            pos: a.pos + normal * a.radius * ROCK_CHILD_OFFSET,
                            radius: a.radius * ROCK_CHILD_RADIUS,
                            hp: ROCK_CHILD_HEALTH,
                            velocity: a.velocity + normal * ROCK_CHILD_SPEED,
                            angle: a.angle,
                            spin: sign * ROCK_CHILD_SPIN,
                            small: true,
                            last_hit: 0,
                        });
                        self.next_id += 1;
                    }
                }
            }
        }
        self.asteroids.retain(|a| a.hp > 0.);
        self.asteroids.extend(children);
        for event in events {
            self.emit(event);
        }
        for (pos, small, owned, id, player) in rewards {
            if self.shared_authority && player & PLAYER_ID_BIT != 0 {
                self.emit(WorldEvent::Reward {
                    id,
                    player,
                    amount: if small { ROCK_CHILD_SCORE } else { ROCK_SCORE },
                });
            }
            if owned {
                self.award(if small { ROCK_CHILD_SCORE } else { ROCK_SCORE });
            }
            self.drop_loot(pos, false);
        }
    }
}

pub(crate) fn empty_loaded() -> HashSet<[i64; 2]> {
    HashSet::new()
}
