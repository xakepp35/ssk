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

## Build evidence / artifacts
Source 962f7bb: Linux release build succeeded (1m29s), Windows GNU release
succeeded (1m15s), sequentially with 2 jobs. cargo check passed earlier.
Linux: target/release/ssk. Windows: dist/ssk-windows-gnu.zip and unpacked exe.
Archive includes LAN guide/config, excludes identities/profiles. Build tools:
tools/build-windows.sh, tools/build-windows.ps1. MinGW locally unpacked under
ignored target/mingw; Rust windows-gnu target installed. No root work.
No Windows runtime/MSVC run or live Linux↔Windows match yet. No tests run.
No active build/game/swarm, no push. Update docs/PLAN with future evidence.

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
