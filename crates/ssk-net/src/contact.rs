use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Contact {
    pub addr: iroh::EndpointAddr,
    pub discovery: String,
    pub expires: u64,
    pub sector: [i64; 2],
    #[serde(default)]
    pub playing: bool,
}
pub(crate) fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(crate) fn discovery_id(key: &iroh::PublicKey) -> Option<libp2p::PeerId> {
    let key = libp2p::identity::ed25519::PublicKey::try_from_bytes(key.as_bytes()).ok()?;
    Some(libp2p::identity::PublicKey::from(key).to_peer_id())
}
impl Contact {
    pub fn decode(origin: &str, bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 880 {
            return None;
        }
        let c: Self = serde_json::from_slice(bytes).ok()?;
        let now = unix_time();
        if c.addr.id.to_string() != origin
            || c.addr.addrs.len() > 16
            || c.expires <= now
            || c.expires > now + 90
            || discovery_id(&c.addr.id)?.to_string() != c.discovery
        {
            return None;
        }
        Some(c)
    }
}
