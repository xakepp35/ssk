use crate::{CombatEvent, Replicas, Snapshot};
use ssk_sim::*;
use std::collections::{HashMap, VecDeque};
#[derive(Default)]
pub struct CombatGuard {
    shots: HashMap<String, VecDeque<(u32, u64, u64, WeaponKind)>>,
    budgets: HashMap<String, (u32, u64, f32)>,
    pub rejected: u64,
}
impl CombatGuard {
    pub fn snapshot(&mut self, old: Option<Snapshot>, s: Snapshot) -> bool {
        let speed = s.class.stats().1 * if s.class == ShipClass::Spark { 1.6 } else { 1. };
        let mut valid = s.hp <= s.class.stats().0 * 2.
            && s.energy <= 2000.
            && s.velocity.length() <= speed + 3.;
        if let Some(old) = old {
            if old.life == s.life {
                let ticks = s.tick.saturating_sub(old.tick);
                valid &= s.class == old.class && s.tick >= old.tick && ticks < 240 * 10;
                let distance = (s.pos.relative_to(old.pos.sector) - old.pos.local).length();
                valid &= distance <= speed * ticks as f32 / 240. + 12.;
            } else {
                valid &= s.life.wrapping_sub(old.life) == 1;
            }
        }
        if !valid {
            self.rejected += 1;
        }
        valid
    }
    pub fn shot(&mut self, peer: &str, event: &CombatEvent, replicas: &Replicas) -> bool {
        let CombatEvent::Shot {
            shot,
            sector,
            life,
            tick,
        } = event
        else {
            return false;
        };
        let Some(history) = replicas.peers.get(peer) else {
            return false;
        };
        let Some((_, latest)) = history.back() else {
            return false;
        };
        let Some((_, source)) = history.iter().min_by_key(|(_, s)| s.tick.abs_diff(*tick)) else {
            return false;
        };
        let mut valid = latest.life == *life
            && source.life == *life
            && source.hp > 0.
            && source.tick.abs_diff(*tick) <= 48
            && shot.pos.finite()
            && shot.pos.x >= 0.
            && shot.pos.y >= 0.
            && shot.pos.x < 4096.
            && shot.pos.y < 4096.
            && shot.angle.is_finite()
            && shot.damage_scale.is_finite();
        valid &= shot.damage_scale == 1.
            || (shot.damage_scale == 4. && source.class == ShipClass::Thunder && source.ability);
        valid &= !shot.blocked || (source.class == ShipClass::Cadet && source.ability);
        let muzzle = WorldPos {
            sector: *sector,
            local: shot.pos,
        }
        .relative_to(source.pos.sector);
        valid &= (muzzle - source.pos.local).length() <= source.class.stats().3 + 8. + 832. * 0.2;
        let shots = self.shots.entry(peer.into()).or_default();
        let interval = (shot.weapon.stats().interval * 240. - 1.).max(1.) as u64;
        valid &= !shots.iter().any(|(l, id, t, w)| {
            *l == *life && (*id == shot.id || (*w == shot.weapon && t.abs_diff(*tick) < interval))
        });
        let budget = self
            .budgets
            .entry(peer.into())
            .or_insert((*life, *tick, 2000.));
        if budget.0 != *life {
            *budget = (*life, *tick, 2000.);
        }
        let replenished = (budget.2 + tick.saturating_sub(budget.1) as f32 / 240. * 80.).min(2000.);
        let cost = shot.weapon.stats().energy * shot.damage_scale;
        valid &= replenished + 0.1 >= cost;
        if valid {
            budget.1 = budget.1.max(*tick);
            budget.2 = replenished - cost;
            shots.push_back((*life, shot.id, *tick, shot.weapon));
            while shots.len() > 128 {
                shots.pop_front();
            }
        } else {
            self.rejected += 1;
        }
        valid
    }
    pub fn forget(&mut self, peer: &str) {
        self.shots.remove(peer);
        self.budgets.remove(peer);
    }
}
