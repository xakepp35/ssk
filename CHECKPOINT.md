# SSK checkpoint

## Current objective / instructions
Owner's sole priority: repair Linux freeze; other product work and further
logging coverage (H3/H4) paused. Four Cargo jobs. Three libraries + binary, Rust 1.99/edition2024.
Small complete steps, PLAN/CHECKPOINT in each commit, no push. No automated
tests, GUI/probe/stress/crash launches or driver/VPN/kernel changes this phase.
Normal development needs no root. Watchdog arming remains unapplied/unapproved.

## Current implementation
Offline alpha and UDP/QUIC duel/PvPvE through B15 implemented; owner's earlier
Linux↔Windows match exposed remote NPC jerkiness. Smoothing/sector retention
fixes compile, but have no subsequent manual acceptance. 64–128 performance
not proven. Rules ID `ssk-rules-4-inertial`; snapshot v2; update both peers.
Simulation TICK_PER_SEC=360, TICK_PER_PACKET=6, render independent. Physics and
CombatGuard in ssk-sim, network only transport/discovery/routing/identity/codecs.
Flight: nose-relative inertial thrust, limited shortest-angle mouse steering;
scout thrust x4/turn x2, full tank shield at 0.5 E/damage; stats/AI/UI aligned.
Nearby terrain retention/prefetch, rebase before cursor, 1320 depth stars.
Root ssk.yml: host, 1600x1000; Windows package: join 192.168.50.155:4202.
F2/Esc menu/class, F3 invite, F11 fullscreen. Separate machine identities.

## Completed current phase
- b93c1c0: subsystem tracing checkpoint; four-job cargo check passed. H3 remains
  incomplete. tools/observe.py is an unexecuted unfinished draft (fcntl Windows
  portability, journal-exit loop and bounds need fixes before advertising it).
- 6bb3caa / I1: replaced Macroquad/Miniquad/OpenGL with eframe/egui0.33.3,
  winit0.30.13 and wgpu27.0.1. Vulkan/std/WGSL only. Newest eframe0.37 conflicts
  with current libp2p wasm dependency pins; kept network untouched.
  Owned Canvas wraps egui Painter/input, preserving menu/effects/depth stars,
  clipboard/fullscreen/DPI coordinates. Winit prefers native Wayland, X11 only
  without a Wayland display. FIFO/2 frames in flight. Independent 360 Hz worker
  and lazy quad-snd native audio. Probe uses same wgpu path, no game/net/audio.
  Four-job cargo check passed, latest 1.24s. No runtime graphics acceptance.
- I2: README/LAN/freeze docs and bounded trace wrapper updated for Vulkan probe;
  obsolete --native/GLX/EGL flags removed. Build defaults are four jobs for
  Linux, cross Windows and native MSVC. No Vulkan SDK required; driver required.

## Freeze evidence / limits
See docs/LINUX_FREEZE.md for detailed preserved evidence. Three reported hard
freezes, one without starting game during a 2-job build. First two preceding
boots had heavy Focusrite USB errors; latest prefreeze boot had none. No matched
Xid/panic/OOM/lockup, pstore/vmcore absent. Old first marker preceded GLX context;
SSK game/net/audio had not started. Current GNOME Wayland, dual RTX3090Ti,
NVIDIA615.71.09/kernel7.0.0-38, mixed four monitors; v2rayN TUN gvisor MTU9000.
No proven cause or driver/VPN changes. Kdump loaded, watchdog enabled, lockup
panic flags off; prepared arm/restore script never armed. Vulkan removes GLX,
but does not establish the cause or prove freeze resolution. No GUI started.

## Active processes / exact next action
Current Linux four-job release/package completed in 1m33s (K below). No active
build, game or diagnostic collector. Windows rebuild deferred while freeze
repair is the sole priority. No push or GUI launch.
Observability contract/sink remains in docs/OBSERVABILITY.md; resume extra
coverage only after owner requests it. No independent diagnostic driver active.

