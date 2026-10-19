# Linux hard-freeze investigation — 2026-10-19

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
Whether the owner separately launched a game is awaiting clarification.

The preceding boot contains **231,225** USB submission errors for the same
USB 1-4 audio path, 06:20:02–06:33:19: 229,260 on URB #0, 1,964 on #8 and one
on #7, all error -16. No matching Xid, OOM-kill, kernel panic, lockup, hardware
error, general-protection-fault or kernel BUG record was found in the searched
kernel journal. No warning-or-higher entries were found after 06:45. The gap
between the USB errors and the last journal entry still prevents causal proof.

The interrupted build's /tmp log disappeared after reboot; no successful release
completion can be claimed. Existing Linux/Windows executable timestamps are
06:08–06:13, before E1/E2, so **the distributed packages do not yet contain these
fixes**. No Cargo/rustc/game build or game process remains. Do not restart builds
or launch a reproduction automatically while the incident is unresolved.
The next action is to establish whether the repeat occurred during compilation
alone or after a separate game launch, and whether network/audio stayed alive.
