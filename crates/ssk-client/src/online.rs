use ssk_net::constants::message;
// Transport adapter: decode authenticated payloads and hand them to the simulation.
use crate::constants::*;
use ssk_net::{CombatEventCodec, NetEvent, Network, SectorAssembler, SharedChunk, SnapshotCodec};
use ssk_sim::*;
pub struct Online {
    pub net: Network,
    game: PeerCombat,
    assemblies: SectorAssembler,
    sequence: u32,
}
impl std::ops::Deref for Online {
    type Target = PeerCombat;
    fn deref(&self) -> &PeerCombat {
        &self.game
    }
}
impl std::ops::DerefMut for Online {
    fn deref_mut(&mut self) -> &mut PeerCombat {
        &mut self.game
    }
}
impl Online {
    pub fn new(config: ssk_net::Config, connect: Option<String>) -> Result<Self, String> {
        Ok(Self {
            net: Network::start(config, connect).map_err(|e| e.to_string())?,
            game: Default::default(),
            assemblies: Default::default(),
            sequence: 0,
        })
    }
    pub fn shared_stats(&self, now: f64) -> (usize, f64) {
        self.game.shared.stats(now)
    }
    pub fn receive(&mut self, world: &mut World, now: f64) {
        for (peer, bytes, at) in self.net.take_snapshots() {
            if let (Some(id), Some(snapshot)) =
                (ssk_net::player_id(&peer), Snapshot::decode(&bytes))
            {
                self.game
                    .snapshot(peer, id, snapshot, now - at.elapsed().as_secs_f64());
            }
        }
        self.game.maintain(world, now);
        for _ in 0..MAX_RECEIVE_EVENTS {
            let Ok(packet) = self.net.incoming.try_recv() else {
                break;
            };
            match packet {
                NetEvent::Packet { peer, kind, bytes } => {
                    let Some(id) = ssk_net::player_id(&peer) else {
                        continue;
                    };
                    match kind {
                        message::STATE => {
                            if let Some(snapshot) = Snapshot::decode(&bytes) {
                                self.game.snapshot(peer, id, snapshot, now);
                            }
                        }
                        message::CRITICAL => {
                            if let Some(event) = CombatEvent::decode(&bytes) {
                                self.game.receive(world, peer, id, event, now);
                            }
                        }
                        message::SECTOR => {
                            if let Some(c) = SharedChunk::decode(&bytes) {
                                if self
                                    .game
                                    .shared
                                    .accepts_image(id, c.sector, c.epoch, c.revision, now)
                                {
                                    if let Some((sector, epoch, state)) =
                                        self.assemblies.push(id, c, now)
                                    {
                                        self.game.shared.image(id, sector, epoch, state, now);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                NetEvent::Disconnected(peer) => self.game.disconnect(&peer),
                NetEvent::Error(error) => self.game.error = error,
                _ => {}
            }
        }
    }
    pub fn snapshot(&mut self, world: &World) {
        self.sequence = self.sequence.wrapping_add(1);
        self.net
            .publish(Snapshot::capture(world, self.sequence, self.game.life).encode());
    }
    pub fn after_frame(&mut self, world: &mut World, dt: f32, now: f64) -> bool {
        let (respawn, images) = self.game.after_step(world, dt, now);
        if let Some(images) = images {
            self.net.shared(encode_images(images));
        }
        while let Some(event) = self.game.outbox.front() {
            if self.net.event(event.encode()).is_err() {
                break;
            }
            self.game.outbox.pop_front();
        }
        respawn
    }
    pub fn respawn(&mut self, world: &mut World, class: ShipClass, seed: u64) {
        self.game.respawn(world, class, seed);
        self.snapshot(world);
    }
    pub fn leave(&mut self) {
        self.net
            .shared(encode_images(self.game.shared.checkpoint()));
        for event in self.game.shared.release() {
            let _ = self.net.event(CombatEvent::Shared(event).encode());
        }
        std::thread::sleep(std::time::Duration::from_millis(LEAVE_GRACE_MILLIS));
    }
}
fn encode_images(images: Vec<SectorImage>) -> Vec<Vec<u8>> {
    images
        .into_iter()
        .flat_map(|i| SharedChunk::snapshot(i.sector, i.epoch, &i.state))
        .collect()
}
