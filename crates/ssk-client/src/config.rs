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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresentMode {
    #[default]
    Fifo,
    Mailbox,
}
impl PresentMode {
    pub(crate) fn wgpu(self) -> eframe::wgpu::PresentMode {
        match self {
            Self::Fifo => eframe::wgpu::PresentMode::Fifo,
            Self::Mailbox => eframe::wgpu::PresentMode::Mailbox,
        }
    }
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
    /// Stable Vulkan deviceUUID (32 hexadecimal digits), unlike an enumeration index.
    pub adapter_uuid: Option<String>,
    /// Canonical PCI address, e.g. 0000:65:00.0. No silent index/name fallback.
    pub adapter_pci: Option<String>,
    pub present_mode: PresentMode,
    /// Idle repaint cadence only for explicit Mailbox diagnostics; FIFO uses vsync.
    pub mailbox_repaint_hz: u32,
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            fullscreen: false,
            adapter: GraphicsAdapter::Hardware,
            adapter_index: None,
            adapter_uuid: None,
            adapter_pci: None,
            present_mode: PresentMode::Fifo,
            mailbox_repaint_hz: DEFAULT_MAILBOX_REPAINT_HZ,
        }
    }
}
impl WindowConfig {
    pub fn validate(&self) -> std::io::Result<()> {
        let selectors = [
            self.adapter_index.is_some(),
            self.adapter_uuid.is_some(),
            self.adapter_pci.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        let valid_pci = self.adapter_pci.as_ref().is_none_or(|pci| {
            let parts: Vec<_> = pci.split([':', '.']).collect();
            parts.len() == GPU_PCI_COMPONENT_WIDTHS.len()
                && parts
                    .iter()
                    .zip(GPU_PCI_COMPONENT_WIDTHS)
                    .all(|(part, width)| {
                        part.len() == width && part.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                && pci == &format!("{}:{}:{}.{}", parts[0], parts[1], parts[2], parts[3])
        });
        let invalid = selectors > 1
            || self.adapter == GraphicsAdapter::Software && selectors > 0
            || !valid_pci
            || self.adapter_uuid.as_ref().is_some_and(|uuid| {
                uuid.len() != GPU_UUID_HEX_LEN || !uuid.bytes().all(|b| b.is_ascii_hexdigit())
            })
            || !MAILBOX_REPAINT_HZ_RANGE.contains(&self.mailbox_repaint_hz);
        if invalid {
            return Err(std::io::Error::other(
                "Invalid Vulkan selection: use hardware with one index, 32-hex UUID or PCI dddd:bb:dd.f; mailbox repaint Hz must be 1..=1000",
            ));
        }
        Ok(())
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
