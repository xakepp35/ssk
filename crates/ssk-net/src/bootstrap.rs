//! Small LAN rendezvous: obtain the current QUIC invitation using an IP address.
//! Game data remains authenticated QUIC; TCP is used only once to discover it.
use crate::{Status, constants::*, decode_ticket};
use anyhow::Context;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::watch,
};

pub(crate) async fn resolve(connect: &str) -> anyhow::Result<String> {
    if connect.starts_with("ssk1:") {
        decode_ticket(connect)?;
        return Ok(connect.into());
    }
    // An explicit numeric LAN address avoids URL/HTTP parsing or unbounded replies.
    let address: SocketAddr = connect
        .parse()
        .context("Expected an ssk1: invitation or IP:TCP-port")?;
    tokio::time::timeout(Duration::from_secs(CONNECT_SECONDS), async {
        let socket = TcpStream::connect(address).await?;
        let mut bytes = Vec::new();
        socket
            .take(MAX_TICKET_BYTES as u64)
            .read_to_end(&mut bytes)
            .await?;
        let ticket = String::from_utf8(bytes)?.trim().to_owned();
        let mut endpoint = decode_ticket(&ticket)?;
        // LAN callers already know a reachable interface on this host.
        endpoint.addrs.retain(|a| {
            !matches!(a, iroh::TransportAddr::Ip(ip)
            if ip.ip().is_unspecified() || (ip.ip().is_loopback() && !address.ip().is_loopback()))
        });
        anyhow::ensure!(
            !endpoint.addrs.is_empty(),
            "LAN server has no reachable game address yet"
        );
        crate::encode_ticket(&endpoint)
    })
    .await
    .context("LAN server invitation timed out")?
    .with_context(|| format!("Connect to LAN server {address}; start the host first"))
}

pub(crate) async fn serve(
    bind: SocketAddr,
    status: watch::Receiver<Status>,
    mut stop: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("Bind LAN server {bind}"))?;
    println!("SSK LAN server: TCP {bind} (public invitation), game uses UDP/QUIC");
    let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_HANDSHAKES));
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                accepted = listener.accept() => {
                    let Ok((mut socket, _)) = accepted else { break; };
                    let Ok(permit) = slots.clone().try_acquire_owned() else { continue; };
                    let mut current = status.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        let _ = tokio::time::timeout(Duration::from_secs(HANDSHAKE_SECONDS), async {
                            loop {
                                let ticket = current.borrow().ticket.clone();
                                if decode_ticket(&ticket).is_ok_and(|addr| !addr.addrs.is_empty()) {
                                    socket.write_all(ticket.as_bytes()).await?;
                                    socket.shutdown().await?;
                                    return Ok::<_, std::io::Error>(());
                                }
                                if current.changed().await.is_err() { return Ok(()); }
                            }
                        }).await;
                    });
                }
            }
        }
    });
    Ok(())
}
