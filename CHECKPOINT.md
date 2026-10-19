# SSK checkpoint

## Objective / authority
Implement README/PLAN in small committed steps. No push, no automated tests.
Latest owner revision: 360 logic ticks/s, packet each 6 ticks, fixed 60 Hz;
monitor vsync independently; named global parameters; game model/validation
in ssk-sim; Linux/Windows build and LAN play instructions. See AGENTS.md.

## Delivered
- A0–A12, B1–B15, B16a–e committed. Alpha tag v0.1.0-alpha.1; no beta tag.
- C1a/b/c, C2/a/b, C3/a/b and C4 complete. Three libraries, one binary.
- ssk-sim/constants.rs owns TICK_PER_SEC = 360, TICK_PER_PACKET = 6 and derived
  timing plus game balance/physics. No configurable 30 Hz mode remains.
- ssk-sim owns World, CombatGuard, PeerCombat, Snapshot/Replicas, shared sector
  ownership/leases/loot/rewards, lag compensation, frags, death and respawn.
- ssk-net owns QUIC/Iroh, authentication, codecs/framing, discovery and routes.
  Named constants cover limits, queues, timing and message/frame schema IDs.
- ssk-client owns a dedicated game worker, bounded render handoff, input,
  Macroquad UI/vsync/audio and a transport adapter. Presentation defaults in
  constants.rs; runtime settings remain owned Config/Profile, no mutable globals.
- Logic or transport lag >100 ms stops/disconnects the session. Per-slot payload
  reservation keeps a short datagram for every remaining physical connection.
- Protocol 3 handshake and signature domain include both timing constants.
- Windows paths and Unix-specific file permissions gated for portability.

## Current LAN revision
D1/D2 implemented: app config in binary composes client WindowConfig and net
Config. Default ssk.yml from cwd, then executable folder; paths relative to
config. Window 1600×1000 overrides old saved window size. F2/Esc menu, F3 invite;
optional --class, saved class; online choice applies at next respawn.
No-argument root mode host; Windows package mode join to 192.168.50.155:4202.
Host combines player and bounded TCP invitation endpoint; server/node is
headless P2P discovery/rendezvous, no ship/central authority. Game remains QUIC.
Headless seeds keep connections rather than pruning against a ship-only graph.
Examples removed; NAT tools migrated to nested network section. No tests run.

## Build evidence / artifacts
F1–F3 source 115b24c: focused cargo checks passed (2.47s / 1.03s / 0.54s).
Linux release succeeded (2m39s), then Windows GNU static release (2m10s),
sequentially with SSK_BUILD_JOBS=1. F4 complete. Ignored persistent logs:
runtime/build-flight-{linux,windows}.log and build-*-check.log.
Windows import check: system DLLs only; no external libwinpthread/libgcc DLL.
Refreshed dist/ssk-linux-x64.tar.gz and dist/ssk-windows-gnu.zip each contain
executable, ssk.yml, PLAY.md. Linux config host; Windows join to
192.168.50.155:4202, both 1600×1000. No examples, DLLs or identities packaged.
Both computers must update: rules ssk-rules-4-inertial and snapshot v2.
MSVC script unexecuted. No automated tests, GUI launch or post-fix live match.
No active game/server/build/swarm. No push. Owner's authors commit c0d298d
retained without changes.

## Historical measurements / open work
22a9737 at 240/30: 2/16/64 loopback completed with zero skipped steps or measured
expiry; 64 p95 replica age 96.8 ms, shared image 502.4 ms. 128 overloaded this host.
Blocked-UDP Docker relay scenario delivered replicas both ways; not internet.
See docs/MEASUREMENTS.md. Raw /tmp reports disappeared across workstation
incident; only previously read aggregate results retained. No 360/60 capacity
claim. Do not repeat heavy swarms after the owner's reported OS freeze.
Next B16 acceptance: short manual new-client/LAN match when available, then
separately planned distributed capacity/failure/internet evidence. 128 remains
unaccepted. No public hosts currently supplied; no beta tag until justified.

