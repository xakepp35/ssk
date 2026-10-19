use serde::{Deserialize, Serialize};
use ssk_sim::*;
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
#[derive(Clone)]
pub struct SharedChunk {
    pub sector: [i64; 2],
    pub epoch: u64,
    pub revision: u64,
    pub part: u16,
    pub total: u16,
    pub data: Vec<u8>,
}
impl SharedChunk {
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        for v in self.sector {
            b.extend(v.to_le_bytes());
        }
        b.extend(self.epoch.to_le_bytes());
        b.extend(self.revision.to_le_bytes());
        b.extend(self.part.to_le_bytes());
        b.extend(self.total.to_le_bytes());
        b.extend(&self.data);
        b
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() < 36 || b.len() > 420 {
            return None;
        }
        let out = Self {
            sector: [
                i64::from_le_bytes(b[..8].try_into().ok()?),
                i64::from_le_bytes(b[8..16].try_into().ok()?),
            ],
            epoch: u64::from_le_bytes(b[16..24].try_into().ok()?),
            revision: u64::from_le_bytes(b[24..32].try_into().ok()?),
            part: u16::from_le_bytes(b[32..34].try_into().ok()?),
            total: u16::from_le_bytes(b[34..36].try_into().ok()?),
            data: b[36..].to_vec(),
        };
        (out.total > 0 && out.total <= 128 && out.part < out.total).then_some(out)
    }
    pub fn snapshot(sector: [i64; 2], epoch: u64, state: &SectorState) -> Vec<Vec<u8>> {
        let Ok(bytes) = postcard::to_allocvec(state) else {
            return vec![];
        };
        if bytes.len() > 384 * 128 {
            return vec![];
        }
        let total = bytes.len().div_ceil(384) as u16;
        bytes
            .chunks(384)
            .enumerate()
            .map(|(i, b)| {
                Self {
                    sector,
                    epoch,
                    revision: state.tick,
                    part: i as u16,
                    total,
                    data: b.to_vec(),
                }
                .encode()
            })
            .collect()
    }
    pub fn restore(parts: &[Option<Vec<u8>>]) -> Option<SectorState> {
        let mut b = Vec::new();
        for p in parts {
            b.extend(p.as_ref()?);
        }
        let state: SectorState = postcard::from_bytes(&b).ok()?;
        state.valid().then_some(state)
    }
}
