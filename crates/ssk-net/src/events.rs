use serde::{Deserialize, Serialize};
use ssk_sim::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CombatEvent {
    Shared(crate::SharedEvent),
    Shot {
        shot: Shot,
        sector: [i64; 2],
        life: u32,
        tick: u64,
    },
    Death {
        life: u32,
        killer: u64,
        shot: u64,
    },
}
impl CombatEvent {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 880 {
            return None;
        }
        serde_json::from_slice(bytes).ok()
    }
}