## Incident / E1–E3
Owner reported live Linux↔Windows gameplay: NPCs smooth on Windows and jerky
on Linux regardless of host/join. Sector images were 5 Hz, NPC.previous=pos,
no remote smoothing. Steward election uses ID/lease, not host/join. E1 adds
bounded visual prediction (400 ms) and correction time constant 100 ms, only
on render copies; canonical gameplay unchanged. E2 adds truly lazy native
audio, --no-audio/audio.enabled, startup markers, steward/receiver-age HUD.
Default audio remains enabled. Both builds include these changes; no live replay.

Two owner-reported hard freezes/reboots. First journal: 110127 Focusrite USB
errors 06:00:22–06:11:55, re-enumerated 06:11:56; last SSK terminal activity
06:15:25, next boot 06:17:42. Second: 231225 USB audio errors, all -16,
06:20:02–06:33:19; prior boot ended 06:47:53, new boot 06:55:39 MSK.
No matching Xid/OOM/panic/lockup/hardware fault found in searched kernel records.
Owner confirms second freeze occurred WITHOUT launching game, while agent had
started only a 2-job build; SSK graphics/audio initialization cannot explain
that occurrence. Cause unknown; no system/driver settings changed. Initial
build was interrupted and /tmp log lost, later explicit 1-job rebuild succeeded.
See docs/LINUX_FREEZE.md. No automatic reproduction or heavy swarms.

## Latest flight / F1–F4 complete
- f8df2bd: named ShipStats hp/traction/turn_rate/radius; inertial nose-relative
  thrusters, finite shortest-path mouse turning at 360 Hz. No speed parameter or
  hard velocity clamp; linear drag 1.5/s. Class HP 750/1400/1000, traction
  900/675/480, turning 300/240/180 deg/s. Scout boost traction x4/turn x2.
  Shield absorbs all affordable damage at 0.5 E/damage; only overflow hits hull.
  AI/control adapter/validation/UI/README aligned; rules ID bumped.
- 572c64d: 1600-unit nearby sector prefetch/retention, at most four sectors per
  player at default viewport. No new encounters in empty prefetch sectors;
  their terrain publishes at 1 Hz. Known active images survive delivery stalls.
  Camera/effects rebase before cursor aim. Snapshot v2 velocity step 1/8 covers
  2400-unit scout boost. Default viewport is covered; oversized viewports need
  separately tuned interest bounds. Remote snapshot delivery latency remains.
- 115b24c: 1320 seeded stars (previously 330), individual continuous depth
  determines brightness/size/parallax. Bounded phases survive origin rebases.
- F4: Linux/Windows static packages built successfully; evidence above. Stable
  flight and continuity requirements saved in AGENTS.md.

## Active investigation / G
Owner reports new freeze after ONLY "SSK startup: graphics init, 1600x1000,
vsync interval 1", before any visible window; VPN v2rayN TUN often enabled.
Previous boot f4352ed4 ended 10:02:06; next kernel start 10:03:47, journal
10:03:57 MSK. No game/GUI/load started by agent. G1–G3 evidence/tools delivered;
G4 root-cause capture still open. See docs/LINUX_FREEZE.md for full evidence.
Important source ordering: marker precedes Window::from_config; run is an async
future, first body line is "graphics context ready", then Game::start (network),
then Effects::new (native audio). Thus this marker is NOT successful video init.
Miniquad 0.4.11 default X11Only + GLX-first, on GNOME Wayland => Xwayland/GLX.
Last boot: 0 USB submission errors; no matching Xid/OOM/panic/lockup found.
Only recent kernel record 10:00:35 perf sample-rate reduction, not causal proof.
Current TUN: root sing-box, gvisor, MTU9000, auto_route/strict_route, interface
singbox_tun, auto_detect_interface; these are current settings, not prior-boot
proof. No VPN servers/credentials logged. SSK source/lock has no io_uring/eBPF.
Read-only Polkit collection: pstore empty in both locations; /var/crash has no
vmcore. 47 currently loaded BPF programs are cgroup_device/cgroup_skb sd_*;
no XDP/tc in snapshot. Ignored local log runtime/freeze-root-summary.log.
kdump-tools active, kexec_crash_loaded=1, watchdog/nmi=1, threshold10; lockup
panic/all-CPU-backtrace flags and panic timeout are 0. Capture initramfs includes
ZFS support; actual saving unverified (no deliberate crash). Settings untouched.

