# SSK checkpoint

## Current objective / instructions
Owner prioritized wgpu/Vulkan migration and four Cargo jobs; further logging
coverage (H3/H4) paused. Three libraries + binary, Rust 1.99/edition2024.
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
Linux four-job release/package running (tools/build-linux.sh), output
runtime/build-vulkan-linux.log. Poll completion before starting Windows target;
then tools/build-windows.sh with four jobs. Fix any compilation/link issues,
record actual packages/import evidence in I3 and commit. No push or GUI launch.
Observability contract/sink remains in docs/OBSERVABILITY.md; resume extra
coverage only after owner requests it. No independent diagnostic driver active.
