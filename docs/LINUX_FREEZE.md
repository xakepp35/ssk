# Linux hard-freeze investigation — 2026-10-19

## Current renderer after the requested migration

Client now uses eframe/egui 0.33.3, winit 0.30.13 and wgpu 27.0.1, Vulkan only
on Linux and Windows. Winit chooses native Wayland first on Wayland sessions;
X11 sessions use X11/Vulkan. There is no OpenGL/GLX fallback. FIFO vsync,
two frames in flight, independent 360 Hz simulation. Selected GPU/driver and
actual window system are recorded by `ssk.client.graphics.ready`; the preceding
`graphics.begin` still does NOT prove initialization completed. Application
network/simulation/native audio are created after graphics readiness.

New attended probe command: `python3 tools/trace-graphics.py` or
`ssk graphics-probe`. Old `--native`/GLX/EGL switches were removed. Probe uses
the same winit/device/painter path without game/audio/network. An update callback
or queued frame is not proof a monitor displayed it. No GUI/probe was launched
by the agent. Four Cargo build jobs are now authorized; targets run sequentially.
Migration is not root-cause evidence and does not guarantee the freezes stopped.
The incident evidence below describes the OLD Macroquad/GLX implementation.

## What was observed

The owner reports occasional complete-looking freezes when starting the Linux
client: frozen picture and Num Lock unresponsive, requiring a power cycle.
The owner also reports a live Linux↔Windows LAN session with remote NPC motion
jerky on Linux and smooth on Windows, regardless of which process is host/join.
These are separate symptoms; no shared root cause has been established.

Read-only journal inspection, without starting the game:

- Previous boot ended with logged terminal activity involving SSK at 06:15:25
  MSK. The next boot began at 06:17:42. The precise freeze time needs confirmation.
- Between 06:00:22 and 06:11:55, kernel logged **110,127** USB submission/descriptor
  errors on the Focusrite Scarlett 18i8 USB device: predominantly URB rejection
  (-1), already-active URBs (-16), and descriptor failures (-71).
- The audio device disconnected/re-enumerated; driver reported it again at
  06:11:56. These errors precede the final recorded SSK startup activity by
  several minutes. They are a lead, **not proof that SSK or audio caused the
  later freeze**. Other programs may also be using the audio device.
- No NVIDIA Xid, OOM-kill, explicit kernel lockup or hardware-error record was
  found in the searched previous-boot kernel journal. A hard lock/power cut can
  leave the decisive last messages unwritten, so absence does not rule them out.
- Observed workstation: kernel 7.0.0-38-generic, NVIDIA open module 615.71.09,
  two RTX 3090 Ti cards. No driver/kernel/firmware/system settings were changed.
- USB keyboard/LED failure by itself does not prove every CPU stopped. Whether
  Linux still answered a ping/SSH from the laptop during the incident is unknown.

