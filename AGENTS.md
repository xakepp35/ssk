# SSK: instructions for implementation

## Objective and authority

Implement README.md and PLAN.md in order: offline alpha, two-player UDP/QUIC
duel, decentralized PvPvE beta with 64–128 nearby players. The owner authorized
implementation of the approved plan. Continue across local milestones.
README is the agreed design; IDEA.md, NET.md and PING.md are source discussions,
including speculative and incorrect claims. Do not copy those claims as facts.

## Delivery

- One small complete step, one descriptive commit. Update PLAN and CHECKPOINT
  in that commit. Split a step if necessary. Do not push without authorization.
- Prioritize implementation and the complete playable loop. Do not add or run
  automated tests unless requested. Before alpha, use only focused compilation
  checks and a short manual game session when a display is available.
- No test framework, CI project, custom engine, ECS, XDP/eBPF, raw sockets or
  io_uring project before a measured game bottleneck requires it.
- Do not claim gameplay, internet connectivity or 128-player performance was
  verified when only compilation or a topology calculation was performed.
- Three library crates and one binary. Rust 1.99 / edition 2024. Keep changes
  small, bounded queues, stable object IDs, simple spatial indexing.
- Simulation TICK_PER_SEC=360; TICK_PER_PACKET=6, fixed 60 game datagrams/s per connection, including
  forwarding. Account for transport control packets separately.
- Each player owns their ship. Sector steward owns NPCs/asteroids/pickups only.
- Maximum one transit intermediary for game data. Never hide extra transport
  relays behind an application relay and call that a single-intermediary path.
- Do not commit secrets, runtime profiles, caches, logs or build output.

## Persistence and handoff

Keep CHECKPOINT.md concise and secret-free: completed steps with evidence,
current implementation, active processes, limitations, and exact next action.
Update after meaningful phases and before compaction/new threads. Read it and
inspect live git state before resuming. Do not use chat as the sole memory.
Save artifacts before any voluntary context rollover.

## Workstation

For authorized privileged work, prefer graphical Polkit/pkexec with a prepared,
reviewed script. Never ask for passwords in chat or run unrelated root actions.
Follow the active execution policy; do not request an unavailable sandbox mode.
The owner is in the docker group; stale agent groups can use
`sg docker -c 'docker ...'` without sudo. No root work is needed for normal SSK
development. Use one mutation driver for this workspace.

## Timing, parameters and crate boundaries

The owner's latest instruction supersedes the earlier 240 Hz / 30–60 Hz design.
Use ssk-sim constants TICK_PER_SEC=360 and TICK_PER_PACKET=6 everywhere; derive
step duration and packet frequency. Render follows monitor vsync independently.
Move game validation/model/physics to ssk-sim. ssk-net owns only transport,
identity/authentication, serialization, discovery and routing. Centralize tuning
values in named constants/configuration modules; do not add mutable global state.
Do not repeat heavy load scenarios after the reported workstation freeze.
