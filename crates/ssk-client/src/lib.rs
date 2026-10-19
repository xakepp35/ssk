use macroquad::prelude::*;
use ssk_sim::{DT, InputFrame, Ship, V2, World};
mod effects;
mod online;
mod profile;
mod render;
mod shared;
mod ui;
pub struct Options {
    pub seed: u64,
    pub class: ssk_sim::ShipClass,
    pub play: bool,
    pub online: bool,
    pub connect: Option<String>,
    pub network: ssk_net::Config,
}
pub fn launch(options: Options) {
    let profile = profile::Profile::load();
    macroquad::Window::from_config(
        Conf {
            window_title: "Syrup Space Kadet".into(),
            window_width: profile.width,
            window_height: profile.height,
            fullscreen: profile.fullscreen,
            high_dpi: true,
            platform: miniquad::conf::Platform {
                swap_interval: Some(0),
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
    let a = screen(pos + forward * radius * 1.5, camera);
    let b = screen(pos - forward * radius + side * radius * 0.8, camera);
    let c = screen(pos - forward * radius - side * radius * 0.8, camera);
    draw_triangle(
        a,
        b,
        c,
        Color::new(color.r * 0.15, color.g * 0.15, color.b * 0.15, 1.),
    );
    draw_triangle_lines(a, b, c, 2., color);
    let p = screen(pos, camera);
    draw_circle(p.x, p.y, 3., WHITE);
    if ship.ability_active {
        let color = match ship.class {
            ssk_sim::ShipClass::Spark => SKYBLUE,
            ssk_sim::ShipClass::Cadet => GREEN,
            ssk_sim::ShipClass::Thunder => RED,
        };
        draw_circle_lines(
            p.x,
            p.y,
            radius + 9. + (get_time() as f32 * 12.).sin() * 2.,
            3.,
            color,
        );
    }
    if ship.velocity.length() > 10. {
        let end = screen(
            pos - forward * (radius + 8. + (get_time() as f32 * 67.).sin() * 4.),
            camera,
        );
        let start = screen(pos - forward * radius, camera);
        draw_line(start.x, start.y, end.x, end.y, 5., SKYBLUE);
    }
}
fn background(camera: V2) {
    clear_background(Color::from_rgba(6, 9, 21, 255));
    for layer in 0..3 {
        for i in 0..110 {
            let parallax = 0.1 + layer as f32 * 0.14;
            let x = (i as f32 * 173.73 + (i * i) as f32 * 2.1 - camera.x * parallax)
                .rem_euclid(screen_width());
            let y = (i as f32 * 91.13 + (i * i) as f32 * 3.3 - camera.y * parallax)
                .rem_euclid(screen_height());
            let b = 0.18 + layer as f32 * 0.11;
            draw_circle(
                x,
                y,
                0.6 + layer as f32 * 0.3,
                Color::new(b, b + 0.07, b + 0.18, 1.),
            );
        }
    }
    let color = Color::new(0.08, 0.16, 0.24, 0.3);
    for x in -8..=8 {
        let p = screen(
            V2::new(((camera.x / 256.).floor() + x as f32) * 256., 0.),
            camera,
        );
        draw_line(p.x, 0., p.x, screen_height(), 1., color);
    }
    for y in -6..=6 {
        let p = screen(
            V2::new(0., ((camera.y / 256.).floor() + y as f32) * 256.),
            camera,
        );
        draw_line(0., p.y, screen_width(), p.y, 1., color);
    }
}
async fn run(options: Options, mut profile: profile::Profile) {
    let mut class = if options.play {
        options.class
    } else {
        ssk_sim::ShipClass::ALL[profile.class]
    };
    if options.play {
        profile.class = class.index();
    }
    let mut world = World::new(
        if options.online {
            options.network.seed
        } else {
            options.seed
        },
        class,
    );
    let mut online = if options.online {
        match online::Online::new(options.network.clone(), options.connect.clone()) {
            Ok(net) => Some(net),
            Err(error) => {
                eprintln!("Network: {error:#}");
                None
            }
        }
    } else {
        None
    };
    if let Some(session) = &mut online {
        world = World::duel(options.network.seed, class, session.net.player_id);
        session.snapshot(&world);
    }
    let mut effects = effects::Effects::new().await;
    let mut accumulator = 0.;
    let mut camera = world.player.pos;
    let mut skipped = 0;
    let mut weapon = 0;
    let mut menu = !options.play;
    let mut started = options.play;
    let mut help = false;
    let mut run_number = 0;
    let mut recorded = false;
    let mut save_error = None;
    show_mouse(false);
    loop {
        let frame_start = std::time::Instant::now();
        if let Some(session) = &mut online {
            session.receive(&mut world, get_time());
        }
        let dt = get_frame_time().min(0.1);
        if is_quit_requested() {
            break;
        }
        if is_key_pressed(KeyCode::Escape) {
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
        world.spawn_radius = screen_width().hypot(screen_height()) * 0.5 + 200.;
        if !menu && !help {
            if is_key_pressed(KeyCode::Q) || mouse_wheel().1 > 0. {
                weapon = (weapon + 2) % 3;
            }
            if is_key_pressed(KeyCode::E) || mouse_wheel().1 < 0. {
                weapon = (weapon + 1) % 3;
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
        let previous_hp = world.player.hp;
        if started && (!(menu || help) || options.online) {
            accumulator += dt;
            let mut steps = 0;
            while accumulator >= DT && steps < 8 {
                world.step(input);
                if let Some(session) = &mut online {
                    if world.tick % (240 / options.network.net_hz as u64) == 0
                        || world.player.hp <= 0.
                    {
                        session.snapshot(&world);
                    }
                }
                accumulator -= DT;
                steps += 1;
            }
            if accumulator >= DT {
                skipped += (accumulator / DT) as u64;
                accumulator = accumulator.rem_euclid(DT);
            }
        } else {
            accumulator = 0.;
        }
        let alpha = accumulator / DT;
        camera -= world.origin_shift;
        effects.shift(world.origin_shift);
        world.origin_shift = V2::ZERO;
        let target =
            world.player.previous.lerp(world.player.pos, alpha) + mouse.clamp_length(200.) * 0.25;
        camera = camera.lerp(target, 1. - (-12. * dt).exp());
        if let Some(session) = &mut online {
            if session.after_frame(&mut world, dt) {
                class = ssk_sim::ShipClass::ALL[profile.class];
                restart = true;
            }
        }
        effects.events(&world.events, camera, profile.volume);
        world.events.clear();
        effects.update(dt);
        if world.player.hp < previous_hp {
            effects.shake = (effects.shake + 3.).min(8.);
            effects.flash = 1.;
        }
        let render_camera = camera + effects.offset(profile.shake);
        background(render_camera);
        render::scene(&world, render_camera, alpha);
        if let Some(session) = &online {
            for peer in session.replicas.peers.keys() {
                if let Some((snapshot, age)) = session.replicas.sample(peer, get_time()) {
                    let ship = snapshot.ship(ssk_net::player_id(peer).unwrap_or(0), world.origin);
                    if ship.hp > 0. {
                        draw_ship(
                            &ship,
                            render_camera,
                            1.,
                            if age > 0.15 { GRAY } else { GOLD },
                        );
                        let p = screen(ship.pos, render_camera);
                        if p.x < 30.
                            || p.y < 30.
                            || p.x > screen_width() - 30.
                            || p.y > screen_height() - 30.
                        {
                            let edge = vec2(
                                p.x.clamp(30., screen_width() - 30.),
                                p.y.clamp(30., screen_height() - 30.),
                            );
                            draw_circle_lines(edge.x, edge.y, 10., 2., GOLD);
                            draw_text(
                                &format!("{:.0}m", (ship.pos - world.player.pos).length()),
                                edge.x - 25.,
                                edge.y - 16.,
                                18.,
                                GOLD,
                            );
                        }
                        draw_text(
                            &format!(
                                "PEER {:.0} HP{}",
                                ship.hp,
                                if age > 0.15 { " !" } else { "" }
                            ),
                            p.x - 40.,
                            p.y - 40.,
                            16.,
                            GOLD,
                        );
                    }
                }
            }
        }
        effects.draw(render_camera, profile.flashes);
        ui::hud(&world, &profile, skipped);
        if world.player.hp <= 0. && !menu {
            draw_rectangle(
                0.,
                0.,
                screen_width(),
                screen_height(),
                Color::new(0.01, 0.02, 0.05, 0.8),
            );
            let x = screen_width() / 2. - 210.;
            let y = screen_height() / 2.;
            draw_text("SHIP LOST", x, y - 60., 56., PINK);
            draw_text(
                &format!(
                    "SCORE {}   BEST {}",
                    world.score,
                    profile.records[class.index()]
                ),
                x,
                y,
                28.,
                WHITE,
            );
            draw_text(
                if options.online {
                    "Respawning...   Esc: choose next ship"
                } else {
                    "R: fly again     Esc: choose ship"
                },
                x,
                y + 50.,
                26.,
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
            }
            if action.new_run {
                class = ssk_sim::ShipClass::ALL[profile.class];
                restart = true;
            }
        }
        if restart {
            run_number += 1;
            if let Some(session) = &mut online {
                session.respawn(&mut world, class, options.network.seed);
            } else {
                world = World::new(options.seed + run_number, class);
            }
            effects.clear();
            camera = world.player.pos;
            accumulator = 0.;
            weapon = 0;
            menu = false;
            started = true;
            recorded = false;
        }
        if help {
            ui::help();
        }
        if let Some(error) = &save_error {
            draw_text(
                &format!("Profile save: {error}"),
                24.,
                screen_height() - 100.,
                18.,
                PINK,
            );
        }
        if options.online {
            let status = online
                .as_ref()
                .map(|n| n.net.status.borrow().clone())
                .unwrap_or_else(|| ssk_net::Status {
                    phase: "Network failed to start; see terminal".into(),
                    ..Default::default()
                });
            if is_key_pressed(KeyCode::F2) {
                miniquad::window::clipboard_set(&status.ticket);
            }
            draw_text(
                &format!(
                    "{} | {} peers | F2 copy invite (also in terminal)",
                    status.phase, status.peers
                ),
                24.,
                screen_height() - 100.,
                20.,
                ORANGE,
            );
            if let Some(session) = &online {
                let stats = &session.net.status.borrow().links;
                let tx: f32 = stats.iter().map(|s| s.tx_bps).sum();
                let rx: f32 = stats.iter().map(|s| s.rx_bps).sum();
                let worst = stats.iter().filter_map(|s| s.rtt_ms).fold(0f32, f32::max);
                let loss = stats.iter().map(|s| s.loss).fold(0f32, f32::max);
                draw_text(
                    &format!(
                        "FRAGS {} | RTT {:.1}ms | loss {:.0}% | TX/RX {:.1}/{:.1}KiB/s | {}",
                        session.frags,
                        worst,
                        loss * 100.,
                        tx / 1024.,
                        rx / 1024.,
                        session.error
                    ),
                    24.,
                    screen_height() - 125.,
                    20.,
                    GOLD,
                );
            }
        }
        draw_circle_lines(mouse_position().0, mouse_position().1, 8., 1., SKYBLUE);
        next_frame().await;
        if let Some(remaining) =
            std::time::Duration::from_secs_f64(1. / 240.).checked_sub(frame_start.elapsed())
        {
            std::thread::sleep(remaining);
        }
    }
    if let Some(session) = &mut online {
        session.leave();
    }
    if !profile.fullscreen {
        profile.width = screen_width() as i32;
        profile.height = screen_height() as i32;
    }
    if let Err(error) = profile.save() {
        eprintln!("Profile save: {error}");
    }
    show_mouse(true);
}
