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
Application source a12f305: cargo check passed; final Linux release succeeded
(1m32s), Windows GNU static release succeeded (4m19s), sequentially with 2 jobs.
Windows PE imports: system DLLs only; no libwinpthread/libgcc external runtime.
D1/D2/D3 complete. tools/build-linux.sh packages dist/ssk-linux-x64.tar.gz;
tools/build-windows.sh packages dist/ssk-windows-gnu.zip. Both contain only
executable, root ssk.yml and PLAY.md. Linux config host; Windows config join
to 192.168.50.155:4202, both 1600×1000. No examples directory or bundled DLL.
MSVC script also requests static CRT; it has not been run. No automated tests,
manual new-client/Windows runtime or live Linux↔Windows match performed.
No active game/server/build/swarm. No push. Owner's pre-existing Cargo.toml
change to authors was preserved and excluded from these implementation commits.

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
