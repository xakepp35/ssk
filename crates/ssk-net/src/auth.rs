use crate::*;
use std::collections::{HashMap, VecDeque};
/// Sign on the async worker, never on the 240 Hz simulation thread.
pub(crate) struct Auth {
    pub stats: CryptoStats,
    key: iroh::SecretKey,
    domain: Vec<u8>,
    session: [u8; 8],
    serial: u64,
    verified: HashMap<[u8; 32], (String, Vec<u8>)>,
    order: VecDeque<[u8; 32]>,
    latest: HashMap<(String, [u8; 8]), u64>,
}
impl Auth {
    pub fn new(key: iroh::SecretKey, config: &Config) -> Self {
        Self {
            stats: CryptoStats::default(),
            key,
            domain: format!(
                "SSK/{PROTOCOL_VERSION}/{RULES_ID}/{}/{}/",
                config.world, config.seed
            )
            .into_bytes(),
            session: iroh::SecretKey::generate().to_bytes()[..8]
                .try_into()
                .unwrap(),
            serial: 0,
            verified: HashMap::new(),
            order: VecDeque::new(),
            latest: HashMap::new(),
        }
    }
    pub fn sign(&mut self, kind: u8, payload: &[u8]) -> Vec<u8> {
        let started = std::time::Instant::now();
        self.serial += 1;
        let mut body = self.key.public().as_bytes().to_vec();
        body.extend(self.session);
        body.extend(self.serial.to_le_bytes());
        body.push(kind);
        body.extend(payload);
        let mut message = self.domain.clone();
        message.extend(&body);
        body.extend(self.key.sign(&message).to_bytes());
        let us = started.elapsed().as_micros() as u64;
        self.stats.signed += 1;
        self.stats.sign_us += us;
        self.stats.sign_max_us = self.stats.sign_max_us.max(us);
        body
    }
    pub fn verify(&mut self, kind: u8, payload: &[u8]) -> Option<(String, Vec<u8>)> {
        if payload.len() < 113 || payload.len() > 1000 || payload[48] != kind {
            return None;
        }
        let key = iroh::PublicKey::from_bytes(payload[..32].try_into().ok()?).ok()?;
        let session: [u8; 8] = payload[32..40].try_into().ok()?;
        let serial = u64::from_le_bytes(payload[40..48].try_into().ok()?);
        let peer = key.to_string();
        let latest = self
            .latest
            .get(&(peer.clone(), session))
            .copied()
            .unwrap_or(0);
        if serial.saturating_add(4096) < latest {
            return None;
        }
        let digest = *blake3::hash(payload).as_bytes();
        if let Some(value) = self.verified.get(&digest) {
            self.stats.cache_hits += 1;
            return Some(value.clone());
        }
        let end = payload.len() - 64;
        let signature = iroh::Signature::from_bytes(payload[end..].try_into().ok()?);
        let mut message = self.domain.clone();
        message.extend(&payload[..end]);
        let started = std::time::Instant::now();
        let valid = key.verify(&message, &signature).is_ok();
        let us = started.elapsed().as_micros() as u64;
        self.stats.verified += 1;
        self.stats.verify_us += us;
        self.stats.verify_max_us = self.stats.verify_max_us.max(us);
        if !valid {
            return None;
        }
        self.latest
            .insert((peer.clone(), session), latest.max(serial));
        if self.latest.len() > 512 {
            self.latest.retain(|(p, _), _| p == &peer);
        }
        let result = (peer, payload[49..end].to_vec());
        self.verified.insert(digest, result.clone());
        self.order.push_back(digest);
        if self.order.len() > 16384 {
            self.verified.remove(&self.order.pop_front()?);
        }
        Some(result)
    }
}
