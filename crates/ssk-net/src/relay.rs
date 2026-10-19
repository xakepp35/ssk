use crate::RelayServerConfig;
use anyhow::Context;
use iroh_relay::server::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct Admission {
    active: AtomicUsize,
    max: usize,
    allow: Vec<iroh::PublicKey>,
}
impl AccessControl for Admission {
    async fn on_connect(&self, request: &ClientRequest) -> Access {
        if (!self.allow.is_empty() && !self.allow.contains(&request.endpoint_id()))
            || self
                .active
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    (n < self.max).then_some(n + 1)
                })
                .is_err()
        {
            Access::Deny {
                reason: Some("SSK relay admission limit".into()),
            }
        } else {
            Access::Allow
        }
    }
    fn on_disconnect(&self, _: iroh::PublicKey, _: ConnectionId) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }
}
pub(crate) async fn start(config: &RelayServerConfig) -> anyhow::Result<Server> {
    let mut relay = RelayConfig::new(config.http_bind);
    relay.key_cache_capacity = Some(4096);
    relay.limits.client_rx = Some(ClientRateLimit::new(
        std::num::NonZeroU32::new(config.bytes_per_second).context("zero relay rate")?,
    ));
    relay.access = Arc::new(Admission {
        active: AtomicUsize::new(0),
        max: config.max_clients,
        allow: config
            .allow
            .iter()
            .map(|s| s.parse())
            .collect::<Result<_, _>>()?,
    });
    if let Some(bind) = config.https_bind {
        let mut certs = std::io::BufReader::new(std::fs::File::open(
            config.cert_file.as_ref().context("relay certificate")?,
        )?);
        let certs = rustls_pemfile::certs(&mut certs).collect::<Result<Vec<_>, _>>()?;
        let mut key = std::io::BufReader::new(std::fs::File::open(
            config.key_file.as_ref().context("relay key")?,
        )?);
        let key = rustls_pemfile::private_key(&mut key)?.context("no PEM private key")?;
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
        relay.tls = Some(TlsConfig::new(
            bind,
            CertConfig::Manual { server_config: tls },
        ));
    }
    let mut server = ServerConfig::default();
    server.relay = Some(relay);
    server.quic = config.quic_bind.map(QuicConfig::new);
    let server = Server::spawn(server).await?;
    eprintln!(
        "SSK relay HTTP: {:?}; HTTPS: {:?}; QUIC: {:?}",
        server.http_addr(),
        server.https_addr(),
        server.quic_addr()
    );
    Ok(server)
}
