//! Shared Vulkan window setup and opt-in graphics-only probe.
use crate::{WindowConfig, constants::*};
use eframe::{egui, egui_wgpu, wgpu};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use std::sync::Arc;

pub(crate) fn native_options(window: &WindowConfig) -> eframe::NativeOptions {
    let mut setup = egui_wgpu::WgpuSetupCreateNew::default();
    // Deliberately override WGPU_BACKEND: this application has no GL backend.
    setup.instance_descriptor.backends = wgpu::Backends::VULKAN;
    eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Syrup Space Kadet — Vulkan")
            .with_app_id("ssk")
            .with_icon(egui::IconData::default())
            .with_inner_size([window.width as f32, window.height as f32])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_fullscreen(window.fullscreen),
        vsync: true,
        persist_window: false,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: Some(MAX_FRAMES_IN_FLIGHT),
            wgpu_setup: setup.into(),
            on_surface_error: Arc::new(|error| {
                // wgpu/winit reconfigure on resize; retain simulation state and
                // skip a failed frame rather than spinning on surface acquisition.
                tracing::warn!(name:"ssk.client.graphics.surface_error",?error,"Surface frame unavailable");
                match error {
                    wgpu::SurfaceError::Lost => egui_wgpu::SurfaceErrorAction::RecreateSurface,
                    wgpu::SurfaceError::OutOfMemory => {
                        panic!("Vulkan surface exhausted GPU memory")
                    }
                    _ => egui_wgpu::SurfaceErrorAction::SkipFrame,
                }
            }),
        },
        ..Default::default()
    }
    // Winit 0.30 prefers WAYLAND_DISPLAY/WAYLAND_SOCKET over DISPLAY on Linux.
    // It only chooses X11 when a Wayland display is absent; neither path uses GL.
}

pub(crate) fn log_ready(cc: &eframe::CreationContext<'_>) {
    let window_system = match cc.display_handle().map(|h| h.as_raw()) {
        Ok(RawDisplayHandle::Wayland(_)) => "wayland",
        Ok(RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_)) => "x11",
        Ok(RawDisplayHandle::Windows(_)) => "windows",
        _ => "other",
    };
    if let Some(state) = &cc.wgpu_render_state {
        let gpu = state.adapter.get_info();
        tracing::info!(name:"ssk.client.graphics.ready",window_system,backend=?gpu.backend,
            adapter=%gpu.name,driver=%gpu.driver,driver_info=%gpu.driver_info,device_type=?gpu.device_type,
            vendor_id=gpu.vendor,device_id=gpu.device,surface_format=?state.target_format,
            present_mode="fifo",max_frames_in_flight=MAX_FRAMES_IN_FLIGHT,
            "Vulkan device and window initialized");
    }
}

/// Uses exactly the client's window/device/painter path, without a game, profile,
/// network or audio. Only run by explicit operator command after hard freezes.
pub fn graphics_probe(window: &WindowConfig) -> std::io::Result<()> {
    tracing::info!(name:"ssk.probe.begin",backend="vulkan",width=window.width,height=window.height,
        "Graphics-only probe; no SSK game/network/audio");
    let initializing =
        tracing::debug_span!("ssk.probe.graphics.initialize", watchdog = true).entered();
    eframe::run_native(
        "SSK Vulkan probe",
        native_options(window),
        Box::new(move |cc| {
            drop(initializing);
            log_ready(cc);
            Ok(Box::new(Probe { frames: 0 }))
        }),
    )
    .map_err(|error| std::io::Error::other(error.to_string()))?;
    tracing::info!(name:"ssk.probe.end","Vulkan event loop and teardown returned");
    Ok(())
}
struct Probe {
    frames: u32,
}
impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.frames > 0 {
            tracing::debug!(name:"ssk.probe.frame.returned","Event loop called update after the first queued frame");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            let painter = ctx.layer_painter(egui::LayerId::background());
            painter.rect_filled(ctx.content_rect(), 0., SPACE_COLOR.egui());
            painter.text(
                ctx.content_rect().center(),
                egui::Align2::CENTER_CENTER,
                "SSK / wgpu / Vulkan",
                egui::FontId::proportional(RESULT_SCORE_FONT),
                palette::SKYBLUE.egui(),
            );
            tracing::debug!(name:"ssk.probe.frame.queued","First Vulkan probe frame queued; presentation follows update");
            ctx.request_repaint();
        }
        self.frames += 1;
    }
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [SPACE_COLOR.r, SPACE_COLOR.g, SPACE_COLOR.b, 1.]
    }
}
