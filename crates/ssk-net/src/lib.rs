//! Bounded bridge between game thread and asynchronous network workers.
mod auth;
mod metrics;
mod overlay;
mod routes;
pub mod topology;
pub use metrics::LinkStats;
mod config;
mod contact;
mod discovery;
mod validation;
pub use validation::*;
mod events;
pub use events::*;
mod reliable;
mod replication;
pub use replication::*;
mod transport;
pub use config::*;
pub use transport::{decode_ticket, encode_ticket};
pub const PROTOCOL_VERSION: u16 = 1;
pub const RULES_ID: &str = "ssk-rules-1-240hz";
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub phase: String,
    pub ticket: String,
    pub peers: usize,
    pub dropped: u64,
    pub links: Vec<LinkStats>,
    pub routes: usize,
    pub route_changes: u64,
    pub unconfirmed: u64,
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
    pub incoming: tokio::sync::mpsc::Receiver<NetEvent>,
    pub status: tokio::sync::watch::Receiver<Status>,
    stop: tokio::sync::watch::Sender<bool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
pub(crate) struct Worker {
    state: tokio::sync::watch::Receiver<Vec<u8>>,
    events: tokio::sync::mpsc::Receiver<Vec<u8>>,
    incoming: tokio::sync::mpsc::Sender<NetEvent>,
    status: tokio::sync::watch::Sender<Status>,
    stop: tokio::sync::watch::Receiver<bool>,
}
impl Network {
    pub fn start(config: Config, connect: Option<String>) -> anyhow::Result<Self> {
        let key = load_identity(&config.identity_file)?;
        let player_id =
            u64::from_le_bytes(key.public().as_bytes()[..8].try_into().unwrap()) | 1 << 63;
        let (state, state_rx) = tokio::sync::watch::channel(Vec::new());
        let (events, events_rx) = tokio::sync::mpsc::channel(128);
        let (incoming_tx, incoming) = tokio::sync::mpsc::channel(512);
        let (status_tx, status) = tokio::sync::watch::channel(Status {
            phase: "Starting network".into(),
            ..Default::default()
        });
        let (stop, stop_rx) = tokio::sync::watch::channel(false);
        let worker = Worker {
            state: state_rx,
            events: events_rx,
            incoming: incoming_tx,
            status: status_tx,
            stop: stop_rx,
        };
        let error_status = worker.status.clone();
        let thread = std::thread::Builder::new()
            .name("ssk-net".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|rt| rt.block_on(transport::run(config, key, connect, worker)));
                if let Err(error) = result {
                    error_status.send_modify(|s| s.phase = format!("Network stopped: {error:#}"));
                    eprintln!("Network: {error:#}");
                }
            })?;
        Ok(Self {
            player_id,
            state,
            events,
            incoming,
            status,
            stop,
            thread: Some(thread),
        })
    }
    pub fn publish(&self, state: Vec<u8>) {
        self.state.send_replace(state);
    }
    pub fn event(&self, event: Vec<u8>) -> Result<(), Vec<u8>> {
        self.events.try_send(event).map_err(|e| e.into_inner())
    }
}
impl Drop for Network {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn run_node(config: Config) -> anyhow::Result<()> {
    let network = Network::start(config, None)?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(tokio::signal::ctrl_c())?;
    drop(network);
    Ok(())
}
