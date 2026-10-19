# Linux freeze: attended investigation and release gate

Goal: stable **hardware Vulkan** on the affected four-monitor Ubuntu workstation.
Software Vulkan is containment/comparison, not hardware acceptance. Other game
features are paused. The recorded incident and thread audit are in
[LINUX_FREEZE.md](LINUX_FREEZE.md) and [GRAPHICS_THREAD_AUDIT.md](GRAPHICS_THREAD_AUDIT.md).

## Before a session

No automatic GUI launches, retries, stress, GPU reset, driver/kernel/VPN/display
changes, watchdog arming or deliberate crashes. Owner chooses an attended
session. One command below means **one** application launch, never a batch loop.
A timer cannot close a window if the event loop or kernel is blocked.

Keep the same build/config, VPN state, monitor layout and background workload
through a comparison. Use one fixed GPU/presentation policy when adding the
worker. Record any deviation before launching. Do not use strace initially:
it changes scheduling and adds I/O; `tools/trace-graphics.py` is a separate
explicit option if a specific syscall question remains.

On Windows, from a copy of this checkout, first start the independent observer:

```powershell
powershell -NoProfile -File .\tools\freeze-liveness.ps1 -Target 192.168.50.155 -Seconds 120
```

It sends one ICMP request/second, writes timestamped NDJSON to the laptop and
flushes to disk. Confirm successful replies in the file **before** SSK. A lost
reply is not proof of kernel lockup: firewall/TUN/routing can drop ICMP. Clock
alignment uses UTC; each capture also has its own monotonic clock. Do not enable
SSH or change firewall/services for this stage.

On Linux, terminal A, start the existing root read-only collector using the
owner's graphical Polkit prompt:

```sh
pkexec /usr/bin/python3 /home/a/github.com/xakepp35/ssk/tools/freeze-stacks.py \
  --seconds 120 --stage cpu-graphics \
  --binary /home/a/github.com/xakepp35/ssk/dist/ssk-linux-x64/ssk \
  --app-config /home/a/github.com/xakepp35/ssk/dist/ssk-linux-x64/ssk.yml \
  --output-dir /home/a/github.com/xakepp35/ssk/runtime/freeze-stacks
```

Wait for the path and `First sample persisted; collector ready` message. The first
sample records boot/kernel, stage and binary/config SHA256; config contents and
keys are never captured. Root-private files stay under runtime. The collector
reads bounded procfs stacks at1Hz for at most120s in this procedure; it never
starts SSK, changes settings or calls GPU APIs. Its child timeout bounds procfs
sampling; an unresponsive filesystem can still block the writer.

## One launch, then inspect

Terminal B, initial CPU baseline only:

```sh
cd /home/a/github.com/xakepp35/ssk/dist/ssk-linux-x64
./ssk graphics-probe --graphics software --duration-secs 10 --simulation off
```

Expected actual ready: Vulkan, native Wayland, Cpu/Mesa; timed exit after window
initialization, `ssk.probe.end reason=duration_elapsed`. Updates are CPU callback
counts, **not proof of GPU completion/presentation**. Confirm the window was
visible and responsive. A manual early close is `window_closed`, not a completed
10-second run. The probe opens no profile, identity, socket or audio. Its worker
option reuses the real offline game loop, channels and shutdown; the drawing is
unchanged. A simulation error makes the process fail.

After this launch, let the collector finish. Summarize its exact file:

```sh
pkexec /usr/bin/python3 /home/a/github.com/xakepp35/ssk/tools/freeze-stacks.py \
  --summary-file /absolute/path/from/capture-ready.ndjson
```

The summary reports a torn final record and the timestamp of the last successful
thread sample separately from later sampling failures. Use `--pid APP_PID` with `--summary-file` to aggregate the application
stacks during its lifetime (preserving TIDs), instead of only after-exit display
stacks. Inspect summary and app
end record, and compare the Windows ICMP timeline.
Record run, build, stage, boot, actual adapter, present mode, visible behaviour
and exit result in an incident note under runtime, not in git.

Proceed only after reviewing the previous stage:

| Stage label | Application change | What stays fixed |
| --- | --- | --- |
| cpu-graphics | software, simulation off,10s | static scene, no network/audio |
| gpu-graphics | hardware on one explicitly identified GPU, simulation off,10s | scene and FIFO |
| gpu-worker | same GPU, simulation worker,10s | scene and presentation |
| offline | `play --no-audio` on the same GPU | no network/audio |
| network | `host --no-audio` on the same GPU | audio disabled |
| audio | `host` on the same GPU | network/presentation |

