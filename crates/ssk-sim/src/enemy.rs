use crate::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Npc {
    pub ship: Ship,
    pub input: InputFrame,
    pub think: f32,
    pub age: f32,
    pub telegraph: f32,
    pub last_hit: u64,
}
impl World {
    pub(crate) fn enemies_step(&mut self) {
        let time = self.tick as f32 * DT;
        let target =
            (NPC_INITIAL_COUNT + (time / NPC_COUNT_INTERVAL) as usize).min(NPC_DIRECTOR_LIMIT);
        if time >= self.next_spawn && self.npcs.len() < target {
            let direction = V2::from_angle(self.rng.range(0., std::f32::consts::TAU));
            let pos = self.player.pos + direction * self.spawn_radius;
            let roll = self.rng.unit();
            let class = if time >= NPC_HEAVY_AFTER && roll > NPC_HEAVY_THRESHOLD {
                ShipClass::Thunder
            } else if roll > NPC_MEDIUM_THRESHOLD {
                ShipClass::Cadet
            } else {
                ShipClass::Spark
            };
            let mut ship = Ship::new(self.next_id, class, pos);
            self.next_id += 1;
            ship.hp *=
                NPC_HEALTH_SCALE * (1. + time / NPC_HEALTH_RAMP_SECONDS).min(NPC_HEALTH_RAMP_MAX);
            self.npcs.push(Npc {
                ship,
                input: InputFrame::default(),
                think: NPC_REACTION_SECONDS,
                age: 0.,
                telegraph: NPC_TELEGRAPH_SECONDS,
                last_hit: 0,
            });
            self.next_spawn = time + NPC_SPAWN_SECONDS;
        }
        let mut shots = Vec::new();
        let mut order: Vec<_> = self
            .npcs
            .iter()
            .map(|n| ((n.ship.pos - self.player.pos).length_squared(), n.ship.id))
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let shooters: Vec<_> = order
            .iter()
            .take(NPC_MAX_SHOOTERS)
            .map(|(_, id)| *id)
            .collect();
        let mut nearby = Vec::new();
        for npc in &mut self.npcs {
            let target = self
                .targets
                .iter()
                .filter(|t| t.hp > 0.)
                .min_by(|a, b| {
                    (a.pos - npc.ship.pos)
                        .length_squared()
                        .total_cmp(&(b.pos - npc.ship.pos).length_squared())
                })
                .unwrap_or(&self.player);
            npc.age += DT;
            npc.telegraph = (npc.telegraph - DT).max(0.);
            if npc.telegraph > 0. {
                continue;
            }
            npc.think -= DT;
            if npc.think <= 0. {
                let delta = target.pos - npc.ship.pos;
                let distance = delta.length();
                let normal = delta.normalized();
                let (range, weapon, strafe) = NPC_TACTICS[npc.ship.class.index()];
                let retreat = matches!(npc.ship.class, ShipClass::Cadet)
                    && npc.ship.hp < npc.ship.class.stats().0 * NPC_RETREAT_HEALTH
                    && distance < NPC_RETREAT_RANGE;
                let phase = (npc.age + npc.ship.id as f32 * NPC_PHASE_ID_SCALE) % NPC_PHASE_SECONDS;
                let ability = match npc.ship.class {
                    ShipClass::Spark => distance > NPC_BOOST_RANGE,
                    ShipClass::Cadet => retreat,
                    ShipClass::Thunder => distance < NPC_QUAD_RANGE && phase < NPC_QUAD_SECONDS,
                };
                let lead = target.velocity
                    * (distance / WeaponKind::from_index(weapon).stats().speed * NPC_AIM_LEAD);
                npc.input = InputFrame {
                    movement: normal
                        * if retreat {
                            -1.
                        } else {
                            ((distance - range) / NPC_MOVEMENT_RANGE).clamp(-1., 1.)
                        }
                        + normal.perp() * strafe * (if npc.ship.id % 2 == 0 { 1. } else { -1. }),
                    aim: (delta + lead).angle() + self.rng.range(-NPC_AIM_ERROR, NPC_AIM_ERROR),
                    fire: distance < range + NPC_FIRE_MARGIN
                        && phase < NPC_FIRE_SECONDS
                        && !retreat,
                    weapon,
                    ability,
                };
                self.grid
                    .query(npc.ship.pos, npc.ship.pos, NPC_AVOID_QUERY, &mut nearby);
                for &index in &nearby {
                    let rock = &self.asteroids[index];
                    let away = npc.ship.pos - rock.pos;
                    let gap = away.length() - rock.radius;
                    if gap < NPC_AVOID_MARGIN {
                        npc.input.movement +=
                            away.normalized() * ((NPC_AVOID_MARGIN - gap) / NPC_AVOID_FORCE);
                    }
                }
                npc.think = NPC_REACTION_SECONDS + self.rng.range(0., NPC_REACTION_JITTER);
            }
            npc.ship.move_step(npc.input);
            self.grid.query(
                npc.ship.pos,
                npc.ship.pos,
                npc.ship.class.stats().3,
                &mut nearby,
            );
            for &index in &nearby {
                let rock = &self.asteroids[index];
                let away = npc.ship.pos - rock.pos;
                let min = rock.radius + npc.ship.class.stats().3;
                if away.length() < min {
                    npc.ship.pos = rock.pos + away.normalized() * (min + 1.);
                    npc.ship.velocity = away.normalized() * NPC_BUMP_SPEED;
                }
            }
            let mut combat_input = npc.input;
            combat_input.fire &= shooters.contains(&npc.ship.id);
            if let Some(mut shot) = npc
                .ship
                .combat_step(combat_input, &mut self.rng, self.next_id)
            {
                self.next_id += 1;
                shot.damage_scale *= NPC_DAMAGE_SCALE;
                shots.push(shot);
            }
        }
        for shot in shots {
            self.spawn_shot(shot);
        }
    }
    pub(crate) fn destroy_enemies(&mut self) {
        let dead: Vec<_> = self
            .npcs
            .iter()
            .filter(|n| n.ship.hp <= 0.)
            .map(|n| {
                (
                    n.ship.pos,
                    n.ship.class,
                    n.last_hit == self.player.id,
                    n.ship.id,
                    n.last_hit,
                )
            })
            .collect();
        self.npcs.retain(|n| n.ship.hp > 0.);
        for (pos, class, owned, id, player) in dead {
            let amount = NPC_SCORE[class.index()];
            if self.shared_authority && player & (1 << 63) != 0 {
                self.emit(WorldEvent::Reward { id, player, amount });
            }
            if owned {
                self.award(amount);
            }
            self.drop_loot(pos, true);
            self.emit(WorldEvent::Destroyed {
                pos,
                size: NPC_DESTRUCTION_SIZE,
            });
        }
        self.npcs
            .retain(|n| (n.ship.pos - self.player.pos).length() < NPC_DESPAWN_RANGE);
    }
}
