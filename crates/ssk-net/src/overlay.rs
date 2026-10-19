use crate::*;
use iroh::{Endpoint, EndpointAddr};
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};
pub(crate) struct Overlay {
    pub id: String,
    pub contacts: BTreeMap<String, EndpointAddr>,
    pub records: BTreeMap<String, Vec<u8>>,
    pub connecting: HashMap<String, Instant>,
    pub last_change: Instant,
    pub routes: crate::routes::Routes,
    pub presence: HashMap<String, crate::contact::Contact>,
    pub positions: HashMap<String, Snapshot>,
    pub radius: f32,
    pub position_seen: HashMap<String, Instant>,
    pub fallback: BTreeMap<String, EndpointAddr>,
    pub friends: std::collections::BTreeSet<String>,
    retries: HashMap<String, (u32, Instant)>,
    cover: HashMap<String, std::collections::BTreeSet<String>>,
    dirty: bool,
}
impl Overlay {
    pub fn new(id: String) -> Self {
        Self {
            id,
            contacts: BTreeMap::new(),
            records: BTreeMap::new(),
            connecting: HashMap::new(),
            last_change: Instant::now(),
            routes: Default::default(),
            presence: HashMap::new(),
            positions: HashMap::new(),
            radius: 3000.,
            position_seen: HashMap::new(),
            fallback: BTreeMap::new(),
            friends: Default::default(),
            retries: HashMap::new(),
            cover: HashMap::new(),
            dirty: true,
        }
    }
    pub fn members(&self) -> Vec<String> {
        self.contacts
            .keys()
            .filter(|id| self.presence.get(*id).is_some_and(|c| c.playing))
            .cloned()
            .collect()
    }
    pub fn learn(&mut self, id: String, bytes: &[u8], signed: &[u8]) -> bool {
        if self.contacts.len() >= 128 && !self.contacts.contains_key(&id) {
            return false;
        }
        let Some(contact) = crate::contact::Contact::decode(&id, bytes) else {
            return false;
        };
        let addr = contact.addr.clone();
        if self
            .presence
            .get(&id)
            .is_some_and(|old| old.expires > contact.expires)
        {
            return false;
        }
        if addr.id.to_string() != id || addr.addrs.len() > 16 {
            return false;
        }
        if !self.contacts.contains_key(&id) {
            self.last_change = Instant::now();
        }
        if self
            .presence
            .get(&id)
            .is_none_or(|old| old.playing != contact.playing)
        {
            self.dirty = true;
        }
        self.contacts.insert(id.clone(), addr);
        self.presence.insert(id.clone(), contact);
        self.records.insert(id, signed.to_vec());
        true
    }
    pub fn route(
        &mut self,
        dest: &str,
        peers: &HashMap<String, crate::reliable::Peer>,
    ) -> Vec<String> {
        if peers
            .get(dest)
            .is_some_and(|p| p.metrics.stats.age_ms < 2000. && p.metrics.stats.queued < 96)
        {
            return vec![dest.into()];
        }
        self.refresh_graph();
        let candidates: Vec<_> = match (self.cover.get(&self.id), self.cover.get(dest)) {
            (Some(a), Some(b)) => a.intersection(b).cloned().collect(),
            _ => vec![],
        };
        let ranked = candidates
            .into_iter()
            .filter_map(|id| {
                let local = peers.get(&id)?;
                if !local.direct()
                    || local.metrics.stats.age_ms > 2000.
                    || local.metrics.stats.queued >= 96
                {
                    return None;
                }
                let (remote, time) = self.routes.legs.get(&(id.clone(), dest.into()))?;
                if !remote.direct
                    || time.elapsed().as_secs() > 5
                    || remote.age_ms > 2000.
                    || remote.queued >= 96
                {
                    return None;
                }
                let cost = |s: &LinkStats| {
                    s.rtt_ms.unwrap_or(1000.) + s.jitter_ms * 2. + s.loss * 100. + s.queued as f32
                };
                let balance = (stable_rank(&self.id, dest, &id) % 1000) as f32 / 1000.;
                Some((id, cost(&local.metrics.stats) + cost(remote) + balance))
            })
            .collect();
        self.routes.select(dest, ranked)
    }
    fn refresh_graph(&mut self) {
        if !self.dirty {
            return;
        }
        let members = self.members();
        self.cover = members
            .iter()
            .map(|id| (id.clone(), topology::neighbours(&members, id)))
            .collect();
        self.dirty = false;
    }
    pub fn desired(&mut self) -> std::collections::BTreeSet<String> {
        self.refresh_graph();
        let mut desired = self.cover.get(&self.id).cloned().unwrap_or_default();
        desired.extend(self.friends.iter().cloned());
        desired
    }
    pub fn connected(&mut self, id: &str) {
        self.connecting.remove(id);
        self.retries
            .insert(id.into(), (0, Instant::now() + Duration::from_secs(1)));
    }
    pub fn envelope(dest: &str, kind: u8, hops: u8, data: &[u8]) -> Option<Vec<u8>> {
        let key: iroh::PublicKey = dest.parse().ok()?;
        let mut b = key.as_bytes().to_vec();
        b.extend([kind, hops]);
        b.extend(data);
        Some(b)
    }
    pub fn queue(
        &mut self,
        kind: u8,
        data: &[u8],
        peers: &mut HashMap<String, crate::reliable::Peer>,
        seq: u64,
    ) {
        let destinations: Vec<_> = self
            .contacts
            .keys()
            .filter(|p| *p != &self.id && self.interested(p))
            .cloned()
            .collect();
        for dest in destinations {
            if kind == 1 {
                if self.routes.pending.len() < 4096 {
                    if let Some(id) = crate::routes::event_id(data) {
                        self.routes.inflight.insert((dest.clone(), id));
                    }
                    self.routes.pending.push_back(crate::routes::Pending {
                        dest,
                        data: data.to_vec(),
                        born: seq,
                        last: 0,
                    });
                } else {
                    self.routes.expired += 1;
                }
            } else {
                self.send(&dest, kind, data, peers, seq, 1);
            }
        }
    }
    pub fn shared(
        &mut self,
        frames: &[Vec<u8>],
        peers: &mut HashMap<String, crate::reliable::Peer>,
    ) {
        let destinations: Vec<_> = self
            .contacts
            .keys()
            .filter(|p| *p != &self.id && self.interested(p))
            .cloned()
            .collect();
        let mut sectors: BTreeMap<Vec<u8>, Vec<&Vec<u8>>> = BTreeMap::new();
        for frame in frames {
            if let Some(sector) = frame.get(49..65) {
                sectors.entry(sector.to_vec()).or_default().push(frame);
            }
        }
        for dest in destinations {
            let Some(next) = self.route(&dest, peers).into_iter().next() else {
                continue;
            };
            for (sector, parts) in &sectors {
                let prefix = format!("shared:{}:{dest}:{sector:?}:", self.id);
                let batch = parts
                    .iter()
                    .filter_map(|data| {
                        let key = format!("{prefix}{:?}", data.get(65..83));
                        Some((key, Self::envelope(&dest, 6, 0, data)?))
                    })
                    .collect();
                peers.get_mut(&next).unwrap().shared_batch(&prefix, batch);
            }
        }
    }
    fn interested(&self, id: &str) -> bool {
        let Some(remote) = self.presence.get(id).filter(|c| c.playing) else {
            return false;
        };
        let Some(local) = self.presence.get(&self.id).filter(|c| c.playing) else {
            return false;
        };
        if self
            .position_seen
            .get(id)
            .is_some_and(|t| t.elapsed().as_secs_f32() < 1.)
        {
            if let (Some(a), Some(b)) = (self.positions.get(&self.id), self.positions.get(id)) {
                return (b.pos.relative_to(a.pos.sector) - a.pos.local).length()
                    <= self.radius + 1560. + 832. * 0.2;
            }
        }
        (remote.sector[0] as i128 - local.sector[0] as i128).abs() <= 1
            && (remote.sector[1] as i128 - local.sector[1] as i128).abs() <= 1
    }
    fn send(
        &mut self,
        dest: &str,
        kind: u8,
        data: &[u8],
        peers: &mut HashMap<String, crate::reliable::Peer>,
        seq: u64,
        copies: usize,
    ) {
        for next in self.route(dest, peers).into_iter().take(copies) {
            if let Some(frame) = Self::envelope(dest, kind, 0, data) {
                let peer = peers.get_mut(&next).unwrap();
                if kind == 0 || kind == 6 {
                    let key = if kind == 6 {
                        format!(
                            "shared:{}:{dest}:{:?}:{:?}",
                            self.id,
                            data.get(49..65),
                            data.get(65..83)
                        )
                    } else {
                        dest.into()
                    };
                    peer.state(key, frame);
                } else if kind == 2 {
                    peer.receipt(frame);
                } else {
                    peer.queue(seq, frame, seq);
                }
            }
        }
    }
    pub fn retry(&mut self, peers: &mut HashMap<String, crate::reliable::Peer>, seq: u64, hz: u32) {
        for _ in 0..self.routes.pending.len() {
            let mut p = self.routes.pending.pop_front().unwrap();
            let key = crate::routes::event_id(&p.data).map(|id| (p.dest.clone(), id));
            if key
                .as_ref()
                .is_some_and(|key| self.routes.acknowledged.remove(key))
            {
                self.routes.inflight.remove(key.as_ref().unwrap());
                continue;
            }
            if seq.saturating_sub(p.born) > hz as u64 * 5 {
                if let Some(key) = key {
                    self.routes.inflight.remove(&key);
                }
                self.routes.expired += 1;
                continue;
            }
            if p.last == 0 || seq - p.last >= (hz / 10) as u64 {
                let copies = if seq.saturating_sub(p.born) >= hz as u64 / 3 {
                    3
                } else {
                    1
                };
                self.send(&p.dest, 1, &p.data, peers, seq, copies);
                p.last = seq;
            }
            self.routes.pending.push_back(p);
        }
    }
    pub fn connect_tasks(
        &mut self,
        peers: &HashMap<String, crate::reliable::Peer>,
        endpoint: &Endpoint,
    ) -> Vec<(String, EndpointAddr)> {
        let _ = endpoint;
        let mut desired = self.desired();
        if peers.is_empty() {
            desired.extend(self.fallback.keys().cloned());
        }
        let desired: Vec<_> = desired
            .into_iter()
            .filter(|id| {
                id != &self.id
                    && !peers.contains_key(id)
                    && !self.connecting.contains_key(id)
                    && (self.id < *id
                        || self.fallback.contains_key(id)
                        || self.friends.contains(id))
                    && self
                        .retries
                        .get(id)
                        .is_none_or(|(_, next)| Instant::now() >= *next)
            })
            .take(4usize.saturating_sub(self.connecting.len()))
            .collect();
        desired
            .into_iter()
            .filter_map(|id| {
                let addr = self
                    .contacts
                    .get(&id)
                    .or_else(|| self.fallback.get(&id))?
                    .clone();
                let failures = self
                    .retries
                    .get(&id)
                    .map_or(0, |(n, _)| *n)
                    .saturating_add(1);
                let delay = (1u64 << failures.min(5)) + stable_rank(&self.id, &id, "retry") % 3;
                self.retries.insert(
                    id.clone(),
                    (failures, Instant::now() + Duration::from_secs(delay)),
                );
                self.connecting.insert(id.clone(), Instant::now());
                Some((id, addr))
            })
            .collect()
    }
    pub fn settled(&self) -> bool {
        self.last_change.elapsed() > Duration::from_secs(10)
    }
    pub fn prune(&mut self) -> Vec<String> {
        let expired: Vec<_> = self
            .presence
            .iter()
            .filter(|(_, c)| c.expires <= crate::contact::unix_time())
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            self.presence.remove(id);
            self.contacts.remove(id);
            self.records.remove(id);
            self.positions.remove(id);
            self.position_seen.remove(id);
        }
        if !expired.is_empty() {
            self.dirty = true;
            self.last_change = Instant::now();
        }
        expired
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
        if frame_kind == 5 {
            if let Some((origin, data)) = auth.verify(5, frame) {
                if origin == remote {
                    self.routes.metrics(&origin, &data);
                }
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
        if ![0, 1, 2, 6].contains(&kind) || hops > 1 {
            return;
        }
        let Some((origin, data)) = auth.verify(kind, &frame[34..]) else {
            return;
        };
        if kind == 0 {
            if let Some(s) = Snapshot::decode(&data) {
                if self.contacts.contains_key(&origin) {
                    self.positions.insert(origin.clone(), s);
                    self.position_seen.insert(origin.clone(), Instant::now());
                }
            }
        }
        if (hops == 0 && origin != remote)
            || (hops == 1 && !peers.get(remote).is_some_and(|p| p.direct()))
        {
            return;
        }
        if dest == self.id {
            if kind == 2 {
                if data.len() == 16 && self.routes.acknowledged.len() < 8192 {
                    let key = (origin, data.as_slice().try_into().unwrap());
                    if self.routes.inflight.contains(&key) {
                        self.routes.acknowledged.insert(key);
                    }
                }
                return;
            }
            let key = crate::routes::event_id(&frame[34..]).map(|id| (origin.clone(), id));
            let duplicate = kind == 1
                && key
                    .as_ref()
                    .is_some_and(|key| self.routes.seen.contains(key));
            let delivered = duplicate
                || worker
                    .incoming
                    .try_send(NetEvent::Packet {
                        peer: origin.clone(),
                        kind,
                        bytes: data,
                    })
                    .is_ok();
            if !delivered {
                worker.status.send_modify(|s| s.dropped += 1);
            }
            if kind == 1 && delivered {
                if let Some(key) = key {
                    if self.routes.seen.insert(key.clone()) {
                        self.routes.order.push_back(key.clone());
                    }
                    while self.routes.order.len() > 8192 {
                        let old = self.routes.order.pop_front().unwrap();
                        self.routes.seen.remove(&old);
                        self.routes.acks.remove(&old);
                    }
                    let ack = self
                        .routes
                        .acks
                        .entry(key.clone())
                        .or_insert_with(|| auth.sign(2, &key.1))
                        .clone();
                    self.send(&origin, 2, &ack, peers, seq, 1);
                }
            }
            return;
        }
        if hops != 0 || !peers.get(remote).is_some_and(|p| p.direct()) {
            return;
        }
        if let Some(peer) = peers.get_mut(&dest).filter(|p| p.direct()) {
            let mut forwarded = frame.to_vec();
            forwarded[33] = 1;
            if kind == 0 || kind == 6 {
                let key = if kind == 6 {
                    format!(
                        "shared:{origin}:{dest}:{:?}:{:?}",
                        data.get(..16),
                        data.get(16..34)
                    )
                } else {
                    format!("{origin}:{dest}")
                };
                peer.state(key, forwarded);
            } else if kind == 2 {
                peer.receipt(forwarded);
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
