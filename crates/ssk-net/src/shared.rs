use crate::constants::*;
use ssk_sim::*;
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
        if b.len() < SHARED_HEADER_BYTES || b.len() > SHARED_FRAME_BYTES {
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
        (out.total > 0 && out.total as usize <= SHARED_MAX_PARTS && out.part < out.total)
            .then_some(out)
    }
    pub fn snapshot(sector: [i64; 2], epoch: u64, state: &SectorState) -> Vec<Vec<u8>> {
        let Ok(bytes) = postcard::to_allocvec(state) else {
            return vec![];
        };
        if bytes.len() > SHARED_PART_BYTES * SHARED_MAX_PARTS {
            return vec![];
        }
        let total = bytes.len().div_ceil(SHARED_PART_BYTES) as u16;
        bytes
            .chunks(SHARED_PART_BYTES)
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

/// Multipart framing only. Game ownership and state validation live in ssk-sim.
#[derive(Default)]
pub struct SectorAssembler {
    pending: std::collections::HashMap<(u64, [i64; 2], u64, u64), (f64, Vec<Option<Vec<u8>>>)>,
}
impl SectorAssembler {
    pub fn push(
        &mut self,
        origin: u64,
        chunk: SharedChunk,
        now: f64,
    ) -> Option<([i64; 2], u64, SectorState)> {
        self.pending
            .retain(|_, (time, _)| now - *time < SHARED_ASSEMBLY_SECONDS);
        let key = (origin, chunk.sector, chunk.epoch, chunk.revision);
        if self.pending.len() >= SHARED_ASSEMBLIES && !self.pending.contains_key(&key) {
            return None;
        }
        let (_, parts) = self
            .pending
            .entry(key)
            .or_insert_with(|| (now, vec![None; chunk.total as usize]));
        if parts.len() != chunk.total as usize {
            return None;
        }
        parts[chunk.part as usize] = Some(chunk.data);
        let state = SharedChunk::restore(parts)?;
        self.pending.retain(|(o, s, e, r), _| {
            *o != origin
                || *s != chunk.sector
                || *e > chunk.epoch
                || (*e == chunk.epoch && *r > chunk.revision)
        });
        Some((chunk.sector, chunk.epoch, state))
    }
}
