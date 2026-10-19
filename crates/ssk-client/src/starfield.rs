//! Owned visual state: bounded phases keep parallax continuous across origin rebases.
use crate::constants::*;
use crate::graphics::*;
use ssk_sim::{Rng, V2};

struct Star {
    uv: V2,
    phase: [f64; 2],
    parallax: f32,
    brightness: f32,
    size: f32,
}
pub struct Starfield {
    stars: Vec<Star>,
    previous_camera: V2,
}
impl Starfield {
    pub fn new(camera: V2) -> Self {
        let mut rng = Rng(STAR_SEED);
        let stars = (0..STAR_COUNT)
            .map(|_| {
                // More distant dim stars, fewer bright foreground stars. All three
                // visual properties follow the same individual depth.
                let near = rng.unit().powf(STAR_DEPTH_BIAS);
                let sample =
                    |range: std::ops::Range<f32>| range.start + near * (range.end - range.start);
                Star {
                    uv: V2::new(rng.unit(), rng.unit()),
                    phase: [0.; 2],
                    parallax: sample(STAR_PARALLAX),
                    brightness: sample(STAR_BRIGHTNESS),
                    size: sample(STAR_SIZE),
                }
            })
            .collect();
        Self {
            stars,
            previous_camera: camera,
        }
    }
    pub fn shift_origin(&mut self, offset: V2) {
        self.previous_camera -= offset;
    }
    pub fn draw(&mut self, canvas: &Canvas, camera: V2) {
        canvas.clear_background(SPACE_COLOR);
        let delta = camera - self.previous_camera;
        self.previous_camera = camera;
        let width = canvas.screen_width().max(1.);
        let height = canvas.screen_height().max(1.);
        for star in &mut self.stars {
            star.phase[0] =
                (star.phase[0] + delta.x as f64 * star.parallax as f64).rem_euclid(width as f64);
            star.phase[1] =
                (star.phase[1] + delta.y as f64 * star.parallax as f64).rem_euclid(height as f64);
            let x = (star.uv.x * width - star.phase[0] as f32).rem_euclid(width);
            let y = (star.uv.y * height - star.phase[1] as f32).rem_euclid(height);
            let b = star.brightness;
            canvas.draw_rectangle(
                x,
                y,
                star.size,
                star.size,
                Color::new(b * STAR_TINT[0], b * STAR_TINT[1], b * STAR_TINT[2], 1.),
            );
        }
        for x in -SPACE_GRID_COLUMNS..=SPACE_GRID_COLUMNS {
            let p = canvas.screen(
                V2::new(
                    ((camera.x / SPACE_GRID_SIZE).floor() + x as f32) * SPACE_GRID_SIZE,
                    0.,
                ),
                camera,
            );
            canvas.draw_line(p.x, 0., p.x, height, 1., SPACE_GRID_COLOR);
        }
        for y in -SPACE_GRID_ROWS..=SPACE_GRID_ROWS {
            let p = canvas.screen(
                V2::new(
                    0.,
                    ((camera.y / SPACE_GRID_SIZE).floor() + y as f32) * SPACE_GRID_SIZE,
                ),
                camera,
            );
            canvas.draw_line(0., p.y, width, p.y, 1., SPACE_GRID_COLOR);
        }
    }
}
