use ssk_sim::CombatEvent;
pub trait CombatEventCodec: Sized {
    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Option<Self>;
}
impl CombatEventCodec for CombatEvent {
    fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).unwrap_or_default()
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 880 {
            return None;
        }
        postcard::from_bytes(bytes).ok()
    }
}
