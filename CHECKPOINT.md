# SSK checkpoint

## Objective and rules
Sole priority: identify and repair whole-Ubuntu freeze. Approved investigation
is PLAN L1–L6 / docs/FREEZE_RUNBOOK.md. Other product work and expanded logging
are paused. One complete step/commit with PLAN/CHECKPOINT; no push. Four Cargo
jobs, targets sequential. No automated tests, GUI/probe/stress/crash launches
without attended owner authorization. No driver/kernel/VPN/service changes.
No active build, game, collector or other mutation driver.

## Active authorized experiment / reboot recovery
Owner approved staged attended experiments2026-10-20; stop at first freeze.
No repeated permission needed for this sequence. No automatic retries or
kernel/driver/VPN/display/service/watchdog changes. After reboot read
runtime/freeze-sessions/current.json and the pointed session.json/app.ndjson/
console.ndjson, compare boot ID and preserve previous-boot evidence BEFORE
any new launch. A start without finish is not itself proof of a host freeze.
Current completed session: runtime/freeze-sessions/20261019T235124Z.
No active SSK/collector. Default driver discovery/layers on headless host/audio
passed28.353s; no freeze yet. Next: build exact incident source de14d7ab668d in
isolated runtime/freeze-baseline-source worktree, four jobs/shared Cargo cache,
no GUI during build. Preserve dist executable/hash; archive baseline separately
under runtime before any launch. Then prepare ONE original-code host comparison
under fresh root capture, using original-compatible config and default driver
policy. Original physical GPU unknown; automatic selection may differ from
explicit current selection, so record that limitation. No automatic retries.

## L5 completed comparisons (current boot, same Linux binary)

| Session UTC | Stage | Runtime | Result |
| --- | --- | --- | --- |
|20261019T232406Z|CPU static|10.444s|exit0,837updates,no worker|
|20261019T232806Z|primary GPU static|10.701s|exit0,1388updates|
|20261019T233145Z|same GPU+360Hz worker|10.548s|exit0,3603world ticks,no overload|
|20261019T233451Z|offline/no audio|230.496s|~140.5FPS,360Hz,no overload/ERROR|
|20261019T234020Z|host/no audio|32.555s|~141.6FPS,360Hz,no peers/overload/ERROR|
|20261019T234225Z|host/audio enabled|12.599s|~140.4FPS,4320ticks,no overload/ERROR|
|20261019T234558Z|headless GPU static|10.702s|exit0,1303updates,60kernel records/0errors|
|20261019T234738Z|headless host/audio|29.344s|~138.3FPS,10080world ticks,no overload/ERROR|
|20261019T234933Z|same/default layers|28.347s|no overload/ERROR; kernel60records/0errors|
|20261019T235124Z|same/default ICD discovery|28.353s|3candidates,no overload/ERROR|

Each directory is under runtime/freeze-sessions; durable metadata/logs and kernel
summaries saved. First six stages did not reproduce freeze. Hardware stages pin
primary PCI65:00.0 and disable implicit layers/use NVIDIA-only ICD; these differ
from the original unrecorded GPU/loader policy. Audio loads queued, not confirmed
audible/device-ready. Game stages ended with one verified owned SIGINT (exit-2),
not normal menu/window exit. Human visual feedback pending; remote Windows
observer absent; no LAN match or hardware acceptance claim.
Audio root capture90records/0errors,13SSK samples/0stack-read errors. An earlier
60s collector expired before launch; fresh90s capture overlapped app (metadata).
Healthy main-thread DRM timeline waits occurred8/11,9/11,90/106 and10/13samples
with progress; a single such snapshot does NOT identify a deadlock. Final KMS
ppoll/NVIDIA idle. Review full PID/TID-specific kernel summaries as needed.

## Completed current work
- 50efcb3/L1: timed probe1–120s/default10s; --simulation off|worker reuses actual
  offline Game/360Hz clock/channels/stop with identical static graphics. No
  network/audio/profile. End reason/callback/tick/overload counters; callbacks
  never prove presentation. Worker early exit becomes a failed probe.
- 9108e2b/L2 and03741aa: bounded independent procfs stack capture, first sample
  boot/kernel/stage/build/config hashes, durable-ready marker; summary handles
  torn final record and reports last successful stacks vs sampling errors.
  Windows freeze-liveness.ps1 sends/logs1Hz ICMP120s/default, persists on laptop;
  ICMP loss alone cannot establish kernel lockup. No services/listeners changed.
