pub mod constants;
use constants::*;
use ssk_sim::{CombatEvent, SharedEvent, Snapshot};
// Bounded bridge between game thread and asynchronous network workers.
mod auth;
mod bootstrap;
mod cache;
mod metrics;
mod overlay;
mod relay;
mod routes;
pub mod topology;
pub use iroh::TransportAddr;
pub use metrics::LinkStats;
mod config;
mod contact;
mod discovery;
mod events;
mod shared;
pub use events::*;
pub use shared::*;
mod reliable;
mod replication;
pub use replication::*;
mod transport;
pub use config::*;
pub use transport::{decode_ticket, encode_ticket};
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub contact: Vec<u8>,
    pub crypto: CryptoStats,
    pub phase: String,
    pub ticket: String,
    pub peers: usize,
    pub dropped: u64,
    pub links: Vec<LinkStats>,
    pub routes: usize,
    pub route_changes: u64,
    pub unconfirmed: u64,
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct CryptoStats {
    pub signed: u64,
    pub verified: u64,
    pub sign_us: u64,
    pub verify_us: u64,
    pub sign_max_us: u64,
    pub verify_max_us: u64,
    pub cache_hits: u64,
}
#[derive(Debug)]
pub enum NetEvent {
    Connected(String),
    Disconnected(String),
    Packet {
        peer: String,
        kind: u8,
        bytes: Vec<u8>,
    },
    Error(String),
}
pub struct Network {
    pub player_id: u64,
    state: tokio::sync::watch::Sender<Vec<u8>>,
    events: tokio::sync::mpsc::Sender<Vec<u8>>,
    shared: tokio::sync::watch::Sender<Vec<Vec<u8>>>,
    contacts: tokio::sync::watch::Sender<Vec<Vec<u8>>>,
    pub incoming: tokio::sync::mpsc::Receiver<NetEvent>,
    pub status: tokio::sync::watch::Receiver<Status>,
    snapshots: SnapshotInbox,
    stop: tokio::sync::watch::Sender<bool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
type SnapshotInbox = std::sync::Arc<
    std::sync::Mutex<std::collections::HashMap<String, (Vec<u8>, std::time::Instant)>>,
>;
pub(crate) struct Worker {
    state: tokio::sync::watch::Receiver<Vec<u8>>,
    events: tokio::sync::mpsc::Receiver<Vec<u8>>,
    shared: tokio::sync::watch::Receiver<Vec<Vec<u8>>>,
    contacts: tokio::sync::watch::Receiver<Vec<Vec<u8>>>,
    incoming: tokio::sync::mpsc::Sender<NetEvent>,
    snapshots: SnapshotInbox,
    status: tokio::sync::watch::Sender<Status>,
    stop: tokio::sync::watch::Receiver<bool>,
}
impl Network {
    pub fn start(config: Config, connect: Option<String>) -> anyhow::Result<Self> {
        let _initializing = tracing::debug_span!("ssk.net.initialize", watchdog = true).entered();
        tracing::debug!(name:"ssk.net.config.accepted",udp_bind=%config.udp_bind,discovery_bind=%config.discovery_bind,
            bootstrap=config.bootstrap_bind.is_some(),relay_count=config.relays.len(),seed_count=config.seeds.len(),
            friend_count=config.friends.len(),connect_requested=connect.is_some(),"Network settings summary; keys/addresses of remote contacts omitted");
        config.validate()?;
        let identity_span =
            tracing::debug_span!("ssk.net.identity.load", watchdog = true).entered();
        let key = load_identity(&config.identity_file)?;
        drop(identity_span);
        tracing::debug!(name:"ssk.net.identity.ready","Identity loaded; key material omitted");
        let player_id = u64::from_le_bytes(key.public().as_bytes()[..8].try_into().unwrap())
            | ssk_sim::PLAYER_ID_BIT;
        let (state, state_rx) = tokio::sync::watch::channel(Vec::new());
        let (events, events_rx) = tokio::sync::mpsc::channel(EVENT_CHANNEL);
        let (shared, shared_rx) = tokio::sync::watch::channel(Vec::new());
        let (contacts, contacts_rx) = tokio::sync::watch::channel(Vec::new());
        let (incoming_tx, incoming) = tokio::sync::mpsc::channel(INCOMING_CHANNEL);
        let snapshots: SnapshotInbox = Default::default();
        let (status_tx, status) = tokio::sync::watch::channel(Status {
            phase: "Starting network".into(),
            ..Default::default()
        });
        let (stop, stop_rx) = tokio::sync::watch::channel(false);
        let worker = Worker {
            state: state_rx,
            events: events_rx,
            shared: shared_rx,
            contacts: contacts_rx,
            incoming: incoming_tx,
            snapshots: snapshots.clone(),
            status: status_tx,
            stop: stop_rx,
        };
        let error_status = worker.status.clone();
        let worker_span = tracing::debug_span!("ssk.net.worker");
        let thread = std::thread::Builder::new()
            .name("ssk-net".into())
            .spawn(move || {
                let _worker=worker_span.enter();
                tracing::debug!(name:"ssk.net.worker.begin",progress_id="ssk.net.transport.sample",progress_active=true,
                    threads=NET_WORKERS,"Network worker entered; constructing Tokio runtime");
                let result = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(NET_WORKERS)
                    .thread_name("ssk-io")
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|rt| rt.block_on(transport::run(config, key, connect, worker)));
                if let Err(error) = result {
                    error_status.send_modify(|s| s.phase = format!("Network stopped: {error:#}"));
                    tracing::error!(name:"ssk.net.worker.failed",error=%format!("{error:#}"),"Network worker failed");
                }
                tracing::debug!(name:"ssk.net.worker.end",progress_id="ssk.net.transport.sample",progress_active=false,"Network worker stopped");
            })?;
        Ok(Self {
            player_id,
            state,
            events,
            shared,
            contacts,
            incoming,
            status,
            snapshots,
            stop,
            thread: Some(thread),
        })
    }
    pub fn publish(&self, state: Vec<u8>) {
        self.state.send_replace(state);
    }
    /// At most one fresh state per origin; old UI backlog is never replayed.
    pub fn take_snapshots(&self) -> Vec<(String, Vec<u8>, std::time::Instant)> {
        self.snapshots
            .lock()
            .unwrap()
            .drain()
            .map(|(peer, (bytes, time))| (peer, bytes, time))
            .collect()
    }
    pub fn shared(&self, frames: Vec<Vec<u8>>) {
        self.shared.send_replace(frames);
    }
    /// Introduce a bounded signed roster; records still pass normal authentication.
    pub fn learn_contacts(&self, records: Vec<Vec<u8>>) {
        self.contacts.send_replace(
            records
                .into_iter()
                .filter(|r| r.len() <= MAX_SIGNED_BYTES)
                .take(MAX_PEERS)
                .collect(),
        );
    }
    pub fn event(&self, event: Vec<u8>) -> Result<(), Vec<u8>> {
        self.events.try_send(event).map_err(|e| e.into_inner())
    }
    pub fn request_stop(&self) {
        let _ = self.stop.send(true);
    }
}
impl Drop for Network {
    fn drop(&mut self) {
        let _joining = tracing::debug_span!("ssk.net.worker.join", watchdog = true).entered();
        let _ = self.stop.send(true);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn run_node(config: Config) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(NET_WORKERS)
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let mut relay = match &config.relay_server {
            Some(c) => Some(relay::start(c).await?),
            None => None,
        };
        let mut network = Network::start(config, None)?;
        let mut status = network.status.clone();
        loop {
            tokio::select! {
                r = tokio::signal::ctrl_c() => { r?; break; },
                r = async {
                    match &mut relay {
                        Some(server) => server.join().await,
                        None => std::future::pending().await,
                    }
                } => anyhow::bail!("Relay terminated: {r:?}"),
                changed = status.changed() => {
                    changed?;
                    let phase = status.borrow().phase.clone();
                    anyhow::ensure!(!phase.starts_with("Network stopped:"), "{phase}");
                },
                event = network.incoming.recv() => {
                    if let Some(NetEvent::Error(error)) = event { tracing::warn!(name:"ssk.net.server.event_failed",%error,"Headless node network error"); }
                },
            }
        }
        drop(network);
        if let Some(server) = relay {
            server.shutdown().await?;
        }
        Ok(())
    })
}
