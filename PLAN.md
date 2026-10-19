# SSK: implementation plan

README.md contains the agreed game/network specification. Each checked step
must name actual evidence in CHECKPOINT.md. One complete step, one commit;
split oversized steps. Never treat compilation as proof of playability.

## A — offline alpha

- [x] A0 — Write README, PLAN, AGENTS and checkpoint. `docs: define game and delivery plan`
- [x] A1 — Rust workspace, CLI, window, headless dispatch. `build: create Rust workspace`
- [x] A2 — 240 Hz movement, aim, camera. `feat: add ship movement and aiming`
- [x] A3 — Machine gun, energy, destructible target. `feat: add first playable combat`
- [x] A4 — Sectors, spatial grid, asteroids, swept collisions. `feat: add procedural asteroid field`
- [x] A5 — Shotgun, plasma, persistent weapon cooldowns. `feat: add three weapon loadout`
- [x] A6 — NPC, death, score, instant restart; first complete run. `feat: add survival combat loop`
- [x] A7 — Three classes and one ability each. `feat: add ship classes and abilities`
- [x] A8 — Pickups, overcap resources, combos. `feat: add pickups and score chains`
- [x] A9 — Three AI types, director, spawn telegraphs. `feat: add varied enemies and pressure`
- [x] A10 — Bounded effects and prepared audio. `feat: add combat effects and audio`
- [x] A11 — Menu, HUD, settings, per-class records, help. `feat: complete local game flow`
- [x] A12 — Remove gameplay blockers, launch docs, alpha tag. `release: finish playable alpha`

Alpha acceptance: offline run with any class, complete survival/death/result/
restart flow, no server requirement. Before alpha: focused compilation only;
no automated tests unless requested. A short manual game session when available.

## B — duel MVP and decentralized beta

- [x] B1 — Identity, configuration, Tokio, bounded queues. `feat: add network foundations`
- [x] B2 — Two peers connect by ticket over QUIC/UDP. `feat: connect two peers`
- [x] B3 — Quantized independent snapshots, sequences, interpolation. `feat: add compressed ship replication`
- [x] B4 — Shots, damage, death, respawn; playable duel MVP. `feat: make peer duel playable`
- [x] B5 — Origin signatures, history, bounded combat validation. `feat: validate peer combat events`
- [x] B6 — Piggyback RTT, loss, load, diagnostics. `feat: measure live peer links`
- [x] B7 — Row/column/diagonal graph, at most one transit. `feat: add bounded-hop peer routing`
- [x] B8 — Prepared backups, dedup, capacity, failover. `feat: balance and recover peer routes`
- [x] B9 — Kademlia, bootstrap, signed contacts, node mode. `feat: add decentralized discovery`
- [x] B10 — Interest regions, prefetch, seamless sector transitions. `feat: add seamless sector discovery`
- [x] B11 — Shared NPCs/asteroids/loot, single pickup confirmation. `feat: synchronize shared sector objects`
- [x] B12 — Leases, snapshots, epochs, owner recovery. `feat: recover sector ownership`
- [x] B13 — Embedded self-hosted relay and NAT traversal. `feat: add self-hosted relay support`
- [x] B14 — Multiple seeds/relays, cache, reconnect, friends. `feat: handle network interruptions`
- [ ] B15 — Headless swarm, measurements, isolated NAT/relay setup. `feat: add network load scenarios`
- [ ] B16 — Measured bottlenecks, operating docs, beta tag. `release: prepare network beta`

## Network implementation invariants

- One intermediary means A→R→B, not an unbounded gossip path.
- Balanced diameter-two graph needs degree Ω(sqrt(N)); O(N log N) links do
  not give this guarantee with bounded maximum degree.
- Dense all-to-all useful state deliveries remain Ω(N²). Report connections,
  packet count, payload volume and control overhead separately.
- q=ceil(sqrt(N)); virtual slots i→i mod N; connect equal row, column or
  (row+column) mod q. At most two virtual slots per physical peer.
- N=64: degree21, edges672. N=128: degree31..53, mean34.984375, edges2239,
  minimum five common intermediaries for nonadjacent pairs (calculated).
- Keep structural cover; optional direct combat edges capped/admitted.
- Ping via sequence echo/ack delay; no waiting for downstream pings. Unknown
  direct RTT stays unknown; route estimates are separate.
- Metrics piggyback bounded deltas, no full graph circulated at 60 Hz.
- 30/60 snapshot Hz on 240 Hz sim; 30–60 game datagrams/s per link including
  forwarding, max min(1100, transport limit). Account wire control separately.
- Source batch signatures survive relays. Fresh state drops old queued state.
- Critical events use IDs, end-to-end ACK and prepared independent routes.
- Relaying through a peer requires two direct legs; don't hide nested NAT relays.

## Deferred acceptance scenarios

After alpha, keep checks narrow: button OR semantics; weapon switch cooldown;
swept hit; duplicate damage/death/pickup; loss/reorder; fail 1/2 transit peers;
steward exit; forced relay with direct blocked; sector transition in combat.
Do not add/run automated tests without a request.

For beta claims, measure 2→16→64→128 moving/firing participants. Record host,
build, transport paths, network conditions, sim/frame percentiles, queues,
PPS/BPS, relay delay and crypto cost. A loopback run is not internet acceptance.
Public hosts are currently unavailable; real internet acceptance remains open.
