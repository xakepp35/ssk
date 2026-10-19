use ssk_net::{CombatEvent, NetEvent, Network, Replicas, Snapshot};
use ssk_sim::*;
use std::collections::{HashSet, VecDeque};
pub struct Online {
    pub net: Network,
    pub replicas: Replicas,
    pub life: u32,
    pub frags: u32,
    pub error: String,
    pub dead_for: f32,
    sequence: u32,
    outbox: VecDeque<Vec<u8>>,
    seen: HashSet<(String, u32, u64)>,
    order: VecDeque<(String, u32, u64)>,
    deaths: HashSet<(String, u32)>,
    death_order: VecDeque<(String, u32)>,
    guard: ssk_net::CombatGuard,
    own_history: VecDeque<(f64, WorldPos)>,
    fired: VecDeque<(u64, f64)>,
    shared: crate::shared::Shared,
}
impl Online {
    pub fn new(config: ssk_net::Config, connect: Option<String>) -> Result<Self, String> {
        Ok(Self {
            net: Network::start(config, connect).map_err(|e| e.to_string())?,
            replicas: Replicas::default(),
            life: 0,
            frags: 0,
            error: String::new(),
            dead_for: 0.,
            sequence: 0,
            outbox: VecDeque::with_capacity(256),
            seen: HashSet::new(),
            order: VecDeque::new(),
            deaths: HashSet::new(),
            death_order: VecDeque::new(),
            guard: Default::default(),
            own_history: VecDeque::new(),
            fired: VecDeque::new(),
            shared: Default::default(),
        })
    }
    pub fn receive(&mut self, world: &mut World, now: f64) {
        self.shared.members(world, self.life, &self.replicas, now);
        let stale: Vec<_> = self
            .replicas
            .peers
            .iter()
            .filter(|(_, q)| q.back().is_none_or(|(t, _)| now - t > 5.))
            .map(|(p, _)| p.clone())
            .collect();
        for peer in stale {
            self.replicas.peers.remove(&peer);
            self.guard.forget(&peer);
        }
        for _ in 0..512 {
            let Ok(event) = self.net.incoming.try_recv() else {
                break;
            };
            match event {
                NetEvent::Packet {
                    peer,
                    kind: 6,
                    bytes,
                } => {
                    if let Some(id) = ssk_net::player_id(&peer) {
                        self.shared.chunk(id, &bytes, now);
                    }
                }
                NetEvent::Packet {
                    peer,
                    kind: 0,
                    bytes,
                } => {
                    if let Some(snapshot) = Snapshot::decode(&bytes) {
                        let old = self
                            .replicas
                            .peers
                            .get(&peer)
                            .and_then(|q| q.back())
                            .map(|(_, s)| *s);
                        if self.guard.snapshot(old, snapshot) {
                            self.replicas.push(peer, snapshot, now);
                        }
                    }
                }
                NetEvent::Packet {
                    peer,
                    kind: 1,
                    bytes,
                } => {
                    if let Some(event) = CombatEvent::decode(&bytes) {
                        if matches!(&event, CombatEvent::Shot { .. })
                            && !self.guard.shot(&peer, &event, &self.replicas)
                        {
                            continue;
                        }
                        match event {
                            CombatEvent::Shared(event) => {
                                if let Some(id) = ssk_net::player_id(&peer) {
                                    let replies =
                                        self.shared.event(id, event, world, self.life, now);
                                    for e in replies {
                                        self.event(CombatEvent::Shared(e));
                                    }
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
                                    continue;
                                }
                                if !shot.pos.finite()
                                    || !shot.angle.is_finite()
                                    || !shot.damage_scale.is_finite()
                                {
                                    continue;
                                }
                                if self
                                    .replicas
                                    .peers
                                    .get(&peer)
                                    .and_then(|q| q.back())
                                    .is_some_and(|(_, s)| s.life != life)
                                {
                                    continue;
                                }
                                shot.owner = ssk_net::player_id(&peer).unwrap_or(0);
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
                                if (shot.pos - world.player.pos).length() > 5000. {
                                    continue;
                                }
                                self.seen.insert(key.clone());
                                self.order.push_back(key);
                                if self.order.len() > 4096 {
                                    self.seen.remove(&self.order.pop_front().unwrap());
                                }
                                let lag = self
                                    .replicas
                                    .peers
                                    .get(&peer)
                                    .and_then(|q| q.back())
                                    .map(|(time, s)| {
                                        ((now - time) + (s.tick as f64 - tick as f64) / 240.)
                                            .clamp(0., 0.2)
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
                                    if self.death_order.len() > 1024 {
                                        self.deaths.remove(&self.death_order.pop_front().unwrap());
                                    }
                                    if killer == world.player.id
                                        && self
                                            .fired
                                            .iter()
                                            .any(|(id, time)| *id == shot && now - time <= 2.)
                                    {
                                        self.frags += 1;
                                        world.award(500);
                                    }
                                }
                            }
                        }
                    }
                }
                NetEvent::Disconnected(peer) => {
                    self.replicas.peers.remove(&peer);
                    self.guard.forget(&peer);
                }
                NetEvent::Error(error) => self.error = error,
                _ => {}
            }
        }
    }
    pub fn leave(&mut self) {
        self.net.shared(self.shared.checkpoint());
        for event in self.shared.release() {
            let _ = self.net.event(CombatEvent::Shared(event).encode());
        }
        // Best effort final checkpoint/release; crash recovery uses the lease timeout.
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    pub fn snapshot(&mut self, world: &World) {
        self.sequence = self.sequence.wrapping_add(1);
        self.net
            .publish(Snapshot::capture(world, self.sequence, self.life).encode());
    }
    fn event(&mut self, event: CombatEvent) {
        if self.outbox.len() < 256 {
            self.outbox.push_back(event.encode());
        } else {
            self.error = "Critical game event backlog full".into();
        }
    }
    pub fn after_frame(&mut self, world: &mut World, dt: f32) -> bool {
        let now = macroquad::time::get_time();
        self.own_history.push_back((
            now,
            WorldPos {
                sector: world.origin,
                local: world.player.pos,
            },
        ));
        while self.own_history.front().is_some_and(|(t, _)| now - t > 2.) {
            self.own_history.pop_front();
        }
        while self.fired.front().is_some_and(|(_, t)| now - t > 2.) {
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
        let (events, frames) = self.shared.tick(world, self.life, &self.replicas, dt, now);
        for event in events {
            self.event(CombatEvent::Shared(event));
        }
        if let Some(frames) = frames {
            self.net.shared(frames);
        }
        if world.player.hp <= 0. {
            if self.dead_for == 0. {
                self.event(CombatEvent::Death {
                    life: self.life,
                    killer: world.last_attacker,
                    shot: world.last_shot,
                });
                self.snapshot(world);
            }
            self.dead_for += dt;
        }
        while let Some(event) = self.outbox.pop_front() {
            if let Err(event) = self.net.event(event) {
                self.outbox.push_front(event);
                break;
            }
        }
        self.dead_for >= 2.
    }
    pub fn respawn(&mut self, world: &mut World, class: ShipClass, seed: u64) {
        self.life = self.life.wrapping_add(1);
        self.dead_for = 0.;
        let score = world.score;
        *world = World::duel(seed, class, self.net.player_id);
        world.score = score;
        self.snapshot(world);
    }
}
