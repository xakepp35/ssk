//! Owner-authoritative multiplayer rules. No sockets, QUIC, codecs or rendering.
use crate::*;
use std::collections::{HashSet, VecDeque};
pub struct PeerCombat {
    pub replicas: Replicas,
    pub life: u32,
    pub frags: u32,
    pub error: String,
    pub dead_for: f32,
    pub outbox: VecDeque<CombatEvent>,
    seen: HashSet<(String, u32, u64)>,
    order: VecDeque<(String, u32, u64)>,
    deaths: HashSet<(String, u32)>,
    death_order: VecDeque<(String, u32)>,
    guard: CombatGuard,
    own_history: VecDeque<(f64, WorldPos)>,
    fired: VecDeque<(u64, f64)>,
    pub shared: SharedWorld,
    last_shared: f64,
}
impl Default for PeerCombat {
    fn default() -> Self {
        Self {
            replicas: Replicas::default(),
            life: 0,
            frags: 0,
            error: String::new(),
            dead_for: 0.,
            outbox: VecDeque::with_capacity(COMBAT_EVENT_QUEUE),
            seen: HashSet::new(),
            order: VecDeque::new(),
            deaths: HashSet::new(),
            death_order: VecDeque::new(),
            guard: Default::default(),
            own_history: VecDeque::new(),
            fired: VecDeque::new(),
            shared: Default::default(),
            last_shared: 0.,
        }
    }
}
impl PeerCombat {
    pub fn snapshot(&mut self, peer: String, id: u64, snapshot: Snapshot, at: f64) {
        let old = self
            .replicas
            .peers
            .get(&peer)
            .and_then(|q| q.back())
            .map(|(_, s)| *s);
        if self.guard.snapshot(old, snapshot) {
            self.replicas.identities.insert(peer.clone(), id);
            self.replicas.push(peer, snapshot, at);
        }
    }
    pub fn maintain(&mut self, world: &World, now: f64) {
        self.shared.members(world, self.life, &self.replicas, now);
        let stale: Vec<_> = self
            .replicas
            .peers
            .iter()
            .filter(|(_, q)| {
                q.back()
                    .is_none_or(|(t, _)| now - t > REPLICA_STALE_SECONDS)
            })
            .map(|(p, _)| p.clone())
            .collect();
        for peer in stale {
            self.disconnect(&peer);
        }
    }
    pub fn disconnect(&mut self, peer: &str) {
        self.replicas.peers.remove(peer);
        self.replicas.identities.remove(peer);
        self.guard.forget(peer);
    }
    pub fn receive(
        &mut self,
        world: &mut World,
        peer: String,
        id: u64,
        event: CombatEvent,
        now: f64,
    ) {
        if matches!(&event, CombatEvent::Shot { .. })
            && !self.guard.shot(&peer, &event, &self.replicas)
        {
            return;
        }
        match event {
            CombatEvent::Shared(event) => {
                let replies = self.shared.event(id, event, world, self.life, now);
                for e in replies {
                    self.event(CombatEvent::Shared(e));
                }
            }
            CombatEvent::Shot {
                mut shot,
                sector,
                life,
                tick,
            } => {
                let key = (peer.clone(), life, shot.id);
                if self.seen.contains(&key) {
                    return;
                }
                if !shot.pos.finite() || !shot.angle.is_finite() || !shot.damage_scale.is_finite() {
                    return;
                }
                if self
                    .replicas
                    .peers
                    .get(&peer)
                    .and_then(|q| q.back())
                    .is_some_and(|(_, s)| s.life != life)
                {
                    return;
                }
                shot.owner = id;
                shot.pos = WorldPos {
                    sector,
                    local: shot.pos,
                }
                .relative_to(world.origin);
                let mut source_shot = shot;
                source_shot.pos = WorldPos {
                    sector: world.origin,
                    local: shot.pos,
                }
                .relative_to(sector);
                self.shared.shot(source_shot, sector);
                if (shot.pos - world.player.pos).length() > MAX_REMOTE_SHOT_RANGE {
                    return;
                }
                self.seen.insert(key.clone());
                self.order.push_back(key);
                if self.order.len() > SHOT_EVENT_MEMORY {
                    self.seen.remove(&self.order.pop_front().unwrap());
                }
                let lag = self
                    .replicas
                    .peers
                    .get(&peer)
                    .and_then(|q| q.back())
                    .map(|(time, s)| {
                        ((now - time) + (s.tick as f64 - tick as f64) / TICK_PER_SEC as f64)
                            .clamp(0., LAG_COMPENSATION_SECONDS as f64)
                            as f32
                    })
                    .unwrap_or(0.);
                let past = self
                    .own_history
                    .iter()
                    .min_by(|(a, _), (b, _)| {
                        (a - (now - lag as f64))
                            .abs()
                            .total_cmp(&(b - (now - lag as f64)).abs())
                    })
                    .map(|(_, p)| p.relative_to(world.origin))
                    .unwrap_or(world.player.pos);
                world.receive_shot(shot, lag, past);
            }
            CombatEvent::Death { life, killer, shot } => {
                let key = (peer, life);
                if self.deaths.insert(key.clone()) {
                    self.death_order.push_back(key);
                    if self.death_order.len() > DEATH_EVENT_MEMORY {
                        self.deaths.remove(&self.death_order.pop_front().unwrap());
                    }
                    if killer == world.player.id
                        && self
                            .fired
                            .iter()
                            .any(|(id, time)| *id == shot && now - time <= HISTORY_SECONDS)
                    {
                        self.frags += 1;
                        world.award(DUEL_FRAG_SCORE);
                    }
                }
            }
        }
    }
    fn event(&mut self, event: CombatEvent) {
        if self.outbox.len() < COMBAT_EVENT_QUEUE {
            self.outbox.push_back(event);
        } else {
            self.error = "Critical game event backlog full".into();
        }
    }
    pub fn after_step(
        &mut self,
        world: &mut World,
        dt: f32,
        now: f64,
    ) -> (bool, Option<Vec<SectorImage>>) {
        self.own_history.push_back((
            now,
            WorldPos {
                sector: world.origin,
                local: world.player.pos,
            },
        ));
        while self
            .own_history
            .front()
            .is_some_and(|(t, _)| now - t > HISTORY_SECONDS)
        {
            self.own_history.pop_front();
        }
        while self
            .fired
            .front()
            .is_some_and(|(_, t)| now - t > HISTORY_SECONDS)
        {
            self.fired.pop_front();
        }
        for event in &world.events {
            if let WorldEvent::Shot(shot) = event {
                if shot.owner == world.player.id {
                    self.fired.push_back((shot.id, now));
                    let pos = WorldPos {
                        sector: world.origin,
                        local: shot.pos,
                    }
                    .normalized();
                    let mut shot = *shot;
                    shot.pos = pos.local;
                    self.shared.shot(shot, pos.sector);
                    self.event(CombatEvent::Shot {
                        shot,
                        sector: pos.sector,
                        life: self.life,
                        tick: world.tick,
                    });
                }
            }
        }
        let mut images_out = None;
        if now - self.last_shared >= TICK_PER_PACKET as f64 / TICK_PER_SEC as f64 {
            let dt = (now - self.last_shared) as f32;
            self.last_shared = now;
            let (events, frames) = self.shared.tick(world, self.life, &self.replicas, dt, now);
            for event in events {
                self.event(CombatEvent::Shared(event));
            }
            if let Some(images) = frames {
                images_out = Some(images);
            }
        }
        if world.player.hp <= 0. {
            if self.dead_for == 0. {
                self.event(CombatEvent::Death {
                    life: self.life,
                    killer: world.last_attacker,
                    shot: world.last_shot,
                });
            }
            self.dead_for += dt;
        }
        (self.dead_for >= RESPAWN_SECONDS, images_out)
    }

    pub fn respawn(&mut self, world: &mut World, class: ShipClass, seed: u64) {
        self.life = self.life.wrapping_add(1);
        self.dead_for = 0.;
        let score = world.score;
        *world = World::duel(seed, class, world.player.id);
        world.score = score;
    }
}
