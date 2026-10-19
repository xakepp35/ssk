//! Opt-in startup diagnostics. No network, simulation, profile or native audio.
use crate::{WindowConfig, constants::*};
use macroquad::{miniquad, prelude::*};
use std::{fs::File, io::Write, path::Path, rc::Rc, time::Instant};

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

pub(crate) struct StartupLog {
    file: Option<File>,
    started: Instant,
}
impl StartupLog {
    pub(crate) fn open(path: Option<&Path>) -> std::io::Result<Self> {
        let file = path
            .map(|path| {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)?;
                }
                let mut options = std::fs::OpenOptions::new();
                options.create(true).append(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(DIAGNOSTIC_FILE_MODE);
                }
                options.open(path)
            })
            .transpose()?;
        Ok(Self {
            file,
            started: Instant::now(),
        })
    }
    pub(crate) fn mark(&self, stage: &str) {
        eprintln!("SSK startup: {stage}");
        if let Some(mut file) = self.file.as_ref() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let line = format!(
                "{}.{:03} pid={} +{}ms {stage}\n",
                timestamp.as_secs(),
                timestamp.subsec_millis(),
                std::process::id(),
                self.started.elapsed().as_millis(),
            );
            if let Err(error) = file
                .write_all(line.as_bytes())
                .and_then(|()| file.sync_data())
            {
                eprintln!("SSK startup log write/sync failed: {error}");
            }
        }
    }
}

/// One clear frame and exit, using the same window settings as normal launch.
/// `native` stops below Macroquad, separating its resource setup from Miniquad.
pub fn graphics_probe(
    window: &WindowConfig,
    native: bool,
    backend: GraphicsBackend,
    path: &Path,
) -> std::io::Result<()> {
    if !cfg!(target_os = "linux") && !matches!(backend, GraphicsBackend::Default) {
        return Err(std::io::Error::other("GLX/EGL overrides are Linux-only"));
    }
    let log = Rc::new(StartupLog::open(Some(path))?);
    log.mark(&format!(
        "probe begin: layer={}, backend={backend:?}, {}x{}, vsync={VSYNC_INTERVAL}; no SSK game/network/audio",
        if native { "miniquad" } else { "macroquad" },
        window.width,
        window.height,
    ));
    let conf = window_conf(window, backend);
    log.mark("entering graphics/window initialization");
    let stage_log = log.clone();
    if native {
        miniquad::start(conf, move || {
            stage_log.mark("native window/context ready; entering rendering backend constructor");
            let context = miniquad::window::new_rendering_backend();
            stage_log.mark("native rendering backend ready");
            Box::new(NativeProbe {
                context,
                log: stage_log,
                drew_frame: false,
                completed: false,
            })
        });
    } else {
        macroquad::Window::from_config(conf, async move {
            stage_log.mark("Macroquad context/resources ready; first future poll");
            clear_background(SPACE_COLOR);
            stage_log.mark("Macroquad clear queued; awaiting first draw/present");
            next_frame().await;
            stage_log.mark("Macroquad loop resumed after first draw/present");
        });
    }
    log.mark("graphics event loop and window teardown returned");
    Ok(())
}

struct NativeProbe {
    context: Box<dyn miniquad::RenderingBackend>,
    log: Rc<StartupLog>,
    drew_frame: bool,
    completed: bool,
}
impl miniquad::EventHandler for NativeProbe {
    fn update(&mut self) {
        if self.drew_frame && !self.completed {
            self.log
                .mark("native event loop resumed after first swap/flush");
            self.completed = true;
            miniquad::window::order_quit();
        }
    }
    fn draw(&mut self) {
        if self.drew_frame {
            return;
        }
        self.log.mark("native first clear/commit begin");
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
        self.log
            .mark("native first clear/commit returned; swap/flush follows");
    }
}
