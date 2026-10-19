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

## J: freeze after the owner's Vulkan/Wayland launch

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
J2 collector syntax compiled; a one-second read-only Polkit baseline captured
eight threads with zero stack-read errors (KMS ppoll, NVIDIA modeset idle).
No GPU/probe/game/tests, sysctl/service/driver changes or reproduction executed.
J3 needs evidence not present in the supplied application log or healthy baseline.

I3 status: Linux four-job release/package completed in 4m34s, then owner reported
this freeze. Windows packaging is deferred while J is the active priority.

## K: repair the unusable Linux client (only current priority)

- [x] K1 — Audit the original worker split and actual window/event/render call
  chain against winit/eframe documentation. Enforce one main-thread owner;
  replace blocking shared-frame mutex with bounded nonblocking handoff, move
  network initialization out of the window callback, fix Outdated recovery and
  remove blocking shutdown-command enqueue.
- [x] K2 — Add explicit hardware/software adapter policy and indexes. For the
  affected Linux package select only Lavapipe before any graphics/thread startup
  through a fresh process exec and Vulkan loader ICD filtering. Reject elevated
  execution/missing software device instead of hardware fallback. Preserve
  wgpu/Vulkan and 360/60 Hz, native Wayland, FIFO; Windows package stays hardware.
- [x] K3 — Four-job focused compilation, Linux release/package. Record actual
  evidence; do not launch GUI, stress or automated tests after hard freezes.
- [ ] K4 — Attended runtime acceptance of software containment and diagnosis/
  repair of hardware Vulkan trigger. Product release remains blocked until a
  usable stable client is demonstrated; compilation alone cannot close this.

K1/K2: docs/GRAPHICS_THREAD_AUDIT.md records exact requirements and code paths.
The main-thread rule was already obeyed. 741d1d9 also enabled vsync (0 -> 1),
so the timing correlation cannot isolate the simulation worker. Actual GUI
blocking and surface recovery weaknesses repaired; full-host causation unproven.
Initial four-job cargo check passed in 1.95s. No GUI/system changes/tests.

K3 evidence: final `cargo check --locked -p ssk --jobs 4` passed (0.52s).
`tools/build-linux.sh` completed four-job release in 1m33s; revision
`v0.1.0-alpha.1-45-gdb478c33b704`. target/dist executable SHA256 both
`5eb7d11808ef90d26ed1b1990cef91b783f5646588aefec3c617fa61298432a3`.
Archive contains only ssk/ssk.yml/PLAY.md, software policy present. Supplied
incident log hash unchanged. Installed loader1.4.341.0 and Mesa26.0.8 support
the chosen ICD path; no package/system changes. No active build/collector/game.
K4 is open; explicit request for one attended 10-second software-only launch
with kernel-stack capture submitted because AGENTS prohibits automatic GUI.

## L: approved freeze investigation and hardware repair

- [x] L1 — Timed static Vulkan probe (1–120s, default10s), optional production
  offline 360Hz worker, no network/audio/profile. Same drawing in both modes;
  explicit exit reason/progress, bounded production handoff. Trace wrapper has
  explicit graphics/index/simulation/duration controls and bounded capture.
- [x] L2 — Independent capture/runbook and Windows timestamped liveness monitor.
- [x] L3 — Stable PCI/deviceUUID identity and selection; strict FIFO/Mailbox
  surface support, finite Mailbox idle redraws, FIFO monitor-driven unchanged.
- [x] L4 — Four-job Linux build/package, fingerprint, attended capture readiness.
- [ ] L5 — Owner-attended comparisons: CPU graphics -> hardware graphics ->
  same hardware plus worker -> offline game -> network -> audio. Start10s;
  first freeze stops the sequence. Never automatically retry or start GUI.
- [ ] L6 — Evidence-selected repair and hardware acceptance:10starts x60s,
  30min local play including resize/focus/move/exit,15min Windows LAN.

Each complete step commits PLAN/CHECKPOINT. Keep GPU/kernel/VPN/monitor state
fixed during comparisons. Software is diagnostic containment, not release
acceptance. Driver/kernel changes or panic capture require a separate reviewed
step with rollback. No automated tests or stress loops. See operator runbook
when L2 is complete. L1 focused four-job cargo check passed; no GUI launched.

L3: optional Vulkan HAL properties2 query guarded by API>=1.1 and PCI extension;
read-only borrowed adapter, no new device/instance. UUID/PCI/index mutually
exclusive and fail without matching compatible hardware. No new dependency
version: ash0.38 was already locked, now direct. Four-job cargo check passed1.62s
before final documentation/config adjustments. No adapter enumeration/GUI run.

L2 follow-up: capture-ready marker now follows first fdatasync, not merely open.
Summary tolerates only a torn unterminated final JSON record after power loss,
reports it, keeps the last successful thread sample even after timeout records,
and counts sampling failures. Malformed complete/interior records still fail.
Python source compilation passed; no collector/probe/test launched. Windows
observer prints first successful ICMP baseline; execution on Windows pending.

L3/L4 documentation: README and packaged PLAY guide describe timed probe,
physical GPU selectors, strict presentation support and CPU-only limits. Old
strace-first instruction replaced by the attended runbook. Linux release is
currently building (four jobs); runtime comparisons remain unexecuted.

