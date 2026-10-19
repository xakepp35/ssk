use crate::*;
use iroh::{Endpoint, EndpointAddr};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{Duration, Instant},
};
pub(crate) struct Overlay {
    pub id: String,
    pub contacts: BTreeMap<String, EndpointAddr>,
    pub records: BTreeMap<String, Vec<u8>>,
    pub connecting: HashSet<String>,
    pub last_change: Instant,
}
impl Overlay {
    pub fn new(id: String) -> Self {
        Self {
            id,
            contacts: BTreeMap::new(),
            records: BTreeMap::new(),
            connecting: HashSet::new(),
            last_change: Instant::now(),
        }
    }
    pub fn members(&self) -> Vec<String> {
        self.contacts.keys().cloned().collect()
    }
    pub fn learn(&mut self, id: String, bytes: &[u8], signed: &[u8]) -> bool {
        if self.contacts.len() >= 128 && !self.contacts.contains_key(&id) {
            return false;
        }
        let Ok(addr) = serde_json::from_slice::<EndpointAddr>(bytes) else {
            return false;
        };
        if addr.id.to_string() != id || addr.addrs.len() > 16 {
            return false;
        }
        if !self.contacts.contains_key(&id) {
            self.last_change = Instant::now();
        }
        self.contacts.insert(id.clone(), addr);
        self.records.insert(id, signed.to_vec());
        true
    }
    pub fn route(
        &self,
        dest: &str,
        peers: &HashMap<String, crate::reliable::Peer>,
    ) -> Option<String> {
        if peers.contains_key(dest) {
            return Some(dest.into());
        }
        let candidates = topology::intermediaries(&self.members(), &self.id, dest);
        candidates
            .into_iter()
            .filter(|p| peers.get(p).is_some_and(|p| p.direct()))
            .min_by(|a, b| {
                let cost = |p: &str| {
                    let s = &peers[p].metrics.stats;
                    s.rtt_ms.unwrap_or(1000.) + s.jitter_ms * 2. + s.loss * 100. + s.queued as f32
                };
                cost(a).total_cmp(&cost(b)).then_with(|| {
                    stable_rank(&self.id, dest, a).cmp(&stable_rank(&self.id, dest, b))
                })
            })
    }
    pub fn desired(&self) -> std::collections::BTreeSet<String> {
        topology::neighbours(&self.members(), &self.id)
    }
    pub fn envelope(dest: &str, kind: u8, hops: u8, data: &[u8]) -> Option<Vec<u8>> {
        let key: iroh::PublicKey = dest.parse().ok()?;
        let mut b = key.as_bytes().to_vec();
        b.extend([kind, hops]);
        b.extend(data);
        Some(b)
    }
    pub fn queue(
        &self,
        kind: u8,
        data: &[u8],
        peers: &mut HashMap<String, crate::reliable::Peer>,
        seq: u64,
    ) {
        for dest in self.contacts.keys().filter(|p| *p != &self.id) {
            if let Some(next) = self.route(dest, peers) {
                if let Some(frame) = Self::envelope(dest, kind, 0, data) {
                    let p = peers.get_mut(&next).unwrap();
                    if kind == 0 {
                        p.state(dest.clone(), frame);
                    } else {
                        p.queue(seq, frame, seq);
                    }
                }
            }
        }
    }
    pub fn connect_tasks(
        &mut self,
        peers: &HashMap<String, crate::reliable::Peer>,
        endpoint: &Endpoint,
    ) -> Vec<(String, EndpointAddr)> {
        let _ = endpoint;
        let desired: Vec<_> = self
            .desired()
            .into_iter()
            .filter(|id| !peers.contains_key(id) && !self.connecting.contains(id) && self.id < *id)
            .collect();
        desired
            .into_iter()
            .filter_map(|id| {
                let addr = self.contacts.get(&id)?.clone();
                self.connecting.insert(id.clone());
                Some((id, addr))
            })
            .collect()
    }
    pub fn settled(&self) -> bool {
        self.last_change.elapsed() > Duration::from_secs(10)
    }
    pub fn receive(
        &mut self,
        remote: &str,
        frame_kind: u8,
        frame: &[u8],
        peers: &mut HashMap<String, crate::reliable::Peer>,
        auth: &mut crate::auth::Auth,
        worker: &Worker,
        seq: u64,
    ) {
        if frame_kind == 3 {
            if let Some((origin, data)) = auth.verify(3, frame) {
                self.learn(origin, &data, frame);
            }
            return;
        }
        if frame_kind != 4 || frame.len() < 34 {
            return;
        }
        let Ok(dest) = iroh::PublicKey::from_bytes(frame[..32].try_into().unwrap()) else {
            return;
        };
        let dest = dest.to_string();
        let kind = frame[32];
        let hops = frame[33];
        if kind > 1 || hops > 1 {
            return;
        }
        let Some((origin, data)) = auth.verify(kind, &frame[34..]) else {
            return;
        };
        if (hops == 0 && origin != remote)
            || (hops == 1 && !peers.get(remote).is_some_and(|p| p.direct()))
        {
            return;
        }
        if dest == self.id {
            if worker
                .incoming
                .try_send(NetEvent::Packet {
                    peer: origin,
                    kind,
                    bytes: data,
                })
                .is_err()
            {
                worker.status.send_modify(|s| s.dropped += 1);
            }
            return;
        }
        if hops != 0 || !peers.get(remote).is_some_and(|p| p.direct()) {
            return;
        }
        if let Some(peer) = peers.get_mut(&dest).filter(|p| p.direct()) {
            let mut forwarded = frame.to_vec();
            forwarded[33] = 1;
            if kind == 0 {
                peer.state(format!("{origin}:{dest}"), forwarded);
            } else if !peer.queue(seq, forwarded, seq) {
                worker.status.send_modify(|s| s.dropped += 1);
            }
        }
    }
}
fn stable_rank(a: &str, b: &str, c: &str) -> u64 {
    a.bytes()
        .chain(b.bytes())
        .chain(c.bytes())
        .fold(0xcbf29ce484222325, |h, x| {
            (h ^ x as u64).wrapping_mul(0x100000001b3)
        })
}
