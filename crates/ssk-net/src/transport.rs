use crate::*;
use base64::Engine;
use iroh::{
    Endpoint, EndpointAddr, RelayMode,
    endpoint::{Connection, presets},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
const ALPN: &[u8] = b"ssk/game/1";
#[derive(Serialize, Deserialize)]
struct Hello {
    version: u16,
    rules: String,
    world: String,
    seed: u64,
}
pub fn encode_ticket(addr: &EndpointAddr) -> anyhow::Result<String> {
    Ok(format!(
        "ssk1:{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(addr)?)
    ))
}
pub fn decode_ticket(ticket: &str) -> anyhow::Result<EndpointAddr> {
    anyhow::ensure!(ticket.len() < 8192, "Ticket too large");
    let data = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(
        ticket
            .trim()
            .strip_prefix("ssk1:")
            .ok_or_else(|| anyhow::anyhow!("Expected ssk1: invite"))?,
    )?;
    Ok(serde_json::from_slice(&data)?)
}
async fn handshake(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    let hello = Hello {
        version: PROTOCOL_VERSION,
        rules: RULES_ID.into(),
        world: config.world.clone(),
        seed: config.seed,
    };
    let mut send = conn.open_uni().await?;
    send.write_all(&serde_json::to_vec(&hello)?).await?;
    send.finish()?;
    let mut recv = conn.accept_uni().await?;
    let remote: Hello = serde_json::from_slice(&recv.read_to_end(1024).await?)?;
    anyhow::ensure!(
        remote.version == hello.version
            && remote.rules == hello.rules
            && remote.world == hello.world
            && remote.seed == hello.seed,
        "Incompatible protocol, world, rules or seed"
    );
    Ok(())
}
enum Internal {
    Ready(Connection),
    Failed(String),
    Data(String, Vec<u8>),
    Gone(String),
}
async fn admit(conn: Connection, config: Config, tx: tokio::sync::mpsc::Sender<Internal>) {
    match tokio::time::timeout(Duration::from_secs(5), handshake(&conn, &config)).await {
        Ok(Ok(())) => {
            let _ = tx.send(Internal::Ready(conn)).await;
        }
        result => {
            conn.close(1u32.into(), b"incompatible handshake");
            let _ = tx.try_send(Internal::Failed(format!("Handshake: {result:?}")));
        }
    }
}
pub(crate) async fn run(
    config: Config,
    key: iroh::SecretKey,
    connect: Option<String>,
    mut worker: Worker,
) -> anyhow::Result<()> {
    let relay = if config.relays.is_empty() {
        RelayMode::Disabled
    } else {
        RelayMode::custom(
            config
                .relays
                .iter()
                .map(|s| s.parse())
                .collect::<Result<Vec<iroh::RelayUrl>, _>>()?,
        )
    };
    let endpoint = Endpoint::builder(presets::Minimal)
        .secret_key(key)
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(relay)
        .bind_addr(config.udp_bind)?
        .bind()
        .await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(512);
    let mut peers: HashMap<String, crate::reliable::Peer> = HashMap::new();
    let mut event_id = 0u64;
    if let Some(ticket) = connect {
        let addr = decode_ticket(&ticket)?;
        let ep = endpoint.clone();
        let tx = tx.clone();
        let cfg = config.clone();
        tokio::spawn(async move {
            match tokio::time::timeout(Duration::from_secs(12), ep.connect(addr, ALPN)).await {
                Ok(Ok(conn)) => admit(conn, cfg, tx).await,
                result => {
                    let _ = tx.try_send(Internal::Failed(format!("Connect: {result:?}")));
                }
            }
        });
    }
    let mut tick = tokio::time::interval(Duration::from_secs_f64(1. / config.net_hz as f64));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut seq = 0u64;
    let mut published = false;
    loop {
        tokio::select! {
            _=worker.stop.changed()=>break,
            incoming=endpoint.accept()=>if let Some(incoming)=incoming {
                if peers.len()>=config.max_peers {incoming.refuse();continue;}
                let tx=tx.clone();let cfg=config.clone();
                tokio::spawn(async move {match incoming.await {Ok(conn)=>admit(conn,cfg,tx).await,Err(error)=>{let _=tx.try_send(Internal::Failed(error.to_string()));}}});
            },
            Some(event)=rx.recv()=>match event {
                Internal::Ready(conn)=>{
                    let id=conn.remote_id().to_string();
                    if peers.contains_key(&id)||peers.len()>=config.max_peers {conn.close(0u32.into(),b"duplicate or full");continue;}
                    peers.insert(id.clone(),crate::reliable::Peer::new(conn.clone()));
                    let _=worker.incoming.try_send(NetEvent::Connected(id.clone()));
                    worker.status.send_modify(|s|{s.phase="Connected".into();s.peers=peers.len();});
                    let tx=tx.clone();tokio::spawn(async move {
                        while let Ok(bytes)=conn.read_datagram().await {if bytes.len()<=1100{let _=tx.try_send(Internal::Data(id.clone(),bytes.to_vec()));}}
                        let _=tx.send(Internal::Gone(id)).await;
                    });
                },
                Internal::Failed(error)=>{worker.status.send_modify(|s|s.phase=error.clone());let _=worker.incoming.try_send(NetEvent::Error(error));},
                Internal::Gone(id)=>{peers.remove(&id);let _=worker.incoming.try_send(NetEvent::Disconnected(id));worker.status.send_modify(|s|{s.peers=peers.len();s.phase=if peers.is_empty(){"Waiting for peer".into()}else{"Connected".into()};});},
                Internal::Data(id,bytes)=>{
                    if let Some(peer)=peers.get_mut(&id){peer.receive(&id,&bytes,&worker);}
                }
            },
            Some(event)=worker.events.recv()=>{
                event_id+=1;
                for peer in peers.values_mut(){if !peer.queue(event_id,event.clone(),seq){let _=worker.incoming.try_send(NetEvent::Error("Critical event queue full".into()));}}

            },
            _=tick.tick()=>{
                if seq%150==0 {
                    let mut addr=endpoint.addr();if let Some(ip)=config.advertise {addr.addrs.insert(iroh::TransportAddr::Ip(ip));}
                    let ticket=encode_ticket(&addr)?;
                    if !published&&!addr.addrs.is_empty(){println!("SSK invite: {ticket}");published=true;}
                    worker.status.send_modify(|s|{s.ticket=ticket;if s.phase=="Starting network"{s.phase="Waiting for peer".into();}});
                }
                seq+=1;let state=worker.state.borrow_and_update().clone();
                for peer in peers.values_mut(){let packet=peer.packet(seq,&state,config.net_hz);if packet.len()<=peer.conn.max_datagram_size().unwrap_or(0){let _=peer.conn.send_datagram(packet.into());}}

            }
        }
    }
    for conn in peers.values() {
        conn.conn.close(0u32.into(), b"leaving");
    }
    endpoint.close().await;
    Ok(())
}