## Latest incident / J (current priority)
Owner's dist launch at 11:50:11.781, log ends 11:50:15.265 after 3.484 s;
boot52421155 -> boot92b301b4 at11:52:05. J1/J2 saved in docs/LINUX_FREEZE.md.
75 NDJSON records queried via jq, no full dump. Actual Vulkan/Wayland/NVIDIA
615.71.09 confirmed; revision de14d7a and current dist/target binary hashes match.
128–141 FPS, 360 ticks/s, max step286us/lag1865us, no overload/panic/admitted
peers/game datagrams. Discovery/control and audio were active, not exonerated.
RSS400MiB, available83.66GiB, logger no I/O errors/max completed write4473us.
I/O PSI full62–63% predates game. DRM main-thread waits are followed by frames,
so they do not identify a deadlock. One optional Vulkan-extension warning only.
Journal tail11:49:18, rsyslog11:49:49; Loki has no new fault stack. Historical
GPU metrics last11:50:01:48/66C, memory12.5/13.8GB, no measured prior exhaustion.
Same-driver upstream NVIDIA1392 and GNOME1289 support a graphics hypothesis,
but no matching local stack/root cause. SSH inactive, kdump loaded, panic flags0.
Prepared tools/freeze-stacks.py reads bounded root procfs kernel stacks, no GPU
calls/settings/SysRq/application launch. One-second Polkit baseline succeeded
at19:58:40:8 threads, zero read errors; KMS idle ppoll/NVIDIA idle _main_loop.
Private file runtime/freeze-stacks/kernel-stacks-20261019T165840Z-641629.ndjson.
--summary-file supports bounded final-stack extraction. Collector has finished;
no active collector/build/game. Needs owner-approved attended incident capture;
do not start SSK or arm panic automatically. No root system settings changed.
J3 incident capture is still available. Current implementation work is K below.
Additional logging expansion and Windows build remain deferred.

## K: thread audit and repair (active)
docs/GRAPHICS_THREAD_AUDIT.md records primary docs/history. 741d1d9 introduced
the worker AND enabled vsync (swap_interval0->1). No window/render move off main,
affinity, RT scheduling or worker graphics use found. New !Send/!Sync graphics
owner verifies Linux main TID and creation/update/exit/drop. Replaced shared frame
mutex with capacity-one nonblocking request/frame/control channels. CPU/network
snapshot work stays in worker; network identity/socket initialization moved out
of window callback. Stop is atomic/unpark instead of blocking send; orderly join
remains. Outdated surface now reconfigures like Lost. 360/60 timing unchanged.
Root ssk.yml now adapter: software; Linux bootstrap execs before logger/threads
with only system Lavapipe ICD, implicit layers off, four CPU render workers.
Elevated/secure execution and missing CPU device fail without hardware fallback.
--graphics hardware/software or --gpu-index N; indexes aren't stable PCI IDs.
Windows packaging explicitly adapter: hardware (not rebuilt yet). Mesa Lavapipe
manifest already exists; nothing installed, no driver/kernel/VPN changes.
K1/K2 committed db478c3. Final four-job cargo check passed0.52s. Linux release/
package passed1m33s, revision v0.1.0-alpha.1-45-gdb478c33b704. target/dist SHA256
5eb7d11808ef90d26ed1b1990cef91b783f5646588aefec3c617fa61298432a3.
Archive only ssk/ssk.yml/PLAY.md; software config included; old incident log
hash unchanged. Installed loader1.4.341.0/Mesa26.0.8; no packages installed.
Full-host cause and software FPS/stability still unproven. No GUI/tests/probe/
enumeration run. K4 attended acceptance and hardware failure identification
remain release blockers. Async request submitted for one 10-second CPU Vulkan
launch with independent kernel stacks: AGENTS bans automatic GUI launches.
Until explicit approval, no dependent launch/capture. No active processes.
