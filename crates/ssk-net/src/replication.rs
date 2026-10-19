use crate::constants::*;
use ssk_sim::*;
pub trait SnapshotCodec: Sized {
    fn encode(self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Option<Self>;
}
impl SnapshotCodec for Snapshot {
    fn encode(self) -> Vec<u8> {
        let mut b = Vec::with_capacity(SNAPSHOT_BYTES);
        b.push(SNAPSHOT_VERSION);
        b.extend(self.sequence.to_le_bytes());
        b.extend(self.tick.to_le_bytes());
        b.extend(self.life.to_le_bytes());
        for s in self.pos.sector {
            b.extend(s.to_le_bytes());
        }
        for p in [self.pos.local.x, self.pos.local.y] {
            b.extend(
                ((p.clamp(0., SECTOR_SIZE - SNAPSHOT_POSITION_EPSILON) / SECTOR_SIZE
                    * u16::MAX as f32) as u16)
                    .to_le_bytes(),
            );
        }
        for v in [self.velocity.x, self.velocity.y] {
            b.extend(
                ((v * SNAPSHOT_VELOCITY_UNITS).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                    .to_le_bytes(),
            );
        }
        b.extend(
            ((self.angle.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
                * u16::MAX as f32) as u16)
                .to_le_bytes(),
        );
        b.extend((self.hp.max(0.) as u16).to_le_bytes());
        b.extend((self.energy.max(0.) as u16).to_le_bytes());
        b.extend([
            self.class as u8,
            self.weapon as u8,
            self.ability as u8 | ((self.invulnerable as u8) << 1),
        ]);
        b
    }
    fn decode(b: &[u8]) -> Option<Self> {
        if b.len() != SNAPSHOT_BYTES
            || b[0] != SNAPSHOT_VERSION
            || b[47] > 2
            || b[48] > 2
            || b[49] > 3
        {
            return None;
        }
        let u16at = |i| u16::from_le_bytes([b[i], b[i + 1]]);
        Some(Self {
            sequence: u32::from_le_bytes(b[1..5].try_into().ok()?),
            tick: u64::from_le_bytes(b[5..13].try_into().ok()?),
            life: u32::from_le_bytes(b[13..17].try_into().ok()?),
            pos: WorldPos {
                sector: [
                    i64::from_le_bytes(b[17..25].try_into().ok()?),
                    i64::from_le_bytes(b[25..33].try_into().ok()?),
                ],
                local: V2::new(
                    u16at(33) as f32 / u16::MAX as f32 * SECTOR_SIZE,
                    u16at(35) as f32 / u16::MAX as f32 * SECTOR_SIZE,
                ),
            },
            velocity: V2::new(
                i16::from_le_bytes(b[37..39].try_into().ok()?) as f32 / SNAPSHOT_VELOCITY_UNITS,
                i16::from_le_bytes(b[39..41].try_into().ok()?) as f32 / SNAPSHOT_VELOCITY_UNITS,
            ),
            angle: u16at(41) as f32 / u16::MAX as f32 * std::f32::consts::TAU,
            hp: u16at(43) as f32,
            energy: u16at(45) as f32,
            class: ShipClass::ALL[b[47] as usize],
            weapon: WeaponKind::from_index(b[48] as usize),
            ability: b[49] & 1 != 0,
            invulnerable: b[49] & 2 != 0,
        })
    }
}
pub fn player_id(peer: &str) -> Option<u64> {
    let key: iroh::PublicKey = peer.parse().ok()?;
    Some(u64::from_le_bytes(key.as_bytes()[..8].try_into().ok()?) | PLAYER_ID_BIT)
}
