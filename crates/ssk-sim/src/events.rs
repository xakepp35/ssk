use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CombatEvent {
    Shared(SharedEvent),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharedEvent {
    Lease {
        sector: [i64; 2],
        epoch: u64,
        until_ms: u64,
    },
    Release {
        sector: [i64; 2],
        epoch: u64,
    },
    Shot {
        sector: [i64; 2],
        epoch: u64,
        shot: Shot,
    },
    Claim {
        sector: [i64; 2],
        epoch: u64,
        pickup: u64,
        life: u32,
    },
    Grant {
        sector: [i64; 2],
        epoch: u64,
        pickup: u64,
        player: u64,
        life: u32,
        kind: PickupKind,
        amount: f32,
    },
    Reward {
        sector: [i64; 2],
        epoch: u64,
        id: u64,
        player: u64,
        amount: u64,
    },
}
