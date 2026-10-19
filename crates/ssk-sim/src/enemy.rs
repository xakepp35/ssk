use crate::*;

pub struct Npc { pub ship:Ship, pub input:InputFrame, pub think:f32, pub age:f32, pub telegraph:f32,pub last_hit:u64 }
impl World {
    pub(crate) fn enemies_step(&mut self) {
        let time=self.tick as f32*DT;
        let target=(2+(time/20.) as usize).min(12);
        if time>=self.next_spawn && self.npcs.len()<target {
            let direction=V2::from_angle(self.rng.range(0.,std::f32::consts::TAU));
            let pos=self.player.pos+direction*self.spawn_radius;
            let roll=self.rng.unit();
            let class=if time>=60.&&roll>0.7 { ShipClass::Thunder } else if roll>0.45 { ShipClass::Cadet } else { ShipClass::Spark };
            let mut ship=Ship::new(self.next_id,class,pos); self.next_id+=1;
            ship.hp*=0.6*(1.+time/1200.).min(1.5);
            self.npcs.push(Npc { ship, input:InputFrame::default(), think:0.25, age:0., telegraph:0.8,last_hit:0 });
            self.next_spawn=time+3.;
        }
        let mut shots=Vec::new();
        let mut order:Vec<_>=self.npcs.iter().map(|n|((n.ship.pos-self.player.pos).length_squared(),n.ship.id)).collect();
        order.sort_by(|a,b|a.0.total_cmp(&b.0));
        let shooters:Vec<_>=order.iter().take(3).map(|(_,id)|*id).collect();
        let mut nearby=Vec::new();
        for npc in &mut self.npcs {
            npc.age+=DT; npc.telegraph=(npc.telegraph-DT).max(0.);
            if npc.telegraph>0. { continue; }
            npc.think-=DT;
            if npc.think<=0. {
                let delta=self.player.pos-npc.ship.pos;
                let distance=delta.length();
                let normal=delta.normalized();
                let (range,weapon,strafe)=match npc.ship.class { ShipClass::Spark=>(500.,0,0.85),ShipClass::Cadet=>(250.,1,0.2),ShipClass::Thunder=>(750.,2,0.25) };
                let retreat=matches!(npc.ship.class,ShipClass::Cadet)&&npc.ship.hp<npc.ship.class.stats().0*0.25&&distance<500.;
                let phase=(npc.age+npc.ship.id as f32*0.37)%3.;
                let ability=match npc.ship.class { ShipClass::Spark=>distance>900.,ShipClass::Cadet=>retreat,ShipClass::Thunder=>distance<950.&&phase<0.75 };
                let lead=self.player.velocity*(distance/WeaponKind::from_index(weapon).stats().speed*0.6);
                npc.input=InputFrame { movement:normal*if retreat {-1.}else{((distance-range)/200.).clamp(-1.,1.)}+normal.perp()*strafe*(if npc.ship.id%2==0 {1.}else{-1.}),
                    aim:(delta+lead).angle()+self.rng.range(-0.10,0.10), fire:distance<range+350. && phase<1.1 && !retreat, weapon, ability };
                self.grid.query(npc.ship.pos,npc.ship.pos,150.,&mut nearby);
                for &index in &nearby {
                    let rock=&self.asteroids[index];let away=npc.ship.pos-rock.pos;
                    let gap=away.length()-rock.radius;
                    if gap<140. { npc.input.movement+=away.normalized()*((140.-gap)/70.); }
                }
                npc.think=0.25+self.rng.range(0.,0.15);
            }
            npc.ship.move_step(npc.input);
            self.grid.query(npc.ship.pos,npc.ship.pos,npc.ship.class.stats().3,&mut nearby);
            for &index in &nearby {
                let rock=&self.asteroids[index];let away=npc.ship.pos-rock.pos;let min=rock.radius+npc.ship.class.stats().3;
                if away.length()<min { npc.ship.pos=rock.pos+away.normalized()*(min+1.);npc.ship.velocity=away.normalized()*80.; }
            }
            let mut combat_input=npc.input;combat_input.fire&=shooters.contains(&npc.ship.id);
            if let Some(mut shot)=npc.ship.combat_step(combat_input,&mut self.rng,self.next_id) {
                self.next_id+=1;shot.damage_scale*=0.6;shots.push(shot);
            }
        }
        for shot in shots { self.spawn_shot(shot); }
    }
    pub(crate) fn destroy_enemies(&mut self) {
        let dead:Vec<_>=self.npcs.iter().filter(|n|n.ship.hp<=0.).map(|n|(n.ship.pos,n.ship.class,n.last_hit==self.player.id)).collect();
        self.npcs.retain(|n|n.ship.hp>0.);
        for (pos,class,owned) in dead { if owned { self.award(match class { ShipClass::Spark=>100,ShipClass::Cadet=>150,ShipClass::Thunder=>250 }); } self.drop_loot(pos,true);self.emit(WorldEvent::Destroyed { pos,size:30. }); }
        self.npcs.retain(|n|(n.ship.pos-self.player.pos).length()<3500.);
    }
}
