//! Presentation, process and bounded client queue defaults. Gameplay lives in ssk-sim.
pub const GAME_COMMAND_CAPACITY: usize = 4;
pub const MAX_RENDER_DELTA: f32 = 0.1;
pub const DEFAULT_WIDTH: i32 = 1600;
pub const DEFAULT_HEIGHT: i32 = 1000;
pub const VSYNC_INTERVAL: i32 = 1;
pub const CLIENT_EVENT_QUEUE: usize = 256;
pub const MAX_RECEIVE_EVENTS: usize = 512;
pub const SHOT_DEDUP_CAPACITY: usize = 4096;
pub const DEATH_DEDUP_CAPACITY: usize = 1024;
pub const LEAVE_GRACE_MILLIS: u64 = 150;

pub const DEFAULT_VOLUME: f32 = 0.6;
pub const DEFAULT_AUDIO_ENABLED: bool = true;
pub const DEFAULT_SHAKE: f32 = 0.7;
pub const UI_SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.75..=1.25;
pub const WINDOW_WIDTH_RANGE: std::ops::RangeInclusive<i32> = 800..=7680;
pub const WINDOW_HEIGHT_RANGE: std::ops::RangeInclusive<i32> = 600..=4320;
pub const MAX_PARTICLES: usize = 2400;
pub const MAX_RINGS: usize = 64;
pub const EFFECT_RNG_SEED: u64 = 82;
pub const WAVE_RNG_SEED: u64 = 118;
pub const SOUND_PRESETS: [(f32, f32, f32); 7] = [
    (160., 0.055, 0.65),
    (95., 0.13, 0.85),
    (480., 0.20, 0.12),
    (250., 0.065, 0.8),
    (65., 0.35, 0.9),
    (880., 0.14, 0.02),
    (110., 0.07, 0.05),
];
pub const PARTICLE_LIFE: std::ops::Range<f32> = 0.12..0.5;
pub const PARTICLE_MIN_SPEED: f32 = 30.;
pub const PARTICLE_SIZE: std::ops::Range<f32> = 1. ..3.5;
pub const EFFECT_DISTANCE: f32 = 1800.;
pub const SOUND_DISTANCE: f32 = 2100.;
pub const SOUND_GAIN: f32 = 0.5;
pub const MUZZLE_PARTICLES: usize = 4;
pub const MUZZLE_PARTICLE_SPEED: f32 = 110.;
pub const IMPACT_PARTICLE_DAMAGE: f32 = 12.;
pub const IMPACT_PARTICLE_COUNT: std::ops::RangeInclusive<f32> = 4. ..=20.;
pub const IMPACT_PARTICLE_SPEED: f32 = 210.;
pub const IMPACT_SOUND_GAIN: f32 = 0.6;
pub const DESTRUCTION_PARTICLES: usize = 32;
pub const DESTRUCTION_PARTICLE_SPEED: f32 = 300.;
pub const RING_LIFE: f32 = 0.4;
pub const RING_SCALE: f32 = 2.;
pub const SHAKE_IMPACT: f32 = 2.;
pub const SHAKE_MAX: f32 = 8.;
pub const PICKUP_PARTICLES: usize = 18;
pub const PICKUP_PARTICLE_SPEED: f32 = 180.;
pub const PARTICLE_DRAG: f32 = 3.;
pub const SHAKE_DECAY: f32 = 15.;
pub const FLASH_DECAY: f32 = 5.;
pub const PARTICLE_TAIL_SECONDS: f32 = 0.025;
pub const FLASH_COLOR: [f32; 4] = [0.9, 0.12, 0.08, 0.15];
pub const AUDIO_SAMPLE_RATE: u32 = 44100;
pub const AUDIO_CHANNELS: u16 = 1;
pub const AUDIO_BITS: u16 = 16;
pub const AUDIO_SAMPLE_BYTES: u16 = AUDIO_CHANNELS * AUDIO_BITS / 8;
pub const WAVE_HEADER_BYTES: usize = 44;
pub const WAVE_FMT_BYTES: u32 = 16;
pub const WAVE_PCM_FORMAT: u16 = 1;
pub const AUDIO_PITCH_DECAY: f32 = 0.6;
pub const AUDIO_ATTACK: f32 = 80.;
pub const AUDIO_AMPLITUDE: f32 = 22000.;
pub const BENCH_MAX_SECONDS: u64 = 3600;
pub const BENCH_MAX_WARMUP: u64 = 300;
pub const BENCH_DEFAULT_PLAYERS: usize = 2;
pub const BENCH_DEFAULT_SECONDS: u64 = 15;
pub const BENCH_DEFAULT_WORKERS: usize = 2;
pub const BENCH_MAX_WORKERS: usize = 4;
pub const BENCH_STARTUP_SECONDS: u64 = 30;
pub const BENCH_RELAY_SECONDS: u64 = 8;
pub const BENCH_POLL_MILLIS: u64 = 10;
pub const BENCH_RELAY_POLL_MILLIS: u64 = 50;
pub const BENCH_MAX_STEP_SAMPLES: usize = 500_000;
pub const BENCH_ORBIT_SPEED: f32 = 0.5;
pub const BENCH_ORBIT_RADIUS: f32 = 300.;
pub const BENCH_FIRE_PERIOD: f64 = 2.;
pub const BENCH_FIRE_SECONDS: f64 = 1.3;
pub const BENCH_ABILITY_PERIOD: f64 = 8.;
pub const BENCH_ABILITY_SECONDS: f64 = 0.7;
pub const BENCH_FRESH_SECONDS: f64 = 0.25;

