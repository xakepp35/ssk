use crate::*;

pub struct Npc { pub ship:Ship, pub input:InputFrame, pub think:f32, pub age:f32, pub telegraph:f32 }
impl World {
    pub(crate) fn enemies_step(&mut self) {
        let time=self.tick as f32*DT;
        if time>=self.next_spawn && self.npcs.len()<4 {
            let direction=V2::from_angle(self.rng.range(0.,std::f32::consts::TAU));
            let pos=self.player.pos+direction*1100.;
            let mut ship=Ship::new(self.next_id,ShipClass::Spark,pos); self.next_id+=1;
            ship.hp*=0.6;
            self.npcs.push(Npc { ship, input:InputFrame::default(), think:0.25, age:0., telegraph:0.8 });
            self.next_spawn=time+3.;
        }
        let mut shots=Vec::new();
        for npc in &mut self.npcs {
            npc.age+=DT; npc.telegraph=(npc.telegraph-DT).max(0.);
            if npc.telegraph>0. { continue; }
            npc.think-=DT;
            if npc.think<=0. {
                let delta=self.player.pos-npc.ship.pos;
                let distance=delta.length();
                let normal=delta.normalized();
                npc.input=InputFrame { movement:normal*((distance-500.)/200.).clamp(-1.,1.)+normal.perp()*0.5,
                    aim:delta.angle()+self.rng.range(-0.12,0.12), fire:distance<1000. && npc.age%1.8<0.65, weapon:0, ability:false };
                npc.think=0.25+self.rng.range(0.,0.15);
            }
            npc.ship.move_step(npc.input);
            if let Some(mut shot)=npc.ship.combat_step(npc.input,&mut self.rng,self.next_id) {
                self.next_id+=1;shot.damage_scale=0.6;shots.push(shot);
            }
        }
        for shot in shots { self.spawn_shot(shot); }
    }
    pub(crate) fn destroy_enemies(&mut self) {
        let dead:Vec<_>=self.npcs.iter().filter(|n|n.ship.hp<=0.).map(|n|(n.ship.pos,n.ship.class)).collect();
        self.npcs.retain(|n|n.ship.hp>0.);
        for (pos,class) in dead { self.score+=match class { ShipClass::Spark=>100,ShipClass::Cadet=>150,ShipClass::Thunder=>250 };self.emit(WorldEvent::Destroyed { pos,size:30. }); }
        self.npcs.retain(|n|(n.ship.pos-self.player.pos).length()<3500.);
    }
}
