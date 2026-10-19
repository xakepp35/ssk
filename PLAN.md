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

## D: simple LAN launch and portable distribution

- [x] D1 — Root ssk.yml, separate application/window/network configuration,
  1600×1000 default, optional CLI class, saved selection, F2 menu/F3 invite.
- [x] D2 — Host (play + accept joins), headless server/node, bounded TCP LAN
  invitation endpoint and IP-based joining to 192.168.50.155:4202. Preserve
  headless seed connections while players form the ordinary P2P graph.
- [x] D3 — Static Windows compiler runtime, no bundled DLL, root config in both
  distributions, release builds and PE import evidence; update LAN guide.

No automated tests or heavy swarms. Live cross-platform match remains manual.

D3 evidence: application source a12f305 built Linux release (1m32s) and
Windows GNU static release (4m19s), sequentially with 2 jobs. PE imports
contain only Windows system DLLs; no libwinpthread/libgcc DLL. Windows ZIP
contains exactly ssk.exe, ssk.yml (join to 192.168.50.155:4202), PLAY.md.
Linux package has executable + root host config + guide. Windows execution,
MSVC script and live cross-platform match remain unverified.

## E: reported hard freeze and remote NPC motion

- [x] E1 — Remote sector images were held at 5 Hz with previous=position.
  Add bounded NPC/asteroid dead reckoning and 100 ms correction decay in
  ssk-sim, sampled only into render copies. Reset by sector epoch/entity ID;
  canonical AI/combat/ownership and wire rate remain unchanged.
- [x] E2 — Record read-only journal findings; real --no-audio/audio.enabled
  switch that skips native audio initialization, plus startup stage messages.
- [x] E3 — Rebuild Linux/Windows packages after owner clarification and explicit
  request. Both targets built sequentially with one Cargo job; no GUI, automated
  tests, freeze reproduction or heavy workloads. Manual follow-up stays separate.

E1/E2 compile evidence: cargo check passed; no GUI or freeze reproduction.
Receiver HUD now distinguishes local/remote stewardship and completed-image age.
Journal evidence and limits: docs/LINUX_FREEZE.md. Root cause remains unknown.


E3 evidence: application fixes ba34ebd/e2d50e0, Linux release completed in
2m11s, Windows GNU static release in 2m18s. Both archives refreshed; Windows
import check reports system DLLs only. Default Linux host / Windows join to
192.168.50.155:4202 preserved. Owner confirms second freeze happened without
launching the game; cause remains unknown. No live post-fix match performed.


## F: requested flight model, continuity and starfield

- [x] F1 — Named ship traction/turn stats; inertial nose-relative thrusters,
  limited mouse steering, class HP rebalance, full shield at 0.5 E/damage,
  stronger scout thrust and double boost turning; update AI/validation/UI/docs.
- [x] F2 — Fix coordinate handoff/camera jumps and sector-content continuity;
  preserve nearby objects across sector boundaries and ownership changes.
- [x] F3 — Fourfold stars with individual depth, brightness and parallax;
  retain background continuity across coordinate rebases.
- [x] F4 — Sequential one-job Linux/Windows static release builds and packages.

No automated tests, GUI launch or heavy swarms. Compilation and package build
only; gameplay improvements require a later manual session.

F1 evidence: cargo check --locked -p ssk --jobs 1 passed (2.47s). Rules ID
ssk-rules-4-inertial prevents matches with old flight/balance clients. No tests.

F2 evidence: cargo check passed (1.03s). Bounded 1600-unit sector prefetch/keep,
no new encounters in empty prefetch sectors, 1 Hz empty terrain checkpoints,
retain known active images during handoff/delivery delay. Camera rebase occurs
before mouse aim. Snapshot v2 covers scout boost speed; no runtime claims.

F3 evidence: cargo check passed (0.54s). 1320 seeded stars, continuous depth
controls parallax/brightness/size; bounded phases compensate coordinate rebases.
Visual result has not been viewed in-game after the workstation incidents.


F4 evidence: source 115b24c built Linux release (2m39s) and Windows GNU static
release (2m10s), sequentially with SSK_BUILD_JOBS=1. Both archives refreshed;
Windows import check: only system DLLs, no external compiler runtime. Root
1600×1000 config retained: Linux host, Windows join to 192.168.50.155:4202.
No automated tests, GUI launch or live gameplay/continuity verification. Stable
flight/continuity rules added to AGENTS.md. F1–F4 complete; manual follow-up open.


## G: startup hard-freeze investigation after ~10:02 reboot

- [x] G1 — Correlate prior boot and startup order; audit io_uring/eBPF/network,
  native audio, Xwayland/GLX, TUN settings and crash records. No game launch.
- [x] G2 — Prepare focused persistent diagnostics for the remaining startup
  interval, preserving secrets and avoiding heavy reproduction/system tuning.
- [x] G3 — Record evidence, unresolved causes and exact next diagnostic action;
  compile only if source changes require it. Do not claim a root cause without
  a corresponding trace, kernel stack or controlled discriminating evidence.
- [ ] G4 — With owner feedback/authorization, capture next natural incident or
  an attended probe: remote liveness, persistent stages/syscalls, kernel dump;
  identify the responsible stack/component before changing driver/VPN settings.

