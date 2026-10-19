use crate::constants::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub records: [u64; 3],
    pub volume: f32,
    pub shake: f32,
    pub flashes: bool,
    pub ui_scale: f32,
    pub fullscreen: bool,
    pub width: i32,
    pub height: i32,
    pub class: usize,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            records: [0; 3],
            volume: DEFAULT_VOLUME,
            shake: DEFAULT_SHAKE,
            flashes: true,
            ui_scale: 1.,
            fullscreen: false,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            class: 1,
        }
    }
}
impl Profile {
    pub fn path() -> PathBuf {
        ssk_net::data_dir().join("profile.json")
    }
    pub fn load() -> Self {
        let mut profile: Self = std::fs::read(Self::path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        profile.volume = profile.volume.clamp(0., 1.);
        profile.shake = profile.shake.clamp(0., 1.);
        profile.ui_scale = profile
            .ui_scale
            .clamp(*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end());
        profile.width = profile
            .width
            .clamp(*WINDOW_WIDTH_RANGE.start(), *WINDOW_WIDTH_RANGE.end());
        profile.height = profile
            .height
            .clamp(*WINDOW_HEIGHT_RANGE.start(), *WINDOW_HEIGHT_RANGE.end());
        profile.class = profile.class.min(ssk_sim::ShipClass::ALL.len() - 1);
        profile
    }
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temp, path)
    }
}
