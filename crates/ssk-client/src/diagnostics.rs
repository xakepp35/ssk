//! Shared Vulkan window setup and opt-in graphics-only probe.
use crate::{GraphicsAdapter, WindowConfig, constants::*};
use eframe::{egui, egui_wgpu, wgpu};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use std::sync::Arc;

pub(crate) fn native_options(window: &WindowConfig) -> eframe::NativeOptions {
    let mut setup = egui_wgpu::WgpuSetupCreateNew::default();
    // Deliberately override WGPU_BACKEND: this application has no GL backend.
    setup.instance_descriptor.backends = wgpu::Backends::VULKAN;
    // Do not let name/power heuristics silently choose one of two identical
    // GPUs. Software mode must never fall back to a hardware device.
    let preference = window.adapter;
    let requested_index = window.adapter_index;
    let requested_uuid = window.adapter_uuid.clone();
    let requested_pci = window.adapter_pci.clone();
    let present_mode = window.present_mode.wgpu();
    setup.native_adapter_selector = Some(Arc::new(move |adapters, surface| {
        let identities: Vec<_> = adapters.iter().map(crate::gpu_identity::query).collect();
        for (index, adapter) in adapters.iter().enumerate() {
            let info = adapter.get_info();
            let compatible = surface.is_some_and(|s| adapter.is_surface_supported(s));
            tracing::info!(name:"ssk.client.graphics.adapter_candidate",index,adapter=%info.name,
                driver=%info.driver,driver_info=%info.driver_info,device_type=?info.device_type,
                device_uuid=?identities[index].uuid,pci_address=?identities[index].pci,
                compatible,"Vulkan adapter candidate");
        }
        let acceptable = |adapter: &&wgpu::Adapter| {
            let cpu = adapter.get_info().device_type == wgpu::DeviceType::Cpu;
            cpu == (preference == GraphicsAdapter::Software)
                && surface.is_some_and(|s| adapter.is_surface_supported(s))
        };
        let selected = if let Some(uuid) = &requested_uuid {
            adapters
                .iter()
                .enumerate()
                .find(|(index, adapter)| {
                    acceptable(adapter)
                        && identities[*index]
                            .uuid
                            .as_ref()
                            .is_some_and(|id| id.eq_ignore_ascii_case(uuid))
                })
                .map(|(_, adapter)| adapter)
        } else if let Some(pci) = &requested_pci {
            adapters
                .iter()
                .enumerate()
                .find(|(index, adapter)| {
                    acceptable(adapter)
                        && identities[*index]
                            .pci
                            .as_ref()
                            .is_some_and(|id| id.eq_ignore_ascii_case(pci))
                })
                .map(|(_, adapter)| adapter)
        } else if let Some(index) = requested_index {
            adapters.get(index).filter(acceptable)
        } else {
            adapters.iter().find(acceptable)
        };
        let selected = selected.ok_or_else(|| format!(
            "No surface-compatible {preference:?} Vulkan adapter (index {requested_index:?}, UUID {requested_uuid:?}, PCI {requested_pci:?}); refusing fallback"
        ))?;
        let index = adapters
            .iter()
            .position(|a| a == selected)
            .expect("selected enumerated adapter");
        let capabilities = surface
            .expect("selected surface-compatible adapter")
            .get_capabilities(selected);
        if !capabilities.present_modes.contains(&present_mode) {
            return Err(format!(
                "Requested {present_mode:?} is unsupported on selected Vulkan GPU; supported: {:?}; refusing fallback",
                capabilities.present_modes
            ));
        }
        tracing::info!(name:"ssk.client.graphics.adapter_selected",index,mode=?preference,
            device_uuid=?identities[index].uuid,pci_address=?identities[index].pci,
            present_mode=?present_mode,supported_present_modes=?capabilities.present_modes,
            "Explicit Vulkan adapter/presentation policy applied");
        Ok(selected.clone())
    }));
    eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Syrup Space Kadet — Vulkan")
            .with_app_id("ssk")
            .with_icon(egui::IconData::default())
            .with_inner_size([window.width as f32, window.height as f32])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_fullscreen(window.fullscreen),
        vsync: window.present_mode == crate::PresentMode::Fifo,
        persist_window: false,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            present_mode,
            desired_maximum_frame_latency: Some(MAX_FRAMES_IN_FLIGHT),
            wgpu_setup: setup.into(),
            on_surface_error: Arc::new(|error| {
                // wgpu/winit reconfigure on resize; retain simulation state and
                // skip a failed frame rather than spinning on surface acquisition.
                tracing::warn!(name:"ssk.client.graphics.surface_error",?error,"Surface frame unavailable");
                match error {
                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                        egui_wgpu::SurfaceErrorAction::RecreateSurface
                    }
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

pub(crate) fn log_ready(
    cc: &eframe::CreationContext<'_>,
    owner: &crate::graphics_thread::GraphicsThread,
    window: &WindowConfig,
) {
    owner.assert_current();
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
            present_mode=?window.present_mode,mailbox_repaint_hz=window.mailbox_repaint_hz,
            max_frames_in_flight=MAX_FRAMES_IN_FLIGHT,
            "Vulkan device and window initialized");
    }
}

/// Explicitly timed diagnostics. Graphics are identical with and without the
/// real offline game worker; no profile, identity, socket or audio is opened.
pub struct ProbeOptions {
    pub duration: std::time::Duration,
    pub simulation: bool,
    pub seed: u64,
}

pub fn graphics_probe(window: &WindowConfig, options: ProbeOptions) -> std::io::Result<()> {
    use std::{cell::RefCell, rc::Rc, time::Duration};
    if options.duration.is_zero() || options.duration > Duration::from_secs(PROBE_MAX_SECONDS) {
        return Err(std::io::Error::other(
            "Probe duration must be 1..=PROBE_MAX_SECONDS",
        ));
    }
    window.validate()?;
    let graphics_thread = crate::graphics_thread::GraphicsThread::enter()?;
    tracing::info!(name:"ssk.probe.begin",backend="vulkan",width=window.width,height=window.height,
        duration_ms=options.duration.as_millis() as u64,simulation=options.simulation,
        logic_hz=ssk_sim::TICK_PER_SEC,"Timed static scene; no SSK network/audio/profile");
    let report = Rc::new(RefCell::new(ProbeReport::default()));
    let app_report = report.clone();
    let window = window.clone();
    let initializing =
        tracing::debug_span!("ssk.probe.graphics.initialize", watchdog = true).entered();
    let result = eframe::run_native(
        "SSK Vulkan probe",
        native_options(&window),
        Box::new(move |cc| {
            drop(initializing);
            log_ready(cc, &graphics_thread, &window);
            // Reuse production clock, bounded channels, stop/join and World::step.
            // The rendered scene does not consume or display simulation contents.
            let game = options.simulation.then(|| {
                let (game, _) = crate::game::Game::start(
                    &crate::Options {
                        seed: options.seed,
                        class: None,
                        window: window.clone(),
                        audio: crate::AudioConfig { enabled: false },
                        play: true,
                        online: false,
                        connect: None,
                        network: ssk_net::Config::default(),
                    },
                    ssk_sim::ShipClass::Cadet,
                );
                game
            });
            Ok(Box::new(Probe {
                report: app_report,
                game,
                started: std::time::Instant::now(),
                sampled: std::time::Instant::now(),
                duration: options.duration,
                window,
                graphics_thread,
            }))
        }),
    );
    let report = report.borrow();
    tracing::info!(name:"ssk.probe.end",reason=if result.is_err() { "graphics_error" } else { report.reason },
        update_callbacks=report.callbacks,elapsed_ms=report.elapsed_ms,
        world_tick=report.world_tick,overloads=report.overloads,error=%report.error,
        "Event loop and teardown returned; callback count is not proof of presentation");
    result.map_err(|error| std::io::Error::other(error.to_string()))?;
    if !report.error.is_empty() {
        return Err(std::io::Error::other(report.error.clone()));
    }
    Ok(())
}
struct ProbeReport {
    callbacks: u64,
    elapsed_ms: u64,
    world_tick: u64,
    overloads: u64,
    reason: &'static str,
    error: String,
}
impl Default for ProbeReport {
    fn default() -> Self {
        Self {
            callbacks: 0,
            elapsed_ms: 0,
            world_tick: 0,
            overloads: 0,
            reason: "window_closed",
            error: String::new(),
        }
    }
}
struct Probe {
    report: std::rc::Rc<std::cell::RefCell<ProbeReport>>,
    game: Option<crate::game::Game>,
    started: std::time::Instant,
    sampled: std::time::Instant,
    duration: std::time::Duration,
    window: WindowConfig,
    graphics_thread: crate::graphics_thread::GraphicsThread,
}
impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.graphics_thread.assert_current();
        let mut report = self.report.borrow_mut();
        report.callbacks += 1;
        report.elapsed_ms = self.started.elapsed().as_millis() as u64;
        if let Some(frame) = self.game.as_ref().and_then(|game| game.take_frame()) {
            report.world_tick = frame.world.tick;
            report.overloads = frame.overloads;
            if !frame.error.is_empty() {
                report.reason = "simulation_error";
                report.error = frame.error;
            }
        }
        if self.game.as_ref().is_some_and(|game| game.is_finished()) && report.error.is_empty() {
            report.reason = "simulation_exited";
            report.error = "Simulation worker exited before the probe finished".into();
        }
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(ctx.content_rect(), 0., SPACE_COLOR.egui());
        painter.text(
            ctx.content_rect().center(),
            egui::Align2::CENTER_CENTER,
            "SSK / wgpu / Vulkan",
            egui::FontId::proportional(RESULT_SCORE_FONT),
            palette::SKYBLUE.egui(),
        );
        if report.callbacks == 1 {
            tracing::debug!(name:"ssk.probe.frame.queued","First probe frame queued; presentation follows update");
        }
        if self.sampled.elapsed().as_secs_f64() >= OBSERVATION_SAMPLE_SECONDS {
            tracing::debug!(name:"ssk.probe.progress",kind="metric",update_callbacks=report.callbacks,
                elapsed_ms=report.elapsed_ms,world_tick=report.world_tick,overloads=report.overloads,
                "Main-thread callback progress; not GPU completion");
            self.sampled = std::time::Instant::now();
        }
        if !report.error.is_empty() || self.started.elapsed() >= self.duration {
            if report.error.is_empty() {
                report.reason = "duration_elapsed";
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            request_repaint(ctx, &self.window);
        }
    }
    fn on_exit(&mut self) {
        self.graphics_thread.assert_current();
        // Join before the graphics owner/painter is dropped, as in the client.
        self.game.take();
    }
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [SPACE_COLOR.r, SPACE_COLOR.g, SPACE_COLOR.b, 1.]
    }
}

/// FIFO remains driven by compositor/monitor presentation. Mailbox diagnostics
/// don't self-schedule a busy loop: request a finite idle repaint interval.
/// Native input/expose events may repaint earlier; this is not an FPS guarantee.
pub(crate) fn request_repaint(ctx: &egui::Context, window: &WindowConfig) {
    match window.present_mode {
        crate::PresentMode::Fifo => ctx.request_repaint(),
        crate::PresentMode::Mailbox => ctx.request_repaint_after(
            std::time::Duration::from_secs_f64(1. / f64::from(window.mailbox_repaint_hz)),
        ),
    }
}
