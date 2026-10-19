use ssk_sim::*;
pub trait SnapshotCodec: Sized {
    fn encode(self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Option<Self>;
}
impl SnapshotCodec for Snapshot {
    fn encode(self) -> Vec<u8> {
        let mut b = Vec::with_capacity(50);
        b.push(1);
        b.extend(self.sequence.to_le_bytes());
        b.extend(self.tick.to_le_bytes());
        b.extend(self.life.to_le_bytes());
        for s in self.pos.sector {
            b.extend(s.to_le_bytes());
        }
        for p in [self.pos.local.x, self.pos.local.y] {
            b.extend(((p.clamp(0., 4095.999) / 4096. * 65535.) as u16).to_le_bytes());
        }
        for v in [self.velocity.x, self.velocity.y] {
            b.extend(((v * 16.).clamp(-32768., 32767.) as i16).to_le_bytes());
        }
        b.extend(
            ((self.angle.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 65535.)
                as u16)
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
        if b.len() != 50 || b[0] != 1 || b[47] > 2 || b[48] > 2 || b[49] > 3 {
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
                    u16at(33) as f32 / 65535. * 4096.,
                    u16at(35) as f32 / 65535. * 4096.,
                ),
            },
            velocity: V2::new(
                i16::from_le_bytes(b[37..39].try_into().ok()?) as f32 / 16.,
                i16::from_le_bytes(b[39..41].try_into().ok()?) as f32 / 16.,
            ),
            angle: u16at(41) as f32 / 65535. * std::f32::consts::TAU,
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
    Some(u64::from_le_bytes(key.as_bytes()[..8].try_into().ok()?) | 1 << 63)
}