/// Shared visual theme and layout, in logical pixels before UI scaling.
pub mod ui {
    use macroquad::prelude::Color;
    pub const BUTTON_SELECTED: Color = Color::new(0.08, 0.27, 0.34, 1.);
    pub const BUTTON_HOVER: Color = Color::new(0.12, 0.2, 0.29, 1.);
    pub const BUTTON_IDLE: Color = Color::new(0.05, 0.09, 0.16, 1.);
    pub const BUTTON_BORDER: Color = Color::new(0.15, 0.25, 0.35, 1.);
    pub const MENU_BACKGROUND: Color = Color::new(0.015, 0.025, 0.055, 0.95);
    pub const HUD_BACKGROUND: Color = Color::new(0.02, 0.04, 0.09, 0.85);
    pub const BAR_BACKGROUND: Color = Color::new(0.08, 0.12, 0.18, 1.);
    pub const WEAPON_BACKGROUND: Color = Color::new(0.025, 0.05, 0.10, 0.9);
    pub const HELP_BACKGROUND: Color = Color::new(0.01, 0.02, 0.05, 0.92);
    pub const SETTING_WRAP: f32 = 0.99;
    pub const VOLUME_STEP: f32 = 0.2;
    pub const SETTING_STEP: f32 = 0.25;
    pub const SCALE_MIN: f32 = 0.75;
    pub const SCALE_WRAP: f32 = 1.24;
    pub const OVERCAP_BAR_HEIGHT: f32 = 4.;
    pub const WEAPON_TEXT_INSET: f32 = 8.;
    pub const TEXT_INSET: f32 = 12.;
    pub const BAR_HEIGHT: f32 = 14.;
    pub const FONT_BAR: f32 = 15.;
    pub const HUD_MARGIN: f32 = 16.;
    pub const FONT_CAPTION: f32 = 17.;
    pub const FONT_BODY: f32 = 20.;
    pub const FONT_DESCRIPTION: f32 = 21.;
    pub const FONT_BUTTON: f32 = 22.;
    pub const FONT_STATS: f32 = 23.;
    pub const PERF_TOP: f32 = 26.;
    pub const BUTTON_BASELINE: f32 = 27.;
    pub const HUD_INSET: f32 = 28.;
    pub const BAR_LABEL_X: f32 = 30.;
    pub const BAR_ROW_HEIGHT: f32 = 32.;
    pub const HELP_ROW_HEIGHT: f32 = 33.;
    pub const COOLDOWN_TOP: f32 = 38.;
    pub const BUTTON_HEIGHT: f32 = 40.;
    pub const WEAPON_HEIGHT: f32 = 44.;
    pub const FONT_TITLE: f32 = 54.;
    pub const TITLE_TOP: f32 = 55.;
    pub const SUBTITLE_TOP: f32 = 105.;
    pub const WEAPON_INFO_TOP: f32 = 137.;
    pub const TAGLINE_TOP: f32 = 142.;
    pub const HUD_HEIGHT: f32 = 150.;
    pub const RECORD_TOP: f32 = 158.;
    pub const WEAPON_WIDTH: f32 = 164.;
    pub const WEAPON_STRIDE: f32 = 172.;
    pub const CLASSES_TOP: f32 = 175.;
    pub const COLUMN_WIDTH: f32 = 205.;
    pub const COLUMN_STRIDE: f32 = 220.;
    pub const CLASS_STATS_TOP: f32 = 250.;
    pub const WEAPONS_HALF_WIDTH: f32 = 252.;
    pub const CLASS_DESCRIPTION_TOP: f32 = 282.;
    pub const BAR_WIDTH: f32 = 290.;
    pub const FULLSCREEN_WIDTH: f32 = 310.;
    pub const START_TOP: f32 = 315.;
    pub const PERF_WIDTH: f32 = 320.;
    pub const MENU_HALF_WIDTH: f32 = 330.;
    pub const FULLSCREEN_X: f32 = 335.;
    pub const HELP_HALF_WIDTH: f32 = 360.;
    pub const NEW_RUN_TOP: f32 = 367.;
    pub const SETTINGS_TOP: f32 = 430.;
    pub const FLASHES_X: f32 = 440.;
    pub const DISPLAY_TOP: f32 = 482.;
    pub const CONTROLS_TOP: f32 = 570.;
    pub const MENU_WIDTH: f32 = 660.;
    pub const QUIT_TOP: f32 = 680.;
    pub const MENU_HEIGHT: f32 = 740.;
    pub const MENU_REFERENCE_HEIGHT: f32 = 850.;
    pub const HUD_REFERENCE_HEIGHT: f32 = 900.;
    pub const WEAPONS_BOTTOM: f32 = 64.;
}

