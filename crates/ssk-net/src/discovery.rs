use crate::*;
use futures::StreamExt;
use libp2p::{
    Multiaddr, StreamProtocol, identity,
    kad::{
        self,
        store::{MemoryStore, MemoryStoreConfig},
    },
    swarm::{NetworkBehaviour, SwarmEvent},
};
use std::time::{Duration, Instant};
#[derive(NetworkBehaviour)]
struct Behaviour {
    kad: kad::Behaviour<MemoryStore>,
    identify: libp2p::identify::Behaviour,
}
pub(crate) struct Discovery {
    pub publish: tokio::sync::watch::Sender<(Vec<u8>, [i64; 2])>,
    pub found: tokio::sync::mpsc::Receiver<Vec<u8>>,
}
impl Discovery {
    pub fn start(
        key: iroh::SecretKey,
        config: Config,
        mut stop: tokio::sync::watch::Receiver<bool>,
    ) -> anyhow::Result<Self> {
        let identity = identity::Keypair::ed25519_from_bytes(key.to_bytes())?;
        let peer = identity.public().to_peer_id();
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
            .with_tokio()
            .with_quic()
            .with_behaviour(move |key| {
                let store = MemoryStore::with_config(
                    peer,
                    MemoryStoreConfig {
                        max_records: 4096,
                        max_value_bytes: 1024,
                        max_provided_keys: 32,
                        max_providers_per_key: 128,
                    },
                );
                let mut cfg = kad::Config::new(StreamProtocol::new("/ssk/kad/1"));
                cfg.set_record_ttl(Some(Duration::from_secs(60)));
                cfg.set_provider_record_ttl(Some(Duration::from_secs(60)));
                let mut kad = kad::Behaviour::with_config(peer, store, cfg);
                kad.set_mode(Some(kad::Mode::Server));
                Behaviour {
                    kad,
                    identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                        "/ssk/discovery/1".into(),
                        key.public(),
                    )),
                }
            })?
            .build();
        let bind = config.discovery_bind;
        let ip = match bind.ip() {
            std::net::IpAddr::V4(ip) => format!("/ip4/{ip}"),
            std::net::IpAddr::V6(ip) => format!("/ip6/{ip}"),
        };
        swarm.listen_on(format!("{ip}/udp/{}/quic-v1", bind.port()).parse()?)?;
        for addr in &config.discovery_advertise {
            swarm.add_external_address(addr.parse()?);
        }
        for seed in config.seeds.iter().filter(|s| s.starts_with('/')) {
            let addr: Multiaddr = seed.parse()?;
            if let Some(libp2p::multiaddr::Protocol::P2p(id)) = addr.iter().last() {
                swarm.behaviour_mut().kad.add_address(&id, addr.clone());
                swarm.dial(addr)?;
            }
        }
        let (publish, mut rx) = tokio::sync::watch::channel((Vec::new(), [0, 0]));
        let (tx, found) = tokio::sync::mpsc::channel(256);
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(15));
            let mut previous = None;
            loop {
                tokio::select! {
                _=stop.changed()=>break,
                changed=rx.changed()=>{if changed.is_err(){break;}timer.reset_immediately();},
                    _=timer.tick()=>{
                        let (value,sector)=rx.borrow_and_update().clone();
                        if value.is_empty(){continue;}
                        let record=kad::Record{key:contact_key(&config.world,&peer),value,publisher:Some(peer),expires:Some(Instant::now()+Duration::from_secs(60))};
                        let _=swarm.behaviour_mut().kad.put_record(record,kad::Quorum::One);
                        if previous!=Some(sector){if let Some(old)=previous{swarm.behaviour_mut().kad.stop_providing(&sector_key(&config.world,old));}previous=Some(sector);}
                        let _=swarm.behaviour_mut().kad.start_providing(sector_key(&config.world,sector));
                        for x in -1..=1{for y in -1..=1{swarm.behaviour_mut().kad.get_providers(sector_key(&config.world,[sector[0].saturating_add(x),sector[1].saturating_add(y)]));}}
                        let _=swarm.behaviour_mut().kad.bootstrap();
                    },
                    event=swarm.select_next_some()=>match event{
                        SwarmEvent::NewListenAddr{address,..}=>println!("SSK discovery seed: {address}/p2p/{peer}"),
                        SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Received{peer_id,info,..}))=>{for addr in info.listen_addrs.into_iter().take(16){swarm.behaviour_mut().kad.add_address(&peer_id,addr);}},
                        SwarmEvent::Behaviour(BehaviourEvent::Kad(kad::Event::OutboundQueryProgressed{result,..}))=>match result{
                            kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders{providers,..}))=>for id in providers.into_iter().take(128){if id!=peer{swarm.behaviour_mut().kad.get_record(contact_key(&config.world,&id));}},
                            kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record)))=>{if record.record.value.len()<=1000{let _=tx.try_send(record.record.value);}},_=>{}
                        },_=>{}
                    }
                }
            }
        });
        Ok(Self { publish, found })
    }
}
fn contact_key(world: &str, id: &libp2p::PeerId) -> kad::RecordKey {
    kad::RecordKey::new(&format!("ssk/{world}/contact/{id}"))
}
fn sector_key(world: &str, sector: [i64; 2]) -> kad::RecordKey {
    kad::RecordKey::new(&format!("ssk/{world}/sector/{}/{}", sector[0], sector[1]))
}
