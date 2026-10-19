# Freeze repair: thread ownership and Vulkan driver isolation

## What the history actually shows

`741d1d9` introduced the 360 Hz simulation worker **and changed Miniquad's
`swap_interval` from `Some(0)` to `Some(1)`**. The reported change in stability
therefore coincides with two changes, not just threading. Earlier rendering and
the current `main -> launch -> eframe::run_native` path both create the window
on the process main thread. The new worker received CPU world/network data,
not a window, GL context, egui painter, wgpu device or surface. No SSK affinity,
real-time scheduling, `fork`, io_uring or eBPF was found in this path.

The supplied incident confirms Vulkan/Wayland/FIFO and 128–141 FPS until the
log ends. It does not identify a permanent mutex/driver deadlock. Initial DRM
wait snapshots are followed by further frames. The full evidence remains in
[LINUX_FREEZE.md](LINUX_FREEZE.md); absence of an Xid does not clear the driver.

## Requirements checked against the actual versions

| Requirement | Source and finding |
| --- | --- |
| One native event loop on the process main thread | [winit 0.30.13 EventLoopBuilder](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopBuilder.html#method.build). Current Linux winit checks this before backend initialization; SSK never enables `any_thread`. |
| Window/device creation and painting stay in that event loop | [eframe 0.33.3 native runner](https://docs.rs/crate/eframe/0.33.3/source/src/native/run.rs) invokes `resumed`, window events and `run_ui_and_paint` from `ApplicationHandler`. [wgpu integration](https://docs.rs/crate/eframe/0.33.3/source/src/native/wgpu_integration.rs) creates the painter/window within that lifecycle and paints on the same callback thread. SSK calls it synchronously from `main`. |
| Vulkan synchronization follows object ownership, not CPU affinity or a current GL context | [Khronos threading guide](https://docs.vulkan.org/guide/latest/threading.html). Shared queues/command pools require synchronization; SSK has no custom pools or concurrent queue access; its later optional HAL query reads only physical-device identity properties under the adapter guard. Eframe/wgpu own those resources; the simulation gets none. Moving a CPU thread between cores does not require reinitializing its graphics state. |
| Workers cannot move the application or issue graphics calls | New `GraphicsThread` contains a `!Send + !Sync` marker, checks the Linux thread-group leader before device setup, and checks ownership in creation/update/exit/drop. The probe follows the same rule. World snapshots contain only CPU game data. |
| Native callbacks must not wait on world copying/network locks | Previously `take_frame()` blocked on the same mutex held during world clone, peer sampling and network watch reads. Replaced with capacity-one request/frame channels using `try_recv`/`try_send`. Session identity/socket startup moved to the simulation worker. |
| Shutdown must not first wait for free command-queue space | The previous blocking `send(Stop)` precedes `unpark`. Replaced with an atomic stop flag and `unpark`; the worker still joins for orderly cleanup. A stuck native network destructor can still delay shutdown. |
| Lost/outdated surfaces need recovery | [wgpu SurfaceError](https://docs.rs/wgpu/27.0.1/wgpu/enum.SurfaceError.html). `Outdated` now requests reconfiguration like `Lost`; it was previously skipped repeatedly until some other resize/event repaired it. |

No violation of the documented main-thread requirement was discovered. The
new ownership checks enforce an already-correct arrangement. The blocking
handoff and missing surface recovery are actual application weaknesses; neither
is established as the cause of the entire workstation freeze.

## Concrete containment: keep wgpu/Vulkan, exclude the NVIDIA application ICD

The root/Linux-package `ssk.yml` now explicitly sets `window.adapter: software`.
This is a temporary incident workaround, **not hardware Vulkan acceptance**.
Mesa Lavapipe is installed on this workstation at
`/usr/share/vulkan/icd.d/lvp_icd.json`; no packages were installed.

Before logging/background threads, the binary reads the window policy and uses
`exec` with a child environment to select only the system Lavapipe manifest.
There is no multithreaded `set_var`, helper daemon or shell interpolation.
`VK_DRIVER_FILES` excludes other Vulkan ICDs during **enumeration as well as
device creation**, rather than selecting a CPU after already probing NVIDIA.
Implicit Vulkan layers are disabled in that process; stale driver filters and
additional ICD variables are removed; Lavapipe uses four rendering workers.
Secure/elevated execution is rejected because the loader ignores overrides.
The regular logged startup then loads configuration normally. A bootstrap
configuration/isolation failure is carried into the logger and emits a stable
`ssk.graphics.bootstrap.failed` event to file and stderr before any window/device.

The adapter selector independently requires a surface-compatible CPU adapter.
Missing Lavapipe or a wrong device produces an error, **never hardware fallback**.
Software mode on Windows is rejected; Windows packaging explicitly uses
`adapter: hardware`. No GL/GLX/EGL renderer was reintroduced.

Primary references:

- [Vulkan loader driver discovery](https://vulkan.lunarg.com/doc/view/latest/linux/LoaderDriverInterface.html#overriding-the-default-driver-discovery): `VK_DRIVER_FILES` replaces normal driver discovery; elevated execution ignores it.
- [Mesa installation / driver selection](https://docs.mesa3d.org/install.html): choosing a Vulkan ICD by manifest.
- [Mesa Wayland WSI source](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/src/vulkan/wsi/wsi_common_wayland.c): software presentation has a `wl_shm` path. The GNOME compositor still uses its GPU; this does not isolate the entire desktop from its driver.
- [NVIDIA upstream #1363](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1363): reports include silent whole-host hangs with 615.71.09, but different GPU/kernel/trigger. This supports investigating the driver path; it does not prove a matching local defect or a particular driver update fixes SSK.

## Operator controls and remaining acceptance

Normal launch from `dist/ssk-linux-x64` uses its software config. To force the
same policy with another config: `./ssk --graphics software`. Expected ready
record: `backend=Vulkan`, `device_type=Cpu`, Mesa/llvmpipe driver, native Wayland
in this session. Simulation remains 360 Hz and game datagrams 60 Hz; FIFO remains
monitor/compositor paced. Software rendering costs CPU and its FPS is unmeasured.

`--graphics hardware` deliberately returns to GPU rendering. `--gpu-index N`
selects a specific surface-compatible hardware adapter. Candidate/selected
events record indexes; an index is local to that enumeration, not a PCI address
or a stable GPU identifier. Invalid indexes fail instead of silently changing
devices. Stable selection is now also available through --gpu-uuid HEX or
--gpu-pci dddd:bb:dd.f; identities are queried on already enumerated Vulkan
adapters with API/extension guards. No affinity pinning or undocumented
main-thread rendering fork exists. Current timed scene, worker comparison,
FIFO/Mailbox controls and operator gates: [FREEZE_RUNBOOK.md](FREEZE_RUNBOOK.md).

No GUI, GPU enumeration probe, stress load or crash was launched automatically.
Compilation/build evidence is in PLAN/CHECKPOINT. Runtime acceptance is still
required: software mode must be shown usable and stable on the owner's desktop;
hardware mode remains an open release blocker until incident stacks or a
controlled comparison identify and resolve its trigger. Software success alone
would implicate the changed application driver/presentation path, not prove a
specific NVIDIA bug or exclude the compositor/OS.

## Attended comparisons on2026-10-20

The owner authorized the staged experiments. The same binary has completed CPU
static, explicit display-GPU static, same GPU+production worker, offline game,
host/no-audio and host/audio-enabled runs without reproducing the hard freeze.
A static scene also passed on the other/headless GPU. Exact durations, hashes,
caveats and session paths are in CHECKPOINT/PLAN; private captures stay in runtime.
These are short comparisons, not the hardware release gate. Healthy samples
include completed DRM timeline waits; those waits alone are not a deadlock.
Hardware comparisons explicitly selected an adapter, NVIDIA-only ICD and disabled
implicit layers. The original incident's physical GPU/loader policy is unknown.

Additional primary incident reports inspected2026-10-20:
[NVIDIA #1289](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1289),
[#1371](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1371),
[#1394](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1394).
Reporters describe modeset/DPMS/resume-triggered display hangs, including
615.71.09, with KMS blocked in NVIDIA locking and a stuck modeset worker. Their
triggers/hardware differ; no matching local freeze stack has been captured.
Do not infer a driver fix, perform DPMS/power-cycle reproduction, or update the
driver based on these reports alone. Compare concrete KMS/NVIDIA stacks if a
local incident occurs. No power/kernel/driver settings were changed.
