//! Opt-in startup diagnostics. No network, simulation, profile or native audio.
use crate::{WindowConfig, constants::*};
use macroquad::{miniquad, prelude::*};

#[derive(Clone, Copy, Debug, Default)]
pub enum GraphicsBackend {
    #[default]
    Default,
    Glx,
    Egl,
}

pub(crate) fn window_conf(window: &WindowConfig, backend: GraphicsBackend) -> Conf {
    let mut conf = Conf {
        window_title: "Syrup Space Kadet".into(),
        window_width: window.width,
        window_height: window.height,
        fullscreen: window.fullscreen,
        high_dpi: true,
        platform: miniquad::conf::Platform {
            swap_interval: Some(VSYNC_INTERVAL),
            ..Default::default()
        },
        ..Default::default()
    };
    // Explicit diagnostic override only; normal startup retains upstream defaults.
    match backend {
        GraphicsBackend::Default => {}
        GraphicsBackend::Glx => conf.platform.linux_x11_gl = miniquad::conf::LinuxX11Gl::GLXOnly,
        GraphicsBackend::Egl => conf.platform.linux_x11_gl = miniquad::conf::LinuxX11Gl::EGLOnly,
    }
    conf
}

/// One clear frame and exit, using the same window settings as normal launch.
/// `native` stops below Macroquad, separating its resource setup from Miniquad.
pub fn graphics_probe(
    window: &WindowConfig,
    native: bool,
    backend: GraphicsBackend,
) -> std::io::Result<()> {
    if !cfg!(target_os = "linux") && !matches!(backend, GraphicsBackend::Default) {
        return Err(std::io::Error::other("GLX/EGL overrides are Linux-only"));
    }
    tracing::info!(name:"ssk.probe.begin",native,backend=?backend,width=window.width,height=window.height,
        vsync=VSYNC_INTERVAL,"Graphics-only probe; no SSK game/network/audio");
    let conf = window_conf(window, backend);
    let initializing =
        tracing::debug_span!("ssk.probe.graphics.initialize", watchdog = true).entered();
    if native {
        miniquad::start(conf, move || {
            drop(initializing);
            tracing::debug!(name:"ssk.probe.native.context.ready","Native window/context ready");
            let _backend =
                tracing::debug_span!("ssk.probe.native.backend.initialize", watchdog = true)
                    .entered();
            let context = miniquad::window::new_rendering_backend();
            tracing::debug!(name:"ssk.probe.native.backend.ready","Native rendering backend ready");
            Box::new(NativeProbe {
                context,
                drew_frame: false,
                completed: false,
            })
        });
    } else {
        macroquad::Window::from_config(conf, async move {
            drop(initializing);
            tracing::debug!(name:"ssk.probe.macroquad.context.ready","Macroquad resources ready; first future poll");
            clear_background(SPACE_COLOR);
            tracing::debug!(name:"ssk.probe.macroquad.frame.queued","Clear queued; draw/present follows");
            next_frame().await;
            tracing::debug!(name:"ssk.probe.macroquad.frame.returned","Loop resumed after first draw/present");
        });
    }
    tracing::info!(name:"ssk.probe.end","Graphics loop and window teardown returned");
    Ok(())
}

struct NativeProbe {
    context: Box<dyn miniquad::RenderingBackend>,
    drew_frame: bool,
    completed: bool,
}
impl miniquad::EventHandler for NativeProbe {
    fn update(&mut self) {
        if self.drew_frame && !self.completed {
            tracing::debug!(name:"ssk.probe.native.frame.returned","Event loop resumed after first swap/flush");
            self.completed = true;
            miniquad::window::order_quit();
        }
    }
    fn draw(&mut self) {
        if self.drew_frame {
            return;
        }
        tracing::debug!(name:"ssk.probe.native.frame.begin","Native first clear/commit begin");
        self.context
            .begin_default_pass(miniquad::PassAction::clear_color(
                SPACE_COLOR.r,
                SPACE_COLOR.g,
                SPACE_COLOR.b,
                SPACE_COLOR.a,
            ));
        self.context.end_render_pass();
        self.context.commit_frame();
        self.drew_frame = true;
        tracing::debug!(name:"ssk.probe.native.frame.committed","Clear/commit returned; swap/flush follows");
    }
}