The kernel describes EBUSY as an already-active URB and EPROTO as a USB transfer
fault that can involve the device, firmware, cable or disconnect:
[Linux USB errors](https://docs.kernel.org/driver-api/usb/error-codes.html).
GPU errors may be reported as [NVIDIA Xid](https://docs.nvidia.com/deploy/xid-errors/introduction.html).
No matching GPU error was actually found here; neither source identifies this
incident's cause.

## Changes that help isolate it

Macroquad 0.4.16's built-in audio feature initializes a native context during
its startup. Setting volume to zero only affects playback and never prevented
that initialization. SSK now disables that built-in feature and owns the same
quad-snd backend explicitly, creating it only when audio is enabled.

`--no-audio`, or this root-config section, skips native audio initialization:

```yaml
audio:
  enabled: false
```

Audio remains enabled by default. This is a diagnostic control, not a claim
that the kernel/audio issue is fixed. It does not disable desktop audio used
by other applications. Startup stderr markers distinguish graphics init,
graphics-context readiness and audio worker request/skip. An audio worker
request does not mean the asynchronous device open has completed.

No automatic GUI or freeze reproduction was attempted. For a later controlled
manual session, the no-audio launch is:

```sh
mkdir -p runtime
./target/release/ssk --no-audio 2>runtime/startup.log
```

A second computer can independently watch `ping -t 192.168.50.155` on Windows.
Continued replies during a frozen display distinguish that case from complete
network unresponsiveness; a missing reply alone still does not prove a kernel
panic. Do not repeatedly power-cycle just to reproduce the issue.

## NPC motion: confirmed code defect

`SharedWorld` elects a sector steward by player ID and lease. host/join controls
rendezvous, not stewardship. Independent starting peers can initially own the
same sector, then converge on one lease; changing host/join does not change IDs.

The steward advances its world locally. Other peers previously copied complete
sector snapshots (prepared at 5 Hz, sometimes slower after multipart delivery)
and set each NPC's previous position equal to its new position. Positions were
held between arrivals. That explains this class of smooth-owner/jerky-replica
behavior without implying Linux's 360 Hz simulation or vsync is intrinsically
slower. The report is consistent with Windows becoming steward; actual ownership
at the time was not recorded.

The fix samples NPC/asteroid visual positions using velocity, bounded to 400 ms
of prediction. New snapshots decay positional/angular correction with a 100 ms
time constant; new IDs/epochs reset visual history and large errors snap.
Only render copies are changed. AI, collisions, damage, rewards and ownership
continue using canonical state. This does not make 5 Hz checkpoints into 60 Hz
fresh authoritative states or remove multipart delivery latency.

HUD now shows SECTOR LOCAL/REMOTE and time since the latest complete image.
This is receiver age, not sender capture age or one-way network latency.
If it regularly exceeds the prediction horizon, the next task is compact motion
updates/packet scheduling rather than unbounded prediction or more game packets.
No automated tests, stress runs or live post-fix LAN session were performed.

## Second reported freeze — later the same morning

The owner reported another freeze while the release-build task was outstanding.
The journal confirms the preceding boot ended at 06:47:53 MSK and the next boot
started at 06:55:39 (uptime reports kernel start 06:55:29). These journal bounds
are not an exact freeze timestamp. The agent had started only a Linux release
build with two Cargo jobs, and had not launched SSK or a stress scenario.
The owner subsequently confirmed the second freeze occurred without launching
the game, and explicitly requested the rebuild. SSK native graphics/audio
initialization cannot explain that occurrence; the underlying cause remains
unknown.

The preceding boot contains **231,225** USB submission errors for the same
USB 1-4 audio path, 06:20:02–06:33:19: 229,260 on URB #0, 1,964 on #8 and one
on #7, all error -16. No matching Xid, OOM-kill, kernel panic, lockup, hardware
error, general-protection-fault or kernel BUG record was found in the searched
kernel journal. No warning-or-higher entries were found after 06:45. The gap
between the USB errors and the last journal entry still prevents causal proof.

The interrupted build's /tmp log disappeared after reboot; no successful release
completion can be claimed. Existing Linux/Windows executable timestamps are
06:08–06:13, before E1/E2, so **the distributed packages do not yet contain these
fixes**. No Cargo/rustc/game build or game process remains. Do not launch a reproduction automatically. The owner explicitly authorized
resuming release builds; run the targets sequentially with one Cargo job.
Network/audio liveness during the freeze remains unknown. No driver or system
settings have been changed.

## Rebuild after clarification

At the owner's explicit request, both release builds subsequently completed,
sequentially with `SSK_BUILD_JOBS=1`: Linux in 2m11s, Windows GNU static in
2m18s. The refreshed `dist/ssk-linux-x64.tar.gz` and `dist/ssk-windows-gnu.zip`
include E1/E2 (ba34ebd/e2d50e0), replacing the old packages described above.
The Windows import check found only system DLLs; no external compiler runtime
DLL is needed. Logs are retained under ignored `runtime/build-incident-*.log`.
No GUI, automated tests, stress scenario or live post-fix match was run.
Successful compilation does not identify or fix the workstation freeze cause.

## Third reported freeze — approximately 10:02 MSK

The owner saw only `SSK startup: graphics init, 1600x1000, vsync interval 1`,
then a frozen desktop before a visible window. The owner reports a correlation
with v2rayN's Enable TUN mode. A correlation is not yet a demonstrated cause.
No game, probe, stress run or GUI reproduction was started during this diagnosis.

### Recorded evidence

- Previous boot `f4352ed498444d49bbff57c8d11aebd4`: journal 06:55:39–10:02:06.
  Current boot `52421155834a4cfe8e06ac79c748ffb6`: kernel start 10:03:47,
  journal begins 10:03:57. Last journal record is not the exact freeze time.
- This preceding boot has **zero USB submission/descriptor errors**, unlike
  the first two incidents. The old Focusrite lead does not explain this one
  on the available evidence.
- No matching NVIDIA Xid, panic, OOM, explicit lockup or hardware fault was
  found in the searched kernel journal. The 10:00:35 perf message reduces the
  sampling rate after an approximately 3.9 ms interrupt; it is not a lockup
  diagnosis. No correlated TUN error was found near the freeze either.
- Read-only root collection found both `/sys/fs/pstore` and
  `/var/lib/systemd/pstore` empty. Backend is `efi_pstore`. `/var/crash` contains
  kdump bookkeeping, no vmcore. An abrupt power cut can lose decisive messages.
- GNOME runs Wayland. Miniquad **0.4.11** defaults to **X11Only** and
  **GLXWithEGLFallback**, so the pre-migration application used Xwayland/GLX on this desktop.
  Kernel is **7.0.0-38-generic**, NVIDIA open module **615.71.09**, two RTX3090Ti.
  Installed libGLX NVIDIA version matches the module; no version mismatch was
  identified. This does not establish that the graphics driver is healthy.
- Audit of SSK source and lockfile found no io_uring, eBPF/XDP, raw-socket or
  libbpf/aya dependencies/calls. Networking uses normal Tokio/QUIC transports.
  The 47 currently loaded BPF programs inspected as root have cgroup_device /
  cgroup_skb types and systemd-style `sd_*` names; no XDP/tc program was present
  in that snapshot. This is current metadata, not historical freeze evidence.
- Current VPN state: v2rayN/xray as user, sing-box as root, interface
  `singbox_tun`; sanitized config fields: gvisor stack, MTU9000,
  auto_route=true, strict_route=true, auto_detect_interface=true. No proxy
  addresses, tokens, keys, traffic or credentials were collected into this doc.
  These settings can affect networking, but neither their presence nor MTU9000
  proves a kernel failure. No VPN configuration was changed.

### What the single startup line actually proves

The old wording meant **starting graphics initialization**, not successful
video initialization. The pre-migration source order was:

| Stage | Work |
| --- | --- |
| Before first marker | Load application config/profile; no game threads |
| `graphics init` | Immediately before `Window::from_config` |
| Unmarked upstream interval | XOpenDisplay, GLX configuration/window/context, make-current/vsync, GL function loading, window/DPI/clipboard setup; Macroquad GPU resources, shaders and UI; event dispatch before first future poll |
| `graphics context ready` | First execution of SSK's async function |
| Game worker | Simulation and, in online modes, Network::start |
| Audio worker request | SSK's explicit quad-snd initialization |

Constructing the async future does not execute its body. Macroquad's built-in
native audio feature is disabled; its dummy audio context does not open a device.
Therefore a stall strictly between the first two markers occurs **before SSK's
game network, 360 Hz worker and native audio start**. This narrows the startup
path, but does not distinguish an NVIDIA/kernel stall from an independently
triggered system fault, including one involving TUN. The earlier freeze without
a game launch also remains relevant.

### Prepared capture tools

The original probes separated Macroquad from Miniquad with optional GLX/EGL
selection. Those paths have been replaced by the Vulkan-only probe described
at the top of this document. Startup stages are NDJSON in a flushed/synced file
and stderr. `--startup-log PATH` remains an alias for `--log-file PATH`.
Upstream native setup remains opaque until the app creation callback;
syscall tracing adds a second level of evidence.

`tools/trace-graphics.py` runs **only** the probe under strace (ptrace, without
`--seccomp-bpf`). It saves a unique `runtime/graphics-TIMESTAMP-PID/` directory:

- `stages.log`: application stages with timestamps and synchronous flushes;
- `syscalls.log`: syscall names/times, library/device paths and raw ioctl/buffer
  pointers; no decoded read/write buffers, game packets or identity contents;
- `metadata.json`, `result.json`: selected graphics/session/boot metadata and
  exit/limit status. No full environment or VPN configuration dump.

The trace uses synchronous local writes, a **20-second / 16 MiB** limit and
terminates the process group when the limit is reached. A whole-kernel hang
can prevent the timeout from running; it is not a freeze-prevention mechanism.
Tracing itself changes timing and may hide an intermittent problem. A final
unfinished ioctl alone is a lead, not a conclusive driver diagnosis.

The workstation lacked strace. Ubuntu's package was downloaded and unpacked
under ignored `runtime/diag-tools/`, without installing or changing the system;
the wrapper finds that copy or a normal installed strace. On another machine,
install/provide strace explicitly. Logs stay local/ignored; do not publish raw
logs or kernel memory dumps without reviewing their contents.

### Kernel capture readiness and next action

Current read-only checks: `kdump-tools` active, `kexec_crash_loaded=1`,
watchdog=1, nmi_watchdog=1, watchdog_thresh=10. Both lockup panic flags are **0**;
all-CPU backtraces are **0** and panic timeout is **0**. Thus a detected lockup
need not enter kdump. Capture initramfs contains ZFS modules/scripts for this
ZFS root, but actual crash-dump saving has **not** been verified by crashing it.

`tools/freeze-watchdog.py status` is read-only. Its separately authorized `arm`
action would temporarily set:

```text
hardlockup_panic=1
softlockup_panic=1
hardlockup_all_cpu_backtrace=1
softlockup_all_cpu_backtrace=1
panic=60
```

It refuses to arm without an already loaded crash kernel and enabled watchdogs,
saves previous values in root-owned `/run/ssk-freeze-watchdog.json`, and supports
`restore`. It writes no persistent configuration and triggers no crash/reboot
itself. Detection can subsequently panic/reboot the machine; permission to apply
this change is requested separately. A reboot also clears these temporary
changes. Neither watchdog nor kdump can guarantee recovery from every hardware
or all-CPU hang. See [kernel lockup watchdogs](https://docs.kernel.org/admin-guide/lockup-watchdogs.html)
and [Ubuntu kdump](https://ubuntu.com/server/docs/how-to/software/kernel-crash-dump/).

Only after explicit authorization, from this checkout, arm/restore using the
owner's graphical Polkit prompt (no passwords in chat):

```sh
pkexec /usr/bin/python3 /home/a/github.com/xakepp35/ssk/tools/freeze-watchdog.py arm
python3 tools/freeze-watchdog.py status
# To restore within this boot:
pkexec /usr/bin/python3 /home/a/github.com/xakepp35/ssk/tools/freeze-watchdog.py restore
```

Do not automatically run a probe or deliberate crash. The next discriminating
observation is whether Windows still gets ping/SSH replies **during** a naturally
occurring frozen display, and whether audio stops. TUN can itself affect ping,
so no reply alone is not proof of total kernel lockup. An authorized temporary
watchdog arm can improve evidence on the next natural incident without
deliberately reproducing it. After a reboot, inspect `/var/crash` and previous
boot/pstore before starting another experiment.

For a deliberately chosen, attended diagnostic session **after capture is
ready**, start with the native default-path probe, not a live network match:

```sh
python3 tools/trace-graphics.py
```

This now runs the Vulkan path. Comparing VPN states or driver versions would
require a separately chosen attended experiment; do not loop risky launches.
**The root cause is still unknown; no fix or runtime success is claimed.**

Build evidence for these diagnostic additions: focused `cargo check --locked
-p ssk --jobs 1` completed in 2.08s; Linux release/package completed with one
Cargo job in 2m15s. `dist/ssk-linux-x64.tar.gz` refreshed. Windows was not rebuilt
for this Linux investigation. No automated tests or graphical/runtime acceptance.
