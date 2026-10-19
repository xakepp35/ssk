use crate::constants::*;
use crate::{draw_ship, screen};
use macroquad::prelude::*;
use ssk_sim::*;
pub fn scene(world: &World, camera: V2, alpha: f32) {
    for target in &world.asteroids {
        let p = screen(target.pos, camera);
        if p.x < -SCENE_CULL_MARGIN
            || p.y < -SCENE_CULL_MARGIN
            || p.x > screen_width() + SCENE_CULL_MARGIN
            || p.y > screen_height() + SCENE_CULL_MARGIN
        {
            continue;
        }
        for i in 0..ROCK_VERTICES {
            let a = target.angle + i as f32 * std::f32::consts::TAU / ROCK_VERTICES as f32;
            let b = target.angle + (i + 1) as f32 * std::f32::consts::TAU / ROCK_VERTICES as f32;
            let r1 = target.radius
                * (ROCK_SHAPE_BASE
                    + ((target.id.wrapping_mul(i + 1) % ROCK_SHAPE_BUCKETS) as f32)
                        * ROCK_SHAPE_SCALE);
            let r2 = target.radius
                * (ROCK_SHAPE_BASE
                    + ((target.id.wrapping_mul((i + 1) % ROCK_VERTICES + 1) % ROCK_SHAPE_BUCKETS)
                        as f32)
                        * ROCK_SHAPE_SCALE);
            draw_line(
                p.x + a.cos() * r1,
                p.y + a.sin() * r1,
                p.x + b.cos() * r2,
                p.y + b.sin() * r2,
                2.,
                ROCK_COLOR,
            );
        }
    }
    for bullet in &world.bullets {
        let p = screen(bullet.previous.lerp(bullet.pos, alpha), camera);
        let tail = screen(bullet.pos - bullet.velocity * BULLET_TAIL_SECONDS, camera);
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
        let pulse = 1.
            + (get_time() as f32 * PICKUP_PULSE_HZ + pickup.id as f32).sin() * PICKUP_PULSE_SCALE;
        draw_poly_lines(
            p.x,
            p.y,
            4,
            PICKUP_ICON_RADIUS * pulse,
            PICKUP_ICON_ANGLE,
            2.,
            color,
        );
        draw_line(
            p.x - PICKUP_ICON_ARM,
            p.y,
            p.x + PICKUP_ICON_ARM,
            p.y,
            2.,
            color,
        );
        if matches!(pickup.kind, ssk_sim::PickupKind::Health) {
            draw_line(
                p.x,
                p.y - PICKUP_ICON_ARM,
                p.x,
                p.y + PICKUP_ICON_ARM,
                2.,
                color,
            );
        }
    }
    for npc in &world.npcs {
        if npc.telegraph > 0. {
            let p = screen(npc.ship.pos, camera);
            draw_circle_lines(
                p.x,
                p.y,
                SPAWN_RING_RADIUS * npc.telegraph / NPC_TELEGRAPH_SECONDS,
                2.,
                RED,
            );
            let edge = vec2(
                p.x.clamp(SPAWN_EDGE_MARGIN, screen_width() - SPAWN_EDGE_MARGIN),
                p.y.clamp(SPAWN_EDGE_TOP, screen_height() - SPAWN_EDGE_MARGIN),
            );
            draw_circle_lines(edge.x, edge.y, SPAWN_EDGE_RADIUS, 2., RED);
        } else {
            draw_ship(&npc.ship, camera, alpha, PINK);
        }
    }
}