### Physical GPU and presentation controls

`adapter_candidate`/`adapter_selected` now record Vulkan deviceUUID and optional
PCI address, plus supported presentation modes. Names and indices are not stable
identities. Selection fails if the requested UUID/PCI is unavailable; it never
silently chooses another card. The app queries read-only properties on adapters
already enumerated by wgpu; no additional GPU instance/device/probe is created.

The previous boot identified GNOME's display GPU as card2/`0000:65:00.0`, with
`0000:17:00.0` headless. Reconfirm the current GNOME primary card from its existing
boot journal before the hardware stage (targeted `primary|/dev/dri/card` entries),
then map its sysfs `device` link to PCI. A card number alone can change on reboot.
Use the confirmed address for the **single** hardware scene launch:

```sh
./ssk graphics-probe --gpu-pci 0000:65:00.0 --present-mode fifo --duration-secs 10 --simulation off
```

This is a later attended stage, not an extra enumeration launch before the CPU
baseline. Confirm the selected event has that PCI address. Record the logged
32-hex UUID and use `--gpu-uuid HEX` for subsequent same-GPU comparisons, adding
`--simulation worker` only at the worker stage. PCI requires VK_EXT_pci_bus_info;
UUID requires Vulkan1.1. Unsupported identity queries report absent identity;
explicit selection then fails rather than guessing. Software mode never loads
the NVIDIA application ICD just to obtain its identities.

If captured evidence points to presentation, the separate comparison on that
same GPU is `--present-mode mailbox --mailbox-repaint-hz 60`. Match the explicit
idle cadence to the intended comparison;60Hz is the conservative diagnostic
default, not a monitor measurement. The selector verifies Mailbox support before
creating the device. Unsupported Mailbox is an error, not AutoVsync fallback.
FIFO remains the default and monitor/compositor driven. Mailbox requests delayed
idle redraws instead of continuously requesting repaint; native input/expose may
repaint earlier, so this is not a strict FPS cap or a simulated vsync guarantee.
Simulation always remains360Hz. Do not change GPU, simulation and presentation
in the same comparison. No presentation difference alone identifies a specific
kernel/driver defect without the corresponding capture.

Change collector `--stage` accordingly. `play` explicitly selects offline even
when packaged config says host. The game stages are manual10-second observations
and normal menu exit; only the probe has a timed close. Start a fresh bounded
capture for each stage. These are operator steps, not an automated launcher.

**At the first freeze stop the sequence.** After reboot preserve the previous
boot journal/pstore/crash files and both machine captures before another launch.
Repeated previously completed DRM waits are not a deadlock; inspect final and
changing stacks, the GNOME/KMS/NVIDIA threads and remote liveness together.

## Evidence selects the repair

- SSK lock: eliminate the observed wait/cycle, keep main-thread graphics ownership.
- Worker-only failure: inspect handoff, startup/shutdown and worker stacks; do not
  move graphics handles to the worker or guess CPU affinity.
- Minimal hardware scene fails: narrow Vulkan/WSI, physical adapter, GNOME/NVIDIA;
  use the probe as the minimal reproducer. Software success alone cannot assign
  blame to a particular NVIDIA defect; GNOME still uses its GPU.
- GPU/presentation-dependent failure: keep the same physical GPU and compare
  FIFO/Mailbox only if supported; no unlimited rendering or silent fallback.
- Network/audio-only failure: investigate that startup/I/O stage separately.
- Missing final stacks/full machine loss: stop reproduction and prepare a
  separately reviewed kernel capture step. Panic settings need explicit approval
  and rollback; kdump is not guaranteed to catch every GPU/host deadlock.

Driver/kernel updates are separate evidence-driven operations with a recorded
rollback, not changes mixed into an application comparison.

## Hardware release gate

After an evidence-selected fix:10 observed starts of60s,30min normal local play
including focus/resize/window moves/fullscreen/exit, then15min Windows LAN play.
No launch loops or synthetic stress. Verify actual hardware Vulkan/Wayland,
360Hz simulation,60 game packets/s online and clean process exit. Keep the
original multi-monitor setup. Record outcomes, build/hash/config and remaining
limits in PLAN/CHECKPOINT; no logs/identities/build outputs in git. A successful
build or CPU-only run cannot close the hardware freeze blocker.