pub const CAMERA_LOOKAHEAD: f32 = 200.;
pub const CAMERA_LOOKAHEAD_SCALE: f32 = 0.25;
pub const CAMERA_SMOOTHING: f32 = 12.;
pub const SPAWN_SCREEN_MARGIN: f32 = 200.;
pub const HIT_SHAKE: f32 = 3.;
pub const PEER_STALE_SECONDS: f64 = 0.15;
pub const PEER_EDGE_MARGIN: f32 = 30.;
pub const SCENE_CULL_MARGIN: f32 = 100.;
pub const ROCK_VERTICES: u64 = 9;
pub const ROCK_SHAPE_BUCKETS: u64 = 17;
pub const ROCK_SHAPE_SCALE: f32 = 0.009;
pub const ROCK_SHAPE_BASE: f32 = 0.85;
pub const ROCK_COLOR: macroquad::prelude::Color =
    macroquad::prelude::Color::new(0.35, 0.4, 0.5, 1.);
pub const BULLET_TAIL_SECONDS: f32 = 0.012;
pub const PICKUP_PULSE_HZ: f32 = 5.;
pub const PICKUP_PULSE_SCALE: f32 = 0.15;
pub const PICKUP_ICON_RADIUS: f32 = 10.;
pub const PICKUP_ICON_ANGLE: f32 = 45.;
pub const PICKUP_ICON_ARM: f32 = 4.;
pub const SPAWN_RING_RADIUS: f32 = 45.;
pub const SPAWN_EDGE_MARGIN: f32 = 24.;
pub const SPAWN_EDGE_TOP: f32 = 180.;
pub const SPAWN_EDGE_RADIUS: f32 = 12.;

