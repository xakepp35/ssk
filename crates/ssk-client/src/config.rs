//! Presentation settings. Network configuration never owns window or UI state.
use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    pub fullscreen: bool,
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            fullscreen: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioConfig {
    /// False skips the native audio context, not just sound playback.
    pub enabled: bool,
}
impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_AUDIO_ENABLED,
        }
    }
}
