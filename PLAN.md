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
- [x] B15 — Headless swarm, measurements, isolated NAT/relay setup. `feat: add network load scenarios`
- [x] B16a — Fix measured signature/graph recomputation and queue starvation; focused compilation. `perf: bound routing and prioritize fresh game state`
- [x] B16b — Separate end-to-end receipts and index in-flight ACKs; compilation passed. `perf: keep delivery receipts out of retry queues`
- [x] B16c — Source multicast, cached verifying keys, parallel swarm and measured counter deltas; compilation passed. `perf: batch source fanout and parallelize swarm clients`
- [x] B16d — Coalesce incoming ship state; refresh leases as expiring latest state. `perf: bound incoming state and refresh sector leases`
- [x] B16e — Record 2/16/64/128 loopback and blocked-UDP relay measurements at 240 Hz. `docs: record measured network operating limits`
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
- 60 snapshot Hz on 360 Hz sim; 60 game datagrams/s per link including
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

## C — owner's timing and architecture revision (supersedes old rates)

- [x] C1a — One timing source: TICK_PER_SEC=360, TICK_PER_PACKET=6; fixed 60 Hz transport and compatible handshake.
- [x] C1b — Finish named simulation/network/client tuning parameters.
- [x] C1c — Name remaining queue bounds, retry/freshness and snapshot quantization parameters.
- [x] C2 — Move CombatGuard, replicated game model/history and shared game rules
  into ssk-sim. Keep authentication/framing/discovery/routing in ssk-net.
- [x] C2b — Pure PeerCombat owns damage, history, frags, death and respawn; Online only adapts authenticated transport.
- [x] C3 — Independent fixed-step game worker, bounded render handoff and explicit
  overload disconnect. No hidden slow-motion online, no unbounded catch-up.
- [x] C3b — Reserve all peer slots within the payload budget; stop/disconnect an overloaded transport worker.
- [x] C4 — Windows portability/build scripts, Linux release build, LAN duel guide.

Avoid further heavy swarm runs on this workstation after a reported OS freeze.
Old 240 Hz measurements are historical and do not establish 360 Hz capacity.

C4 evidence: source 962f7bb built release for x86_64-unknown-linux-gnu and
x86_64-pc-windows-gnu, sequentially with 2 Cargo jobs. Windows ZIP created.
MSVC script prepared; no Windows execution/manual LAN match yet.
