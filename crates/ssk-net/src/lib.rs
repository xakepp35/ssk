//! Bounded bridge between game thread and asynchronous network workers.
mod config;
pub use config::*;
pub const PROTOCOL_VERSION: u16 = 1;
pub const RULES_ID: &str = "ssk-rules-1-240hz";
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub phase: String,
    pub ticket: String,
    pub peers: usize,
    pub dropped: u64,
}
#[derive(Debug)]
pub enum NetEvent {
    Connected(String),
    Disconnected(String),
    Packet { peer: String, bytes: Vec<u8> },
    Error(String),
}
pub struct Network {
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
        let thread = std::thread::Builder::new()
            .name("ssk-net".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|rt| rt.block_on(run(config, key, connect, worker)));
                if let Err(error) = result {
                    eprintln!("Network: {error:#}");
                }
            })?;
        Ok(Self {
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
async fn run(
    _config: Config,
    _key: iroh::SecretKey,
    _connect: Option<String>,
    mut worker: Worker,
) -> anyhow::Result<()> {
    worker
        .status
        .send_modify(|s| s.phase = "Network foundations ready; transport pending".into());
    let _ = (&worker.state, &worker.events, &worker.incoming);
    worker.stop.changed().await?;
    Ok(())
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
