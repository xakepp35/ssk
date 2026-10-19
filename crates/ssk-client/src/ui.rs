use crate::constants::{HUD_WIDTH, ui::*};
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
    let r = Rect::new(x, y, w * s, BUTTON_HEIGHT * s);
    let hover = r.contains(mouse_position().into());
    let color = if selected {
        BUTTON_SELECTED
    } else if hover {
        BUTTON_HOVER
    } else {
        BUTTON_IDLE
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
            BUTTON_BORDER
        },
    );
    draw_text(
        label,
        x + TEXT_INSET * s,
        y + BUTTON_BASELINE * s,
        FONT_BUTTON * s,
        WHITE,
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
pub fn menu(profile: &mut Profile, started: bool, online: bool) -> MenuAction {
    draw_rectangle(0., 0., screen_width(), screen_height(), MENU_BACKGROUND);
    let s = (screen_height() / MENU_REFERENCE_HEIGHT).min(1.) * profile.ui_scale;
    let x = screen_width() / 2. - MENU_HALF_WIDTH * s;
    let y = (screen_height() - MENU_HEIGHT * s) * 0.5;
    draw_text("SYRUP SPACE", x, y + TITLE_TOP * s, FONT_TITLE * s, WHITE);
    draw_text("KADET", x, y + SUBTITLE_TOP * s, FONT_TITLE * s, SKYBLUE);
    draw_text(
        if online {
            "PEER DUEL"
        } else {
            "ONE SHIP. THREE GUNS. SURVIVE."
        },
        x,
        y + TAGLINE_TOP * s,
        FONT_BODY * s,
        GRAY,
    );
    for (i, class) in ShipClass::ALL.into_iter().enumerate() {
        if button(
            class.name(),
            x + i as f32 * COLUMN_STRIDE * s,
            y + CLASSES_TOP * s,
            COLUMN_WIDTH,
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
        y + CLASS_STATS_TOP * s,
        FONT_STATS * s,
        WHITE,
    );
    let description = match class {
        ShipClass::Spark => "BOOST: fast flanking, fragile hull",
        ShipClass::Cadet => "SHIELD: absorb half damage; stop firing!",
        ShipClass::Thunder => "QUAD: devastating bursts, heavy energy use",
    };
    draw_text(
        description,
        x,
        y + CLASS_DESCRIPTION_TOP * s,
        FONT_DESCRIPTION * s,
        GREEN,
    );
    let mut action = MenuAction::default();
    if button(
        if started {
            "RESUME  [Esc / Enter]"
        } else {
            "START SURVIVAL  [Enter]"
        },
        x,
        y + START_TOP * s,
        MENU_WIDTH,
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
            y + NEW_RUN_TOP * s,
            MENU_WIDTH,
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
        &format!("VOLUME  {:.0}%", profile.volume * crate::constants::PERCENT),
        x,
        y + SETTINGS_TOP * s,
        COLUMN_WIDTH,
        s,
        false,
    ) {
        profile.volume = if profile.volume >= SETTING_WRAP {
            0.
        } else {
            (profile.volume + VOLUME_STEP).min(1.)
        };
    }
    if button(
        &format!("SHAKE  {:.0}%", profile.shake * crate::constants::PERCENT),
        x + COLUMN_STRIDE * s,
        y + SETTINGS_TOP * s,
        COLUMN_WIDTH,
        s,
        false,
    ) {
        profile.shake = if profile.shake >= SETTING_WRAP {
            0.
        } else {
            (profile.shake + SETTING_STEP).min(1.)
        };
    }
    if button(
        if profile.flashes {
            "FLASHES  ON"
        } else {
            "FLASHES  OFF"
        },
        x + FLASHES_X * s,
        y + SETTINGS_TOP * s,
        COLUMN_WIDTH,
        s,
        false,
    ) {
        profile.flashes = !profile.flashes;
    }
    if button(
        &format!(
            "HUD SCALE  {:.0}%",
            profile.ui_scale * crate::constants::PERCENT
        ),
        x,
        y + DISPLAY_TOP * s,
        START_TOP,
        s,
        false,
    ) {
        profile.ui_scale = if profile.ui_scale >= SCALE_WRAP {
            SCALE_MIN
        } else {
            profile.ui_scale + SETTING_STEP
        };
    }
    if button(
        "FULLSCREEN  [F11]",
        x + FULLSCREEN_X * s,
        y + DISPLAY_TOP * s,
        FULLSCREEN_WIDTH,
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
        draw_text(
            line,
            x,
            y + (CONTROLS_TOP + i as f32 * HUD_INSET) * s,
            FONT_BODY * s,
            GRAY,
        );
    }
    if button("QUIT", x, y + QUIT_TOP * s, MENU_WIDTH, s, false) {
        action.quit = true;
    }
    action
}
pub fn hud(world: &World, profile: &Profile, skipped: u64) {
    let s = (screen_height() / HUD_REFERENCE_HEIGHT).min(1.) * profile.ui_scale;
    let ship = &world.player;
    draw_rectangle(
        HUD_MARGIN * s,
        HUD_MARGIN * s,
        HUD_WIDTH * s,
        HUD_HEIGHT * s,
        HUD_BACKGROUND,
    );
    draw_text(
        &format!(
            "{}  SCORE {}  x{:.2}",
            ship.class.name(),
            world.score,
            world.combo
        ),
        HUD_INSET * s,
        BUTTON_HEIGHT * s,
        FONT_BUTTON * s,
        WHITE,
    );
    for (row, value, base, color, label) in [
        (0, ship.hp, ship.class.stats().0, GREEN, "HULL"),
        (1, ship.energy, ENERGY_BASE, SKYBLUE, "ENERGY"),
    ] {
        let y = (TITLE_TOP + row as f32 * BAR_ROW_HEIGHT) * s;
        draw_rectangle(
            HUD_INSET * s,
            y,
            BAR_WIDTH * s,
            BAR_HEIGHT * s,
            BAR_BACKGROUND,
        );
        draw_rectangle(
            HUD_INSET * s,
            y,
            BAR_WIDTH * s * (value / base).clamp(0., 1.),
            BAR_HEIGHT * s,
            color,
        );
        if value > base {
            draw_rectangle(
                HUD_INSET * s,
                y,
                BAR_WIDTH * s * ((value - base) / base).clamp(0., 1.),
                OVERCAP_BAR_HEIGHT * s,
                YELLOW,
            );
        }
        draw_text(
            &format!("{} {:.0}", label, value.max(0.)),
            BAR_LABEL_X * s,
            y + TEXT_INSET * s,
            FONT_BAR * s,
            BLACK,
        );
    }
    let ready = if ship.ability_active {
        "ACTIVE"
    } else if ship.ability_latched {
        "RELEASE"
    } else if ship.energy >= ABILITY_MIN_ENERGY {
        "READY"
    } else {
        "LOW E"
    };
    draw_text(
        &format!("{} | {}", ship.weapon.name(), ready),
        HUD_INSET * s,
        WEAPON_INFO_TOP * s,
        FONT_BODY * s,
        WHITE,
    );
    draw_text(
        &format!(
            "{:.0}s  |  BEST {}  |  Esc menu",
            world.tick as f32 * DT,
            profile.records[ship.class.index()]
        ),
        HUD_INSET * s,
        RECORD_TOP * s,
        FONT_CAPTION * s,
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
        let x = screen_width() / 2. - WEAPONS_HALF_WIDTH * s + i as f32 * WEAPON_STRIDE * s;
        let y = screen_height() - WEAPONS_BOTTOM * s;
        draw_rectangle(x, y, WEAPON_WIDTH * s, WEAPON_HEIGHT * s, WEAPON_BACKGROUND);
        draw_rectangle_lines(
            x,
            y,
            WEAPON_WIDTH * s,
            WEAPON_HEIGHT * s,
            1.,
            if ship.weapon == weapon {
                SKYBLUE
            } else {
                DARKGRAY
            },
        );
        draw_text(
            &format!("{} {}", i + 1, weapon.name()),
            x + WEAPON_TEXT_INSET * s,
            y + FONT_BUTTON * s,
            FONT_CAPTION * s,
            WHITE,
        );
        let ready = 1. - (ship.cooldowns[i] / weapon.stats().interval).clamp(0., 1.);
        draw_rectangle(
            x,
            y + COOLDOWN_TOP * s,
            WEAPON_WIDTH * s * ready,
            3. * s,
            SKYBLUE,
        );
    }
    draw_text(
        &format!(
            "{} Hz logic | FPS {} | overload {}",
            TICK_PER_SEC,
            get_fps(),
            skipped
        ),
        screen_width() - PERF_WIDTH * s,
        PERF_TOP * s,
        FONT_CAPTION * s,
        GRAY,
    );
}
pub fn help() {
    draw_rectangle(0., 0., screen_width(), screen_height(), HELP_BACKGROUND);
    let x = screen_width() / 2. - HELP_HALF_WIDTH;
    let y = screen_height() / 2. - COLUMN_STRIDE;
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
            y + i as f32 * HELP_ROW_HEIGHT,
            FONT_STATS,
            if i == 0 { SKYBLUE } else { WHITE },
        );
    }
}
