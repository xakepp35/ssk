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
        })
    }
    pub fn receive(&mut self, world: &mut World, now: f64) {
        for _ in 0..512 {
            let Ok(event) = self.net.incoming.try_recv() else {
                break;
            };
            match event {
                NetEvent::Packet {
                    peer,
                    kind: 0,
                    bytes,
                } => {
                    if let Some(snapshot) = Snapshot::decode(&bytes) {
                        self.replicas.push(peer, snapshot, now);
                    }
                }
                NetEvent::Packet {
                    peer,
                    kind: 1,
                    bytes,
                } => {
                    if let Some(event) = CombatEvent::decode(&bytes) {
                        match event {
                            CombatEvent::Shot {
                                mut shot,
                                sector,
                                life,
                                ..
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
                                if (shot.pos - world.player.pos).length() > 5000. {
                                    continue;
                                }
                                self.seen.insert(key.clone());
                                self.order.push_back(key);
                                if self.order.len() > 4096 {
                                    self.seen.remove(&self.order.pop_front().unwrap());
                                }
                                world.spawn_shot(shot);
                            }
                            CombatEvent::Death {
                                life,
                                killer,
                                shot: _,
                            } => {
                                let key = (peer, life);
                                if self.deaths.insert(key.clone()) {
                                    self.death_order.push_back(key);
                                    if self.death_order.len() > 1024 {
                                        self.deaths.remove(&self.death_order.pop_front().unwrap());
                                    }
                                    if killer == world.player.id {
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
                }
                NetEvent::Error(error) => self.error = error,
                _ => {}
            }
        }
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
    pub fn after_frame(&mut self, world: &World, dt: f32) -> bool {
        for event in &world.events {
            if let WorldEvent::Shot(shot) = event {
                if shot.owner == world.player.id {
                    self.event(CombatEvent::Shot {
                        shot: *shot,
                        sector: world.origin,
                        life: self.life,
                        tick: world.tick,
                    });
                }
            }
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
