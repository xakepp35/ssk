use crate::*;
use base64::Engine;
use iroh::{
    Endpoint, EndpointAddr, RelayMode,
    endpoint::{Connection, presets},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
#[derive(Serialize, Deserialize)]
struct Hello {
    version: u16,
    tick_per_sec: u64,
    tick_per_packet: u64,
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
    anyhow::ensure!(ticket.len() < MAX_TICKET_BYTES, "Ticket too large");
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
        tick_per_sec: ssk_sim::TICK_PER_SEC,
        tick_per_packet: ssk_sim::TICK_PER_PACKET,
        rules: RULES_ID.into(),
        world: config.world.clone(),
        seed: config.seed,
    };
    let mut send = conn.open_uni().await?;
    send.write_all(&serde_json::to_vec(&hello)?).await?;
    send.finish()?;
    let mut recv = conn.accept_uni().await?;
    let remote: Hello = serde_json::from_slice(&recv.read_to_end(MAX_SIGNED_BYTES).await?)?;
    anyhow::ensure!(
        remote.version == hello.version
            && remote.tick_per_sec == hello.tick_per_sec
            && remote.tick_per_packet == hello.tick_per_packet
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
    match tokio::time::timeout(
        Duration::from_secs(HANDSHAKE_SECONDS),
        handshake(&conn, &config),
    )
    .await
    {
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
    let (tx, mut rx) = tokio::sync::mpsc::channel(INCOMING_CHANNEL);
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
            if let Some((origin, data)) = auth.verify(message::CONTACT, &record) {
                if let Ok(contact) = serde_json::from_slice::<crate::contact::Contact>(&data) {
                    if contact.addr.id.to_string() == origin
                        && contact.addr.addrs.len() <= MAX_ENDPOINT_ADDRS
                        && contact.expires.saturating_add(CACHE_FALLBACK_SECONDS)
                            > crate::contact::unix_time()
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
    let admission = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_HANDSHAKES));
    let mut tick = tokio::time::interval(ssk_sim::PACKET_DURATION);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_slot = std::time::Instant::now();
    let mut seq = 0u64;
    let mut published = false;
    let mut last_addr = None;
    let mut signed_state = Vec::new();
    let mut leases: HashMap<[i64; 2], (Vec<u8>, std::time::Instant)> = HashMap::new();
    let mut metric_record = Vec::new();
    let mut control_budget = CONTROL_BYTES_PER_SEC;
    let mut control_cursor = 0usize;
    let payload_rate = PAYLOAD_BYTES_PER_SEC;
    let mut payload_budget = payload_rate as f32 * PAYLOAD_BURST_SECONDS;
    loop {
        tokio::select! {
            _=worker.stop.changed()=>break,
            changed=worker.contacts.changed()=>{if changed.is_ok(){let records=worker.contacts.borrow_and_update().clone();for record in records{if let Some((origin,data))=auth.verify(message::CONTACT,&record){overlay.learn(origin,&data,&record);}}}},
            Some(record)=discovery.found.recv()=>{if let Some((origin,data))=auth.verify(message::CONTACT,&record){overlay.learn(origin,&data,&record);}},
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
                        while let Ok(bytes)=conn.read_datagram().await {if bytes.len()<=MAX_DATAGRAM_BYTES{let _=tx.try_send(Internal::Data(id.clone(),bytes.to_vec()));}}
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
                match CombatEvent::decode(&event) {
                    Some(CombatEvent::Shared(SharedEvent::Lease{sector,..}))=>{
                        if leases.len()<SHARED_LEASE_SECTORS||leases.contains_key(&sector){leases.insert(sector,(auth.sign(message::LEASE,&event),std::time::Instant::now()));}
                    },
                    decoded=>{
                        if let Some(CombatEvent::Shared(SharedEvent::Release{sector,..}))=decoded {leases.remove(&sector);}
                        let event=auth.sign(message::CRITICAL,&event);
                        overlay.queue(message::CRITICAL,&event,&mut peers,seq);
                    }
                }

            },
            _=tick.tick()=>{
                anyhow::ensure!(last_slot.elapsed() <= ssk_sim::MAX_LOGIC_LAG,
                    "Network cannot sustain {} Hz; session disconnected", ssk_sim::PACKET_PER_SEC);
                last_slot = std::time::Instant::now();
                let snapshot=Snapshot::decode(&worker.state.borrow());
                let sector=snapshot.map(|s|s.pos.sector).unwrap_or([0,0]);
                let presence_changed=overlay.presence.get(&overlay.id).is_none_or(|c|c.sector!=sector||c.playing!=snapshot.is_some());
                let mut addr=endpoint.addr();if let Some(ip)=config.advertise {addr.addrs.insert(iroh::TransportAddr::Ip(ip));}
                if seq%(ssk_sim::PACKET_PER_SEC * CONTACT_RENEW_SECONDS)==0||presence_changed||last_addr.as_ref()!=Some(&addr) {
                    last_addr=Some(addr.clone());
                    let ticket=encode_ticket(&addr)?;
                    let contact=crate::contact::Contact{addr:addr.clone(),discovery:discovery_id.clone(),expires:crate::contact::unix_time()+CONTACT_TTL_SECONDS,sector,playing:snapshot.is_some()};
                    let bytes=serde_json::to_vec(&contact)?;let signed=auth.sign(message::CONTACT,&bytes);overlay.learn(overlay.id.clone(),&bytes,&signed);worker.status.send_modify(|s|s.contact=signed.clone());discovery.publish.send_replace((signed,sector));
                    if !published&&!addr.addrs.is_empty(){println!("SSK invite: {ticket}");published=true;}
                    worker.status.send_modify(|s|{s.ticket=ticket;if s.phase=="Starting network"{s.phase="Waiting for peer".into();}});
                }
                seq+=1;
                if let Some(snapshot)=snapshot {overlay.positions.insert(overlay.id.clone(),snapshot);}
                if worker.state.has_changed().unwrap_or(false) {let state=worker.state.borrow_and_update().clone();signed_state=auth.sign(message::STATE,&state);}
                if !signed_state.is_empty(){overlay.queue(message::STATE,&signed_state,&mut peers,seq);}
                leases.retain(|_,(_,time)|time.elapsed().as_secs_f64()<ssk_sim::LEASE_SECONDS);
                if seq%(ssk_sim::PACKET_PER_SEC/LEASE_REPEAT_HZ).max(1) as u64==0 {for (lease,_) in leases.values(){overlay.queue(message::LEASE,lease,&mut peers,seq);}}
                if worker.shared.has_changed().unwrap_or(false){let frames=worker.shared.borrow_and_update().clone().into_iter().take(PEER_STATE_QUEUE).filter(|f| f.len()<=SHARED_FRAME_BYTES).map(|f| auth.sign(message::SECTOR,&f)).collect::<Vec<_>>();overlay.shared(&frames,&mut peers);}
                overlay.retry(&mut peers,seq);
                control_budget=(control_budget+CONTROL_BYTES_PER_SEC/ssk_sim::PACKET_PER_SEC as f32).min(CONTROL_BYTES_PER_SEC);
                payload_budget=(payload_budget+payload_rate as f32/ssk_sim::PACKET_PER_SEC as f32).min(payload_rate as f32 * PAYLOAD_BURST_SECONDS);
                let mut ids:Vec<_>=peers.keys().cloned().collect();ids.sort();let count=ids.len();if count>0{ids.rotate_left(seq as usize%count);}
                for (index,id) in ids.into_iter().enumerate(){let peer=peers.get_mut(&id).unwrap();
                    let phase=id.bytes().fold(0u64,|h,b| h.wrapping_mul(PHASE_HASH_MULTIPLIER).wrapping_add(b as u64))%ssk_sim::PACKET_PER_SEC as u64;
                    let slot=(seq+phase)%ssk_sim::PACKET_PER_SEC as u64;
                    let (kind,contact)=if slot==0{(message::METRICS,metric_record.clone())}else if slot==ssk_sim::PACKET_PER_SEC as u64/2{(message::CONTACT,overlay.records.values().nth(control_cursor%overlay.records.len().max(1)).cloned().unwrap_or_default())}else{(message::CONTACT,Vec::new())};
                    // Reserve a heartbeat for every remaining connection in this slot.
                    let reserve=(count-index-1)*MIN_DATAGRAM_BYTES;
                    let available=(payload_budget as usize).saturating_sub(reserve);
                    let limit=MAX_DATAGRAM_BYTES.min(peer.conn.max_datagram_size().unwrap_or(0)).min(available);
                    if limit<MIN_DATAGRAM_BYTES{continue;}
                    let control=if !contact.is_empty()&&control_budget>=contact.len() as f32&&contact.len()+METRIC_HEADER_BYTES+FRAME_HEADER_BYTES<=limit{control_budget-=contact.len() as f32;control_cursor+=usize::from(kind==message::CONTACT);contact}else{Vec::new()};
                    let packet=peer.packet(seq,kind,&control,limit);let len=packet.len();match peer.conn.send_datagram(packet.into()) {
                        Ok(())=>{payload_budget-=len as f32;peer.metrics.sent(len);},
                        Err(error)=>{peer.conn.close(1u32.into(),b"datagram send failed");let _=worker.incoming.try_send(NetEvent::Error(format!("Peer disconnected: {error}")));}
                    }
                }
                if seq%ssk_sim::PACKET_PER_SEC as u64==0 {
                    for id in overlay.prune(){if let Some(p)=peers.remove(&id){p.conn.close(0u32.into(),b"presence expired");}let _=worker.incoming.try_send(NetEvent::Disconnected(id));}
                    let links=peers.iter_mut().map(|(id,p)|{p.metrics.stats.peer=id.clone();p.metrics.stats.direct=p.conn.paths().iter().any(|path|path.is_selected()&&path.is_ip());p.metrics.sample()}).collect();
                    worker.status.send_modify(|s|{s.links=links;s.crypto=auth.stats.clone();});
                    let data=crate::routes::Routes::advertise(&peers,(seq/ssk_sim::PACKET_PER_SEC as u64 * METRIC_ROWS as u64) as usize);metric_record=auth.sign(message::METRICS,&data);
                    worker.status.send_modify(|s|{s.routes=overlay.routes.chosen.len();s.route_changes=overlay.routes.changes;s.unconfirmed=overlay.routes.expired;});
                    if seq%(ssk_sim::PACKET_PER_SEC * CONTACT_RENEW_SECONDS)==0 {if let Some(path)=&config.contact_cache {
                        let records=overlay.records.iter().filter(|(id,_)| *id!=&overlay.id).map(|(_,r)|r.clone()).take(MAX_PEERS).collect();
                        if let Err(e)=crate::cache::write(path,records){let _=worker.incoming.try_send(NetEvent::Error(format!("Contact cache: {e}")));}
                    }}
                    overlay.connecting.retain(|_,time|time.elapsed().as_secs()<CONNECTING_SECONDS);
                    for (_id,addr) in overlay.connect_tasks(&peers,&endpoint){
                        let ep=endpoint.clone();let cfg=config.clone();let tx=tx.clone();
                        tokio::spawn(async move{match tokio::time::timeout(Duration::from_secs(CONNECT_SECONDS),ep.connect(addr,ALPN)).await{Ok(Ok(conn))=>admit(conn,cfg,tx).await,result=>{let _=tx.try_send(Internal::Failed(format!("Topology connect: {result:?}")));}}});
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
