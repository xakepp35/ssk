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
- Completed B4: Bounded ACK/retry events, target-owned damage and death attribution, two-second life-epoch respawn, menu restrictions and duel instructions. Offline graphical session observed earlier; full manual duel acceptance remains pending.
- `cargo check --workspace` passed at this step. No automated tests run.
- No manual play session yet. No public hosts for internet acceptance.
- No active project processes. Source discussions preserved. No push.

## Next
B5: signed origin envelopes, two-second history and bounded shot/movement/death validation.

## Constraints
240 Hz simulation; 30–60 game PPS/link; three libraries, one binary.
Owner-authoritative ships, at most one intermediary, bounded queues.
Do not claim gameplay/network/performance acceptance from compilation alone.
Read PLAN.md for all completed milestones; README.md for design.
