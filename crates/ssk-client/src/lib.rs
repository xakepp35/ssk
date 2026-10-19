use constants::*;
use macroquad::prelude::*;
use ssk_sim::{DT, InputFrame, Ship, V2};
pub mod bench;
mod config;
pub mod constants;
pub use config::{AudioConfig, WindowConfig};
mod effects;
mod game;
mod online;
mod profile;
mod render;
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
pub fn launch(options: Options) {
    let mut profile = profile::Profile::load();
    profile.width = options.window.width;
    profile.height = options.window.height;
    profile.fullscreen = options.window.fullscreen;
    eprintln!(
        "SSK startup: graphics init, {}x{}, vsync interval {}",
        profile.width, profile.height, VSYNC_INTERVAL
    );
    macroquad::Window::from_config(
        Conf {
            window_title: "Syrup Space Kadet".into(),
            window_width: profile.width,
            window_height: profile.height,
            fullscreen: profile.fullscreen,
            high_dpi: true,
            platform: miniquad::conf::Platform {
                swap_interval: Some(VSYNC_INTERVAL),
                ..Default::default()
            },
            ..Default::default()
        },
        run(options, profile),
    );
}
fn screen(p: V2, camera: V2) -> Vec2 {
    vec2(
        p.x - camera.x + screen_width() / 2.,
        p.y - camera.y + screen_height() / 2.,
    )
}
fn draw_ship(ship: &Ship, camera: V2, alpha: f32, color: Color) {
    let pos = ship.previous.lerp(ship.pos, alpha);
    let forward = V2::from_angle(ship.angle);
    let side = forward.perp();
    let radius = ship.class.stats().3;
    let a = screen(pos + forward * radius * SHIP_NOSE_SCALE, camera);
    let b = screen(
        pos - forward * radius + side * radius * SHIP_WING_SCALE,
        camera,
    );
    let c = screen(
        pos - forward * radius - side * radius * SHIP_WING_SCALE,
        camera,
    );
    draw_triangle(
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
    draw_triangle_lines(a, b, c, 2., color);
    let p = screen(pos, camera);
    draw_circle(p.x, p.y, SHIP_CORE_RADIUS, WHITE);
    if ship.ability_active {
        let color = match ship.class {
            ssk_sim::ShipClass::Spark => SKYBLUE,
            ssk_sim::ShipClass::Cadet => GREEN,
            ssk_sim::ShipClass::Thunder => RED,
        };
        draw_circle_lines(
            p.x,
            p.y,
            radius
                + ABILITY_RING_OFFSET
                + (get_time() as f32 * ABILITY_RING_SPEED).sin() * ABILITY_RING_PULSE,
            ABILITY_RING_WIDTH,
            color,
        );
    }
    if ship.velocity.length() > EXHAUST_MIN_SPEED {
        let end = screen(
            pos - forward
                * (radius
                    + EXHAUST_OFFSET
                    + (get_time() as f32 * EXHAUST_PULSE_SPEED).sin() * EXHAUST_PULSE_SIZE),
            camera,
        );
        let start = screen(pos - forward * radius, camera);
        draw_line(start.x, start.y, end.x, end.y, EXHAUST_WIDTH, SKYBLUE);
    }
}
fn background(camera: V2) {
    clear_background(SPACE_COLOR);
    for layer in 0..STAR_LAYERS {
        for i in 0..STARS_PER_LAYER {
            let parallax = STAR_PARALLAX[0] + layer as f32 * STAR_PARALLAX[1];
            let x = (i as f32 * STAR_POSITION_X[0] + (i * i) as f32 * STAR_POSITION_X[1]
                - camera.x * parallax)
                .rem_euclid(screen_width());
            let y = (i as f32 * STAR_POSITION_Y[0] + (i * i) as f32 * STAR_POSITION_Y[1]
                - camera.y * parallax)
                .rem_euclid(screen_height());
            let b = STAR_BRIGHTNESS[0] + layer as f32 * STAR_BRIGHTNESS[1];
            draw_circle(
                x,
                y,
                STAR_RADIUS[0] + layer as f32 * STAR_RADIUS[1],
                Color::new(b, b + STAR_TINT[0], b + STAR_TINT[1], 1.),
            );
        }
    }
    let color = SPACE_GRID_COLOR;
    for x in -SPACE_GRID_COLUMNS..=SPACE_GRID_COLUMNS {
        let p = screen(
            V2::new(
                ((camera.x / SPACE_GRID_SIZE).floor() + x as f32) * SPACE_GRID_SIZE,
                0.,
            ),
            camera,
        );
        draw_line(p.x, 0., p.x, screen_height(), 1., color);
    }
    for y in -SPACE_GRID_ROWS..=SPACE_GRID_ROWS {
        let p = screen(
            V2::new(
                0.,
                ((camera.y / SPACE_GRID_SIZE).floor() + y as f32) * SPACE_GRID_SIZE,
            ),
            camera,
        );
        draw_line(0., p.y, screen_width(), p.y, 1., color);
    }
}
async fn run(options: Options, mut profile: profile::Profile) {
    eprintln!("SSK startup: graphics context ready");
    let mut class = options
        .class
        .unwrap_or(ssk_sim::ShipClass::ALL[profile.class]);
    profile.class = class.index();
    let (game, initial) = game::Game::start(&options, class);
    let mut view = initial;
    let mut world = view.world.clone();
    eprintln!(
        "SSK startup: audio {}",
        if options.audio.enabled {
            "worker requested"
        } else {
            "disabled; native audio initialization skipped"
        }
    );
    let mut effects = effects::Effects::new(options.audio.enabled);
    let mut camera = world.player.pos;
    let mut weapon = 0;
    let mut menu = !options.play;
    let mut started = options.play;
    let mut help = false;
    let mut run_number = 0;
    let mut recorded = false;
    let mut save_error = None;
    show_mouse(false);
    loop {
        let previous_hp = world.player.hp;
        if let Some(frame) = game.take_frame() {
            view = frame;
            world = view.world.clone();
        }
        let dt = get_frame_time().min(MAX_RENDER_DELTA);
        if is_quit_requested() {
            break;
        }
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::F2) {
            menu = !menu;
            help = false;
        }
        if is_key_pressed(KeyCode::F1) {
            help = !help;
        }
        if is_key_pressed(KeyCode::F11) {
            profile.fullscreen = !profile.fullscreen;
            set_fullscreen(profile.fullscreen);
        }
        let mut restart =
            !options.online && world.player.hp <= 0. && is_key_pressed(KeyCode::R) && !menu;
        if world.player.hp <= 0. && !recorded && !options.online {
            profile.records[class.index()] = profile.records[class.index()].max(world.score);
            recorded = true;
            if let Err(e) = profile.save() {
                save_error = Some(e.to_string());
            }
        }
        if !menu && !help {
            if is_key_pressed(KeyCode::Q) || mouse_wheel().1 > 0. {
                weapon = (weapon + ssk_sim::WEAPON_COUNT - 1) % ssk_sim::WEAPON_COUNT;
            }
            if is_key_pressed(KeyCode::E) || mouse_wheel().1 < 0. {
                weapon = (weapon + 1) % ssk_sim::WEAPON_COUNT;
            }
            for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
                .into_iter()
                .enumerate()
            {
                if is_key_pressed(key) {
                    weapon = i;
                }
            }
        }
        let mouse = V2::new(
            mouse_position().0 - screen_width() / 2.,
            mouse_position().1 - screen_height() / 2.,
        );
        let input = if menu || help {
            InputFrame {
                aim: world.player.angle,
                weapon,
                ..Default::default()
            }
        } else {
            InputFrame {
                movement: V2::new(
                    (is_key_down(KeyCode::D) as i8 - is_key_down(KeyCode::A) as i8) as f32,
                    (is_key_down(KeyCode::S) as i8 - is_key_down(KeyCode::W) as i8) as f32,
                ),
                aim: (camera + mouse - world.player.pos).angle(),
                fire: is_mouse_button_down(MouseButton::Left) || is_key_down(KeyCode::Space),
                ability: is_mouse_button_down(MouseButton::Right)
                    || is_key_down(KeyCode::LeftShift)
                    || is_key_down(KeyCode::RightShift),
                weapon,
            }
        };
        game.update(game::Controls {
            input,
            paused: !started || menu || help,
            next_class: ssk_sim::ShipClass::ALL[profile.class],
            spawn_radius: screen_width().hypot(screen_height()) * 0.5 + SPAWN_SCREEN_MARGIN,
        });
        let alpha = (view.at.elapsed().as_secs_f32() / DT).clamp(0., 1.);
        camera -= world.origin_shift;
        effects.shift(world.origin_shift);
        world.origin_shift = V2::ZERO;
        let target = world.player.previous.lerp(world.player.pos, alpha)
            + mouse.clamp_length(CAMERA_LOOKAHEAD) * CAMERA_LOOKAHEAD_SCALE;
        camera = camera.lerp(target, 1. - (-CAMERA_SMOOTHING * dt).exp());
        effects.events(&world.events, camera, profile.volume);
        world.events.clear();
        effects.update(dt);
        if world.player.hp < previous_hp {
            effects.shake = (effects.shake + HIT_SHAKE).min(SHAKE_MAX);
            effects.flash = 1.;
        }
        let render_camera = camera + effects.offset(profile.shake);
        background(render_camera);
        render::scene(&world, render_camera, alpha);
        {
            for (ship, age) in &view.peers {
                {
                    if ship.hp > 0. {
                        draw_ship(
                            &ship,
                            render_camera,
                            1.,
                            if *age > PEER_STALE_SECONDS {
                                GRAY
                            } else {
                                GOLD
                            },
                        );
                        let p = screen(ship.pos, render_camera);
                        if p.x < PEER_EDGE_MARGIN
                            || p.y < PEER_EDGE_MARGIN
                            || p.x > screen_width() - PEER_EDGE_MARGIN
                            || p.y > screen_height() - PEER_EDGE_MARGIN
                        {
                            let edge = vec2(
                                p.x.clamp(PEER_EDGE_MARGIN, screen_width() - PEER_EDGE_MARGIN),
                                p.y.clamp(PEER_EDGE_MARGIN, screen_height() - PEER_EDGE_MARGIN),
                            );
                            draw_circle_lines(edge.x, edge.y, PEER_EDGE_RADIUS, LINE_WIDTH, GOLD);
                            draw_text(
                                &format!("{:.0}m", (ship.pos - world.player.pos).length()),
                                edge.x - PEER_DISTANCE_OFFSET[0],
                                edge.y - PEER_DISTANCE_OFFSET[1],
                                PEER_DISTANCE_FONT,
                                GOLD,
                            );
                        }
                        draw_text(
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
        effects.draw(render_camera, profile.flashes);
        ui::hud(&world, &profile, view.overloads);
        if world.player.hp <= 0. && !menu {
            draw_rectangle(0., 0., screen_width(), screen_height(), RESULT_BACKGROUND);
            let x = screen_width() / 2. - RESULT_HALF_WIDTH;
            let y = screen_height() / 2.;
            draw_text(
                "SHIP LOST",
                x,
                y - RESULT_TITLE_TOP,
                RESULT_TITLE_FONT,
                PINK,
            );
            draw_text(
                &format!(
                    "SCORE {}   BEST {}",
                    world.score,
                    profile.records[class.index()]
                ),
                x,
                y,
                RESULT_SCORE_FONT,
                WHITE,
            );
            draw_text(
                if options.online {
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
        if menu {
            let action = ui::menu(&mut profile, started, options.online);
            if action.quit {
                break;
            }
            if action.resume {
                menu = false;
                if let Err(e) = profile.save() {
                    save_error = Some(e.to_string());
                }
            }
            if action.new_run {
                class = ssk_sim::ShipClass::ALL[profile.class];
                restart = true;
            }
        }
        if restart {
            run_number += 1;
            game.restart(options.seed + run_number, class);
            effects.clear();
            camera = world.player.pos;
            weapon = 0;
            menu = false;
            started = true;
            recorded = false;
        }
        if !options.online && !view.error.is_empty() {
            draw_text(&view.error, 24., screen_height() - NOTICE_BOTTOM, 18., PINK);
        }
        if help {
            ui::help();
        }
        if let Some(error) = &save_error {
            draw_text(
                &format!("Profile save: {error}"),
                NOTICE_INSET,
                screen_height() - NOTICE_BOTTOM,
                NOTICE_FONT,
                PINK,
            );
        }
        if options.online {
            let status = &view.status;
            if is_key_pressed(KeyCode::F3) {
                miniquad::window::clipboard_set(&status.ticket);
            }
            draw_text(
                &format!(
                    "{} | {} peers | SECTOR {} / image {} | F2 menu / ship | F3 invite",
                    status.phase,
                    status.peers,
                    if view.sector.local { "LOCAL" } else { "REMOTE" },
                    view.sector
                        .age_ms
                        .map_or_else(|| "waiting".into(), |age| format!("{age:.0}ms"))
                ),
                NOTICE_INSET,
                screen_height() - NOTICE_BOTTOM,
                NETWORK_FONT,
                ORANGE,
            );
            {
                let stats = &status.links;
                let tx: f32 = stats.iter().map(|s| s.tx_bps).sum();
                let rx: f32 = stats.iter().map(|s| s.rx_bps).sum();
                let worst = stats.iter().filter_map(|s| s.rtt_ms).fold(0f32, f32::max);
                let loss = stats.iter().map(|s| s.loss).fold(0f32, f32::max);
                draw_text(
                    &format!(
                        "FRAGS {} | RTT {:.1}ms | loss {:.0}% | TX/RX {:.1}/{:.1}KiB/s | {}",
                        view.frags,
                        worst,
                        loss * PERCENT,
                        tx / BYTES_PER_KIB,
                        rx / BYTES_PER_KIB,
                        view.error
                    ),
                    NOTICE_INSET,
                    screen_height() - NETWORK_STATS_BOTTOM,
                    NETWORK_FONT,
                    GOLD,
                );
            }
        }
        draw_circle_lines(
            mouse_position().0,
            mouse_position().1,
            CROSSHAIR_RADIUS,
            1.,
            SKYBLUE,
        );
        next_frame().await;
    }
    drop(game);
    if !profile.fullscreen {
        profile.width = screen_width() as i32;
        profile.height = screen_height() as i32;
    }
    if let Err(error) = profile.save() {
        eprintln!("Profile save: {error}");
    }
    show_mouse(true);
}
