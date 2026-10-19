# Isolated forced-relay scenario

Run from the repository root after `cargo build --release -p ssk`:

```sh
docker compose -f tools/nat/compose.yaml up -d --build
docker compose -f tools/nat/compose.yaml wait a b
docker compose -f tools/nat/compose.yaml cp a:/run/ssk/a.json /tmp/ssk-nat-a.json
docker compose -f tools/nat/compose.yaml cp b:/run/ssk/b.json /tmp/ssk-nat-b.json
docker compose -f tools/nat/compose.yaml down -v
```

Use a fresh volume for each run; it contains disposable identities and an invite.
Wait for both clients: stopping the stack when the first exits can interrupt the
other before it writes its report. The relay stays up until `down -v`.
If the current agent shell predates docker group membership, prefix commands with
`sg docker -c 'docker ...'`. No host firewall or root session is used.

The relay joins two internal Docker bridges. Each client joins only its own
bridge; its container firewall blocks all non-loopback UDP except DNS. Both
clients reach the same `http://relay:3340` relay via TCP. This forces Iroh away
from direct UDP; the end-to-end game connection remains QUIC encrypted.
Inspect `relayed_link_samples`, `fresh_peer_fraction` and connection errors in
both reports. Zero peer snapshots means no demonstrated connectivity regardless
of process exit status. TCP retransmission delay is part of this scenario.

This is an isolated direct-UDP-blocked fallback scenario, not a model of every
symmetric NAT or an internet latency measurement. Two real public networks,
certificate deployment and relay failover still need separate acceptance.