- 6619f4e/L3: optional Vulkan deviceUUID/PCI on already enumerated adapters,
  read-only HAL properties guarded by Vulkan1.1 and PCI extension. Explicit
  --gpu-pci / --gpu-uuid / --gpu-index mutually exclusive and fail without a
  matching compatible card. Strict --present-mode fifo|mailbox support check;
  FIFO monitor/vsync normal, Mailbox finite idle redraw requests default60Hz
  (--mailbox-repaint-hz), native input may repaint sooner. No new dependency
  version; direct ash0.38 was already locked. Root software config retained.
- fb5436b: README/PLAY/audit aligned; obsolete strace-first procedure removed.
- L4: locked four-job cargo check passed0.90s; Linux four-job release/package
  completed1m31s. Embedded revision v0.1.0-alpha.1-49-g6619f4ef6db1.
  target/release/ssk == dist/ssk-linux-x64/ssk SHA256:
  afb4fcfcf3c30f64c7904f99902a6a9e0952b26acea269b0f4dc60df23d294ce
  dist/ssk-linux-x64.tar.gz SHA256:
  5cbccc3e36a10a020c6407e5acdbeae1e2a5ec48cb5738bebc77d195fbb3d474
  Archive only ssk/ssk.yml/PLAY.md, software/FIFO. Clap probe --help printed
  without initializing logger/window/GPU; Python py_compile passed for capture/
  trace tools. No automated tests, GUI or runtime acceptance. Windows archive
  remains old; PowerShell script not executed. No GPU enumeration/API probe.

## Thread audit / prior repairs
741d1d9 introduced the worker AND enabled VSync0->1. Window/device/event/render
already obeyed main-thread requirement; no graphics handle moved to worker,
affinity/RT/fork/io_uring/eBPF found. db478c3 enforces !Send/!Sync owner and
main-TID checks, removes shared world/status mutex from graphics callbacks in
favour of capacity-one nonblocking handoff, moves network startup into worker,
uses atomic/unpark stop, reconfigures Outdated like Lost. Full-host causation
not established. Software bootstrap execs before threads with only Lavapipe ICD,
implicit layers disabled, four CPU workers; elevated/missing CPU fails without
GPU fallback. GNOME compositor still uses GPU. No settings/packages changed.

## Incident and workstation evidence
Detailed evidence: docs/LINUX_FREEZE.md and docs/GRAPHICS_THREAD_AUDIT.md.
User log ab0e7f...:75records/92,467bytes,2026-10-19 11:50:11.781–15.265MSK,
Vulkan/Wayland/NVIDIA615.71.09,128–141FPS,360Hz, no overload/panic/game peers.
DRM wait snapshots are followed by frames, not captured permanent deadlock.
No matching Xid/OOM/panic/lockup/vmcore. Journal stops before game; IO pressure
predates it. One earlier freeze without game; older USB errors absent latest.
Dual RTX3090Ti, kernel7.0.0-38, GNOME Wayland, mixed4monitors, v2rayN TUN. On
current boot journal reconfirmed card2 primary, sysfs PCI0000:65:00.0; card1
PCI0000:17:00.0. This is not the old application's selected physical GPU proof.
SSH inactive; kdump loaded/watchdogs enabled/panic flags off; arm/restore tool
never armed. Earlier healthy1s root stack baseline had8threads/zero errors,
KMS ppoll/NVIDIA idle; no incident captured. No root-cause/runtime fix claim.

## Game implementation to preserve
Three libraries+binary, Rust1.99/edition2024. TICK_PER_SEC360/TICK_PER_PACKET6,
60game datagrams/s; render independent. Model/physics/CombatGuard in ssk-sim.
Offline alpha and LAN/PvPvE through B15 implemented;128-player performance and
NPC smoothing after owner's Linux/Windows match unverified. Rules
ssk-rules-4-inertial, snapshotv2. Inertial nose-relative thrust/finite steering,
scout tractionx4/turnx2, full tank shield0.5E/damage, retained sectors/rebases,
1320 depth stars. Root config host1600x1000/software; Windows joins
192.168.50.155:4202/hardware; F2/Esc menu, F3invite, F11fullscreen. Separate keys.
Observability contract docs/OBSERVABILITY.md; tools/observe.py is unfinished,
unexecuted draft (fcntl portability/journal exit loop/bounds); do not advertise.
