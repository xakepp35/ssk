use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeaponKind { #[default] MachineGun, Shotgun, Plasma }
impl WeaponKind {
    pub fn from_index(i: usize) -> Self { match i % 3 { 1 => Self::Shotgun, 2 => Self::Plasma, _ => Self::MachineGun } }
    pub fn name(self) -> &'static str { match self { Self::MachineGun => "MACHINE GUN", Self::Shotgun => "SHOTGUN", Self::Plasma => "PLASMA" } }
    pub fn stats(self) -> WeaponStats {
        WeaponStats { damage:40., interval:0.1, energy:18., speed:1300., radius:2., life:1.2, pellets:1, spread:2f32.to_radians() }
    }
}
pub struct WeaponStats { pub damage:f32, pub interval:f32, pub energy:f32, pub speed:f32, pub radius:f32, pub life:f32, pub pellets:usize, pub spread:f32 }

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub id:u64, pub owner:u64, pub pos:V2, pub angle:f32,
    pub weapon:WeaponKind, pub seed:u64, pub damage_scale:f32,
}
#[derive(Clone, Debug)]
pub struct Bullet { pub shot:Shot, pub pos:V2, pub previous:V2, pub velocity:V2, pub life:f32, pub damage:f32, pub radius:f32 }
#[derive(Clone, Copy, Debug)]
pub enum WorldEvent {
    Shot(Shot), Dry, Impact { pos:V2, damage:f32 }, Destroyed { pos:V2, size:f32 },
}
#[derive(Clone, Debug)]
pub struct Target { pub pos:V2, pub radius:f32, pub hp:f32 }

impl Ship {
    pub fn combat_step(&mut self, input:InputFrame, rng:&mut Rng, shot_id:u64) -> Option<Shot> {
        for c in &mut self.cooldowns { *c=(*c-DT).max(0.); }
        self.quiet+=DT;
        self.weapon=WeaponKind::from_index(input.weapon);
        let index=self.weapon as usize;
        let stats=self.weapon.stats();
        if input.fire && self.cooldowns[index]<=0. && self.energy>=stats.energy {
            self.energy-=stats.energy; self.quiet=0.; self.cooldowns[index]=stats.interval;
            Some(Shot { id:shot_id, owner:self.id, pos:self.pos+V2::from_angle(self.angle)*(self.class.stats().3+5.), angle:self.angle, weapon:self.weapon, seed:rng.next_u64(), damage_scale:1. })
        } else {
            if self.quiet>0.6 && self.energy<1000. { self.energy=(self.energy+80.*DT).min(1000.); }
            None
        }
    }
}

/// Fraction of the first intersection, including starting inside the circle.
pub fn swept_circle(a:V2,b:V2,center:V2,radius:f32) -> Option<f32> {
    let m=a-center; let d=b-a; let c=m.dot(m)-radius*radius;
    if c<=0. { return Some(0.); }
    let aa=d.dot(d); let bb=m.dot(d); let disc=bb*bb-aa*c;
    if aa<1e-8 || bb>=0. || disc<0. { return None; }
    let t=(-bb-disc.sqrt())/aa;
    (t<=1.).then_some(t)
}

impl World {
    pub fn emit(&mut self,event:WorldEvent) { if self.events.len()<512 { self.events.push(event); } }
    pub fn spawn_shot(&mut self,shot:Shot) {
        let stats=shot.weapon.stats(); let mut rng=Rng(shot.seed);
        for _ in 0..stats.pellets {
            if self.bullets.len()>=4096 { break; }
            let direction=V2::from_angle(shot.angle+rng.range(-stats.spread,stats.spread));
            self.bullets.push(Bullet { shot, pos:shot.pos, previous:shot.pos, velocity:direction*stats.speed, life:stats.life, damage:stats.damage*shot.damage_scale, radius:stats.radius });
        }
        self.emit(WorldEvent::Shot(shot));
    }
    pub(crate) fn combat(&mut self,input:InputFrame) {
        let shot_id=self.next_id;
        if let Some(shot)=self.player.combat_step(input,&mut self.rng,shot_id) { self.next_id+=1; self.spawn_shot(shot); }
        let mut impacts=Vec::new();
        for bullet in &mut self.bullets {
            bullet.previous=bullet.pos; bullet.pos+=bullet.velocity*DT; bullet.life-=DT;
            let mut first=None;
            for (index,target) in self.targets.iter().enumerate() {
                if target.hp<=0. { continue; }
                if let Some(t)=swept_circle(bullet.previous,bullet.pos,target.pos,target.radius+bullet.radius) {
                    if first.is_none_or(|(best,_)| t<best) { first=Some((t,index)); }
                }
            }
            if let Some((t,index))=first {
                self.targets[index].hp-=bullet.damage; bullet.life=0.;
                impacts.push(WorldEvent::Impact { pos:bullet.previous.lerp(bullet.pos,t), damage:bullet.damage });
            }
        }
        self.bullets.retain(|b| b.life>0.);
        for target in &self.targets { if target.hp<=0. { impacts.push(WorldEvent::Destroyed { pos:target.pos, size:target.radius }); } }
        self.targets.retain(|t| t.hp>0.);
        for event in impacts { self.emit(event); }
    }
}
