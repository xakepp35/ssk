use crate::profile::Profile;
use macroquad::prelude::*;
use ssk_sim::*;
#[derive(Default)]
pub struct MenuAction {
    pub resume: bool,
    pub new_run: bool,
    pub quit: bool,
}
fn button(label: &str, x: f32, y: f32, w: f32, s: f32, selected: bool) -> bool {
    let r = Rect::new(x, y, w * s, 40. * s);
    let hover = r.contains(mouse_position().into());
    let color = if selected {
        Color::new(0.08, 0.27, 0.34, 1.)
    } else if hover {
        Color::new(0.12, 0.2, 0.29, 1.)
    } else {
        Color::new(0.05, 0.09, 0.16, 1.)
    };
    draw_rectangle(r.x, r.y, r.w, r.h, color);
    draw_rectangle_lines(
        r.x,
        r.y,
        r.w,
        r.h,
        1.,
        if hover || selected {
            SKYBLUE
        } else {
            Color::new(0.15, 0.25, 0.35, 1.)
        },
    );
    draw_text(label, x + 12. * s, y + 27. * s, 22. * s, WHITE);
    hover && is_mouse_button_pressed(MouseButton::Left)
}
pub fn menu(profile: &mut Profile, started: bool, online: bool) -> MenuAction {
    draw_rectangle(
        0.,
        0.,
        screen_width(),
        screen_height(),
        Color::new(0.015, 0.025, 0.055, 0.95),
    );
    let s = (screen_height() / 850.).min(1.) * profile.ui_scale;
    let x = screen_width() / 2. - 330. * s;
    let y = (screen_height() - 740. * s) * 0.5;
    draw_text("SYRUP SPACE", x, y + 55. * s, 54. * s, WHITE);
    draw_text("KADET", x, y + 105. * s, 54. * s, SKYBLUE);
    draw_text(
        if online {
            "PEER DUEL"
        } else {
            "ONE SHIP. THREE GUNS. SURVIVE."
        },
        x,
        y + 142. * s,
        20. * s,
        GRAY,
    );
    for (i, class) in ShipClass::ALL.into_iter().enumerate() {
        if button(
            class.name(),
            x + i as f32 * 220. * s,
            y + 175. * s,
            205.,
            s,
            profile.class == i,
        ) {
            profile.class = i;
        }
    }
    let class = ShipClass::ALL[profile.class];
    let (hp, speed, _, _) = class.stats();
    draw_text(
        &format!(
            "HP {hp:.0}    SPEED {speed:.0}    BEST {}",
            profile.records[profile.class]
        ),
        x,
        y + 250. * s,
        23. * s,
        WHITE,
    );
    let description = match class {
        ShipClass::Spark => "BOOST: fast flanking, fragile hull",
        ShipClass::Cadet => "SHIELD: absorb half damage; stop firing!",
        ShipClass::Thunder => "QUAD: devastating bursts, heavy energy use",
    };
    draw_text(description, x, y + 282. * s, 21. * s, GREEN);
    let mut action = MenuAction::default();
    if button(
        if started {
            "RESUME  [Esc / Enter]"
        } else {
            "START SURVIVAL  [Enter]"
        },
        x,
        y + 315. * s,
        660.,
        s,
        true,
    ) || is_key_pressed(KeyCode::Enter)
    {
        if started {
            action.resume = true;
        } else {
            action.new_run = true;
        }
    }
    if started
        && !online
        && button(
            "NEW RUN WITH SELECTED SHIP  [N]",
            x,
            y + 367. * s,
            660.,
            s,
            false,
        )
    {
        action.new_run = true;
    }
    if started && !online && is_key_pressed(KeyCode::N) {
        action.new_run = true;
    }
    if button(
        &format!("VOLUME  {:.0}%", profile.volume * 100.),
        x,
        y + 430. * s,
        205.,
        s,
        false,
    ) {
        profile.volume = if profile.volume >= 0.99 {
            0.
        } else {
            (profile.volume + 0.2).min(1.)
        };
    }
    if button(
        &format!("SHAKE  {:.0}%", profile.shake * 100.),
        x + 220. * s,
        y + 430. * s,
        205.,
        s,
        false,
    ) {
        profile.shake = if profile.shake >= 0.99 {
            0.
        } else {
            (profile.shake + 0.25).min(1.)
        };
    }
    if button(
        if profile.flashes {
            "FLASHES  ON"
        } else {
            "FLASHES  OFF"
        },
        x + 440. * s,
        y + 430. * s,
        205.,
        s,
        false,
    ) {
        profile.flashes = !profile.flashes;
    }
    if button(
        &format!("HUD SCALE  {:.0}%", profile.ui_scale * 100.),
        x,
        y + 482. * s,
        315.,
        s,
        false,
    ) {
        profile.ui_scale = if profile.ui_scale >= 1.24 {
            0.75
        } else {
            profile.ui_scale + 0.25
        };
    }
    if button(
        "FULLSCREEN  [F11]",
        x + 335. * s,
        y + 482. * s,
        310.,
        s,
        false,
    ) {
        profile.fullscreen = !profile.fullscreen;
        set_fullscreen(profile.fullscreen);
    }
    for (i, line) in [
        "WASD move   |   Mouse aim   |   LMB / Space fire",
        "RMB / Shift ability   |   Q/E / wheel / 1-3 weapon",
        "Esc menu   |   F1 help   |   R restart after death",
    ]
    .into_iter()
    .enumerate()
    {
        draw_text(line, x, y + (570. + i as f32 * 28.) * s, 20. * s, GRAY);
    }
    if button("QUIT", x, y + 680. * s, 660., s, false) {
        action.quit = true;
    }
    action
}
pub fn hud(world: &World, profile: &Profile, skipped: u64) {
    let s = (screen_height() / 900.).min(1.) * profile.ui_scale;
    let ship = &world.player;
    draw_rectangle(
        16. * s,
        16. * s,
        324. * s,
        150. * s,
        Color::new(0.02, 0.04, 0.09, 0.85),
    );
    draw_text(
        &format!(
            "{}  SCORE {}  x{:.2}",
            ship.class.name(),
            world.score,
            world.combo
        ),
        28. * s,
        40. * s,
        22. * s,
        WHITE,
    );
    for (row, value, base, color, label) in [
        (0, ship.hp, ship.class.stats().0, GREEN, "HULL"),
        (1, ship.energy, 1000., SKYBLUE, "ENERGY"),
    ] {
        let y = (55. + row as f32 * 32.) * s;
        draw_rectangle(
            28. * s,
            y,
            290. * s,
            14. * s,
            Color::new(0.08, 0.12, 0.18, 1.),
        );
        draw_rectangle(
            28. * s,
            y,
            290. * s * (value / base).clamp(0., 1.),
            14. * s,
            color,
        );
        if value > base {
            draw_rectangle(
                28. * s,
                y,
                290. * s * ((value - base) / base).clamp(0., 1.),
                4. * s,
                YELLOW,
            );
        }
        draw_text(
            &format!("{} {:.0}", label, value.max(0.)),
            30. * s,
            y + 12. * s,
            15. * s,
            BLACK,
        );
    }
    let ready = if ship.ability_active {
        "ACTIVE"
    } else if ship.ability_latched {
        "RELEASE"
    } else if ship.energy >= 160. {
        "READY"
    } else {
        "LOW E"
    };
    draw_text(
        &format!("{} | {}", ship.weapon.name(), ready),
        28. * s,
        137. * s,
        20. * s,
        WHITE,
    );
    draw_text(
        &format!(
            "{:.0}s  |  BEST {}  |  Esc menu",
            world.tick as f32 * DT,
            profile.records[ship.class.index()]
        ),
        28. * s,
        158. * s,
        17. * s,
        GRAY,
    );
    for (i, weapon) in [
        WeaponKind::MachineGun,
        WeaponKind::Shotgun,
        WeaponKind::Plasma,
    ]
    .into_iter()
    .enumerate()
    {
        let x = screen_width() / 2. - 252. * s + i as f32 * 172. * s;
        let y = screen_height() - 64. * s;
        draw_rectangle(x, y, 164. * s, 44. * s, Color::new(0.025, 0.05, 0.10, 0.9));
        draw_rectangle_lines(
            x,
            y,
            164. * s,
            44. * s,
            1.,
            if ship.weapon == weapon {
                SKYBLUE
            } else {
                DARKGRAY
            },
        );
        draw_text(
            &format!("{} {}", i + 1, weapon.name()),
            x + 8. * s,
            y + 22. * s,
            17. * s,
            WHITE,
        );
        let ready = 1. - (ship.cooldowns[i] / weapon.stats().interval).clamp(0., 1.);
        draw_rectangle(x, y + 38. * s, 164. * s * ready, 3. * s, SKYBLUE);
    }
    draw_text(
        &format!(
            "{} Hz logic | FPS {} | overload {}",
            TICK_PER_SEC,
            get_fps(),
            skipped
        ),
        screen_width() - 320. * s,
        26. * s,
        17. * s,
        GRAY,
    );
}
pub fn help() {
    draw_rectangle(
        0.,
        0.,
        screen_width(),
        screen_height(),
        Color::new(0.01, 0.02, 0.05, 0.92),
    );
    let x = screen_width() / 2. - 360.;
    let y = screen_height() / 2. - 220.;
    for (i, line) in [
        "FLIGHT MANUAL  [F1 to close]",
        "WASD: accelerate; release to coast and slow down.",
        "Aim with mouse. LMB OR Space fires.",
        "RMB OR Shift holds your class ability (costs energy).",
        "Spark boosts. Cadet shields. Thunder deals quad damage.",
        "Release ability to rearm after energy depletion.",
        "Your own shield blocks your shots and consumes energy!",
        "Q/E, wheel or 1/2/3: select a weapon; cooldowns persist.",
        "Energy regenerates after 0.6 s without shots/abilities.",
        "Blue capsules: energy. Green: hull. Yellow bar: overcap.",
        "Chain kills within 4 s for combo. Enemy hits reset it.",
        "No passive hull regeneration. Stay mobile. R after death.",
        "Esc pauses local play. Online combat keeps running.",
    ]
    .into_iter()
    .enumerate()
    {
        draw_text(
            line,
            x,
            y + i as f32 * 33.,
            23.,
            if i == 0 { SKYBLUE } else { WHITE },
        );
    }
}
