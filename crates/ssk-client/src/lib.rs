use macroquad::prelude::*;

pub struct Options {
    pub seed: u64,
    pub class: ssk_sim::ShipClass,
    pub play: bool,
    pub online: bool,
    pub connect: Option<String>,
}

pub fn launch(options: Options) {
    macroquad::Window::from_config(Conf {
        window_title: "Syrup Space Kadet".into(),
        window_width: 1440,
        window_height: 900,
        high_dpi: true,
        platform: miniquad::conf::Platform { swap_interval: Some(0), ..Default::default() },
        ..Default::default()
    }, run(options));
}

async fn run(_options: Options) {
    loop {
        clear_background(Color::from_rgba(7, 10, 24, 255));
        draw_text("SYRUP SPACE KADET", 60., 100., 48., SKYBLUE);
        draw_text("Flight systems starting up", 60., 150., 24., WHITE);
        if is_key_pressed(KeyCode::Escape) { break; }
        next_frame().await;
    }
}
