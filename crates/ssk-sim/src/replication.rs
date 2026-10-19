use crate::*;
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
    pub identities: HashMap<String, u64>,
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
        while queue.len() >= HISTORY_CAPACITY
            || queue
                .front()
                .is_some_and(|(time, _)| now - time > HISTORY_SECONDS)
        {
            queue.pop_front();
        }
        queue.push_back((now, snapshot));
        true
    }
    pub fn sample(&self, peer: &str, now: f64) -> Option<(Snapshot, f64)> {
        let q = self.peers.get(peer)?;
        let &(last_time, last) = q.back()?;
        let target = now - INTERPOLATION_SECONDS;
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
        out.pos.local +=
            last.velocity * ((target - last_time).clamp(0., EXTRAPOLATION_SECONDS) as f32);
        Some((out, now - last_time))
    }
}

pub fn in_interest(local: WorldPos, remote: WorldPos, radius: f32) -> bool {
    (remote.relative_to(local.sector) - local.local).length()
        <= radius
            + WeaponKind::MachineGun.stats().speed * WeaponKind::MachineGun.stats().life
            + MAX_SHIP_SPEED * LAG_COMPENSATION_SECONDS
}
