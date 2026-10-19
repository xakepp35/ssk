use crate::*;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};
pub(crate) struct Pending {
    pub dest: String,
    pub data: Vec<u8>,
    pub born: u64,
    pub last: u64,
}
#[derive(Default)]
pub(crate) struct Routes {
    pub pending: VecDeque<Pending>,
    pub inflight: HashSet<(String, [u8; 16])>,
    pub acknowledged: HashSet<(String, [u8; 16])>,
    pub seen: HashSet<(String, [u8; 16])>,
    pub order: VecDeque<(String, [u8; 16])>,
    pub acks: HashMap<(String, [u8; 16]), Vec<u8>>,
    pub legs: HashMap<(String, String), (LinkStats, Instant)>,
    pub chosen: HashMap<String, (String, f32, Instant)>,
    pub changes: u64,
    pub expired: u64,
}
impl Routes {
    pub fn metrics(&mut self, origin: &str, b: &[u8]) {
        if b.len() % 40 != 0 || b.len() > 800 {
            return;
        }
        for row in b.chunks_exact(40) {
            let Ok(id) = iroh::PublicKey::from_bytes(row[..32].try_into().unwrap()) else {
                continue;
            };
            let rtt = u16::from_le_bytes(row[32..34].try_into().unwrap()) as f32 / 10.;
            let jitter = u16::from_le_bytes(row[34..36].try_into().unwrap()) as f32 / 10.;
            self.legs.insert(
                (origin.into(), id.to_string()),
                (
                    LinkStats {
                        rtt_ms: Some(rtt),
                        jitter_ms: jitter,
                        loss: row[36] as f32 / 255.,
                        queued: row[37] as usize,
                        direct: row[38] == 1,
                        age_ms: row[39] as f32 * 20.,
                        ..Default::default()
                    },
                    Instant::now(),
                ),
            );
        }
        self.legs.retain(|_, (_, t)| t.elapsed().as_secs() < 6);
    }
    pub fn advertise(peers: &HashMap<String, crate::reliable::Peer>, cursor: usize) -> Vec<u8> {
        let mut ids: Vec<_> = peers.keys().collect();
        ids.sort();
        let mut b = Vec::new();
        for i in 0..ids.len().min(20) {
            let id = ids[(cursor + i) % ids.len()];
            let p = &peers[id];
            let s = &p.metrics.stats;
            let Ok(key) = id.parse::<iroh::PublicKey>() else {
                continue;
            };
            b.extend(key.as_bytes());
            b.extend(((s.rtt_ms.unwrap_or(6553.) * 10.).clamp(0., 65535.) as u16).to_le_bytes());
            b.extend(((s.jitter_ms * 10.).clamp(0., 65535.) as u16).to_le_bytes());
            b.extend([
                (s.loss * 255.) as u8,
                s.queued.min(255) as u8,
                p.direct() as u8,
                (s.age_ms / 20.).min(255.) as u8,
            ]);
        }
        b
    }
    pub fn select(&mut self, dest: &str, candidates: Vec<(String, f32)>) -> Vec<String> {
        let mut candidates = candidates;
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((best, cost)) = candidates.first() {
            let selected = self.chosen.get(dest);
            let previous = selected.and_then(|(id, _, _)| candidates.iter().find(|(p, _)| p == id));
            let keep = selected
                .zip(previous)
                .is_some_and(|((_, _, time), (_, old))| {
                    time.elapsed().as_secs_f32() < 1. || old - cost <= 5f32.max(*old * 0.1)
                });
            if !keep {
                if selected.is_some_and(|(id, _, _)| id != best) {
                    self.changes += 1;
                }
                self.chosen
                    .insert(dest.into(), (best.clone(), *cost, Instant::now()));
            }
            if let Some((id, _, _)) = self.chosen.get(dest) {
                if let Some(i) = candidates.iter().position(|(p, _)| p == id) {
                    candidates.swap(0, i);
                }
            }
        }
        candidates.into_iter().take(3).map(|(id, _)| id).collect()
    }
}
pub(crate) fn event_id(data: &[u8]) -> Option<[u8; 16]> {
    data.get(32..48)?.try_into().ok()
}