G1–G3 evidence: first marker precedes native graphics/Macroquad resource init;
SSK network, simulation and native audio start only after the second marker.
Latest preceding boot has 0 USB errors, no matching Xid/panic/OOM/lockup;
pstore and crash dump absent. Current GNOME Wayland uses Miniquad X11/GLX;
TUN gvisor/MTU9000 present but causality unknown. Read-only BPF metadata has
cgroup programs, no XDP/tc. Loaded kdump and enabled watchdogs found, but lockup
panic flags are off. No system/VPN/driver settings changed.

Graphics-only native/Macroquad probes, synced startup stages, bounded ptrace
wrapper and explicit arm/restore watchdog script prepared. Temporary arming
awaits owner response; no crash or GUI was triggered. cargo check passed (2.08s);
Linux release/package succeeded with one Cargo job (2m15s). Windows was not
rebuilt for this Linux diagnosis. No automated tests or gameplay verification.
Exact evidence, limits and capture procedure: docs/LINUX_FREEZE.md.

## H: structured local observability

- [x] H1 — Define versioned NDJSON schema, stable callsite IDs, correlation,
  durability/rotation/privacy, subsystem coverage and limits. docs/OBSERVABILITY.md.
- [x] H2 — Binary-owned tracing sink, file+stderr immediate flush/sync,
  rotation/filtering/suppression, panic backtrace and host/progress monitoring.
- [ ] H3 — Instrument startup/client/simulation/network/audio and teardown;
  add independent host journal/thread collector and usage/query documentation.
- [ ] H4 — Focused compilation, sequential four-job Linux/Windows release packages;
  record evidence without launching graphics, stress loads or automated tests.

Hard-freeze root cause remains open (G4). Logging cannot guarantee recovery of
kernel state from a total lockup; watchdog arming remains unapplied/unapproved.

H2: logging initialized before config, random run/trace IDs, stable named events,
span lifecycle/durations, bounded rotated file+stderr durable sink, panic FD,
rate/drop/IO counters, Linux read-only host/thread/interface/pressure monitor,
startup operation/progress ages and doctor command. Startup/probe markers migrated
to the same sink. Focused cargo check passed; no runtime/GUI/tests executed.


## I: modern native graphics (owner priority, H3/H4 paused)

- [x] I1 — Replace Macroquad/Miniquad/OpenGL with eframe/egui + wgpu;
  Vulkan renderer, native Wayland on Wayland sessions, X11 compatibility on X11.
  Preserve drawing, menus/input, window preferences, 360 Hz worker and audio.
- [x] I2 — Move graphics-only probe to the same wgpu path; document backend,
  monitor/DPI behavior, startup evidence and remaining hard-freeze uncertainty.
- [ ] I3 — Four-job focused compilation and sequential Linux/Windows static
  release packages; update guides/config and record actual build evidence.

H3 checkpoint: subsystem instrumentation compiles with four jobs. Independent
`tools/observe.py` is an unfinished draft, not advertised for use. Its journal
exit handling and Windows summary import need correction before H3 completion.
No GUI, tests, crashes or system changes. More logging coverage deferred by owner.

I1 evidence: `cargo check --locked -p ssk --jobs 4` passed (1.38s).
Client, primitives, menus, effects and probe now compile on eframe/egui 0.33.3,
wgpu 27.0.1, winit 0.30.13; only Vulkan backend enabled. Native Wayland preferred
by winit on Wayland sessions. Explicit FIFO/2 frames in flight. Physics/network
rules unchanged. No GUI, runtime probe, automated tests or freeze reproduction.

I2: README/LAN and freeze history distinguish new Vulkan behavior from old
incident evidence; graphics trace wrapper matches the new probe CLI. All build
scripts default to four jobs. Linux release build in progress; no GUI/tests.

## J: freeze after the owner's Vulkan/Wayland launch (current priority)

- [x] J1 — Analyze the supplied NDJSON through bounded queries, fingerprint the
  executable, correlate journal/rsyslog/existing metrics and inspect primary
  driver reports. Preserve evidence and distinguish hypotheses from proof.
- [x] J2 — Prepare a bounded independent read-only kernel-stack collector and
  document exactly which missing evidence could identify the failing subsystem.
- [ ] J3 — Owner-approved attended kernel-stack/liveness capture; identify the
  failure before selecting a driver/application workaround or further launches.

J1 evidence: 75 records / 92,467 bytes, actual NVIDIA615.71.09 + Vulkan/Wayland,
3.484 s until log end; 360 Hz, 128–141 FPS, no overload/panic/admitted game peers.
GPU wait snapshots recover in subsequent records. Host memory/write metrics
show no resource runaway; pre-existing I/O pressure is recorded separately.
Historical metrics and journal/rsyslog/Loki have no fault stack at the incident.
Root cause remains unproven; see docs/LINUX_FREEZE.md for exact times/hashes.
J2 collector syntax compiled only; no collector, GPU/probe/game/tests or root
changes executed. J3 needs evidence not present in the supplied application log.

I3 status: Linux four-job release/package completed in 4m34s, then owner reported
this freeze. Windows packaging is deferred while J is the active priority.
