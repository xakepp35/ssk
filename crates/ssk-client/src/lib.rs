use macroquad::prelude::*;
use ssk_sim::{DT, InputFrame, Ship, V2, World};
pub struct Options {
    pub seed: u64, pub class: ssk_sim::ShipClass, pub play: bool,
    pub online: bool, pub connect: Option<String>,
}
pub fn launch(options: Options) {
    macroquad::Window::from_config(Conf {
        window_title: "Syrup Space Kadet".into(), window_width: 1440, window_height: 900, high_dpi: true,
        platform: miniquad::conf::Platform { swap_interval: Some(0), ..Default::default() },
        ..Default::default()
    }, run(options));
}
fn screen(p: V2, camera: V2) -> Vec2 {
    vec2(p.x-camera.x+screen_width()/2.,p.y-camera.y+screen_height()/2.)
}
fn draw_ship(ship: &Ship, camera: V2, alpha: f32, color: Color) {
    let pos=ship.previous.lerp(ship.pos,alpha);
    let forward=V2::from_angle(ship.angle); let side=forward.perp(); let radius=ship.class.stats().3;
    let a=screen(pos+forward*radius*1.5,camera);
    let b=screen(pos-forward*radius+side*radius*0.8,camera);
    let c=screen(pos-forward*radius-side*radius*0.8,camera);
    draw_triangle(a,b,c,Color::new(color.r*0.15,color.g*0.15,color.b*0.15,1.));
    draw_triangle_lines(a,b,c,2.,color);
    let p=screen(pos,camera); draw_circle(p.x,p.y,3.,WHITE);
    if ship.velocity.length()>10. {
        let end=screen(pos-forward*(radius+8.+(get_time() as f32*67.).sin()*4.),camera);
        let start=screen(pos-forward*radius,camera);
        draw_line(start.x,start.y,end.x,end.y,5.,SKYBLUE);
    }
}
fn background(camera: V2) {
    clear_background(Color::from_rgba(6,9,21,255));
    for layer in 0..3 { for i in 0..110 {
        let parallax=0.1+layer as f32*0.14;
        let x=(i as f32*173.73+(i*i) as f32*2.1-camera.x*parallax).rem_euclid(screen_width());
        let y=(i as f32*91.13+(i*i) as f32*3.3-camera.y*parallax).rem_euclid(screen_height());
        let b=0.18+layer as f32*0.11;
        draw_circle(x,y,0.6+layer as f32*0.3,Color::new(b,b+0.07,b+0.18,1.));
    }}
    let color=Color::new(0.08,0.16,0.24,0.3);
    for x in -8..=8 {
        let p=screen(V2::new(((camera.x/256.).floor()+x as f32)*256.,0.),camera);
        draw_line(p.x,0.,p.x,screen_height(),1.,color);
    }
    for y in -6..=6 {
        let p=screen(V2::new(0.,((camera.y/256.).floor()+y as f32)*256.),camera);
        draw_line(0.,p.y,screen_width(),p.y,1.,color);
    }
}
async fn run(options: Options) {
    let mut world=World::new(options.seed,options.class);
    let mut accumulator=0.; let mut camera=V2::ZERO; let mut fullscreen=false; let mut dropped_steps=0u64; let mut weapon=0;
    loop {
        let frame_start=std::time::Instant::now();
        if is_key_pressed(KeyCode::Escape) { break; }
        if is_key_pressed(KeyCode::F11) { fullscreen=!fullscreen; set_fullscreen(fullscreen); }
        let dt=get_frame_time().min(0.1);
        if is_key_pressed(KeyCode::Q)||mouse_wheel().1>0. { weapon=(weapon+2)%3; }
        if is_key_pressed(KeyCode::E)||mouse_wheel().1<0. { weapon=(weapon+1)%3; }
        for (i,key) in [KeyCode::Key1,KeyCode::Key2,KeyCode::Key3].into_iter().enumerate() { if is_key_pressed(key) { weapon=i; } }
        let mouse=V2::new(mouse_position().0-screen_width()/2.,mouse_position().1-screen_height()/2.);
        let input=InputFrame {
            movement: V2::new((is_key_down(KeyCode::D) as i8-is_key_down(KeyCode::A) as i8) as f32,(is_key_down(KeyCode::S) as i8-is_key_down(KeyCode::W) as i8) as f32),
            aim:(camera+mouse-world.player.pos).angle(),
            fire:is_mouse_button_down(MouseButton::Left)||is_key_down(KeyCode::Space),
            ability:is_mouse_button_down(MouseButton::Right)||is_key_down(KeyCode::LeftShift)||is_key_down(KeyCode::RightShift), weapon,
        };
        accumulator+=dt; let mut steps=0;
        while accumulator>=DT && steps<8 { world.step(input); accumulator-=DT; steps+=1; }
        if accumulator>=DT { dropped_steps+=(accumulator/DT) as u64; accumulator=accumulator.rem_euclid(DT); }
        let alpha=accumulator/DT;
        camera-=world.origin_shift; world.origin_shift=V2::ZERO;
        let target=world.player.previous.lerp(world.player.pos,alpha)+mouse.clamp_length(200.)*0.25;
        camera=camera.lerp(target,1.-(-12.*dt).exp());
        background(camera);
        for target in &world.asteroids {
            let p=screen(target.pos,camera);
            if p.x < -100. || p.y < -100. || p.x>screen_width()+100. || p.y>screen_height()+100. { continue; }
            for i in 0..9 {
                let a=target.angle+i as f32*std::f32::consts::TAU/9.; let b=target.angle+(i+1) as f32*std::f32::consts::TAU/9.;
                let r1=target.radius*(0.85+((target.id.wrapping_mul(i+1)%17) as f32)*0.009);
                let r2=target.radius*(0.85+((target.id.wrapping_mul((i+1)%9+1)%17) as f32)*0.009);
                draw_line(p.x+a.cos()*r1,p.y+a.sin()*r1,p.x+b.cos()*r2,p.y+b.sin()*r2,2.,Color::new(0.35,0.4,0.5,1.));
            }
        }
        for bullet in &world.bullets {
            let p=screen(bullet.previous.lerp(bullet.pos,alpha),camera);
            let tail=screen(bullet.pos-bullet.velocity*0.012,camera);
            let color=match bullet.shot.weapon { ssk_sim::WeaponKind::Plasma=>VIOLET, ssk_sim::WeaponKind::Shotgun=>ORANGE, _=>YELLOW };
            draw_line(tail.x,tail.y,p.x,p.y,2.,color); draw_circle(p.x,p.y,bullet.radius,color);
        }
        draw_ship(&world.player,camera,alpha,SKYBLUE);
        draw_rectangle(24.,52.,240.,10.,DARKGRAY);
        draw_rectangle(24.,52.,240.*world.player.energy/1000.,10.,SKYBLUE);
        draw_text(&format!("[{}] {}  | Q/E or wheel",weapon+1,world.player.weapon.name()),24.,90.,24.,WHITE);
        world.events.clear();
        draw_text(&format!("{} | WASD move  MOUSE aim | 240 Hz  skipped {}",world.player.class.name(),dropped_steps),24.,32.,22.,GRAY);
        draw_circle_lines(mouse_position().0,mouse_position().1,8.,1.,SKYBLUE);
        next_frame().await;
        if let Some(remaining)=std::time::Duration::from_secs_f64(1./240.).checked_sub(frame_start.elapsed()) { std::thread::sleep(remaining); }
    }
}
