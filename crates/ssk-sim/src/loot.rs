use crate::*;
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum PickupKind {
    Energy,
    Health,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pickup {
    pub id: u64,
    pub pos: V2,
    pub kind: PickupKind,
    pub amount: f32,
    pub life: f32,
}
impl World {
    pub fn award(&mut self, base: u64) {
        let time = self.tick as f32 * DT;
        self.combo = if self.last_kill.is_some_and(|last| time - last <= 4.) {
            (self.combo + 0.25).min(2.)
        } else {
            1.
        };
        self.last_kill = Some(time);
        self.score += (base as f32 * self.combo) as u64;
    }
    pub(crate) fn drop_loot(&mut self, pos: V2, npc: bool) {
        let roll = self.rng.unit();
        let drop = if npc {
            if roll < 0.7 {
                Some((PickupKind::Energy, 300.))
            } else if roll < 0.85 {
                Some((PickupKind::Health, 250.))
            } else {
                None
            }
        } else if roll < 0.1 {
            Some((PickupKind::Energy, 150.))
        } else if roll < 0.13 {
            Some((PickupKind::Health, 100.))
        } else {
            None
        };
        if let Some((kind, amount)) = drop {
            if self.pickups.len() < 256 {
                self.pickups.push(Pickup {
                    id: self.next_id,
                    pos,
                    kind,
                    amount,
                    life: 15.,
                });
                self.next_id += 1;
            }
        }
    }
    pub(crate) fn resources_step(&mut self) {
        let base = self.player.class.stats().0;
        if self.player.hp > base {
            self.player.hp = (self.player.hp - base * 0.025 * DT).max(base);
        }
        let mut collected = Vec::new();
        for pickup in &mut self.pickups {
            pickup.life -= DT;
            let delta = self.player.pos - pickup.pos;
            let distance = delta.length();
            if distance < 180. {
                pickup.pos += delta.normalized() * (100. + (180. - distance) * 2.) * DT;
            }
            if distance < self.player.class.stats().3 + 12. && pickup.life > 0. {
                match pickup.kind {
                    PickupKind::Energy => {
                        self.player.energy = (self.player.energy + pickup.amount).min(2000.)
                    }
                    PickupKind::Health => {
                        self.player.hp = (self.player.hp + pickup.amount).min(2. * base)
                    }
                }
                collected.push(WorldEvent::Pickup {
                    pos: pickup.pos,
                    kind: pickup.kind,
                });
                pickup.life = 0.;
            }
        }
        self.pickups.retain(|p| p.life > 0.);
        for event in collected {
            self.emit(event);
        }
        if self
            .last_kill
            .is_some_and(|last| self.tick as f32 * DT - last > 4.)
        {
            self.combo = 1.;
        }
    }
}
