use crate::*;
use iroh::endpoint::Connection;
use std::collections::{HashSet, VecDeque};
struct Pending {
    id: u64,
    data: Vec<u8>,
    last: u64,
    born: u64,
}
pub(crate) struct Peer {
    pub conn: Connection,
    pending: VecDeque<Pending>,
    acks: VecDeque<u64>,
    seen: HashSet<u64>,
    order: VecDeque<u64>,
}
impl Peer {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            pending: VecDeque::new(),
            acks: VecDeque::new(),
            seen: HashSet::new(),
            order: VecDeque::new(),
        }
    }
    pub fn queue(&mut self, id: u64, data: Vec<u8>, tick: u64) -> bool {
        if self.pending.len() >= 128 || data.len() > 880 {
            return false;
        }
        self.pending.push_back(Pending {
            id,
            data,
            last: 0,
            born: tick,
        });
        true
    }
    pub fn packet(&mut self, seq: u64, state: &[u8], hz: u32) -> Vec<u8> {
        let mut packet = b"SSK1".to_vec();
        packet.extend(seq.to_le_bytes());
        for _ in 0..16 {
            if let Some(id) = self.acks.pop_front() {
                append(&mut packet, 2, &id.to_le_bytes());
            } else {
                break;
            }
        }
        append(&mut packet, 0, state);
        self.pending
            .retain(|e| seq.saturating_sub(e.born) < hz as u64 * 5);
        for _ in 0..self.pending.len() {
            let mut event = self.pending.pop_front().unwrap();
            if event.last == 0 || seq.saturating_sub(event.last) >= (hz / 10).max(1) as u64 {
                let mut payload = event.id.to_le_bytes().to_vec();
                payload.extend(&event.data);
                if append(&mut packet, 1, &payload) {
                    event.last = seq;
                }
            }
            self.pending.push_back(event);
        }
        packet
    }
    pub fn receive(&mut self, id: &str, bytes: &[u8], worker: &Worker) {
        if !bytes.starts_with(b"SSK1") || bytes.len() < 12 {
            return;
        }
        let mut rest = &bytes[12..];
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
                self.pending.retain(|e| e.id != ack);
                continue;
            }
            let (event_id, data) = if kind == 1 && len > 8 {
                (
                    Some(u64::from_le_bytes(payload[..8].try_into().unwrap())),
                    &payload[8..],
                )
            } else if kind == 0 {
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
            if worker
                .incoming
                .try_send(NetEvent::Packet {
                    peer: id.into(),
                    kind,
                    bytes: data.to_vec(),
                })
                .is_ok()
            {
                if let Some(e) = event_id {
                    self.seen.insert(e);
                    self.order.push_back(e);
                    if self.order.len() > 2048 {
                        self.seen.remove(&self.order.pop_front().unwrap());
                    }
                    self.ack(e);
                }
            } else {
                worker.status.send_modify(|s| s.dropped += 1);
            }
        }
    }
    fn ack(&mut self, id: u64) {
        if self.acks.len() < 128 && !self.acks.contains(&id) {
            self.acks.push_back(id);
        }
    }
}
fn append(packet: &mut Vec<u8>, kind: u8, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    if packet.len() + bytes.len() + 3 > 1100 {
        return false;
    }
    packet.push(kind);
    packet.extend((bytes.len() as u16).to_le_bytes());
    packet.extend(bytes);
    true
}
