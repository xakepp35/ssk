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
E1/E2 code ba34ebd/e2d50e0: cargo check passed (1.71s). After owner's explicit
rebuild request, Linux release completed (2m11s), then Windows GNU static release
(2m18s), sequentially with SSK_BUILD_JOBS=1. E3 complete. Ignored persistent logs:
runtime/build-incident-linux.log and runtime/build-incident-windows.log.
Windows import check: system DLLs only; no external libwinpthread/libgcc DLL.
Refreshed dist/ssk-linux-x64.tar.gz and dist/ssk-windows-gnu.zip each contain
executable, ssk.yml, PLAY.md. Linux config host; Windows join to
192.168.50.155:4202, both 1600×1000. No examples, DLLs or identities packaged.
MSVC script unexecuted. No automated tests, GUI launch or post-fix live match.
No active game/server/build/swarm. No push. Owner's pre-existing Cargo.toml
authors edit preserved and excluded from implementation commits.

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

## Active request / F
Owner requests nose-relative inertial thrusters, finite class turn rates,
750/1400/1000 class HP, full shield for half damage in energy, stronger scout
boost/double turning, no sector/camera teleports, fourfold depth-based stars.
F1 complete: nose-relative inertial thrust and finite turning, named ShipStats,
full shield absorption at 0.5 E/damage, HP 750/1400/1000; cargo check passed
(2.47s), no tests. Rules ID ssk-rules-4-inertial. Next bounded sector/camera continuity,
starfield, sequential one-job Linux/Windows packages. No tests/GUI/stress runs.
F2 complete: at most 4 sectors per player with 1600-unit prefetch/keep margin;
empty prefetch sectors do not spawn encounters and publish terrain at 1 Hz.
Known active images survive delivery stalls; camera rebases before mouse aim.
Snapshot v2 uses velocity step 1/8 (covers boost); cargo check passed (1.03s).
F3 complete: 1320 seeded depth-based stars (previously 330), dim/slower/finer
far stars and bright/faster/larger near stars; bounded phase continuity across
rebases. cargo check passed (0.54s); no GUI/visual runtime verification.
Next: Linux release then static Windows release, one job each; refresh packages.
