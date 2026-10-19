use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Asteroid {
    pub id:u64, pub pos:V2, pub radius:f32, pub hp:f32,
    pub velocity:V2, pub angle:f32, pub spin:f32, pub small:bool,
}
#[derive(Default)]
pub struct SpatialGrid { cells:HashMap<(i32,i32),Vec<usize>> }
impl SpatialGrid {
    pub fn rebuild(&mut self,asteroids:&[Asteroid]) {
        for bucket in self.cells.values_mut() { bucket.clear(); }
        for (i,a) in asteroids.iter().enumerate() {
            for x in ((a.pos.x-a.radius)/128.).floor() as i32..=((a.pos.x+a.radius)/128.).floor() as i32 {
                for y in ((a.pos.y-a.radius)/128.).floor() as i32..=((a.pos.y+a.radius)/128.).floor() as i32 { self.cells.entry((x,y)).or_default().push(i); }
            }
        }
        self.cells.retain(|_,v|!v.is_empty());
    }
    pub fn query(&self,a:V2,b:V2,r:f32,result:&mut Vec<usize>) {
        result.clear();
        for x in ((a.x.min(b.x)-r)/128.).floor() as i32..=((a.x.max(b.x)+r)/128.).floor() as i32 {
            for y in ((a.y.min(b.y)-r)/128.).floor() as i32..=((a.y.max(b.y)+r)/128.).floor() as i32 {
                if let Some(bucket)=self.cells.get(&(x,y)) { for &i in bucket { if !result.contains(&i) { result.push(i); } } }
            }
        }
    }
}

impl World {
    pub(crate) fn field_step(&mut self) {
        self.rebase();
        for a in &mut self.asteroids { a.pos+=a.velocity*DT; a.angle+=a.spin*DT; }
        self.asteroids.retain(|a|(a.pos-self.player.pos).length()<SECTOR_SIZE*2.5);
        self.grid.rebuild(&self.asteroids);
        self.player.bump_cooldown=(self.player.bump_cooldown-DT).max(0.);
        let mut nearby=Vec::new();
        self.grid.query(self.player.pos,self.player.pos,self.player.class.stats().3,&mut nearby);
        for i in nearby {
            let a=&self.asteroids[i]; let delta=self.player.pos-a.pos; let distance=delta.length();
            let min=a.radius+self.player.class.stats().3;
            if distance<min {
                let normal=if distance>0.001 { delta/distance } else { V2::new(1.,0.) };
                self.player.pos=a.pos+normal*(min+0.1);
                let incoming=(self.player.velocity-a.velocity).dot(normal);
                if incoming<0. {
                    self.player.velocity-=normal*(incoming*1.6);
                    if self.player.bump_cooldown<=0. && incoming < -80. {
                        let damage=(-incoming*0.2).min(100.);
                        self.player.hp-=damage; self.player.bump_cooldown=0.35;
                        self.emit(WorldEvent::Impact { pos:self.player.pos, damage });
                    }
                }
            }
        }
    }
    fn rebase(&mut self) {
        let shift=[(self.player.pos.x/SECTOR_SIZE).floor() as i64,(self.player.pos.y/SECTOR_SIZE).floor() as i64];
        if shift!=[0,0] {
            self.origin[0]+=shift[0];self.origin[1]+=shift[1];
            let offset=V2::new(shift[0] as f32*SECTOR_SIZE,shift[1] as f32*SECTOR_SIZE);
            self.player.pos-=offset;self.player.previous-=offset;
            for a in &mut self.asteroids { a.pos-=offset; }
            for b in &mut self.bullets { b.pos-=offset;b.previous-=offset;b.shot.pos-=offset; }
            self.origin_shift+=offset;
        }
        self.loaded.retain(|s|(s[0]-self.origin[0]).abs()<=2&&(s[1]-self.origin[1]).abs()<=2);
        for dx in -1..=1 { for dy in -1..=1 {
            let sector=[self.origin[0]+dx,self.origin[1]+dy];
            if !self.loaded.insert(sector) { continue; }
            let mut rng=Rng(self.seed ^ (sector[0] as u64).wrapping_mul(0x517cc1b727220a95) ^ (sector[1] as u64).wrapping_mul(0x9e3779b97f4a7c15));
            for _ in 0..40 {
                let local=V2::new(rng.range(0.,SECTOR_SIZE),rng.range(0.,SECTOR_SIZE));
                let pos=WorldPos { sector, local }.relative_to(self.origin);
                if (pos-self.player.pos).length()<500. { continue; }
                let a=Asteroid { id:self.next_id, pos, radius:rng.range(30.,64.), hp:360., velocity:V2::from_angle(rng.range(0.,6.2831855))*rng.range(3.,20.), angle:rng.range(0.,6.2831855), spin:rng.range(-0.3,0.3), small:false };
                self.next_id+=1;self.asteroids.push(a);
            }
        }}
    }
    pub(crate) fn destroy_asteroids(&mut self) {
        let mut children=Vec::new(); let mut events=Vec::new();
        for a in &self.asteroids { if a.hp<=0. {
            events.push(WorldEvent::Destroyed { pos:a.pos, size:a.radius });
            if !a.small { for sign in [-1.,1.] {
                let normal=V2::from_angle(a.angle)*sign;
                children.push(Asteroid { id:self.next_id, pos:a.pos+normal*a.radius*0.45, radius:a.radius*0.48, hp:100., velocity:a.velocity+normal*45., angle:a.angle, spin:sign*0.8, small:true });self.next_id+=1;
            }}
        }}
        self.asteroids.retain(|a|a.hp>0.);self.asteroids.extend(children);
        for event in events { self.emit(event); }
    }
}

pub(crate) fn empty_loaded() -> HashSet<[i64;2]> { HashSet::new() }
