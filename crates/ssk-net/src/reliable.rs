use iroh::endpoint::Connection;
use std::collections::{HashMap, HashSet, VecDeque};
struct Pending {
    id: u64,
    digest: [u8; 32],
    data: Vec<u8>,
    last: u64,
    born: u64,
}
pub(crate) struct Peer {
    pub conn: Connection,
    pending: VecDeque<Pending>,
    queued: HashSet<[u8; 32]>,
    acks: VecDeque<u64>,
    seen: HashSet<u64>,
    order: VecDeque<u64>,
    pub metrics: crate::metrics::Metrics,
    states: HashMap<String, Vec<u8>>,
    state_order: VecDeque<String>,
    next_event: u64,
}
impl Peer {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            pending: VecDeque::new(),
            queued: HashSet::new(),
            acks: VecDeque::new(),
            seen: HashSet::new(),
            order: VecDeque::new(),
            metrics: crate::metrics::Metrics::new(),
            states: HashMap::new(),
            state_order: VecDeque::new(),
            next_event: 0,
        }
    }
    pub fn queue(&mut self, _id: u64, data: Vec<u8>, tick: u64) -> bool {
        let digest = *blake3::hash(&data).as_bytes();
        if self.queued.contains(&digest) {
            return true;
        }
        if self.pending.len() >= 128 || data.len() > 960 {
            return false;
        }
        if let Some(origin) = data.get(34..66) {
            if self
                .pending
                .iter()
                .filter(|p| p.data.get(34..66) == Some(origin))
                .count()
                >= 32
            {
                return false;
            }
        }
        self.next_event += 1;
        self.queued.insert(digest);
        self.pending.push_back(Pending {
            id: self.next_event,
            digest,
            data,
            last: 0,
            born: tick,
        });
        true
    }
    pub fn direct(&self) -> bool {
        self.conn
            .paths()
            .iter()
            .any(|p| p.is_selected() && p.is_ip())
    }
    pub fn state(&mut self, key: String, data: Vec<u8>) {
        if !self.states.contains_key(&key) {
            if self.states.len() >= 256 {
                return;
            }
            self.state_order.push_back(key.clone());
        }
        self.states.insert(key, data);
    }
    pub fn shared_batch(&mut self, prefix: &str, frames: Vec<(String, Vec<u8>)>) {
        // Admit an entire revision, and finish the previous one before replacing it.
        if self.states.keys().any(|k| k.starts_with(prefix))
            || self.states.len() + frames.len() > 256
        {
            return;
        }
        for (key, data) in frames {
            self.state(key, data);
        }
    }
    pub fn packet(
        &mut self,
        seq: u64,
        control_kind: u8,
        contact: &[u8],
        hz: u32,
        limit: usize,
    ) -> Vec<u8> {
        let mut packet = self.metrics.header(seq);
        append(&mut packet, control_kind, contact, limit);
        for _ in 0..16 {
            let Some(id) = self.acks.front() else {
                break;
            };
            if !append(&mut packet, 2, &id.to_le_bytes(), limit) {
                break;
            }
            self.acks.pop_front();
        }
        // Ship positions must advance even while reliable combat traffic is busy.
        let ship_limit = limit.min(packet.len() + 420);
        self.drain_states(&mut packet, false, ship_limit);
        // Give sector checkpoints a bounded share instead of starving indefinitely.
        if seq % 4 == 0 {
            let shared_limit = limit.min(packet.len() + 570);
            self.drain_states(&mut packet, true, shared_limit);
        }
        let before = self.pending.len();
        self.pending.retain(|e| {
            let keep = seq.saturating_sub(e.born) < hz as u64 * 5;
            if !keep {
                self.queued.remove(&e.digest);
            }
            keep
        });
        self.metrics.stats.expired += (before - self.pending.len()) as u64;
        self.metrics.stats.queued = self.pending.len();
        for _ in 0..self.pending.len() {
            let mut event = self.pending.pop_front().unwrap();
            if event.last == 0 || seq.saturating_sub(event.last) >= (hz / 10).max(1) as u64 {
                let mut payload = event.id.to_le_bytes().to_vec();
                payload.extend(&event.data);
                if append(&mut packet, 1, &payload, limit) {
                    event.last = seq;
                }
            }
            self.pending.push_back(event);
        }
        self.drain_states(&mut packet, false, limit);
        self.drain_states(&mut packet, true, limit);
        packet
    }
    fn drain_states(&mut self, packet: &mut Vec<u8>, shared: bool, limit: usize) {
        for _ in 0..self.state_order.len() {
            let key = self.state_order.pop_front().unwrap();
            if key.starts_with("shared:") != shared {
                self.state_order.push_back(key);
                continue;
            }
            let data = self.states.remove(&key).unwrap();
            if !append(packet, 4, &data, limit) {
                self.states.insert(key.clone(), data);
                self.state_order.push_back(key);
            }
        }
    }
    pub fn receive(&mut self, bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
        let mut frames = Vec::new();
        if !self.metrics.receive(bytes) {
            return frames;
        }
        let mut rest = &bytes[24..];
        while rest.len() >= 3 {
            let kind = rest[0];
            let len = u16::from_le_bytes([rest[1], rest[2]]) as usize;
            rest = &rest[3..];
            if rest.len() < len {
                break;
            }
            let payload = &rest[..len];
            rest = &rest[len..];
            if kind == 2 && len == 8 {
                let ack = u64::from_le_bytes(payload.try_into().unwrap());
                self.pending.retain(|e| {
                    if e.id == ack {
                        self.queued.remove(&e.digest);
                        false
                    } else {
                        true
                    }
                });
                continue;
            }
            let (event_id, data) = if kind == 1 && len > 8 {
                (
                    Some(u64::from_le_bytes(payload[..8].try_into().unwrap())),
                    &payload[8..],
                )
            } else if kind == 4 || kind == 3 || kind == 5 {
                (None, payload)
            } else {
                continue;
            };
            if let Some(e) = event_id {
                if self.seen.contains(&e) {
                    self.ack(e);
                    continue;
                }
            }
            frames.push((if kind == 1 { 4 } else { kind }, data.to_vec()));
            if let Some(e) = event_id {
                self.seen.insert(e);
                self.order.push_back(e);
                if self.order.len() > 2048 {
                    self.seen.remove(&self.order.pop_front().unwrap());
                }
                self.ack(e);
            }
        }
        frames
    }
    fn ack(&mut self, id: u64) {
        if self.acks.len() < 128 && !self.acks.contains(&id) {
            self.acks.push_back(id);
        }
    }
}
fn append(packet: &mut Vec<u8>, kind: u8, bytes: &[u8], limit: usize) -> bool {
    if bytes.is_empty() {
        return true;
    }
    if packet.len() + bytes.len() + 3 > limit {
        return false;
    }
    packet.push(kind);
    packet.extend((bytes.len() as u16).to_le_bytes());
    packet.extend(bytes);
    true
}