pub const SHIP_NOSE_SCALE: f32 = 1.5;
pub const SHIP_WING_SCALE: f32 = 0.8;
pub const SHIP_FILL_BRIGHTNESS: f32 = 0.15;
pub const SHIP_CORE_RADIUS: f32 = 3.;
pub const ABILITY_RING_OFFSET: f32 = 9.;
pub const ABILITY_RING_SPEED: f32 = 12.;
pub const ABILITY_RING_PULSE: f32 = 2.;
pub const ABILITY_RING_WIDTH: f32 = 3.;
pub const EXHAUST_MIN_SPEED: f32 = 10.;
pub const EXHAUST_OFFSET: f32 = 8.;
pub const EXHAUST_PULSE_SPEED: f32 = 67.;
pub const EXHAUST_PULSE_SIZE: f32 = 4.;
pub const EXHAUST_WIDTH: f32 = 5.;
pub const SPACE_COLOR: macroquad::prelude::Color =
    macroquad::prelude::Color::from_rgba(6, 9, 21, 255);
pub const STAR_LAYERS: usize = 3;
pub const STARS_PER_LAYER: usize = 110;
pub const STAR_PARALLAX: [f32; 2] = [0.1, 0.14];
pub const STAR_POSITION_X: [f32; 2] = [173.73, 2.1];
pub const STAR_POSITION_Y: [f32; 2] = [91.13, 3.3];
pub const STAR_BRIGHTNESS: [f32; 2] = [0.18, 0.11];
pub const STAR_RADIUS: [f32; 2] = [0.6, 0.3];
pub const STAR_TINT: [f32; 2] = [0.07, 0.18];
pub const SPACE_GRID_COLOR: macroquad::prelude::Color =
    macroquad::prelude::Color::new(0.08, 0.16, 0.24, 0.3);
pub const SPACE_GRID_SIZE: f32 = 256.;
pub const SPACE_GRID_COLUMNS: i32 = 8;
pub const SPACE_GRID_ROWS: i32 = 6;
pub const PEER_EDGE_RADIUS: f32 = 10.;
pub const PEER_DISTANCE_OFFSET: [f32; 2] = [25., 16.];
pub const PEER_DISTANCE_FONT: f32 = 18.;
pub const PEER_LABEL_OFFSET: f32 = 40.;
pub const PEER_LABEL_FONT: f32 = 16.;
pub const RESULT_BACKGROUND: macroquad::prelude::Color =
    macroquad::prelude::Color::new(0.01, 0.02, 0.05, 0.8);
pub const RESULT_HALF_WIDTH: f32 = 210.;
pub const RESULT_TITLE_TOP: f32 = 60.;
pub const RESULT_TITLE_FONT: f32 = 56.;
pub const RESULT_SCORE_FONT: f32 = 28.;
pub const RESULT_HELP_TOP: f32 = 50.;
pub const RESULT_HELP_FONT: f32 = 26.;
pub const NOTICE_INSET: f32 = 24.;
pub const NOTICE_BOTTOM: f32 = 100.;
pub const NOTICE_FONT: f32 = 18.;
pub const NETWORK_FONT: f32 = 20.;
pub const NETWORK_STATS_BOTTOM: f32 = 125.;
pub const CROSSHAIR_RADIUS: f32 = 8.;
pub const LINE_WIDTH: f32 = 2.;
pub const BYTES_PER_KIB: f32 = 1024.;
pub const PERCENT: f32 = 100.;
pub const MILLIS_PER_SECOND: f64 = 1000.;
pub const MICROS_PER_SECOND: f64 = 1_000_000.;
pub const QUANTILE_PROBABILITIES: [f64; 5] = [0.05, 0.5, 0.95, 0.99, 1.];

pub const HUD_WIDTH: f32 = 324.;

#[repr(usize)]
pub enum SoundKind {
    MachineGun,
    Shotgun,
    Plasma,
    Impact,
    Destroyed,
    Pickup,
    Dry,
}
