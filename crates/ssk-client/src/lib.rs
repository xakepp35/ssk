use constants::*;
use eframe::egui;
use graphics::*;
use ssk_sim::{DT, InputFrame, Ship, V2};
use std::time::Instant;
pub mod bench;
mod config;
pub mod constants;
pub use config::{AudioConfig, WindowConfig};
pub mod diagnostics;
mod effects;
mod game;
mod graphics;
mod online;
mod profile;
mod render;
mod starfield;
mod ui;
pub struct Options {
    pub seed: u64,
    pub class: Option<ssk_sim::ShipClass>,
    pub window: WindowConfig,
    pub audio: AudioConfig,
    pub play: bool,
    pub online: bool,
    pub connect: Option<String>,
    pub network: ssk_net::Config,
}
pub fn launch(options: Options) -> std::io::Result<()> {
    let mut profile = profile::Profile::load();
    profile.width = options.window.width;
    profile.height = options.window.height;
    profile.fullscreen = options.window.fullscreen;
    tracing::info!(name:"ssk.client.graphics.begin",backend="vulkan",width=profile.width,height=profile.height,
        present_mode="fifo",fullscreen=profile.fullscreen,"Entering winit/wgpu window and device setup");
    let initializing =
        tracing::debug_span!("ssk.client.graphics.initialize", watchdog = true).entered();
    eframe::run_native(
        "Syrup Space Kadet",
        diagnostics::native_options(&options.window),
        Box::new(move |cc| {
            drop(initializing);
            diagnostics::log_ready(cc);
            Ok(Box::new(ClientApp::new(options, profile)))
        }),
    )
    .map_err(|error| std::io::Error::other(error.to_string()))?;
    tracing::info!(name:"ssk.client.graphics.end","Vulkan event loop and teardown returned");
    Ok(())
}
fn draw_ship(canvas: &Canvas, ship: &Ship, camera: V2, alpha: f32, color: Color) {
    let pos = ship.previous.lerp(ship.pos, alpha);
    let forward = V2::from_angle(ship.angle);
    let side = forward.perp();
    let radius = ship.class.stats().radius;
    let a = canvas.screen(pos + forward * radius * SHIP_NOSE_SCALE, camera);
    let b = canvas.screen(
        pos - forward * radius + side * radius * SHIP_WING_SCALE,
        camera,
    );
    let c = canvas.screen(
        pos - forward * radius - side * radius * SHIP_WING_SCALE,
        camera,
    );
    canvas.draw_triangle(
        a,
        b,
        c,
        Color::new(
            color.r * SHIP_FILL_BRIGHTNESS,
            color.g * SHIP_FILL_BRIGHTNESS,
            color.b * SHIP_FILL_BRIGHTNESS,
            1.,
        ),
    );
    canvas.draw_triangle_lines(a, b, c, 2., color);
    let p = canvas.screen(pos, camera);
    canvas.draw_circle(p.x, p.y, SHIP_CORE_RADIUS, WHITE);
    if ship.ability_active {
        let color = match ship.class {
            ssk_sim::ShipClass::Spark => SKYBLUE,
            ssk_sim::ShipClass::Cadet => GREEN,
            ssk_sim::ShipClass::Thunder => RED,
        };
        canvas.draw_circle_lines(
            p.x,
            p.y,
            radius
                + ABILITY_RING_OFFSET
                + (canvas.get_time() as f32 * ABILITY_RING_SPEED).sin() * ABILITY_RING_PULSE,
            ABILITY_RING_WIDTH,
            color,
        );
    }
    if ship.velocity.length() > EXHAUST_MIN_SPEED {
        let end = canvas.screen(
            pos - forward
                * (radius
                    + EXHAUST_OFFSET
                    + (canvas.get_time() as f32 * EXHAUST_PULSE_SPEED).sin() * EXHAUST_PULSE_SIZE),
            camera,
        );
        let start = canvas.screen(pos - forward * radius, camera);
        canvas.draw_line(start.x, start.y, end.x, end.y, EXHAUST_WIDTH, SKYBLUE);
    }
}
struct ClientApp {
    options: Options,
    profile: profile::Profile,
    class: ssk_sim::ShipClass,
    game: Option<game::Game>,
    view: game::Frame,
    world: ssk_sim::World,
    effects: effects::Effects,
    camera: V2,
    stars: starfield::Starfield,
    weapon: usize,
    menu: bool,
    started: bool,
    help: bool,
    run_number: u64,
    recorded: bool,
    save_error: Option<String>,
    first_frame: bool,
    epoch: Instant,
    last_frame: Instant,
    sample_at: Instant,
    sample_frames: u64,
    max_frame_ms: f64,
}
impl ClientApp {
    fn new(options: Options, mut profile: profile::Profile) -> Self {
        let class = options
            .class
            .unwrap_or(ssk_sim::ShipClass::ALL[profile.class]);
        profile.class = class.index();
        let (game, view) = game::Game::start(&options, class);
        let world = view.world.clone();
        let camera = world.player.pos;
        let effects = effects::Effects::new(options.audio.enabled);
        let started = options.play;
        let now = Instant::now();
        tracing::debug!(name:"ssk.client.render.begin",progress_id="ssk.client.render.sample",progress_active=true,
            "Vulkan client ready to render");
        Self {
            options,
            profile,
            class,
            game: Some(game),
            view,
            world,
            effects,
            camera,
            stars: starfield::Starfield::new(camera),
            weapon: 0,
            menu: !started,
            started,
            help: false,
            run_number: 0,
            recorded: false,
            save_error: None,
            first_frame: true,
            epoch: now,
            last_frame: now,
            sample_at: now,
            sample_frames: 0,
            max_frame_ms: 0.,
        }
    }
}
impl eframe::App for ClientApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let frame_started = std::time::Instant::now();
        let frame_dt = frame_started.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = frame_started;
        let canvas = Canvas::new(
            ctx,
            ctx.content_rect(),
            frame_dt,
            self.epoch.elapsed().as_secs_f64(),
        );
        let focused = canvas.focused();
        ctx.set_cursor_icon(if self.menu || self.help {
            egui::CursorIcon::Default
        } else {
            egui::CursorIcon::None
        });
        if !self.profile.fullscreen {
            self.profile.width = canvas.screen_width().round() as i32;
            self.profile.height = canvas.screen_height().round() as i32;
        }
        let previous_hp = self.world.player.hp;
        if let Some(frame) = self.game.as_ref().unwrap().take_frame() {
            self.view = frame;
            self.world = self.view.world.clone();
        }
        // Rebase before computing cursor aim; camera and ship must use the same
        // coordinate frame, including the first rendered frame after a seam.
        self.stars.shift_origin(self.world.origin_shift);
        self.camera -= self.world.origin_shift;
        self.effects.shift(self.world.origin_shift);
        self.world.origin_shift = V2::ZERO;
        let dt = canvas.get_frame_time().min(MAX_RENDER_DELTA);
        if canvas.is_key_pressed(KeyCode::Escape) || canvas.is_key_pressed(KeyCode::F2) {
            self.menu = !self.menu;
            self.help = false;
            tracing::debug!(name:"ssk.client.menu.changed",open=self.menu,"Menu visibility changed");
        }
        if canvas.is_key_pressed(KeyCode::F1) {
            self.help = !self.help;
        }
        if canvas.is_key_pressed(KeyCode::F11) {
            self.profile.fullscreen = !self.profile.fullscreen;
            canvas.set_fullscreen(self.profile.fullscreen);
            tracing::debug!(name:"ssk.client.window.fullscreen",enabled=self.profile.fullscreen,"Fullscreen requested");
        }
        let mut restart = !self.options.online
            && self.world.player.hp <= 0.
            && canvas.is_key_pressed(KeyCode::R)
            && !self.menu;
        if self.world.player.hp <= 0. && !self.recorded && !self.options.online {
            self.profile.records[self.class.index()] =
                self.profile.records[self.class.index()].max(self.world.score);
            self.recorded = true;
            if let Err(e) = self.profile.save() {
                self.save_error = Some(e.to_string());
            }
        }
        if !self.menu && !self.help {
            if canvas.is_key_pressed(KeyCode::Q) || canvas.mouse_wheel().1 > 0. {
                self.weapon = (self.weapon + ssk_sim::WEAPON_COUNT - 1) % ssk_sim::WEAPON_COUNT;
            }
            if canvas.is_key_pressed(KeyCode::E) || canvas.mouse_wheel().1 < 0. {
                self.weapon = (self.weapon + 1) % ssk_sim::WEAPON_COUNT;
            }
            for (i, key) in [KeyCode::Num1, KeyCode::Num2, KeyCode::Num3]
                .into_iter()
                .enumerate()
            {
                if canvas.is_key_pressed(key) {
                    self.weapon = i;
                }
            }
        }
        let mouse = V2::new(
            canvas.mouse_position().0 - canvas.screen_width() / 2.,
            canvas.mouse_position().1 - canvas.screen_height() / 2.,
        );
        let input = if self.menu || self.help || !focused || !canvas.pointer_inside() {
            InputFrame {
                aim: self.world.player.angle,
                weapon: self.weapon,
                ..Default::default()
            }
        } else {
            InputFrame {
                movement: V2::new(
                    (canvas.is_key_down(KeyCode::D) as i8 - canvas.is_key_down(KeyCode::A) as i8)
                        as f32,
                    (canvas.is_key_down(KeyCode::W) as i8 - canvas.is_key_down(KeyCode::S) as i8)
                        as f32,
                ),
                aim: (self.camera + mouse - self.world.player.pos).angle(),
                fire: canvas.is_mouse_button_down(MouseButton::Primary)
                    || canvas.is_key_down(KeyCode::Space),
                ability: canvas.is_mouse_button_down(MouseButton::Secondary) || canvas.shift_down(),
                weapon: self.weapon,
            }
        };
        self.game.as_ref().unwrap().update(game::Controls {
            input,
            paused: !self.started || self.menu || self.help || !focused,
            next_class: ssk_sim::ShipClass::ALL[self.profile.class],
            spawn_radius: canvas.screen_width().hypot(canvas.screen_height()) * 0.5
                + SPAWN_SCREEN_MARGIN,
        });
        let alpha = (self.view.at.elapsed().as_secs_f32() / DT).clamp(0., 1.);
        let target = self
            .world
            .player
            .previous
            .lerp(self.world.player.pos, alpha)
            + mouse.clamp_length(CAMERA_LOOKAHEAD) * CAMERA_LOOKAHEAD_SCALE;
        self.camera = self
            .camera
            .lerp(target, 1. - (-CAMERA_SMOOTHING * dt).exp());
        self.effects
            .events(&self.world.events, self.camera, self.profile.volume);
        self.world.events.clear();
        self.effects.update(dt);
        if self.world.player.hp < previous_hp {
            self.effects.shake = (self.effects.shake + HIT_SHAKE).min(SHAKE_MAX);
            self.effects.flash = 1.;
        }
        let render_camera = self.camera + self.effects.offset(self.profile.shake);
        self.stars.draw(&canvas, render_camera);
        render::scene(&canvas, &self.world, render_camera, alpha);
        {
            for (ship, age) in &self.view.peers {
                {
                    if ship.hp > 0. {
                        draw_ship(
                            &canvas,
                            &ship,
                            render_camera,
                            1.,
                            if *age > PEER_STALE_SECONDS {
                                GRAY
                            } else {
                                GOLD
                            },
                        );
                        let p = canvas.screen(ship.pos, render_camera);
                        if p.x < PEER_EDGE_MARGIN
                            || p.y < PEER_EDGE_MARGIN
                            || p.x > canvas.screen_width() - PEER_EDGE_MARGIN
                            || p.y > canvas.screen_height() - PEER_EDGE_MARGIN
                        {
                            let edge = V2::new(
                                p.x.clamp(
                                    PEER_EDGE_MARGIN,
                                    canvas.screen_width() - PEER_EDGE_MARGIN,
                                ),
                                p.y.clamp(
                                    PEER_EDGE_MARGIN,
                                    canvas.screen_height() - PEER_EDGE_MARGIN,
                                ),
                            );
                            canvas.draw_circle_lines(
                                edge.x,
                                edge.y,
                                PEER_EDGE_RADIUS,
                                LINE_WIDTH,
                                GOLD,
                            );
                            canvas.draw_text(
                                &format!("{:.0}m", (ship.pos - self.world.player.pos).length()),
                                edge.x - PEER_DISTANCE_OFFSET[0],
                                edge.y - PEER_DISTANCE_OFFSET[1],
                                PEER_DISTANCE_FONT,
                                GOLD,
                            );
                        }
                        canvas.draw_text(
                            &format!(
                                "PEER {:.0} HP{}",
                                ship.hp,
                                if *age > PEER_STALE_SECONDS { " !" } else { "" }
                            ),
                            p.x - PEER_LABEL_OFFSET,
                            p.y - PEER_LABEL_OFFSET,
                            PEER_LABEL_FONT,
                            GOLD,
                        );
                    }
                }
            }
        }
        self.effects
            .draw(&canvas, render_camera, self.profile.flashes);
        ui::hud(&canvas, &self.world, &self.profile, self.view.overloads);
        if self.world.player.hp <= 0. && !self.menu {
            canvas.draw_rectangle(
                0.,
                0.,
                canvas.screen_width(),
                canvas.screen_height(),
                RESULT_BACKGROUND,
            );
            let x = canvas.screen_width() / 2. - RESULT_HALF_WIDTH;
            let y = canvas.screen_height() / 2.;
            canvas.draw_text(
                "SHIP LOST",
                x,
                y - RESULT_TITLE_TOP,
                RESULT_TITLE_FONT,
                PINK,
            );
            canvas.draw_text(
                &format!(
                    "SCORE {}   BEST {}",
                    self.world.score,
                    self.profile.records[self.class.index()]
                ),
                x,
                y,
                RESULT_SCORE_FONT,
                WHITE,
            );
            canvas.draw_text(
                if self.options.online {
                    "Respawning...   F2: choose next ship"
                } else {
                    "R: fly again     F2: choose ship"
                },
                x,
                y + RESULT_HELP_TOP,
                RESULT_HELP_FONT,
                SKYBLUE,
            );
        }
        if self.menu {
            let action = ui::menu(
                &canvas,
                &mut self.profile,
                self.started,
                self.options.online,
            );
            if action.quit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            if action.resume {
                self.menu = false;
                if let Err(e) = self.profile.save() {
                    self.save_error = Some(e.to_string());
                }
            }
            if action.new_run {
                self.class = ssk_sim::ShipClass::ALL[self.profile.class];
                restart = true;
            }
        }
        if restart {
            self.run_number += 1;
            self.game
                .as_ref()
                .unwrap()
                .restart(self.options.seed + self.run_number, self.class);
            self.effects.clear();
            self.camera = self.world.player.pos;
            self.weapon = 0;
            self.menu = false;
            self.started = true;
            self.recorded = false;
            tracing::info!(name:"ssk.client.session.restart",run_number=self.run_number,class=?self.class,"Restart requested");
        }
        if !self.options.online && !self.view.error.is_empty() {
            canvas.draw_text(
                &self.view.error,
                24.,
                canvas.screen_height() - NOTICE_BOTTOM,
                18.,
                PINK,
            );
        }
        if self.help {
            ui::help(&canvas);
        }
        if let Some(error) = &self.save_error {
            canvas.draw_text(
                &format!("Profile save: {error}"),
                NOTICE_INSET,
                canvas.screen_height() - NOTICE_BOTTOM,
                NOTICE_FONT,
                PINK,
            );
        }
        if self.options.online {
            let status = &self.view.status;
            if canvas.is_key_pressed(KeyCode::F3) {
                ctx.copy_text(status.ticket.clone());
            }
            canvas.draw_text(
                &format!(
                    "{} | {} peers | SECTOR {} / image {} | F2 menu / ship | F3 invite",
                    status.phase,
                    status.peers,
                    if self.view.sector.local {
                        "LOCAL"
                    } else {
                        "REMOTE"
                    },
                    self.view
                        .sector
                        .age_ms
                        .map_or_else(|| "waiting".into(), |age| format!("{age:.0}ms"))
                ),
                NOTICE_INSET,
                canvas.screen_height() - NOTICE_BOTTOM,
                NETWORK_FONT,
                ORANGE,
            );
            {
                let stats = &status.links;
                let tx: f32 = stats.iter().map(|s| s.tx_bps).sum();
                let rx: f32 = stats.iter().map(|s| s.rx_bps).sum();
                let worst = stats.iter().filter_map(|s| s.rtt_ms).fold(0f32, f32::max);
                let loss = stats.iter().map(|s| s.loss).fold(0f32, f32::max);
                canvas.draw_text(
                    &format!(
                        "FRAGS {} | RTT {:.1}ms | loss {:.0}% | TX/RX {:.1}/{:.1}KiB/s | {}",
                        self.view.frags,
                        worst,
                        loss * PERCENT,
                        tx / BYTES_PER_KIB,
                        rx / BYTES_PER_KIB,
                        self.view.error
                    ),
                    NOTICE_INSET,
                    canvas.screen_height() - NETWORK_STATS_BOTTOM,
                    NETWORK_FONT,
                    GOLD,
                );
            }
        }
        canvas.draw_circle_lines(
            canvas.mouse_position().0,
            canvas.mouse_position().1,
            CROSSHAIR_RADIUS,
            1.,
            SKYBLUE,
        );
        if self.first_frame {
            tracing::debug!(name:"ssk.client.frame.first.queued","First game frame queued; draw/present follows");
        }
        tracing::trace!(name:"ssk.client.render.frame",frame_time_ms=canvas.get_frame_time() as f64*MILLIS_PER_SECOND,
            simulation_tick=self.world.tick,width=canvas.screen_width(),height=canvas.screen_height(),"Frame commands queued");
        ctx.request_repaint();
        self.sample_frames += 1;
        self.max_frame_ms = self
            .max_frame_ms
            .max(frame_started.elapsed().as_secs_f64() * MILLIS_PER_SECOND);
        let elapsed = self.sample_at.elapsed().as_secs_f64();
        if elapsed >= OBSERVATION_SAMPLE_SECONDS {
            tracing::debug!(name:"ssk.client.render.sample",kind="metric",frames=self.sample_frames,window_ms=elapsed*MILLIS_PER_SECOND,
                fps=self.sample_frames as f64/elapsed,max_frame_ms=self.max_frame_ms,simulation_tick=self.world.tick,
                render_handoff_age_ms=self.view.at.elapsed().as_secs_f64()*MILLIS_PER_SECOND,
                sector_local=self.view.sector.local,sector_image_age_ms=self.view.sector.age_ms.unwrap_or(f64::NAN),
                peers=self.view.peers.len(),overloads=self.view.overloads,menu=self.menu,help=self.help,"Render progress sample");
            self.sample_at = std::time::Instant::now();
            self.sample_frames = 0;
            self.max_frame_ms = 0.;
        }
        self.first_frame = false;
    }
    fn on_exit(&mut self) {
        tracing::debug!(name:"ssk.client.render.end",progress_id="ssk.client.render.sample",progress_active=false,"Render loop exiting");
        drop(self.game.take());
        if let Err(error) = self.profile.save() {
            tracing::warn!(name:"ssk.client.profile.save_failed",%error,"Profile save failed");
        }
    }
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [SPACE_COLOR.r, SPACE_COLOR.g, SPACE_COLOR.b, 1.]
    }
}