Prepared --startup-log synced markers and graphics-probe, with --native skipping
Macroquad, optional explicit GLX/EGL selection; no game/network/profile/audio.
tools/trace-graphics.py bounds ptrace capture to 20s/16MiB, syncs local writes;
strace unpacked into runtime/diag-tools without system installation. This does
not protect against a kernel hang. tools/freeze-watchdog.py status/arm/restore
supports temporary lockup->panic->existing kdump with 60s fallback reboot,
saves originals under /run; ARM NOT APPLIED, owner response pending.
cargo check passed 2.08s, sequential one-job Linux release/package 2m15s:
runtime/build-freeze-diagnostics-{check,linux}.log; Linux archive refreshed.
Windows remains previous flight revision; no network-rule change this phase.
No automated tests, GUI/probe/crash launched; no active builds/diagnostic drivers.

Exact next: owner feedback on audio/remote ping during freeze and temporary
watchdog arming. If authorized, pkexec /usr/bin/python3 with absolute
tools/freeze-watchdog.py arm; then status. Do not launch GUI or deliberate crash
automatically. Next natural incident: inspect previous journal/pstore/var/crash
before any experiment. Root cause remains unknown; never claim this was fixed.

## Active observability / H
Owner requests detailed DEBUG NDJSON simultaneously to terminal/file, immediate
flush and unique per-callsite keys, system/subsystem/stack diagnostics. H1 design
saved in docs/OBSERVABILITY.md. H2 sink/CLI/panic/progress+host monitor implemented;
startup/probe events migrated from legacy text to binary-owned tracing. Random
run/trace IDs; span begin/end, sync each record, rotation and explicit rate limit,
Linux proc/sysfs typed samples and doctor command. Focused cargo check passed.
No logger runtime or GUI executed. Next H3: render/sim/net/audio instrumentation,
independent host collector and offline log reconstruction; then H4 sequential
one-job Linux/Windows packages. Cargo.lock updated only for already present deps.
No GUI/stress/tests/root changes authorized by this request. Existing watchdog
arm question has no answer; do not infer permission from the logging request.


## Current priority / I: graphics migration
Owner explicitly paused observability coverage and requested wgpu/Vulkan and
increased build parallelism. Use four Cargo jobs, sequential target builds.
H3 instrumentation saved; cargo check -p ssk --jobs 4 passed. tools/observe.py
is an unfinished, unexecuted draft (fcntl portability/journal-exit handling/
bounds require follow-up); H3/H4 remain open. No runtime/GUI/tests started.
Next: replace Macroquad/Miniquad with eframe/egui/wgpu, preserve gameplay and
window/UI features; Vulkan on Linux, native Wayland selection. Inspect current
crate APIs, port client and probe, build both packages. Vulkan removes the old
OpenGL route but does not prove or guarantee a fix for the workstation freeze.
No new authorization for watchdog arming or automatic graphics reproduction.


I1 implemented: Macroquad/Miniquad dependencies removed, owned per-frame Canvas
uses egui Painter and winit input. eframe 0.33.3 + wgpu 27.0.1 (Vulkan/std/WGSL
only). Latest eframe 0.37 conflicts with libp2p's pinned wasm-bindgen-futures/
js-sys; selected compatible stable series rather than changing networking.
Wayland selected before X11 by winit; no GL fallback. FIFO presentation with
2 frames in flight; independent 360 Hz worker and lazy quad-snd audio preserved.
Window/fullscreen/DPI/clipboard/menu and depth-star/effect drawing ported. Probe
uses same window/device/painter path, no game/audio/network. Four-job Linux
cargo check passed (1.38s). No GUI, probe, tests or root actions. No active build.
Next I2: update obsolete GLX/EGL probe wrapper/docs; I3: sequential four-job
Linux and Windows static packages. Build script defaults already edited to 4.
