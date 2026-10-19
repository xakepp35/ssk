use crate::{draw_ship, screen};
use macroquad::prelude::*;
use ssk_sim::*;
pub fn scene(world: &World, camera: V2, alpha: f32) {
    for target in &world.asteroids {
        let p = screen(target.pos, camera);
        if p.x < -100. || p.y < -100. || p.x > screen_width() + 100. || p.y > screen_height() + 100.
        {
            continue;
        }
        for i in 0..9 {
            let a = target.angle + i as f32 * std::f32::consts::TAU / 9.;
            let b = target.angle + (i + 1) as f32 * std::f32::consts::TAU / 9.;
            let r1 = target.radius * (0.85 + ((target.id.wrapping_mul(i + 1) % 17) as f32) * 0.009);
            let r2 = target.radius
                * (0.85 + ((target.id.wrapping_mul((i + 1) % 9 + 1) % 17) as f32) * 0.009);
            draw_line(
                p.x + a.cos() * r1,
                p.y + a.sin() * r1,
                p.x + b.cos() * r2,
                p.y + b.sin() * r2,
                2.,
                Color::new(0.35, 0.4, 0.5, 1.),
            );
        }
    }
    for bullet in &world.bullets {
        let p = screen(bullet.previous.lerp(bullet.pos, alpha), camera);
        let tail = screen(bullet.pos - bullet.velocity * 0.012, camera);
        let color = match bullet.shot.weapon {
            ssk_sim::WeaponKind::Plasma => VIOLET,
            ssk_sim::WeaponKind::Shotgun => ORANGE,
            _ => YELLOW,
        };
        draw_line(tail.x, tail.y, p.x, p.y, 2., color);
        draw_circle(p.x, p.y, bullet.radius, color);
    }
    draw_ship(&world.player, camera, alpha, SKYBLUE);
    for pickup in &world.pickups {
        let p = screen(pickup.pos, camera);
        let color = match pickup.kind {
            ssk_sim::PickupKind::Energy => SKYBLUE,
            ssk_sim::PickupKind::Health => GREEN,
        };
        let pulse = 1. + (get_time() as f32 * 5. + pickup.id as f32).sin() * 0.15;
        draw_poly_lines(p.x, p.y, 4, 10. * pulse, 45., 2., color);
        draw_line(p.x - 4., p.y, p.x + 4., p.y, 2., color);
        if matches!(pickup.kind, ssk_sim::PickupKind::Health) {
            draw_line(p.x, p.y - 4., p.x, p.y + 4., 2., color);
        }
    }
    for npc in &world.npcs {
        if npc.telegraph > 0. {
            let p = screen(npc.ship.pos, camera);
            draw_circle_lines(p.x, p.y, 45. * npc.telegraph / 0.8, 2., RED);
            let edge = vec2(
                p.x.clamp(24., screen_width() - 24.),
                p.y.clamp(180., screen_height() - 24.),
            );
            draw_circle_lines(edge.x, edge.y, 12., 2., RED);
        } else {
            draw_ship(&npc.ship, camera, alpha, PINK);
        }
    }
}
