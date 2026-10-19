# SSK checkpoint

## Objective
Implement the approved plan through offline alpha, UDP/QUIC duel MVP and
decentralized beta. Continue in small committed steps. Full design: README.md;
ordered work: PLAN.md; stable instructions: AGENTS.md.

## Completed
- A0: saved all agreed artifacts, including PING.md corrections, full gameplay
  baseline and 64–128-player topology/bandwidth requirements.
- Existing source discussions IDEA.md, NET.md, PING.md preserved.

## Live state
- Initial repository, no code yet. Rust 1.99 installed. No tests/builds run.
- No active project processes. No public hosts available for internet acceptance.
- Plan mode ended; implementation authorized. Commit A0, then implement A1.

## Next
Create 3-library/1-binary Rust workspace, Macroquad client, CLI with headless
dispatch. Use compilation checks; don't add/run tests unless requested.

## Constraints
240 Hz sim, 30–60 game PPS/link; one ability/class; owner-authoritative ships;
one intermediary, bounded load; no performance claims without measurement.
No push; no secrets. Update this file before context rollover.
