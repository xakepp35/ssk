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
- Completed B15: Implemented real headless moving/firing clients with JSON metrics, signed preintroduced roster, external single-agent mode and bounded peer removal; isolated two-bridge forced-relay Docker scenario supplied. Release build in progress; measurement and NAT execution remain B16 acceptance work.
- `cargo check --workspace` passed at this step. No automated tests run.
- One short offline window observation completed; full class/death/restart and internet acceptance remain open.
- No active project processes. Source discussions preserved. No push.

## Next
B16: run 2/16/64/128 measurements, fix observed bottlenecks, exercise isolated relay and document limits. Active release build exec session 89999.

## Constraints
240 Hz simulation; 30–60 game PPS/link; three libraries, one binary.
Owner-authoritative ships, at most one intermediary, bounded queues.
Do not claim gameplay/network/performance acceptance from compilation alone.
Read PLAN.md for all completed milestones; README.md for design.
