//! Fixed-step owner of the game session. The graphics thread never advances physics.
use crate::{Options, online::Online};
use ssk_sim::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct Controls {
    pub input: InputFrame,
    pub paused: bool,
    pub next_class: ShipClass,
    pub spawn_radius: f32,
}
pub struct Frame {
    pub world: World,
    pub peers: Vec<(Ship, f64)>,
    pub status: ssk_net::Status,
    pub frags: u32,
    pub error: String,
    pub overloads: u64,
    pub at: Instant,
}
enum Command {
    Restart { seed: u64, class: ShipClass },
    Stop,
}
pub struct Game {
    controls: Arc<Mutex<Controls>>,
    frame: Arc<Mutex<Option<Frame>>>,
    requested: Arc<AtomicBool>,
    commands: mpsc::SyncSender<Command>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Game {
    pub fn start(options: &Options, class: ShipClass) -> (Self, Frame) {
        let mut error = String::new();
        let mut online = if options.online {
            match Online::new(options.network.clone(), options.connect.clone()) {
                Ok(n) => Some(n),
                Err(e) => {
                    error = e;
                    None
                }
            }
        } else {
            None
        };
        let seed = if options.online {
            options.network.seed
        } else {
            options.seed
        };
        let world = if let Some(n) = &mut online {
            let w = World::duel(seed, class, n.net.player_id);
            n.snapshot(&w);
            w
        } else {
            World::new(seed, class)
        };
        let initial = Frame {
            world: world.clone(),
            peers: Vec::new(),
            status: Default::default(),
            frags: 0,
            error: error.clone(),
            overloads: 0,
            at: Instant::now(),
        };
        let controls = Arc::new(Mutex::new(Controls {
            input: Default::default(),
            paused: !options.play,
            next_class: class,
            spawn_radius: DEFAULT_SPAWN_RADIUS,
        }));
        let frame = Arc::new(Mutex::new(None));
        let requested = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::sync_channel(crate::constants::GAME_COMMAND_CAPACITY);
        let (c, f, r) = (controls.clone(), frame.clone(), requested.clone());
        let is_online = options.online;
        let thread=std::thread::Builder::new().name("ssk-sim".into()).spawn(move || {
            let mut world=world;
            let epoch=Instant::now();
            let mut deadline=epoch;
            let mut clock_tick=0u64;
            let mut overloads=0;
            let mut failed=is_online && online.is_none();
            let mut events=Vec::new();
            let mut shift=V2::ZERO;
            'game: loop {
                while let Ok(command)=rx.try_recv() {
                    match command {
                        Command::Stop=>break 'game,
                        Command::Restart {seed,class}=>{
                            if !is_online { world=World::new(seed,class); failed=false; error.clear();events.clear();shift=V2::ZERO;deadline=Instant::now(); }
                        }
                    }
                }
                let now=Instant::now();
                if now < deadline { std::thread::park_timeout(deadline-now); continue; }
                if now.duration_since(deadline)>MAX_LOGIC_LAG && !failed {
                    overloads+=1; failed=true;
                    error=format!("Simulation cannot sustain {TICK_PER_SEC} Hz; session stopped. Relaunch to reconnect.");
                    if let Some(n)=online.take(){n.net.request_stop();drop(n);}
                }
                if failed {deadline=now;}
                clock_tick=clock_tick.wrapping_add(1);
                let control=*c.lock().unwrap();
                let time=epoch.elapsed().as_secs_f64();
                if !failed && (!control.paused || is_online) {
                    if let Some(n)=&mut online {
                        if clock_tick%TICK_PER_PACKET==0 {n.receive(&mut world,time);}
                    }
                    world.spawn_radius=control.spawn_radius;
                    world.step(control.input);
                    if let Some(n)=&mut online {
                        if clock_tick%TICK_PER_PACKET==0 {n.snapshot(&world);}
                        if n.after_frame(&mut world,DT,time) {n.respawn(&mut world,control.next_class,seed);}
                    }
                    // Cosmetic events are bounded separately from reliable game events.
                    let offset=world.origin_shift;
                    if offset.length_squared()>0. {
                        for event in &mut events {WorldEvent::shift(event,offset);}
                        shift+=offset;world.origin_shift=V2::ZERO;
                    }
                    let capacity=MAX_WORLD_EVENTS.saturating_sub(events.len());
                    events.extend(world.events.drain(..).take(capacity));world.events.clear();
                }
                if r.load(Ordering::Acquire) {
                    let mut slot=f.lock().unwrap();
                    if slot.is_none() {
                        let mut render_world=world.clone();render_world.events=std::mem::take(&mut events);render_world.origin_shift=shift;shift=V2::ZERO;
                        let (peers,status,frags,net_error)=if let Some(n)=&online {
                            let peers=n.replicas.peers.keys().filter_map(|p| {let (s,age)=n.replicas.sample(p,time)?;Some((s.ship(ssk_net::player_id(p)?,world.origin),age))}).collect();
                            (peers,n.net.status.borrow().clone(),n.frags,n.error.clone())
                        } else {(Vec::new(),ssk_net::Status {phase:if failed {error.clone()}else {"Offline".into()},..Default::default()},0,String::new())};
                        *slot=Some(Frame {world:render_world,peers,status,frags,error:if error.is_empty(){net_error}else{error.clone()},overloads,at:Instant::now()});
                        r.store(false,Ordering::Release);
                    }
                }
                deadline+=TICK_DURATION;
            }
            if let Some(n)=&mut online {n.leave();}
        }).expect("spawn simulation worker");
        (
            Self {
                controls,
                frame,
                requested,
                commands: tx,
                thread: Some(thread),
            },
            initial,
        )
    }
    pub fn update(&self, controls: Controls) {
        *self.controls.lock().unwrap() = controls;
    }
    pub fn take_frame(&self) -> Option<Frame> {
        let frame = self.frame.lock().unwrap().take();
        self.requested.store(true, Ordering::Release);
        frame
    }
    pub fn restart(&self, seed: u64, class: ShipClass) {
        let _ = self.commands.try_send(Command::Restart { seed, class });
    }
}
impl Drop for Game {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(t) = self.thread.take() {
            t.thread().unpark();
            let _ = t.join();
        }
    }
}
