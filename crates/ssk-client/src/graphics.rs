//! Thin drawing/input adapter over egui's wgpu painter. All frame state is owned;
//! this module has no graphics context globals or simulation responsibilities.
use eframe::egui::{self, Align2, FontId, Pos2, Shape, Stroke};
use ssk_sim::V2;

pub use crate::constants::palette::*;
pub use egui::Key as KeyCode;
pub use egui::PointerButton as MouseButton;

#[derive(Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }
    pub fn egui(self) -> egui::Color32 {
        let channel = |v: f32| (v.clamp(0., 1.) * u8::MAX as f32).round() as u8;
        egui::Color32::from_rgba_unmultiplied(
            channel(self.r),
            channel(self.g),
            channel(self.b),
            channel(self.a),
        )
    }
}

pub struct Canvas {
    pub ctx: egui::Context,
    painter: egui::Painter,
    rect: egui::Rect,
    pub dt: f32,
    pub time: f64,
}
impl Canvas {
    pub fn new(ctx: &egui::Context, rect: egui::Rect, dt: f32, time: f64) -> Self {
        Self {
            ctx: ctx.clone(),
            rect,
            dt,
            time,
            painter: ctx
                .layer_painter(egui::LayerId::background())
                .with_clip_rect(rect),
        }
    }
    pub fn screen_width(&self) -> f32 {
        self.rect.width()
    }
    pub fn screen_height(&self) -> f32 {
        self.rect.height()
    }
    pub fn get_time(&self) -> f64 {
        self.time
    }
    pub fn get_frame_time(&self) -> f32 {
        self.dt
    }
    pub fn get_fps(&self) -> i32 {
        self.dt.recip().round() as i32
    }
    pub fn focused(&self) -> bool {
        self.ctx.input(|i| i.focused)
    }
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.ctx.input(|i| i.key_pressed(key))
    }
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.focused() && self.ctx.input(|i| i.key_down(key))
    }
    pub fn shift_down(&self) -> bool {
        self.focused() && self.ctx.input(|i| i.modifiers.shift)
    }
    pub fn is_mouse_button_down(&self, key: MouseButton) -> bool {
        self.focused() && self.ctx.input(|i| i.pointer.button_down(key))
    }
    pub fn is_mouse_button_pressed(&self, key: MouseButton) -> bool {
        self.focused() && self.ctx.input(|i| i.pointer.button_pressed(key))
    }
    pub fn mouse_position(&self) -> (f32, f32) {
        let p = self
            .ctx
            .input(|i| i.pointer.hover_pos())
            .unwrap_or(self.rect.center())
            - self.rect.min;
        (p.x, p.y)
    }
    pub fn pointer_inside(&self) -> bool {
        self.ctx
            .input(|i| i.pointer.hover_pos().is_some_and(|p| self.rect.contains(p)))
    }
    pub fn mouse_wheel(&self) -> (f32, f32) {
        self.ctx
            .input(|i| (i.raw_scroll_delta.x, i.raw_scroll_delta.y))
    }
    pub fn set_fullscreen(&self, fullscreen: bool) {
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
    }
    pub fn screen(&self, p: V2, camera: V2) -> V2 {
        p - camera + V2::new(self.screen_width(), self.screen_height()) * 0.5
    }
    fn point(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + egui::vec2(x, y)
    }
    pub fn clear_background(&self, color: Color) {
        self.painter.rect_filled(self.rect, 0., color.egui());
    }
    pub fn draw_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: Color) {
        self.painter.line_segment(
            [self.point(x1, y1), self.point(x2, y2)],
            Stroke::new(width, color.egui()),
        );
    }
    pub fn draw_rectangle(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.painter.rect_filled(
            egui::Rect::from_min_size(self.point(x, y), egui::vec2(w.max(0.), h.max(0.))),
            0.,
            color.egui(),
        );
    }
    pub fn draw_rectangle_lines(&self, x: f32, y: f32, w: f32, h: f32, width: f32, color: Color) {
        self.painter.rect_stroke(
            egui::Rect::from_min_size(self.point(x, y), egui::vec2(w, h)),
            0.,
            Stroke::new(width, color.egui()),
            egui::StrokeKind::Inside,
        );
    }
    pub fn draw_circle(&self, x: f32, y: f32, radius: f32, color: Color) {
        self.painter
            .circle_filled(self.point(x, y), radius.max(0.), color.egui());
    }
    pub fn draw_circle_lines(&self, x: f32, y: f32, radius: f32, width: f32, color: Color) {
        self.painter.circle_stroke(
            self.point(x, y),
            radius.max(0.),
            Stroke::new(width, color.egui()),
        );
    }
    pub fn draw_triangle(&self, a: V2, b: V2, c: V2, color: Color) {
        self.painter.add(Shape::convex_polygon(
            vec![
                self.point(a.x, a.y),
                self.point(b.x, b.y),
                self.point(c.x, c.y),
            ],
            color.egui(),
            Stroke::NONE,
        ));
    }
    pub fn draw_triangle_lines(&self, a: V2, b: V2, c: V2, width: f32, color: Color) {
        self.painter.add(Shape::closed_line(
            vec![
                self.point(a.x, a.y),
                self.point(b.x, b.y),
                self.point(c.x, c.y),
            ],
            Stroke::new(width, color.egui()),
        ));
    }
    pub fn draw_poly_lines(
        &self,
        x: f32,
        y: f32,
        sides: usize,
        radius: f32,
        degrees: f32,
        width: f32,
        color: Color,
    ) {
        let points = (0..sides)
            .map(|i| {
                let angle = degrees.to_radians() + i as f32 * std::f32::consts::TAU / sides as f32;
                self.point(x + angle.cos() * radius, y + angle.sin() * radius)
            })
            .collect();
        self.painter
            .add(Shape::closed_line(points, Stroke::new(width, color.egui())));
    }
    pub fn draw_text(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.painter.text(
            self.point(x, y),
            Align2::LEFT_BOTTOM,
            text,
            FontId::proportional(size),
            color.egui(),
        );
    }
}
