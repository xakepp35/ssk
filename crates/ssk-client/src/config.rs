//! Presentation settings. Network configuration never owns window or UI state.
use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphicsAdapter {
    #[default]
    Hardware,
    Software,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    pub fullscreen: bool,
    /// Software uses an isolated Mesa Lavapipe Vulkan driver on Linux.
    pub adapter: GraphicsAdapter,
    /// Explicit wgpu enumeration index; distinguishes identical GPUs.
    pub adapter_index: Option<usize>,
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            fullscreen: false,
            adapter: GraphicsAdapter::Hardware,
            adapter_index: None,
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
