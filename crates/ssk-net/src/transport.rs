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
    let mut auth = crate::auth::Auth::new(key.clone(), &config);
    let discovery_id = crate::contact::discovery_id(&key.public())
        .unwrap()
        .to_string();
    let mut discovery =
        crate::discovery::Discovery::start(key.clone(), config.clone(), worker.stop.clone())?;
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
    let mut overlay = crate::overlay::Overlay::new(endpoint.id().to_string());
    overlay.radius = config.interest_radius;
    for ticket in config
        .seeds
        .iter()
        .filter(|s| s.starts_with("ssk1:"))
        .chain(config.friends.iter())
        .chain(connect.iter())
    {
        let addr = decode_ticket(ticket)?;
        let id = addr.id.to_string();
        if config.friends.contains(ticket) || connect.as_ref() == Some(ticket) {
            overlay.friends.insert(id.clone());
        }
        overlay.fallback.insert(id, addr);
    }
    if let Some(path) = &config.contact_cache {
        for record in crate::cache::read(path) {
            if let Some((origin, data)) = auth.verify(3, &record) {
                if let Ok(contact) = serde_json::from_slice::<crate::contact::Contact>(&data) {
                    if contact.addr.id.to_string() == origin
                        && contact.addr.addrs.len() <= 16
                        && contact.expires.saturating_add(86400) > crate::contact::unix_time()
                    {
                        overlay
                            .fallback
                            .entry(origin.clone())
                            .or_insert(contact.addr);
                    }
                }
                overlay.learn(origin, &data, &record);
            }
        }
    }
    let admission = std::sync::Arc::new(tokio::sync::Semaphore::new(16));
    let mut tick = tokio::time::interval(Duration::from_secs_f64(1. / config.net_hz as f64));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut seq = 0u64;
    let mut published = false;
    let mut signed_state = Vec::new();
    let mut metric_record = Vec::new();
    let mut control_budget = 8192f32;
    let mut control_cursor = 0usize;
    loop {
        tokio::select! {
            _=worker.stop.changed()=>break,
            Some(record)=discovery.found.recv()=>{if let Some((origin,data))=auth.verify(3,&record){overlay.learn(origin,&data,&record);}},
            incoming=endpoint.accept()=>if let Some(incoming)=incoming {
                if peers.len()>=config.max_peers {incoming.refuse();continue;}
                let Ok(permit)=admission.clone().try_acquire_owned() else {incoming.refuse();continue;};
                let tx=tx.clone();let cfg=config.clone();
                tokio::spawn(async move {let _permit=permit;match incoming.await {Ok(conn)=>admit(conn,cfg,tx).await,Err(error)=>{let _=tx.try_send(Internal::Failed(error.to_string()));}}});
            },
            Some(event)=rx.recv()=>match event {
                Internal::Ready(conn)=>{
                    let id=conn.remote_id().to_string();
                    overlay.connected(&id);
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
                    let frames=peers.get_mut(&id).map(|p|p.receive(&bytes)).unwrap_or_default();
                    for (kind,data) in frames{overlay.receive(&id,kind,&data,&mut peers,&mut auth,&worker,seq);}
                }
            },
            Some(event)=worker.events.recv()=>{
                let event=auth.sign(1,&event);
                overlay.queue(1,&event,&mut peers,seq);

            },
            _=tick.tick()=>{
                let snapshot=Snapshot::decode(&worker.state.borrow());
                let sector=snapshot.map(|s|s.pos.sector).unwrap_or([0,0]);
                let presence_changed=overlay.presence.get(&overlay.id).is_none_or(|c|c.sector!=sector||c.playing!=snapshot.is_some());
                if seq%(config.net_hz as u64*15)==0||presence_changed {
                    let mut addr=endpoint.addr();if let Some(ip)=config.advertise {addr.addrs.insert(iroh::TransportAddr::Ip(ip));}
                    let ticket=encode_ticket(&addr)?;
                    let contact=crate::contact::Contact{addr:addr.clone(),discovery:discovery_id.clone(),expires:crate::contact::unix_time()+60,sector,playing:snapshot.is_some()};
                    let bytes=serde_json::to_vec(&contact)?;let signed=auth.sign(3,&bytes);overlay.learn(overlay.id.clone(),&bytes,&signed);discovery.publish.send_replace((signed,sector));
                    if !published&&!addr.addrs.is_empty(){println!("SSK invite: {ticket}");published=true;}
                    worker.status.send_modify(|s|{s.ticket=ticket;if s.phase=="Starting network"{s.phase="Waiting for peer".into();}});
                }
                seq+=1;
                if let Some(snapshot)=snapshot {overlay.positions.insert(overlay.id.clone(),snapshot);}
                if worker.state.has_changed().unwrap_or(false) {let state=worker.state.borrow_and_update().clone();signed_state=auth.sign(0,&state);}
                if !signed_state.is_empty(){overlay.queue(0,&signed_state,&mut peers,seq);}
                if worker.shared.has_changed().unwrap_or(false){let frames=worker.shared.borrow_and_update().clone().into_iter().take(256).filter(|f| f.len()<=420).map(|f| auth.sign(6,&f)).collect::<Vec<_>>();overlay.shared(&frames,&mut peers);}
                overlay.retry(&mut peers,seq,config.net_hz);
                control_budget=(control_budget+8192./config.net_hz as f32).min(8192.);
                let mut ids:Vec<_>=peers.keys().cloned().collect();ids.sort();let count=ids.len();if count>0{ids.rotate_left(seq as usize%count);}
                for id in ids{let peer=peers.get_mut(&id).unwrap();
                    let (kind,contact)=if control_cursor%4==0{(5,metric_record.clone())}else{(3,overlay.records.values().nth((control_cursor/4)%overlay.records.len().max(1)).cloned().unwrap_or_default())};
                    let control=if !contact.is_empty()&&control_budget>=contact.len() as f32{control_budget-=contact.len() as f32;control_cursor+=1;contact}else{Vec::new()};
                    let budget=if config.net_hz==30{1024*1024}else{2*1024*1024};
                    let limit=1100.min(peer.conn.max_datagram_size().unwrap_or(0)).min(budget/config.net_hz as usize/count.max(1));
                    if limit<64{continue;}
                    let packet=peer.packet(seq,kind,&control,config.net_hz,limit);let len=packet.len();if peer.conn.send_datagram(packet.into()).is_ok(){peer.metrics.sent(len);}
                }
                if seq%config.net_hz as u64==0 {
                    for id in overlay.prune(){if let Some(p)=peers.remove(&id){p.conn.close(0u32.into(),b"presence expired");}let _=worker.incoming.try_send(NetEvent::Disconnected(id));}
                    let links=peers.iter_mut().map(|(id,p)|{p.metrics.stats.peer=id.clone();p.metrics.stats.direct=p.conn.paths().iter().any(|path|path.is_selected()&&path.is_ip());p.metrics.sample()}).collect();
                    worker.status.send_modify(|s|s.links=links);
                    let data=crate::routes::Routes::advertise(&peers,(seq/config.net_hz as u64*12) as usize);metric_record=auth.sign(5,&data);
                    worker.status.send_modify(|s|{s.routes=overlay.routes.chosen.len();s.route_changes=overlay.routes.changes;s.unconfirmed=overlay.routes.expired;});
                    if seq%(config.net_hz as u64*15)==0 {if let Some(path)=&config.contact_cache {
                        let records=overlay.records.iter().filter(|(id,_)| *id!=&overlay.id).map(|(_,r)|r.clone()).take(128).collect();
                        if let Err(e)=crate::cache::write(path,records){let _=worker.incoming.try_send(NetEvent::Error(format!("Contact cache: {e}")));}
                    }}
                    overlay.connecting.retain(|_,time|time.elapsed().as_secs()<15);
                    for (_id,addr) in overlay.connect_tasks(&peers,&endpoint){
                        let ep=endpoint.clone();let cfg=config.clone();let tx=tx.clone();
                        tokio::spawn(async move{match tokio::time::timeout(Duration::from_secs(12),ep.connect(addr,ALPN)).await{Ok(Ok(conn))=>admit(conn,cfg,tx).await,result=>{let _=tx.try_send(Internal::Failed(format!("Topology connect: {result:?}")));}}});
                    }
                    if overlay.settled(){let desired=overlay.desired();if desired.iter().all(|id|peers.contains_key(id)){for (id,p) in &peers{if !desired.contains(id){p.conn.close(0u32.into(),b"topology epoch settled");}}}}
                }

            }
        }
    }
    for conn in peers.values() {
        conn.conn.close(0u32.into(), b"leaving");
    }
    endpoint.close().await;
    Ok(())
}
