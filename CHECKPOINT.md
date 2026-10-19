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
- Completed B8: Signed bounded link-metric deltas, measured two-leg route costs, three prepared critical paths, E2E ACK/retry/dedup with visible expiry, hysteresis, per-origin queue admission, 8 KiB/s control token budget and aggregate payload send budget. Failure scenarios remain unexercised.
- `cargo check --workspace` passed at this step. No automated tests run.
- No manual play session yet. No public hosts for internet acceptance.
- No active project processes. Source discussions preserved. No push.

## Next
B9: libp2p Kademlia bootstrap, identity-bound signed presence records and headless discovery.

## Constraints
240 Hz simulation; 30–60 game PPS/link; three libraries, one binary.
Owner-authoritative ships, at most one intermediary, bounded queues.
Do not claim gameplay/network/performance acceptance from compilation alone.
Read PLAN.md for all completed milestones; README.md for design.
