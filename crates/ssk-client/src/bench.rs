//! Real headless clients; no window, mocks or assertions. Reports measurements only.
use crate::online::Online;
use rayon::prelude::*;
use ssk_net::{Config, SnapshotCodec};
use ssk_sim::*;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct Options {
    pub players: usize,
    pub seconds: u64,
    pub warmup: u64,
    pub fire: bool,
    pub connect: Option<String>,
    pub invite_file: Option<PathBuf>,
    pub drop_count: usize,
    pub workers: usize,
}
struct Bot {
    online: Online,
    world: World,
}
fn quantiles(mut values: Vec<f64>) -> serde_json::Value {
    if values.is_empty() {
        return serde_json::Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let q = |p: f64| values[((values.len() - 1) as f64 * p) as usize];
    serde_json::json!({"min":values.first(),"p05":q(0.05),"p50":q(0.5),"p95":q(0.95),"p99":q(0.99),"max":values.last(),"samples":values.len()})
}
pub fn run(mut config: Config, opt: Options) -> Result<serde_json::Value, String> {
    if !(1..=128).contains(&opt.players)
        || !(1..=3600).contains(&opt.seconds)
        || opt.warmup > 300
        || opt.drop_count >= opt.players
        || !(1..=128).contains(&opt.workers)
    {
        return Err("players 1..128, seconds 1..3600, warmup <=300, drop_count < players".into());
    }
    let external = opt.connect.is_some() || opt.players == 1;
    if external && opt.players != 1 {
        return Err("external mode needs --players 1 and a unique identity per process".into());
    }
    let workers = opt.workers.min(opt.players);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|e| e.to_string())?;
    config.contact_cache = None;
    config.relay_server = None;
    let temp = std::env::temp_dir().join(format!(
        "ssk-bench-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    let mut bots = Vec::new();
    for i in 0..opt.players {
        let mut c = config.clone();
        if !external {
            c.identity_file = temp.join(format!("{i}.key"));
            c.seeds.clear();
            c.friends.clear();
            c.udp_bind = "127.0.0.1:0".parse().unwrap();
            c.advertise = None;
            c.discovery_bind = "127.0.0.1:0".parse().unwrap();
            c.discovery_advertise.clear();
        }
        let online = Online::new(c, if i == 0 { opt.connect.clone() } else { None })?;
        let world = World::duel(config.seed, ShipClass::ALL[i % 3], online.net.player_id);
        online.net.publish(Snapshot::capture(&world, 1, 0).encode());
        bots.push(Some(Bot { online, world }));
    }
    let ready = Instant::now();
    loop {
        if bots
            .iter()
            .flatten()
            .all(|b| !b.online.net.status.borrow().contact.is_empty())
        {
            break;
        }
        if ready.elapsed().as_secs() > 30 {
            return Err("network startup timed out".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if !external {
        let roster: Vec<_> = bots
            .iter()
            .flatten()
            .map(|b| b.online.net.status.borrow().contact.clone())
            .collect();
        for b in bots.iter().flatten() {
            b.online.net.learn_contacts(roster.clone());
        }
    }
    if let Some(path) = &opt.invite_file {
        // Give the configured relay time to enter the endpoint address before export.
        let start = Instant::now();
        while !config.relays.is_empty() && start.elapsed().as_secs() < 8 {
            let ticket = bots[0]
                .as_ref()
                .unwrap()
                .online
                .net
                .status
                .borrow()
                .ticket
                .clone();
            if ssk_net::decode_ticket(&ticket).is_ok_and(|a| {
                a.addrs
                    .iter()
                    .any(|a| matches!(a, ssk_net::TransportAddr::Relay(_)))
            }) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let ticket = bots[0]
            .as_ref()
            .unwrap()
            .online
            .net
            .status
            .borrow()
            .ticket
            .clone();
        std::fs::write(path, ticket).map_err(|e| e.to_string())?;
    }
    let start = Instant::now();
    let mut previous = start;
    let mut sim_acc = 0f32;
    let mut samples_at = 0.;
    let mut frames = Vec::new();
    let mut sim_steps = Vec::new();
    let mut client_work = Vec::new();
    let mut receive_work = Vec::new();
    let mut ages = Vec::new();
    let mut tx = Vec::new();
    let mut rx = Vec::new();
    let mut degrees = Vec::new();
    let mut link_pps = Vec::new();
    let mut rtts = Vec::new();
    let mut queues = Vec::new();
    let mut shared_ages = Vec::new();
    let mut shared_images = Vec::new();
    let mut rocks = Vec::new();
    let mut npcs = Vec::new();
    let mut fresh = Vec::new();
    let mut max_dropped = 0;
    let mut max_unconfirmed = 0;
    let mut dropped_base = vec![None; opt.players];
    let mut unconfirmed_base = vec![None; opt.players];
    let mut measured_dropped = 0;
    let mut measured_unconfirmed = 0;
    let mut relayed_links = 0;
    let mut skipped_steps = 0u64;
    let mut removed = false;
    while start.elapsed().as_secs_f64() < (opt.warmup + opt.seconds) as f64 {
        let frame = Instant::now();
        let elapsed = frame.duration_since(previous).as_secs_f32();
        previous = frame;
        let dt = elapsed.min(0.1);
        let now = start.elapsed().as_secs_f64();
        let measured = now >= opt.warmup as f64;
        if opt.drop_count > 0 && !removed && now >= opt.warmup as f64 + opt.seconds as f64 / 2. {
            for bot in bots.iter_mut().rev().take(opt.drop_count) {
                *bot = None;
            }
            removed = true;
        }
        sim_acc += dt;
        let mut steps = 0;
        while sim_acc >= DT && steps < MAX_CATCH_UP_TICKS {
            sim_acc -= DT;
            steps += 1;
        }
        if sim_acc >= DT {
            skipped_steps += (sim_acc / DT) as u64;
            sim_acc = sim_acc.min(DT);
        }
        let work: Vec<_> = pool.install(|| {
            bots.par_iter_mut()
                .enumerate()
                .filter_map(|(i, bot)| {
                    let b = bot.as_mut()?;
                    let client_start = Instant::now();
                    b.online.receive(&mut b.world, now);
                    let receive_ms = client_start.elapsed().as_secs_f64() * 1000.;
                    let mut timings = Vec::new();
                    for _ in 0..steps {
                        let t = now as f32 * 0.5
                            + i as f32 * std::f32::consts::TAU / opt.players as f32;
                        let target = V2::new(2048., 2048.) + V2::from_angle(t) * 300.;
                        let input = InputFrame {
                            movement: (target - b.world.player.pos).normalized(),
                            aim: (V2::new(2048., 2048.) - b.world.player.pos).angle(),
                            fire: opt.fire && now % 2. < 1.3,
                            weapon: i % 3,
                            ability: now % 8. < 0.7,
                        };
                        let step = Instant::now();
                        b.world.step(input);
                        if b.world.tick % TICK_PER_PACKET == 0 {
                            b.online.snapshot(&b.world);
                        }
                        if measured {
                            timings.push(step.elapsed().as_secs_f64() * 1e6);
                        }
                    }
                    if b.online.after_frame(&mut b.world, dt, now) {
                        let class = b.world.player.class;
                        b.online.respawn(&mut b.world, class, config.seed);
                    }
                    b.world.events.clear();
                    Some((
                        client_start.elapsed().as_secs_f64() * 1000.,
                        receive_ms,
                        timings,
                    ))
                })
                .collect()
        });
        if measured {
            for (client_ms, receive_ms, timings) in work {
                client_work.push(client_ms);
                receive_work.push(receive_ms);
                if sim_steps.len() < 500_000 {
                    sim_steps.extend(timings);
                }
            }
            frames.push(frame.elapsed().as_secs_f64() * 1000.);
            if now - samples_at >= 1. {
                samples_at = now;
                let expected = if external {
                    1
                } else {
                    bots.iter().flatten().count().saturating_sub(1)
                };
                for (i, bot) in bots.iter().enumerate() {
                    let Some(b) = bot else {
                        continue;
                    };
                    let (images, age) = b.online.shared_stats(now);
                    shared_images.push(images as f64);
                    shared_ages.push(age);
                    rocks.push(b.world.asteroids.len() as f64);
                    npcs.push(b.world.npcs.len() as f64);
                    let s = b.online.net.status.borrow();
                    tx.push(s.links.iter().map(|l| l.tx_bps as f64).sum());
                    rx.push(s.links.iter().map(|l| l.rx_bps as f64).sum());
                    degrees.push(s.peers as f64);
                    max_dropped = max_dropped.max(s.dropped);
                    max_unconfirmed = max_unconfirmed.max(s.unconfirmed);
                    let base = *dropped_base[i].get_or_insert(s.dropped);
                    measured_dropped = measured_dropped.max(s.dropped.saturating_sub(base));
                    let base = *unconfirmed_base[i].get_or_insert(s.unconfirmed);
                    measured_unconfirmed =
                        measured_unconfirmed.max(s.unconfirmed.saturating_sub(base));
                    for l in &s.links {
                        link_pps.push(l.tx_pps as f64);
                        queues.push(l.queued as f64);
                        if let Some(r) = l.rtt_ms {
                            rtts.push(r as f64);
                        }
                        if !l.direct {
                            relayed_links += 1;
                        }
                    }
                    let mut n = 0;
                    for q in b.online.replicas.peers.values() {
                        if let Some((t, _)) = q.back() {
                            let age = now - t;
                            ages.push(age * 1000.);
                            if age < 0.25 {
                                n += 1;
                            }
                        }
                    }
                    fresh.push(if expected == 0 {
                        1.
                    } else {
                        n.min(expected) as f64 / expected as f64
                    });
                }
            }
        }
        if let Some(rest) = PACKET_DURATION.checked_sub(frame.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    let crypto: Vec<_> = bots
        .iter()
        .flatten()
        .map(|b| b.online.net.status.borrow().crypto.clone())
        .collect();
    let errors: Vec<_> = bots
        .iter()
        .flatten()
        .map(|b| b.online.error.clone())
        .filter(|s| !s.is_empty())
        .collect();
    let report = serde_json::json!({"players":opt.players,"remaining":bots.iter().flatten().count(),"seconds":opt.seconds,"warmup":opt.warmup,"hz":PACKET_PER_SEC,"tick_per_sec":TICK_PER_SEC,"tick_per_packet":TICK_PER_PACKET,"firing":opt.fire,"simulation_workers":workers,
        "scenario":if external{"external-configured"}else{"loopback-preintroduced-signed-roster"},
        "scope":"game payload only; excludes QUIC/DHT/NAT wire overhead; no rendered frames; crypto counters include warmup",
        "frame_work_ms":quantiles(frames),"client_work_ms":quantiles(client_work),"client_receive_work_ms":quantiles(receive_work),"ship_step_us":quantiles(sim_steps),"replica_age_ms":quantiles(ages),"fresh_peer_fraction":quantiles(fresh),
        "node_tx_bytes_sec":quantiles(tx),"node_rx_bytes_sec":quantiles(rx),"degree":quantiles(degrees),"link_tx_pps":quantiles(link_pps),
        "direct_or_nat_rtt_ms":quantiles(rtts),"link_critical_queue":quantiles(queues),"max_node_dropped":max_dropped,"max_node_unconfirmed":max_unconfirmed,
        "shared_image_count":quantiles(shared_images),"shared_oldest_image_age_ms":quantiles(shared_ages),"visible_asteroids":quantiles(rocks),"visible_npcs":quantiles(npcs),
        "relayed_link_samples":relayed_links,"skipped_sim_steps":skipped_steps,"crypto":crypto,"last_errors":errors,
        "max_node_measured_dropped":measured_dropped,"max_node_measured_unconfirmed":measured_unconfirmed,
        "cpu":std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default().lines().find(|s|s.starts_with("model name")).unwrap_or("unknown"),
        "logical_cpus":std::thread::available_parallelism().map(|n|n.get()).unwrap_or(0),"target":std::env::consts::ARCH});
    for b in bots.iter().flatten() {
        b.online.net.request_stop();
    }
    drop(bots);
    let _ = std::fs::remove_dir_all(temp);
    Ok(report)
}
