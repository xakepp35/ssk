//! Presentation, process and bounded client queue defaults. Gameplay lives in ssk-sim.
pub const GAME_COMMAND_CAPACITY: usize = 4;
pub const MAX_RENDER_DELTA: f32 = 0.1;
pub const DEFAULT_WIDTH: i32 = 1440;
pub const DEFAULT_HEIGHT: i32 = 900;
pub const VSYNC_INTERVAL: i32 = 1;
pub const CLIENT_EVENT_QUEUE: usize = 256;
pub const MAX_RECEIVE_EVENTS: usize = 512;
pub const SHOT_DEDUP_CAPACITY: usize = 4096;
pub const DEATH_DEDUP_CAPACITY: usize = 1024;
pub const LEAVE_GRACE_MILLIS: u64 = 150;
