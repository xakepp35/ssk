use ssk_sim::*;
use std::collections::{HashMap, VecDeque};
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub sequence: u32,
    pub tick: u64,
    pub life: u32,
    pub pos: WorldPos,
    pub velocity: V2,
    pub angle: f32,
    pub hp: f32,
    pub energy: f32,
    pub class: ShipClass,
    pub weapon: WeaponKind,
    pub ability: bool,
    pub invulnerable: bool,
}
impl Snapshot {
    pub fn capture(world: &World, sequence: u32, life: u32) -> Self {
        let ship = &world.player;
        Self {
            sequence,
            tick: world.tick,
            life,
            pos: WorldPos {
                sector: world.origin,
                local: ship.pos,
            },
            velocity: ship.velocity,
            angle: ship.angle,
            hp: ship.hp,
            energy: ship.energy,
            class: ship.class,
            weapon: ship.weapon,
            ability: ship.ability_active,
            invulnerable: ship.invulnerable > 0.,
        }
    }
    pub fn encode(self) -> Vec<u8> {
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
    pub fn decode(b: &[u8]) -> Option<Self> {
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
    pub fn ship(self, id: u64, origin: [i64; 2]) -> Ship {
        let mut ship = Ship::new(id, self.class, self.pos.relative_to(origin));
        ship.velocity = self.velocity;
        ship.angle = self.angle;
        ship.hp = self.hp;
        ship.energy = self.energy;
        ship.weapon = self.weapon;
        ship.ability_active = self.ability;
        ship.invulnerable = if self.invulnerable { 1. } else { 0. };
        ship
    }
}
#[derive(Default)]
pub struct Replicas {
    pub peers: HashMap<String, VecDeque<(f64, Snapshot)>>,
}
impl Replicas {
    pub fn push(&mut self, peer: String, snapshot: Snapshot, now: f64) -> bool {
        let queue = self.peers.entry(peer).or_default();
        if let Some((_, old)) = queue.back() {
            if snapshot.sequence.wrapping_sub(old.sequence) as i32 <= 0 {
                return false;
            }
        }
        if queue
            .back()
            .is_some_and(|(_, old)| old.life != snapshot.life)
        {
            queue.clear();
        }
        while queue.len() >= 128 || queue.front().is_some_and(|(time, _)| now - time > 2.) {
            queue.pop_front();
        }
        queue.push_back((now, snapshot));
        true
    }
    pub fn sample(&self, peer: &str, now: f64) -> Option<(Snapshot, f64)> {
        let q = self.peers.get(peer)?;
        let &(last_time, last) = q.back()?;
        let target = now - 0.050;
        for i in 1..q.len() {
            let (ta, a) = q[i - 1];
            let (tb, b) = q[i];
            if ta <= target && target <= tb {
                let t = ((target - ta) / (tb - ta).max(0.00001)) as f32;
                let mut out = b;
                out.pos.local = a.pos.relative_to(b.pos.sector).lerp(b.pos.local, t);
                let angle = (b.angle - a.angle + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                out.angle = a.angle + angle * t;
                return Some((out, now - last_time));
            }
        }
        let mut out = last;
        if target < q.front()?.0 {
            return Some((q.front()?.1, now - last_time));
        }
        out.pos.local += last.velocity * ((target - last_time).clamp(0., 0.1) as f32);
        Some((out, now - last_time))
    }
}
pub fn player_id(peer: &str) -> Option<u64> {
    let key: iroh::PublicKey = peer.parse().ok()?;
    Some(u64::from_le_bytes(key.as_bytes()[..8].try_into().ok()?) | 1 << 63)
}
