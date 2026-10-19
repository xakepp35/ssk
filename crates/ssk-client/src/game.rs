//! Fixed-step owner of the game session. The graphics thread never advances physics.
use crate::{Options, online::Online};
use ssk_sim::*;
use std::sync::{
    Arc,
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
    pub sector: SectorStatus,
    pub at: Instant,
}
enum Command {
    Restart { seed: u64, class: ShipClass },
}
pub struct Game {
    controls: mpsc::SyncSender<Controls>,
    frames: mpsc::Receiver<Frame>,
    requests: mpsc::SyncSender<()>,
    stopping: Arc<AtomicBool>,
    commands: mpsc::SyncSender<Command>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Game {
    pub fn start(options: &Options, class: ShipClass) -> (Self, Frame) {
        let _starting = tracing::debug_span!(
            "ssk.client.game.initialize",
            watchdog = true,
            online = options.online
        )
        .entered();
        let seed = if options.online {
            options.network.seed
        } else {
            options.seed
        };
        // A CPU-only preview is available immediately. Identity, sockets and
        // session initialization belong to the worker, never the window callback.
        let world = World::new(seed, class);
        let initial = Frame {
            world: world.clone(),
            peers: Vec::new(),
            status: Default::default(),
            frags: 0,
            error: String::new(),
            overloads: 0,
            sector: Default::default(),
            at: Instant::now(),
        };
        let initial_controls = Controls {
            input: Default::default(),
            paused: !options.play,
            next_class: class,
            spawn_radius: DEFAULT_SPAWN_RADIUS,
        };
        let capacity = crate::constants::GAME_HANDOFF_CAPACITY;
        let (controls, control_rx) = mpsc::sync_channel(capacity);
        let (frame_tx, frames) = mpsc::sync_channel(capacity);
        let (requests, request_rx) = mpsc::sync_channel(capacity);
        requests.try_send(()).expect("initial frame request fits");
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stop = stopping.clone();
        let (tx, rx) = mpsc::sync_channel(crate::constants::GAME_COMMAND_CAPACITY);
        let is_online = options.online;
        let network = options.network.clone();
        let connect = options.connect.clone();
        let worker_span = tracing::debug_span!("ssk.client.simulation.worker");
        let work = move || {
            let _worker = worker_span.enter();
            let mut error = String::new();
            let mut online = if is_online {
                match Online::new(network, connect) {
                    Ok(n) => Some(n),
                    Err(e) => {
                        tracing::error!(name:"ssk.client.network.start_failed",error=%e,"Online initialization failed");
                        error = e;
                        None
                    }
                }
            } else {
                None
            };
            let mut world = if let Some(n) = &mut online {
                let w = World::duel(seed, class, n.net.player_id);
                n.snapshot(&w);
                w
            } else {
                world
            };
            tracing::debug!(name:"ssk.client.simulation.begin",progress_id="ssk.client.simulation.sample",progress_active=true,
                logic_hz=TICK_PER_SEC,packet_ticks=TICK_PER_PACKET,"Simulation worker entered");
            let epoch = Instant::now();
            let mut deadline = epoch;
            let mut clock_tick = 0u64;
            let mut overloads = 0;
            let mut failed = is_online && online.is_none();
            let mut events = Vec::new();
            let mut shift = V2::ZERO;
            let mut control = initial_controls;
            let mut pending_frame = None;
            let mut sample_at = Instant::now();
            let mut window_ticks = 0u64;
            let mut max_step_us = 0u64;
            let mut max_lag_us = 0u64;
            'game: loop {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                while let Ok(command) = rx.try_recv() {
                    match command {
                        Command::Restart { seed, class } => {
                            tracing::debug!(name:"ssk.client.simulation.restart",class=?class,online=is_online,"Simulation restart command");
                            if !is_online {
                                world = World::new(seed, class);
                                failed = false;
                                error.clear();
                                events.clear();
                                shift = V2::ZERO;
                                deadline = Instant::now();
                            }
                        }
                    }
                }
                let now = Instant::now();
                if now < deadline {
                    std::thread::park_timeout(deadline - now);
                    continue;
                }
                if now.duration_since(deadline) > MAX_LOGIC_LAG && !failed {
                    tracing::error!(name:"ssk.client.simulation.overloaded",lag_us=now.duration_since(deadline).as_micros() as u64,
                        limit_us=MAX_LOGIC_LAG.as_micros() as u64,logic_hz=TICK_PER_SEC,"Simulation timing contract exceeded");
                    overloads += 1;
                    failed = true;
                    error = format!(
                        "Simulation cannot sustain {TICK_PER_SEC} Hz; session stopped. Relaunch to reconnect."
                    );
                    if let Some(n) = online.take() {
                        n.net.request_stop();
                        drop(n);
                    }
                }
                if failed {
                    deadline = now;
                }
                clock_tick = clock_tick.wrapping_add(1);
                let step_at = Instant::now();
                max_lag_us = max_lag_us.max(now.duration_since(deadline).as_micros() as u64);
                while let Ok(next) = control_rx.try_recv() {
                    control = next;
                }
                let time = epoch.elapsed().as_secs_f64();
                if !failed && (!control.paused || is_online) {
                    if let Some(n) = &mut online {
                        if clock_tick % TICK_PER_PACKET == 0 {
                            n.receive(&mut world, time);
                        }
                    }
                    world.spawn_radius = control.spawn_radius;
                    world.step(control.input);
                    if let Some(n) = &mut online {
                        if clock_tick % TICK_PER_PACKET == 0 {
                            n.snapshot(&world);
                        }
                        if n.after_frame(&mut world, DT, time) {
                            n.respawn(&mut world, control.next_class, seed);
                        }
                    }
                    // Cosmetic events are bounded separately from reliable game events.
                    let offset = world.origin_shift;
                    if offset.length_squared() > 0. {
                        for event in &mut events {
                            WorldEvent::shift(event, offset);
                        }
                        shift += offset;
                        world.origin_shift = V2::ZERO;
                    }
                    let capacity = MAX_WORLD_EVENTS.saturating_sub(events.len());
                    events.extend(world.events.drain(..).take(capacity));
                    world.events.clear();
                }
                // Only the simulation touches live state or network locks. One
                // request follows consumption of one frame: no GUI wait and no
                // replacement that could lose rebase offsets/cosmetic events.
                if pending_frame.is_none() && request_rx.try_recv().is_ok() {
                    let mut render_world = world.clone();
                    render_world.events = std::mem::take(&mut events);
                    render_world.origin_shift = shift;
                    shift = V2::ZERO;
                    let (peers, status, frags, net_error) = if let Some(n) = &online {
                        n.shared.present(&mut render_world, time);
                        let peers = n
                            .replicas
                            .peers
                            .keys()
                            .filter_map(|p| {
                                let (s, age) = n.replicas.sample(p, time)?;
                                Some((s.ship(ssk_net::player_id(p)?, world.origin), age))
                            })
                            .collect();
                        (
                            peers,
                            n.net.status.borrow().clone(),
                            n.frags,
                            n.error.clone(),
                        )
                    } else {
                        (
                            Vec::new(),
                            ssk_net::Status {
                                phase: if failed {
                                    error.clone()
                                } else {
                                    "Offline".into()
                                },
                                ..Default::default()
                            },
                            0,
                            String::new(),
                        )
                    };
                    let sector = online
                        .as_ref()
                        .map(|n| n.shared.sector_status(world.origin, time))
                        .unwrap_or_default();
                    pending_frame = Some(Frame {
                        world: render_world,
                        peers,
                        status,
                        frags,
                        error: if error.is_empty() {
                            net_error
                        } else {
                            error.clone()
                        },
                        overloads,
                        sector,
                        at: Instant::now(),
                    });
                }
                if let Some(frame) = pending_frame.take() {
                    match frame_tx.try_send(frame) {
                        Ok(()) => {}
                        Err(mpsc::TrySendError::Full(frame)) => pending_frame = Some(frame),
                        Err(mpsc::TrySendError::Disconnected(_)) => break 'game,
                    }
                }
                let step_us = step_at.elapsed().as_micros() as u64;
                window_ticks += 1;
                max_step_us = max_step_us.max(step_us);
                tracing::trace!(name:"ssk.client.simulation.tick",tick=clock_tick,world_tick=world.tick,step_us,failed,paused=control.paused,"Simulation iteration completed");
                if clock_tick % TICK_PER_SEC == 0 {
                    tracing::debug!(name:"ssk.client.simulation.sample",kind="metric",clock_tick,world_tick=world.tick,
                        ticks=window_ticks,window_ms=sample_at.elapsed().as_secs_f64()*crate::constants::MILLIS_PER_SECOND,
                        max_step_us,max_lag_us,failed,paused=control.paused,npcs=world.npcs.len(),asteroids=world.asteroids.len(),
                        bullets=world.bullets.len(),pickups=world.pickups.len(),overloads,"Simulation progress sample");
                    sample_at = Instant::now();
                    window_ticks = 0;
                    max_step_us = 0;
                    max_lag_us = 0;
                }
                deadline += TICK_DURATION;
            }
            if let Some(n) = &mut online {
                n.leave();
            }
            tracing::debug!(name:"ssk.client.simulation.end",progress_id="ssk.client.simulation.sample",progress_active=false,"Simulation worker stopped");
        };
        let thread = std::thread::Builder::new()
            .name("ssk-sim".into())
            .spawn(work)
            .expect("spawn simulation worker");
        (
            Self {
                controls,
                frames,
                requests,
                stopping,
                commands: tx,
                thread: Some(thread),
            },
            initial,
        )
    }
    pub fn update(&self, controls: Controls) {
        // The worker drains this slot at 360 Hz. A stalled simulation must
        // never stall native input/events or let an input queue grow.
        let _ = self.controls.try_send(controls);
    }
    pub fn take_frame(&self) -> Option<Frame> {
        let frame = self.frames.try_recv().ok()?;
        let _ = self.requests.try_send(());
        Some(frame)
    }
    pub fn restart(&self, seed: u64, class: ShipClass) {
        let _ = self.commands.try_send(Command::Restart { seed, class });
    }
    pub fn is_finished(&self) -> bool {
        self.thread
            .as_ref()
            .is_none_or(|thread| thread.is_finished())
    }
}
impl Drop for Game {
    fn drop(&mut self) {
        let _joining =
            tracing::debug_span!("ssk.client.simulation.join", watchdog = true).entered();
        // Shutdown cannot wait for space in a full restart-command queue.
        self.stopping.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            t.thread().unpark();
            let _ = t.join();
        }
    }
}
