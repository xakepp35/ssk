# SSK checkpoint

## Objective and rules
Sole priority: identify and repair whole-Ubuntu freeze. Approved investigation
is PLAN L1–L6 / docs/FREEZE_RUNBOOK.md. Other product work and expanded logging
are paused. One complete step/commit with PLAN/CHECKPOINT; no push. Four Cargo
jobs, targets sequential. No automated tests, GUI/probe/stress/crash launches
without attended owner authorization. No driver/kernel/VPN/service changes.
No active build, game, collector or other mutation driver.

## Active authorized experiment / reboot recovery
Owner explicitly approved staged attended experiments2026-10-20 and will report
any freeze after reboot. No repeated permission needed for this sequence. Stop
at first freeze; no automatic retries, driver/kernel/VPN/display changes, panic
arming or crashes. CPU-only10s scene first, independent120s kernel stacks.
Before any resumed action read runtime/freeze-sessions/current.json and the
session.json/app.ndjson/console.ndjson there; inspect boot ID and preserve
previous boot evidence. Prepared session: runtime/freeze-sessions/20261019T232406Z.
Current stage cpu-graphics (software Vulkan/FIFO, simulation off, no net/audio).
CPU stage finished successfully; game not running. Next: root read-only collector via Polkit,
wait for durable-ready, one launch, bounded readout. Remote Windows observer is
not attached; no claim about remote liveness. No kernel/driver changes authorized.
If session has a start without a finish, don't rerun; correlate with owner report
and boot change first. L5/L6 and hardware freeze remain open.

## L5 runtime evidence / current stage
CPU session runtime/freeze-sessions/20261019T232406Z finished: exit0/10.444s,
Wayland/Vulkan/Mesa llvmpipe/Cpu/FIFO,837update callbacks, duration_elapsed,
world_tick0/overloads0/no ERROR.120kernel samples/0sample errors;11SSK samples,
93thread observations/0stack-read errors, final KMS ppoll/NVIDIA idle. No matched
new kernel fault. Five optional-display-extension warnings; rendering proceeds.
Human visual confirmation pending; remote observer absent. Collector has exited.
No long-term stability or hardware fix claim. Next: one hardware graphics-only
10s scene on confirmed GNOME PCI0000:65:00.0, same FIFO, with implicit layers
disabled as in CPU baseline, separate durable session/root capture. No auto retry.

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