L4 evidence (2026-10-20): focused locked cargo check passed0.90s; four-job
Linux release/package completed1m31s, embedded revision
v0.1.0-alpha.1-49-g6619f4ef6db1. target/dist executable SHA256 both
afb4fcfcf3c30f64c7904f99902a6a9e0952b26acea269b0f4dc60df23d294ce.
Tar SHA2565cbccc3e36a10a020c6407e5acdbeae1e2a5ec48cb5738bebc77d195fbb3d474;
only ssk/ssk.yml/PLAY.md, CPU isolation/FIFO config retained. Probe --help printed
successfully without logger/window/GPU initialization (Clap early exit).
Current boot GNOME journal selects card2 primary; sysfs maps card2 to
0000:65:00.0 and card1 to0000:17:00.0. No GPU API/enumeration/GUI launched.

L5 pending explicit owner presence/authorization: one10s CPU-only probe, no
worker/net/audio, with independent120s kernel capture. Request sent; do not
interpret silence as authorization. No collector/game/build currently active.
Windows build/script runtime acceptance and all hardware comparisons deferred.
L6/K4/J3 remain open; no root-cause or hardware-fix claim.

L5 activation2026-10-20: owner explicitly authorized staged attended
experiments, with stop at first freeze and reboot recovery from saved session.
First cpu-graphics10s scene, independent120s procfs collector; remote Windows
observer not attached. Before launch persist session/build/config/boot/command
and update checkpoint. No implicit driver/kernel/watchdog/VPN changes or retry.

L5 cpu-graphics:2026-10-20 02:24:37.853–02:24:48.297MSK, exit0/10.444s,
actual Wayland/Vulkan/Mesa llvmpipe CPU/FIFO,837update callbacks, duration_elapsed,
world_tick0/overloads0/no ERROR.120kernel samples,0sampling errors;11samples
contain SSK/93thread observations/0stack-read errors. Final KMS ppoll/NVIDIA
idle. No matching new kernel fault entry. Visual confirmation still pending;
no remote observer. This is a short software baseline, not hardware acceptance.
Capture/session: runtime/freeze-sessions/20261019T232406Z. Summary now supports
--pid to inspect the application's stacks during its lifetime, not just final
after-exit display stacks; preserve TIDs so same-named threads aren't merged.

L5 next prepared: gpu-graphics,10s static scene on primary PCI0000:65:00.0,
FIFO/no worker/net/audio; child NVIDIA-only ICD/implicit layers off, same layer
policy as CPU baseline. Session saved before root capture/application launch;
if reboot occurs, preserve it and previous boot before another experiment.

L5 gpu-graphics: session20261019T232806Z, primary PCI0000:65:00.0 on
NVIDIA615.71.09/Wayland/Vulkan/FIFO,10.701s/exit0,1388callbacks, no ERROR or
new matching kernel fault.120kernel samples/0sampling errors,11SSK samples/
53thread observations/0stack-read errors. Main DRM timeline wait occurs8/11
samples despite continuing callbacks and successful exit: that signature alone
cannot identify the previous freeze. Final KMS ppoll/NVIDIA idle. Capture and
root report persisted under runtime; visual confirmation/remote liveness absent.
Next controlled change: add actual offline360Hz worker on SAME physical GPU,
static scene/FIFO/ICD/layer policy unchanged. Not hardware acceptance.

L5 next prepared gpu-worker: same static scene/GPU/FIFO/ICD/layers, only actual
offline360Hz Game worker enabled. Durable session and checkpoint saved before
launch. Runtime one-shot helper refuses repeated/elevated/cross-boot starts.

L5 gpu-worker: session20261019T233145Z, same primary GPU/static scene/FIFO/
ICD/layers.10.548s/exit0,1384callbacks/world_tick3603,10logic samples360ticks
each, max step765us/max lag12082us/no overload or ERROR.120kernel records,
0sampling/read errors,11SSK samples/64thread observations; main DRM wait9/11
while callbacks/ticks continue, final KMS/NVIDIA idle. No freeze reproduced.
Next offline full game without audio/network, manual10–15s and normal exit.

L5 next prepared offline game on same GPU/FIFO/layers, no audio/network, class
Cadet/seed42 as probe worker. Owner normal window exit after10–15s; this game
mode has no timed close. Durable session/checkpoint saved before launch.

L5 offline live: game PID1491412 continues beyond120s capture, ~140FPS/360Hz,
no logged errors/overloads.120kernel records/0errors,106SSK samples/632thread
observations/0stack errors, main DRM wait90/106with frames continuing, final
KMS/NVIDIA idle. Window remains open;35s launcher timeout did not kill it and
is not a freeze. Owner normal-close request pending; do not start second game.

Offline game continued>3min without freeze; owner normal-close feedback absent.
Prepare30s independent shutdown capture then one SIGINT to verified owned PID
(no kill spam/retry). This does not verify normal window/menu shutdown.

L5 offline end: 230.496s observed, 228render samples,
last FPS140.51, no overload/error; stopped one verified owned PID
with SIGINT, PID gone.30s shutdown capture also completed. Normal menu exit
unverified. Next network/no-audio20s on same GPU with60s stack capture (same1Hz,
shorter total window only); one SIGINT if owner does not close normally.

Network session prepared: host/no-audio20s, same hardware/FIFO/ICD/layers,
60s independent stacks, no promised LAN peer. Save state before launch.
