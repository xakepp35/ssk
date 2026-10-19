use crate::*;
use std::collections::{HashMap, HashSet, VecDeque};
/// Sign on the async worker, never on the 240 Hz simulation thread.
pub(crate) struct Auth {
    key: iroh::SecretKey,
    domain: Vec<u8>,
    session: [u8; 8],
    serial: u64,
    seen: HashSet<(String, [u8; 8], u64)>,
    order: VecDeque<(String, [u8; 8], u64)>,
    latest: HashMap<(String, [u8; 8]), u64>,
}
impl Auth {
    pub fn new(key: iroh::SecretKey, config: &Config) -> Self {
        Self {
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
            seen: HashSet::new(),
            order: VecDeque::new(),
            latest: HashMap::new(),
        }
    }
    pub fn sign(&mut self, kind: u8, payload: &[u8]) -> Vec<u8> {
        self.serial += 1;
        let mut body = self.key.public().as_bytes().to_vec();
        body.extend(self.session);
        body.extend(self.serial.to_le_bytes());
        body.push(kind);
        body.extend(payload);
        let mut message = self.domain.clone();
        message.extend(&body);
        body.extend(self.key.sign(&message).to_bytes());
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
        let id = (peer.clone(), session, serial);
        let latest = self
            .latest
            .get(&(peer.clone(), session))
            .copied()
            .unwrap_or(0);
        if serial.saturating_add(4096) < latest {
            return None;
        }
        let end = payload.len() - 64;
        let signature = iroh::Signature::from_bytes(payload[end..].try_into().ok()?);
        let mut message = self.domain.clone();
        message.extend(&payload[..end]);
        key.verify(&message, &signature).ok()?;
        self.latest
            .insert((peer.clone(), session), latest.max(serial));
        if self.latest.len() > 512 {
            self.latest.retain(|(p, _), _| p == &peer);
        }
        self.seen.insert(id.clone());
        self.order.push_back(id);
        if self.order.len() > 16384 {
            self.seen.remove(&self.order.pop_front()?);
        }
        Some((peer, payload[49..end].to_vec()))
    }
}
