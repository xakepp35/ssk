# SSK checkpoint

## Objective / authority
Implement README and PLAN in small committed steps; no push, no automated tests.
Latest owner revision: 360 simulation ticks/s, one packet each 6 ticks (60 Hz),
monitor-vsync render independently; remove magic tuning numbers; game validation
and model in ssk-sim; build Linux and prepare Windows/LAN play. See AGENTS.md.

## Completed / evidence
A0–A12, B1–B15, B16a–d committed; alpha tag v0.1.0-alpha.1. No beta tag.
Historical HEAD 22a9737 (240 Hz /30 Hz): 2/16/64 loopback clients, zero skipped
steps or measured event expiry. 64: p95 replica age96.8 ms, shared image502.4 ms,
client work1.02 ms. 128 overloaded this host (p95 age3524 ms,3121 skipped steps).
Final reports /tmp/ssk-final-{2,16,64,128}.json, sanitized artifact being saved.
Blocked-UDP two-bridge Docker relay scenario completed: both sides received
replicas over relay, p95 age32.4/34.0 ms, measured expiry0; not public internet.
Reports /tmp/ssk-nat-{a,b}.json. Containers/temporary volume removed.
One short offline window observation only; full manual class/death/restart and
real LAN/internet acceptance remain open. Latest offline launch exited.

## Active work / next
C1a/b, C2/a/b, C3 implemented; cargo check workspace passed, no automated tests.
TICK_PER_SEC360/TICK_PER_PACKET6 derives DT, packet60 Hz and timing windows.
Protocol3 handshake and signature domain include timing. 30 Hz config/CLI gone.
Game worker owns World and Online; render gets bounded snapshots under monitor
vsync. Lag >100 ms stops session and disconnects; no busy-spin/unbounded catch-up.
CombatGuard, Snapshot/Replicas, CombatEvent/SharedEvent, shared sector leases,
pickups/rewards/ownership all live in ssk-sim. Network retains codecs and framing.
Constants in all three libraries now own tuning, UI theme/layout, audio, queues,
timeouts and defaults. Wire offsets/indices/unit conversions remain schema/math.
Pure PeerCombat owns the entire gameplay side of Online; no network dependency
in ssk-sim. Windows paths and Unix-only permissions gated for portability.
C4 first Windows GNU release build succeeded (4m20s), no Windows runtime run.
Final follow-up reserves a heartbeat per peer and disconnects a transport worker
that misses the100ms deadline; constants name message/frame IDs. cargo check
passed. Final Windows+Linux rebuild starting, jobs2, sequential. MinGW unpacked under
ignored target/mingw; target installed. tools/build-windows.{sh,ps1}, examples/lan
and docs/LAN.md prepared. After GNU build, build Linux release jobs2 and record
actual artifact paths. No Windows runtime session yet.
Historical measurements saved in docs/MEASUREMENTS.md; raw /tmp JSON disappeared
across the workstation incident and are not committed. No new capacity claims.
No active game/swarm. Build concurrency2. Do not run stress scenarios.

## Constraints
Three library crates, one binary. Bounded queues, one transit maximum.
Old 240 Hz measurements cannot establish new 360 Hz performance.
