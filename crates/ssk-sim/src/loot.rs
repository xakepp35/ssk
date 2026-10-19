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
        self.combo = if self
            .last_kill
            .is_some_and(|last| time - last <= COMBO_SECONDS)
        {
            (self.combo + COMBO_INCREMENT).min(COMBO_MAX)
        } else {
            1.
        };
        self.last_kill = Some(time);
        self.score += (base as f32 * self.combo) as u64;
    }
    pub(crate) fn drop_loot(&mut self, pos: V2, npc: bool) {
        let roll = self.rng.unit();
        let drop = if npc {
            if roll < NPC_LOOT_ENERGY_CHANCE {
                Some((PickupKind::Energy, NPC_ENERGY_PICKUP))
            } else if roll < NPC_LOOT_HEALTH_THRESHOLD {
                Some((PickupKind::Health, NPC_HEALTH_PICKUP))
            } else {
                None
            }
        } else if roll < ROCK_LOOT_ENERGY_CHANCE {
            Some((PickupKind::Energy, ROCK_ENERGY_PICKUP))
        } else if roll < ROCK_LOOT_HEALTH_THRESHOLD {
            Some((PickupKind::Health, ROCK_HEALTH_PICKUP))
        } else {
            None
        };
        if let Some((kind, amount)) = drop {
            if self.pickups.len() < MAX_PICKUPS {
                self.pickups.push(Pickup {
                    id: self.next_id,
                    pos,
                    kind,
                    amount,
                    life: PICKUP_LIFETIME,
                });
                self.next_id += 1;
            }
        }
    }
    pub(crate) fn resources_step(&mut self) {
        let base = self.player.class.stats().0;
        if self.player.hp > base {
            self.player.hp = (self.player.hp - base * HEALTH_DECAY * DT).max(base);
        }
        let mut collected = Vec::new();
        for pickup in &mut self.pickups {
            pickup.life -= DT;
            if self.shared_replica || self.shared_authority {
                continue;
            }
            let delta = self.player.pos - pickup.pos;
            let distance = delta.length();
            if distance < PICKUP_MAGNET_RADIUS {
                pickup.pos += delta.normalized()
                    * (PICKUP_MAGNET_SPEED
                        + (PICKUP_MAGNET_RADIUS - distance) * PICKUP_MAGNET_ACCELERATION)
                    * DT;
            }
            if distance < self.player.class.stats().3 + PICKUP_REACH && pickup.life > 0. {
                match pickup.kind {
                    PickupKind::Energy => {
                        self.player.energy = (self.player.energy + pickup.amount).min(ENERGY_MAX)
                    }
                    PickupKind::Health => {
                        self.player.hp = (self.player.hp + pickup.amount).min(HEALTH_OVERCAP * base)
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
            .is_some_and(|last| self.tick as f32 * DT - last > COMBO_SECONDS)
        {
            self.combo = 1.;
        }
    }
}
