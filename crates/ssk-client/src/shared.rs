use ssk_net::{Replicas, SharedChunk, SharedEvent, Snapshot};
use ssk_sim::*;
use std::collections::{HashMap, HashSet, VecDeque};
struct Owned {
    world: World,
    epoch: u64,
}
struct Image {
    state: SectorState,
    epoch: u64,
    revision: u64,
    time: f64,
}
struct Assembly {
    epoch: u64,
    revision: u64,
    parts: Vec<Option<Vec<u8>>>,
    time: f64,
}
struct Lease {
    owner: u64,
    epoch: u64,
    until: f64,
    announced: f64,
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
#[derive(Default)]
pub struct Shared {
    owners: HashMap<[i64; 2], u64>,
    leases: HashMap<[i64; 2], Lease>,
    owned: HashMap<[i64; 2], Owned>,
    images: HashMap<[i64; 2], Image>,
    assemblies: HashMap<([i64; 2], u64, u64), Assembly>,
    players: Vec<(u64, u32, Snapshot)>,
    seen: HashSet<([i64; 2], u64, u64, u8)>,
    order: VecDeque<([i64; 2], u64, u64, u8)>,
    claims: HashMap<([i64; 2], u64), f64>,
    accumulator: f32,
    published: f64,
}
impl Shared {
    pub fn stats(&self, now: f64) -> (usize, f64) {
        (
            self.images.len(),
            self.images
                .values()
                .map(|i| (now - i.time).max(0.) * 1000.)
                .fold(0., f64::max),
        )
    }
    pub fn members(&mut self, w: &World, life: u32, replicas: &Replicas, now: f64) {
        self.players.clear();
        self.players
            .push((w.player.id, life, Snapshot::capture(w, 0, life)));
        for (peer, q) in &replicas.peers {
            if let Some((t, s)) = q.back() {
                if now - t < 2. {
                    if let Some(id) = ssk_net::player_id(peer) {
                        self.players.push((id, s.life, *s));
                    }
                }
            }
        }
        self.owners.clear();
        for (id, _, s) in &self.players {
            let sector = s.pos.sector;
            if (sector[0] as i128 - w.origin[0] as i128).abs() > 1
                || (sector[1] as i128 - w.origin[1] as i128).abs() > 1
            {
                continue;
            }
            self.owners
                .entry(sector)
                .and_modify(|owner| *owner = (*owner).min(*id))
                .or_insert(*id);
        }
        for (sector, owner) in &mut self.owners {
            if let Some(lease) = self.leases.get(sector) {
                if lease.until > now
                    && self
                        .players
                        .iter()
                        .any(|(id, _, s)| *id == lease.owner && s.pos.sector == *sector)
                {
                    *owner = lease.owner;
                }
            }
        }
        self.leases
            .retain(|s, _| self.owners.contains_key(s) || self.images.contains_key(s));
    }
    fn remember(&mut self, sector: [i64; 2], epoch: u64, id: u64, kind: u8) -> bool {
        let key = (sector, epoch, id, kind);
        if !self.seen.insert(key) {
            return false;
        }
        self.order.push_back(key);
        while self.order.len() > 8192 {
            self.seen.remove(&self.order.pop_front().unwrap());
        }
        true
    }
    pub fn chunk(&mut self, origin: u64, bytes: &[u8], now: f64) {
        let Some(c) = SharedChunk::decode(bytes) else {
            return;
        };
        if !self.valid_owner(c.sector, c.epoch, origin, now) || self.owned.contains_key(&c.sector) {
            return;
        }
        if self
            .images
            .get(&c.sector)
            .is_some_and(|i| i.epoch == c.epoch && i.revision >= c.revision)
        {
            return;
        }
        self.assemblies.retain(|_, a| now - a.time < 5.);
        let key = (c.sector, c.epoch, c.revision);
        if self.assemblies.len() >= 27 && !self.assemblies.contains_key(&key) {
            return;
        }
        let a = self.assemblies.entry(key).or_insert_with(|| Assembly {
            epoch: c.epoch,
            revision: c.revision,
            parts: vec![None; c.total as usize],
            time: now,
        });
        if c.epoch != a.epoch || c.revision > a.revision {
            *a = Assembly {
                epoch: c.epoch,
                revision: c.revision,
                parts: vec![None; c.total as usize],
                time: now,
            };
        }
        if a.epoch != c.epoch || a.revision != c.revision || a.parts.len() != c.total as usize {
            return;
        }
        a.parts[c.part as usize] = Some(c.data);
        if let Some(state) = SharedChunk::restore(&a.parts) {
            self.images.insert(
                c.sector,
                Image {
                    state,
                    epoch: c.epoch,
                    revision: c.revision,
                    time: now,
                },
            );
            self.assemblies.retain(|(sector, epoch, revision), _| {
                *sector != c.sector
                    || *epoch > c.epoch
                    || (*epoch == c.epoch && *revision > c.revision)
            });
        }
    }
    pub fn shot(&mut self, shot: Shot, sector: [i64; 2]) {
        for (origin, owned) in &mut self.owned {
            let mut shot = shot;
            shot.pos = WorldPos {
                sector,
                local: shot.pos,
            }
            .relative_to(*origin);
            if shot.pos.x >= -1600.
                && shot.pos.y >= -1600.
                && shot.pos.x <= 5696.
                && shot.pos.y <= 5696.
            {
                owned.world.spawn_shot(shot);
            }
        }
    }
    pub fn event(
        &mut self,
        origin: u64,
        event: SharedEvent,
        w: &mut World,
        life: u32,
        now: f64,
    ) -> Vec<SharedEvent> {
        match event {
            SharedEvent::Lease {
                sector,
                epoch,
                until_ms,
            } => {
                let wall = unix_ms();
                if epoch == 0
                    || until_ms <= wall
                    || until_ms.saturating_sub(wall) > 2500
                    || !self.owners.contains_key(&sector)
                    || !self
                        .players
                        .iter()
                        .any(|(id, _, s)| *id == origin && s.pos.sector == sector)
                {
                    return vec![];
                }
                let wins = self
                    .leases
                    .get(&sector)
                    .is_none_or(|l| epoch > l.epoch || (epoch == l.epoch && origin <= l.owner));
                if wins {
                    self.leases.insert(
                        sector,
                        Lease {
                            owner: origin,
                            epoch,
                            until: now + (until_ms - wall) as f64 / 1000.,
                            announced: now,
                        },
                    );
                    self.owners.insert(sector, origin);
                }
                vec![]
            }
            SharedEvent::Release { sector, epoch } => {
                if let Some(l) = self.leases.get_mut(&sector) {
                    if l.owner == origin && l.epoch == epoch {
                        l.until = 0.;
                    }
                }
                vec![]
            }
            SharedEvent::Claim {
                sector,
                epoch,
                pickup,
                life: claim_life,
            } => {
                if !self.valid_owner(sector, epoch, w.player.id, now) {
                    return vec![];
                }
                let Some(owner) = self.owned.get_mut(&sector).filter(|o| o.epoch == epoch) else {
                    return vec![];
                };
                let Some((_, _, player)) = self
                    .players
                    .iter()
                    .find(|(id, l, s)| *id == origin && *l == claim_life && s.hp > 0.)
                else {
                    return vec![];
                };
                let pos = player.pos.relative_to(sector);
                let Some(index) = owner.world.pickups.iter().position(|p| {
                    p.id == pickup && (p.pos - pos).length() < player.class.stats().3 + 60.
                }) else {
                    return vec![];
                };
                let p = owner.world.pickups.remove(index);
                let event = SharedEvent::Grant {
                    sector,
                    epoch,
                    pickup,
                    player: origin,
                    life: claim_life,
                    kind: p.kind,
                    amount: p.amount,
                };
                self.event(w.player.id, event.clone(), w, life, now);
                vec![event]
            }
            SharedEvent::Shot {
                sector,
                epoch,
                mut shot,
            } => {
                if !self.valid_owner(sector, epoch, origin, now)
                    || !shot.pos.finite()
                    || !shot.angle.is_finite()
                    || ![0.6, 2.4].contains(&shot.damage_scale)
                {
                    return vec![];
                }
                let exists = self.images.get(&sector).is_some_and(|i| {
                    i.epoch == epoch && i.state.npcs.iter().any(|n| n.ship.id == shot.owner)
                });
                if !exists || !self.remember(sector, epoch, shot.id, 0) {
                    return vec![];
                }
                shot.pos = WorldPos {
                    sector,
                    local: shot.pos,
                }
                .relative_to(w.origin);
                w.spawn_shot(shot);
                vec![]
            }
            SharedEvent::Grant {
                sector,
                epoch,
                pickup,
                player,
                life: grant_life,
                kind,
                amount,
            } => {
                if !self.valid_owner(sector, epoch, origin, now)
                    || player != w.player.id
                    || grant_life != life
                    || !amount.is_finite()
                    || !(0. ..=300.).contains(&amount)
                    || !self.remember(sector, 0, pickup, 1)
                {
                    return vec![];
                }
                match kind {
                    PickupKind::Energy => w.player.energy = (w.player.energy + amount).min(2000.),
                    PickupKind::Health => {
                        w.player.hp = (w.player.hp + amount).min(w.player.class.stats().0 * 2.)
                    }
                };
                w.emit(WorldEvent::Pickup {
                    pos: w.player.pos,
                    kind,
                });
                vec![]
            }
            SharedEvent::Reward {
                sector,
                epoch,
                id,
                player,
                amount,
            } => {
                if self.valid_owner(sector, epoch, origin, now)
                    && player == w.player.id
                    && amount <= 250
                    && self.remember(sector, 0, id, 2)
                {
                    w.award(amount);
                }
                vec![]
            }
        }
    }
    fn valid_owner(&self, sector: [i64; 2], epoch: u64, origin: u64, now: f64) -> bool {
        self.leases
            .get(&sector)
            .is_some_and(|l| l.owner == origin && l.epoch == epoch && l.until > now)
    }
    pub fn checkpoint(&self) -> Vec<Vec<u8>> {
        self.owned
            .iter()
            .flat_map(|(&s, o)| SharedChunk::snapshot(s, o.epoch, &SectorState::capture(&o.world)))
            .collect()
    }
    pub fn release(&mut self) -> Vec<SharedEvent> {
        self.owned
            .iter()
            .map(|(&sector, o)| SharedEvent::Release {
                sector,
                epoch: o.epoch,
            })
            .collect()
    }
    pub fn tick(
        &mut self,
        w: &mut World,
        life: u32,
        replicas: &Replicas,
        dt: f32,
        now: f64,
    ) -> (Vec<SharedEvent>, Option<Vec<Vec<u8>>>) {
        self.members(w, life, replicas, now);
        let id = w.player.id;
        let mut events = Vec::new();
        self.owned.retain(|sector, o| {
            let keep = self.owners.get(sector) == Some(&id) && *sector == w.origin;
            if !keep {
                events.push(SharedEvent::Release {
                    sector: *sector,
                    epoch: o.epoch,
                });
            }
            keep
        });
        for (&sector, &owner) in &self.owners {
            if owner == id && !self.owned.contains_key(&sector) {
                let epoch = self
                    .leases
                    .get(&sector)
                    .map_or(0, |l| l.epoch)
                    .max(self.images.get(&sector).map_or(0, |i| i.epoch))
                    .saturating_add(1);
                let mut world = World::sector(w.seed, sector);
                if let Some(image) = self.images.get(&sector) {
                    image.state.restore(&mut world);
                }
                self.leases.insert(
                    sector,
                    Lease {
                        owner: id,
                        epoch,
                        until: now + 2.,
                        announced: now - 1.,
                    },
                );
                self.owned.insert(sector, Owned { world, epoch });
            }
        }
        self.owned.retain(|sector, o| {
            self.leases
                .get(sector)
                .is_some_and(|l| l.owner == id && l.epoch == o.epoch)
        });
        for (&sector, o) in &self.owned {
            let l = self.leases.get_mut(&sector).unwrap();
            if now - l.announced >= 0.5 {
                l.announced = now;
                l.until = now + 2.;
                events.push(SharedEvent::Lease {
                    sector,
                    epoch: o.epoch,
                    until_ms: unix_ms() + 2000,
                });
            }
        }
        self.accumulator += dt.min(0.1);
        let mut steps = 0;
        while self.accumulator >= DT && steps < 8 {
            for (&sector, owned) in &mut self.owned {
                let targets = self
                    .players
                    .iter()
                    .filter(|(_, _, s)| s.pos.sector == sector && s.hp > 0.)
                    .map(|(id, _, s)| s.ship(*id, sector))
                    .collect();
                owned.world.sector_step(targets);
                for event in owned.world.events.drain(..) {
                    match event {
                        WorldEvent::Shot(shot) if shot.owner & (1 << 63) == 0 => {
                            events.push(SharedEvent::Shot {
                                sector,
                                epoch: owned.epoch,
                                shot,
                            })
                        }
                        WorldEvent::Reward { id, player, amount } => {
                            events.push(SharedEvent::Reward {
                                sector,
                                epoch: owned.epoch,
                                id,
                                player,
                                amount,
                            })
                        }
                        _ => {}
                    }
                }
            }
            self.accumulator -= DT;
            steps += 1;
        }
        self.accumulator = self.accumulator.min(DT);
        for event in events.clone() {
            self.event(id, event, w, life, now);
        }
        for (&sector, owned) in &self.owned {
            self.images.insert(
                sector,
                Image {
                    state: SectorState::capture(&owned.world),
                    epoch: owned.epoch,
                    revision: owned.world.tick,
                    time: now,
                },
            );
        }
        self.images
            .retain(|s, i| self.owners.contains_key(s) && now - i.time < 5.);
        self.assemblies
            .retain(|(s, _, _), _| self.owners.contains_key(s));
        w.asteroids.clear();
        w.npcs.clear();
        w.pickups.clear();
        w.shared_replica = true;
        let mut claims = Vec::new();
        for (&sector, image) in &self.images {
            let offset = WorldPos {
                sector,
                local: V2::ZERO,
            }
            .relative_to(w.origin);
            for a in &image.state.asteroids {
                let mut a = a.clone();
                a.pos += offset;
                w.asteroids.push(a);
            }
            for n in &image.state.npcs {
                let mut n = n.clone();
                n.ship.pos += offset;
                n.ship.previous = n.ship.pos;
                w.npcs.push(n);
            }
            for p in &image.state.pickups {
                let mut p = p.clone();
                p.pos += offset;
                if w.player.hp > 0.
                    && (p.pos - w.player.pos).length() < w.player.class.stats().3 + 18.
                    && self
                        .claims
                        .get(&(sector, p.id))
                        .is_none_or(|t| now - t > 1.)
                {
                    self.claims.insert((sector, p.id), now);
                    claims.push(SharedEvent::Claim {
                        sector,
                        epoch: image.epoch,
                        pickup: p.id,
                        life,
                    });
                }
                w.pickups.push(p);
            }
        }
        self.claims.retain(|_, t| now - *t < 3.);
        for claim in claims {
            events.extend(self.event(id, claim.clone(), w, life, now));
            events.push(claim);
        }
        let frames = if now - self.published >= 0.2 {
            self.published = now;
            Some(
                self.owned
                    .iter()
                    .flat_map(|(&s, o)| {
                        SharedChunk::snapshot(s, o.epoch, &SectorState::capture(&o.world))
                    })
                    .collect(),
            )
        } else {
            None
        };
        (events, frames)
    }
}
