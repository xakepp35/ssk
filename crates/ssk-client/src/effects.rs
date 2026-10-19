use macroquad::{audio::{Sound,PlaySoundParams,load_sound_from_bytes,play_sound},prelude::*};
use ssk_sim::{V2,Rng,WorldEvent,WeaponKind,PickupKind};
use super::screen;
struct Particle { pos:V2,velocity:V2,life:f32,max:f32,color:Color,size:f32 }
struct Ring { pos:V2,life:f32,size:f32,color:Color }
pub struct Effects {
    particles:Vec<Particle>,rings:Vec<Ring>,sounds:Vec<Option<Sound>>,rng:Rng,
    pub shake:f32,pub flash:f32,
}
impl Effects {
    pub async fn new() -> Self {
        let mut sounds=Vec::new();
        for (frequency,duration,noise) in [(160.,0.055,0.65),(95.,0.13,0.85),(480.,0.20,0.12),(250.,0.065,0.8),(65.,0.35,0.9),(880.,0.14,0.02),(110.,0.07,0.05)] {
            sounds.push(load_sound_from_bytes(&wave(frequency,duration,noise)).await.ok());
        }
        Self { particles:Vec::with_capacity(2400),rings:Vec::with_capacity(64),sounds,rng:Rng(82),shake:0.,flash:0. }
    }
    pub fn clear(&mut self) { self.particles.clear();self.rings.clear();self.shake=0.;self.flash=0.; }
    pub fn shift(&mut self,offset:V2) { for p in &mut self.particles { p.pos-=offset; } for r in &mut self.rings {r.pos-=offset;} }
    fn sound(&self,index:usize,volume:f32) { if volume>0. { if let Some(sound)=&self.sounds[index] { play_sound(sound,PlaySoundParams {looped:false,volume}); } } }
    fn burst(&mut self,pos:V2,count:usize,color:Color,speed:f32) {
        for _ in 0..count {
            if self.particles.len()>=2400 { break; }
            let life=self.rng.range(0.12,0.5);
            self.particles.push(Particle { pos,velocity:V2::from_angle(self.rng.range(0.,std::f32::consts::TAU))*self.rng.range(30.,speed),life,max:life,color,size:self.rng.range(1.,3.5) });
        }
    }
    pub fn events(&mut self,events:&[WorldEvent],camera:V2,volume:f32) {
        for event in events {
            let pos=match *event { WorldEvent::Shot(s)=>s.pos,WorldEvent::Impact{pos,..}|WorldEvent::Destroyed{pos,..}|WorldEvent::Pickup{pos,..}=>pos,_=>camera };
            let distance=(pos-camera).length();
            if distance>1800. { continue; }
            let volume=volume*(1.-distance/2100.).max(0.)*0.5;
            match *event {
                WorldEvent::Shot(shot)=>{
                    let (index,color)=match shot.weapon {WeaponKind::MachineGun=>(0,YELLOW),WeaponKind::Shotgun=>(1,ORANGE),WeaponKind::Plasma=>(2,VIOLET)};
                    self.burst(pos,4,color,110.);self.sound(index,volume);
                }
                WorldEvent::Impact{damage,..}=>{ self.burst(pos,(damage/12.).clamp(4.,20.) as usize,ORANGE,210.);self.sound(3,volume*0.6); }
                WorldEvent::Destroyed{size,..}=>{
                    self.burst(pos,32,ORANGE,300.);self.sound(4,volume);
                    if self.rings.len()<64 { self.rings.push(Ring{pos,life:0.4,size:size*2.,color:ORANGE}); }
                    self.shake=(self.shake+2.*(1.-distance/1800.)).min(8.);
                }
                WorldEvent::Pickup{kind,..}=>{
                    let color=match kind {PickupKind::Energy=>SKYBLUE,PickupKind::Health=>GREEN};
                    self.burst(pos,18,color,180.);self.sound(5,volume);
                }
                WorldEvent::Dry=>self.sound(6,volume),
            }
        }
    }
    pub fn update(&mut self,dt:f32) {
        for p in &mut self.particles {p.pos+=p.velocity*dt;p.velocity*=(-3.*dt).exp();p.life-=dt;}
        self.particles.retain(|p|p.life>0.);
        for r in &mut self.rings {r.life-=dt;} self.rings.retain(|r|r.life>0.);
        self.shake*=(-15.*dt).exp();self.flash=(self.flash-dt*5.).max(0.);
    }
    pub fn offset(&mut self,strength:f32)->V2 { V2::new(self.rng.range(-1.,1.),self.rng.range(-1.,1.))*self.shake*strength }
    pub fn draw(&self,camera:V2,flashes:bool) {
        for p in &self.particles {let point=screen(p.pos,camera);let tail=screen(p.pos-p.velocity*0.025,camera);let mut color=p.color;color.a=p.life/p.max;draw_line(tail.x,tail.y,point.x,point.y,p.size,color);}
        for r in &self.rings {let p=screen(r.pos,camera);let mut color=r.color;color.a=r.life*2.;draw_circle_lines(p.x,p.y,(1.-r.life/0.4)*r.size,2.,color);}
        if flashes&&self.flash>0. {draw_rectangle(0.,0.,screen_width(),screen_height(),Color::new(0.9,0.12,0.08,self.flash*0.15));}
    }
}
fn wave(frequency:f32,duration:f32,noise:f32)->Vec<u8> {
    let samples=(44100.*duration) as usize;let size=(samples*2) as u32;let mut bytes=Vec::with_capacity(samples*2+44);
    bytes.extend(b"RIFF");bytes.extend((size+36).to_le_bytes());bytes.extend(b"WAVEfmt ");bytes.extend(16u32.to_le_bytes());bytes.extend(1u16.to_le_bytes());bytes.extend(1u16.to_le_bytes());bytes.extend(44100u32.to_le_bytes());bytes.extend(88200u32.to_le_bytes());bytes.extend(2u16.to_le_bytes());bytes.extend(16u16.to_le_bytes());bytes.extend(b"data");bytes.extend(size.to_le_bytes());
    let mut rng=Rng(118);let mut phase=0.;
    for i in 0..samples {let t=i as f32/samples as f32;phase+=std::f32::consts::TAU*frequency*(1.-t*0.6)/44100.;let envelope=(1.-t).powf(2.)*(t*80.).min(1.);let sample=((phase.sin()*(1.-noise)+rng.range(-1.,1.)*noise)*envelope*22000.) as i16;bytes.extend(sample.to_le_bytes());}
    bytes
}
