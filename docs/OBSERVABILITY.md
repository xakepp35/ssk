# SSK observability

## Contract / H1 design

Local diagnostics use Rust `tracing` and one process-wide subscriber owned by
the binary. Libraries emit events/spans without choosing files or installing
subscribers. Keep three library crates and one binary. No telemetry server,
network exporter, root daemon or global mutable gameplay state.

Each accepted record is one UTF-8 JSON object plus newline, mirrored to stderr.
The file is written first, flushed and `sync_data` is requested immediately;
stderr is then written and flushed. No asynchronous logging queue can silently
lose the startup tail. This is diagnostic I/O, not a guarantee against failed
storage, a hung kernel or power-loss-unsafe hardware. Synchronous writes can
stall their caller, so hot loops use periodic aggregates, not default per-tick
logging. Bounded files and explicit suppression counters limit runaway output.

### Schema `ssk.obs/1`

| Key | Meaning |
| --- | --- |
| schema, kind | Version; log, metric, span.begin, span.end |
| ts_unix_ms, mono_us | Wall-clock correlation and monotonic time since initialization |
| seq | Sequence assigned by this process's sink |
| event_id | Stable semantic callsite ID, e.g. `ssk.client.graphics.begin` |
| level, target | Severity and emitting subsystem/module |
| run_id, trace_id | Random 128-bit process-run identity; trace rooted in that run |
| span_id, parent_span_id | Local operation identity and parent, when available |
| pid, thread_id, thread_name | Process and emitting Rust thread |
| boot_id | Linux boot correlation; null where unavailable |
| source | File, line and module for the actual callsite |
| fields | Typed booleans/numbers/strings with units in names |

Every SSK logging callsite MUST specify a unique static `name: "ssk.…"`.
That metadata name becomes `event_id`; never derive our IDs from line numbers,
reuse another callsite's ID or interpolate IDs/metric names from peer/user data.
Renaming code must preserve the ID when its meaning remains the same. Source
line is supplemental. Repeated emissions from one callsite retain the ID;
`(run_id, seq)` identifies a particular record. Span lifecycle IDs append
`.begin`/`.end` to the static span name; span_id links the pair. Dependency
events receive a deterministic `ext.*` source hash, stable only for that build.

Duration fields end in `_us`/`_ms`; counters are cumulative or explicitly windowed.
Metrics use `kind=metric`, fixed event IDs and typed fields in the same stream.
These are local logs/traces/metric samples, not a Prometheus server or OTLP
pipeline. Trace context is not added to game packets; no wire-rule change.

### Levels and volume

- ERROR: failure, panic, inability to sustain timing; retained without sampling.
- WARN: recoverable subsystem trouble, stalled progress, queue rejection.
- INFO: process/session lifecycle, selected non-secret settings and availability.
- DEBUG: initialization boundaries and one-second worker/render/network samples.
- TRACE: explicitly enabled frame/tick/datagram details. Intrusive with durable
  I/O and unsuitable for judging 360 Hz performance.

Default filter: own crates DEBUG, dependencies WARN. A per-callsite rate limit
caps repetitive non-error records; totals and suppressed counts are observable.
Log files rotate by size; old SSK session files are retained within a fixed
budget. A dedicated panic file bypasses the ordinary sink mutex, records a
forced process backtrace and syncs immediately. Unwind symbols depend on build
debug information; it is not a kernel or other-thread stack dump.

### Coverage and privacy

Instrument before/after configuration, profile, native graphics, first frame,
game/transport workers, endpoint/discovery setup, audio context/sound requests,
shutdown and errors. Carry spans across std threads and instrument async work;
never hold an entered thread-local span guard across a Tokio `.await`.

The application monitor periodically samples process/system CPU/memory,
Linux pressure, interface state/counters (including TUN), thread states/wait
channels and progress ages. Long-open startup operations are distinguishable
from a stopped render/simulation heartbeat. Missing/denied files are recorded
as unavailable, not invented zeroes. Do not poll GPU ioctls/nvidia-smi in the
watchdog: that could block on the very driver being investigated.

An optional independent user-mode host collector records relevant kernel
journal messages and thread wait-state snapshots. It can keep logging when
only the SSK process/desktop is stuck; it cannot run when the whole kernel
has stopped. Kernel stacks still require available journal traces/kdump or
separately authorized privileged capture. No automatic probe/game/crash launch.

Never log identity/private keys, tickets/invitations, proxy credentials, full
config/environment, input/packet payloads or authentication material. Sensitive
structured field names are redacted, strings/records bounded, files private
on Unix. Free-form dependency/kernel messages can contain sensitive context;
logs remain local and must be reviewed before sharing. No upload/export by default.

## Delivery

H2 implements the sink, CLI controls, panic hook and host/progress monitor.
H3 instruments client/network and adds the independent host collector/query
guide. H4 compiles and builds Linux/Windows sequentially with one Cargo job.
No automated tests, GUI probe, gameplay or freeze reproduction unless requested.

## Available controls (H2)

Every normal launch now initializes logging **before** application config is
read. No flag is required: `runtime/logs/ssk-RUNID-0000.ndjson`; the first
`ssk.observe.ready` record gives its path. stderr contains the same NDJSON.
Stdout remains reserved for explicit command output such as bench reports.
The process run ID is OS-random; no identity key is loaded to construct it.

```sh
# Read-only process/host snapshot; no window or game/transport workers:
./target/release/ssk doctor

# Ordinary launch, optional destination override:
./target/release/ssk --log-file runtime/session.ndjson

# Own TRACE, dependencies WARN, no per-callsite suppression (intrusive):
./target/release/ssk --log-filter 'warn,ssk=trace,ssk_client=trace,ssk_net=trace' --log-rate-limit 0
```

CLI: `--log-dir`, `--log-file` (alias `--startup-log`), `--log-filter`,
`--log-sync always|flush`, `--log-no-console`, `--log-rate-limit N`.
`SSK_LOG` supplies the filter when no CLI filter is given. Default sync is
`always`; `flush` explicitly omits the disk sync. Default rate is 20 accepted
non-error records per event ID per second; 0 disables it, errors bypass it.
Suppressed counts appear on the next accepted record and logger metric samples.
Maximum field 32KiB, record 128KiB; oversized records retain the envelope and
report truncation. Repetitive DEBUG/TRACE output can therefore be incomplete.

Files rotate at 16MiB, up to eight parts for this run (also explicit destinations).
Default-named inactive files are also pruned to eight; advisory-locked files of
other live processes are excluded. The active file path is in logger samples.
Private Unix files/directories use 0600/0700; Windows uses the user's inherited
ACL. Panic/file-write failures use a separate `ssk-RUNID-panic.ndjson` FD without
taking the normal writer mutex. Release builds include line debug information.

Linux host samples include typed memory/IO/CPU-tick counters, pressure, thread
state/wchan and interface counters. TUN interfaces are prioritized before veth
rows. Thread/interface rows are capped and that limitation is exposed. Wait
channels can be null due to permission restrictions; a `wchan` is not a stack.
Other operating systems explicitly report the unavailable proc/sysfs metrics;
portable application/transport metrics remain available. Sampling is one second;
an overdue worker sample or startup operation warns after five seconds.

Primary APIs: [tracing named events](https://docs.rs/tracing/latest/tracing/macro.event.html),
[subscriber layers](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html),
[File::sync_data](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_data).
