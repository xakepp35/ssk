# SSK checkpoint

## Objective
Implement the approved plan through offline alpha, UDP/QUIC duel MVP and
decentralized beta. Continue in small committed steps. Full design: README.md;
ordered work: PLAN.md; stable instructions: AGENTS.md.

## Completed
- A0: saved all agreed artifacts, including PING.md corrections, full gameplay
  baseline and 64–128-player topology/bandwidth requirements.
- Existing source discussions IDEA.md, NET.md, PING.md preserved.

- A2: 240 Hz movement, cursor aim, interpolated ship, aim lookahead camera, procedural stars.
- `cargo check --workspace` passed after A2; no tests/manual session.

## Live state
- A0–A12 and B1–B15 committed. B16 is in progress; no beta tag.
- B16 first fixes: verified-envelope cache, cached structural graph, compact
  protocol v2 events, cached end-to-end ACKs, pending-event deduplication,
  fair ship/checkpoint queues and staggered control with payload token bucket.
- `cargo check --workspace` passes. No automated tests run.
- Baseline loopback 2/16 clients stayed fresh; 64 failed: median fresh fraction
  0.492, p95 replica age 3548 ms, 4585 unconfirmed events. These are diagnostic
  baselines, not beta acceptance. JSON/logs in /tmp/ssk-baseline-{2,16,64}.*.
- One short offline window observation completed; full class/death/restart
  and real internet acceptance remain open. No push.

- B16b: separate E2E receipts and indexed ACK membership. 64-client rerun:
  fresh fraction median 1.0, p95 replica age 98 ms; shared images still missing
  for most clients; cumulative event expiry 2299. Frame is all 64 clients on
  one serial simulation thread (~46 ms), with skipped steps. Not accepted.

- B16c: bounded source multicast (one intermediary), verifying-key cache,
  parallel headless simulations and measured drop/expiry deltas. README weapon
  numbers now match existing simulation; AOI range corrected to 4096.
- Serial 128-client diagnostic failed: ~467 ms aggregate frame, p95 age546 ms,
  missing shared images; this exposed both workload and harness bottlenecks.

## Next
Build release, rerun isolated 64 then 128 moving/firing clients, measure shared
object freshness too. Execute tools/nat two-bridge relay scenario, document
actual results and remaining acceptance limits. No active benchmark; release rebuild in exec session 88655.

## Constraints
240 Hz simulation; 30–60 game PPS/link; three libraries, one binary.
Owner-authoritative ships, at most one intermediary, bounded queues.
Do not claim gameplay/network/performance acceptance from compilation alone.
Read PLAN.md for all completed milestones; README.md for design.
