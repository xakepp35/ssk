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
            radius: DEFAULT_INTEREST_RADIUS,
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
        if self.contacts.len() >= MAX_PEERS && !self.contacts.contains_key(&id) {
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
        if addr.id.to_string() != id || addr.addrs.len() > MAX_ENDPOINT_ADDRS {
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
        if peers.get(dest).is_some_and(|p| {
            p.metrics.stats.age_ms < ROUTE_FRESH_MS && p.metrics.stats.queued < ROUTE_QUEUE_LIMIT
        }) {
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
                    || local.metrics.stats.age_ms > ROUTE_FRESH_MS
                    || local.metrics.stats.queued >= ROUTE_QUEUE_LIMIT
                {
                    return None;
                }
                let (remote, time) = self.routes.legs.get(&(id.clone(), dest.into()))?;
                if !remote.direct
                    || time.elapsed().as_secs() > ROUTE_METRIC_SECONDS
                    || remote.age_ms > ROUTE_FRESH_MS
                    || remote.queued >= ROUTE_QUEUE_LIMIT
                {
                    return None;
                }
                let cost = |s: &LinkStats| {
                    s.rtt_ms.unwrap_or(ROUTE_DEFAULT_RTT_MS)
                        + s.jitter_ms * ROUTE_JITTER_WEIGHT
                        + s.loss * ROUTE_LOSS_WEIGHT
                        + s.queued as f32
                };
                let balance = (stable_rank(&self.id, dest, &id) % ROUTE_BALANCE_BUCKETS) as f32
                    / ROUTE_BALANCE_BUCKETS as f32;
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
        self.retries.insert(
            id.into(),
            (
                0,
                Instant::now() + Duration::from_secs(CONNECT_INITIAL_RETRY_SECONDS),
            ),
        );
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
        if kind == message::STATE || kind == message::LEASE {
            self.batch(kind, &[data.to_vec()], peers);
            return;
        }
        let destinations: Vec<_> = self
            .contacts
            .keys()
            .filter(|p| *p != &self.id && self.interested(p))
            .cloned()
            .collect();
        for dest in destinations {
            if kind == message::CRITICAL {
                if self.routes.pending.len() < ROUTE_EVENT_QUEUE {
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
        self.batch(message::SECTOR, frames, peers);
    }
    fn batch(
        &mut self,
        kind: u8,
        frames: &[Vec<u8>],
        peers: &mut HashMap<String, crate::reliable::Peer>,
    ) {
        let destinations: Vec<_> = self
            .contacts
            .keys()
            .filter(|p| *p != &self.id && self.interested(p))
            .cloned()
            .collect();
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for dest in destinations {
            let Some(next) = self.route(&dest, peers).into_iter().next() else {
                continue;
            };
            groups.entry(next).or_default().push(dest);
        }
        let mut sectors: BTreeMap<Vec<u8>, Vec<&Vec<u8>>> = BTreeMap::new();
        for frame in frames {
            let key = if kind == message::SECTOR {
                frame.get(49..65).unwrap_or_default().to_vec()
            } else {
                vec![]
            };
            sectors.entry(key).or_default().push(frame);
        }
        for (next, destinations) in groups {
            let peer = peers.get_mut(&next).unwrap();
            for (group, recipients) in destinations.chunks(MULTICAST_RECIPIENTS).enumerate() {
                let mut header = vec![kind, recipients.len() as u8];
                for dest in recipients {
                    header.extend(dest.parse::<iroh::PublicKey>().unwrap().as_bytes());
                }
                for (sector, parts) in &sectors {
                    let prefix = format!("shared:multi:{}:{sector:?}:{group}:", self.id);
                    let batch: Vec<_> = parts
                        .iter()
                        .map(|data| {
                            let mut payload = header.clone();
                            payload.extend_from_slice(data);
                            let key = if kind == message::SECTOR {
                                format!("{prefix}{:?}", data.get(65..83))
                            } else {
                                format!("multi:{kind}:{}:{group}", self.id)
                            };
                            (key, payload)
                        })
                        .collect();
                    if kind == message::SECTOR {
                        peer.shared_batch(&prefix, batch);
                    } else {
                        for (key, data) in batch {
                            peer.state(key, data);
                        }
                    }
                }
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
            .is_some_and(|t| t.elapsed().as_secs_f32() < POSITION_FRESH_SECONDS)
        {
            if let (Some(a), Some(b)) = (self.positions.get(&self.id), self.positions.get(id)) {
                return ssk_sim::in_interest(a.pos, b.pos, self.radius);
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
                if kind == message::STATE || kind == message::SECTOR || kind == message::LEASE {
                    let key = if kind == message::SECTOR {
                        format!(
                            "shared:{}:{dest}:{:?}:{:?}",
                            self.id,
                            data.get(49..65),
                            data.get(65..83)
                        )
                    } else {
                        format!("{kind}:{dest}")
                    };
                    peer.state(key, frame);
                } else if kind == message::RECEIPT {
                    peer.receipt(frame);
                } else {
                    peer.queue(seq, frame, seq);
                }
            }
        }
    }
    pub fn retry(&mut self, peers: &mut HashMap<String, crate::reliable::Peer>, seq: u64) {
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
            if seq.saturating_sub(p.born) > EVENT_TTL_PACKETS {
                if let Some(key) = key {
                    self.routes.inflight.remove(&key);
                }
                self.routes.expired += 1;
                continue;
            }
            if p.last == 0 || seq - p.last >= EVENT_RETRY_PACKETS {
                let copies = if seq.saturating_sub(p.born) >= EVENT_BACKUP_PACKETS {
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
            .take(CONNECT_CONCURRENCY.saturating_sub(self.connecting.len()))
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
                let delay = (1u64 << failures.min(CONNECT_MAX_BACKOFF_SHIFT))
                    + stable_rank(&self.id, &id, "retry") % CONNECT_JITTER_SECONDS;
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
        self.last_change.elapsed() > Duration::from_secs(TOPOLOGY_SETTLE_SECONDS)
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
        if frame_kind == frame::MULTICAST {
            if frame.len() < 2
                || ![message::STATE, message::SECTOR, message::LEASE].contains(&frame[0])
                || !(1..=MULTICAST_RECIPIENTS as u8).contains(&frame[1])
            {
                return;
            }
            let header = 2 + frame[1] as usize * 32;
            let Some(signed) = frame.get(header..) else {
                return;
            };
            if !auth
                .verify(frame[0], signed)
                .is_some_and(|(origin, _)| origin == remote)
            {
                return;
            }
            // Multicast is source -> one neighbour -> final recipients only.
            for key in frame[2..header].chunks_exact(32) {
                let Ok(dest) = iroh::PublicKey::from_bytes(key.try_into().unwrap()) else {
                    continue;
                };
                if let Some(envelope) = Self::envelope(&dest.to_string(), frame[0], 0, signed) {
                    self.receive(remote, frame::ROUTED, &envelope, peers, auth, worker, seq);
                }
            }
            return;
        }
        if frame_kind == frame::CONTACT {
            if let Some((origin, data)) = auth.verify(message::CONTACT, frame) {
                self.learn(origin, &data, frame);
            }
            return;
        }
        if frame_kind == frame::METRICS {
            if let Some((origin, data)) = auth.verify(message::METRICS, frame) {
                if origin == remote {
                    self.routes.metrics(&origin, &data);
                }
            }
            return;
        }
        if frame_kind != frame::ROUTED || frame.len() < 34 {
            return;
        }
        let Ok(dest) = iroh::PublicKey::from_bytes(frame[..32].try_into().unwrap()) else {
            return;
        };
        let dest = dest.to_string();
        let kind = frame[32];
        let hops = frame[33];
        if ![
            message::STATE,
            message::CRITICAL,
            message::RECEIPT,
            message::SECTOR,
            message::LEASE,
        ]
        .contains(&kind)
            || hops > 1
        {
            return;
        }
        let Some((origin, data)) = auth.verify(kind, &frame[34..]) else {
            return;
        };
        if kind == message::STATE {
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
            if kind == message::STATE {
                let mut inbox = worker.snapshots.lock().unwrap();
                if inbox.len() < MAX_PEERS || inbox.contains_key(&origin) {
                    let newer = inbox
                        .get(&origin)
                        .and_then(|(old, _)| Snapshot::decode(old))
                        .zip(Snapshot::decode(&data))
                        .is_none_or(|(old, new)| {
                            new.sequence.wrapping_sub(old.sequence) as i32 > 0
                        });
                    if newer {
                        inbox.insert(origin, (data, Instant::now()));
                    }
                }
                return;
            }
            if kind == message::RECEIPT {
                if data.len() == 16 && self.routes.acknowledged.len() < ROUTE_EVENT_MEMORY {
                    let key = (origin, data.as_slice().try_into().unwrap());
                    if self.routes.inflight.contains(&key) {
                        self.routes.acknowledged.insert(key);
                    }
                }
                return;
            }
            let key = crate::routes::event_id(&frame[34..]).map(|id| (origin.clone(), id));
            let duplicate = kind == message::CRITICAL
                && key
                    .as_ref()
                    .is_some_and(|key| self.routes.seen.contains(key));
            let delivered = duplicate
                || worker
                    .incoming
                    .try_send(NetEvent::Packet {
                        peer: origin.clone(),
                        kind: if kind == message::LEASE { 1 } else { kind },
                        bytes: data,
                    })
                    .is_ok();
            if !delivered {
                worker.status.send_modify(|s| s.dropped += 1);
            }
            if kind == message::CRITICAL && delivered {
                if let Some(key) = key {
                    if self.routes.seen.insert(key.clone()) {
                        self.routes.order.push_back(key.clone());
                    }
                    while self.routes.order.len() > ROUTE_EVENT_MEMORY {
                        let old = self.routes.order.pop_front().unwrap();
                        self.routes.seen.remove(&old);
                        self.routes.acks.remove(&old);
                    }
                    let ack = self
                        .routes
                        .acks
                        .entry(key.clone())
                        .or_insert_with(|| auth.sign(message::RECEIPT, &key.1))
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
            if kind == message::STATE || kind == message::SECTOR || kind == message::LEASE {
                let key = if kind == message::SECTOR {
                    format!(
                        "shared:{origin}:{dest}:{:?}:{:?}",
                        data.get(..16),
                        data.get(16..34)
                    )
                } else {
                    format!("{kind}:{origin}:{dest}")
                };
                peer.state(key, forwarded);
            } else if kind == message::RECEIPT {
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
