use super::screen;
use crate::constants::*;
use macroquad::{
    audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound},
    prelude::*,
};
use ssk_sim::{PickupKind, Rng, V2, WeaponKind, WorldEvent};
struct Particle {
    pos: V2,
    velocity: V2,
    life: f32,
    max: f32,
    color: Color,
    size: f32,
}
struct Ring {
    pos: V2,
    life: f32,
    size: f32,
    color: Color,
}
pub struct Effects {
    particles: Vec<Particle>,
    rings: Vec<Ring>,
    sounds: Vec<Option<Sound>>,
    rng: Rng,
    pub shake: f32,
    pub flash: f32,
}
impl Effects {
    pub async fn new() -> Self {
        let mut sounds = Vec::new();
        for (frequency, duration, noise) in SOUND_PRESETS {
            sounds.push(
                load_sound_from_bytes(&wave(frequency, duration, noise))
                    .await
                    .ok(),
            );
        }
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
            rings: Vec::with_capacity(MAX_RINGS),
            sounds,
            rng: Rng(EFFECT_RNG_SEED),
            shake: 0.,
            flash: 0.,
        }
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.rings.clear();
        self.shake = 0.;
        self.flash = 0.;
    }
    pub fn shift(&mut self, offset: V2) {
        for p in &mut self.particles {
            p.pos -= offset;
        }
        for r in &mut self.rings {
            r.pos -= offset;
        }
    }
    fn sound(&self, index: usize, volume: f32) {
        if volume > 0. {
            if let Some(sound) = &self.sounds[index] {
                play_sound(
                    sound,
                    PlaySoundParams {
                        looped: false,
                        volume,
                    },
                );
            }
        }
    }
    fn burst(&mut self, pos: V2, count: usize, color: Color, speed: f32) {
        for _ in 0..count {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }
            let life = self.rng.range(PARTICLE_LIFE.start, PARTICLE_LIFE.end);
            self.particles.push(Particle {
                pos,
                velocity: V2::from_angle(self.rng.range(0., std::f32::consts::TAU))
                    * self.rng.range(PARTICLE_MIN_SPEED, speed),
                life,
                max: life,
                color,
                size: self.rng.range(PARTICLE_SIZE.start, PARTICLE_SIZE.end),
            });
        }
    }
    pub fn events(&mut self, events: &[WorldEvent], camera: V2, volume: f32) {
        for event in events {
            let pos = match *event {
                WorldEvent::Shot(s) => s.pos,
                WorldEvent::Impact { pos, .. }
                | WorldEvent::Destroyed { pos, .. }
                | WorldEvent::Pickup { pos, .. } => pos,
                _ => camera,
            };
            let distance = (pos - camera).length();
            if distance > EFFECT_DISTANCE {
                continue;
            }
            let volume = volume * (1. - distance / SOUND_DISTANCE).max(0.) * SOUND_GAIN;
            match *event {
                WorldEvent::Shot(shot) => {
                    let (index, color) = match shot.weapon {
                        WeaponKind::MachineGun => (0, YELLOW),
                        WeaponKind::Shotgun => (1, ORANGE),
                        WeaponKind::Plasma => (2, VIOLET),
                    };
                    self.burst(pos, MUZZLE_PARTICLES, color, MUZZLE_PARTICLE_SPEED);
                    self.sound(index, volume);
                }
                WorldEvent::Impact { damage, .. } => {
                    self.burst(
                        pos,
                        (damage / IMPACT_PARTICLE_DAMAGE)
                            .clamp(*IMPACT_PARTICLE_COUNT.start(), *IMPACT_PARTICLE_COUNT.end())
                            as usize,
                        ORANGE,
                        IMPACT_PARTICLE_SPEED,
                    );
                    self.sound(3, volume * IMPACT_SOUND_GAIN);
                }
                WorldEvent::Destroyed { size, .. } => {
                    self.burst(
                        pos,
                        DESTRUCTION_PARTICLES,
                        ORANGE,
                        DESTRUCTION_PARTICLE_SPEED,
                    );
                    self.sound(4, volume);
                    if self.rings.len() < MAX_RINGS {
                        self.rings.push(Ring {
                            pos,
                            life: RING_LIFE,
                            size: size * RING_SCALE,
                            color: ORANGE,
                        });
                    }
                    self.shake = (self.shake + SHAKE_IMPACT * (1. - distance / EFFECT_DISTANCE))
                        .min(SHAKE_MAX);
                }
                WorldEvent::Pickup { kind, .. } => {
                    let color = match kind {
                        PickupKind::Energy => SKYBLUE,
                        PickupKind::Health => GREEN,
                    };
                    self.burst(pos, PICKUP_PARTICLES, color, PICKUP_PARTICLE_SPEED);
                    self.sound(5, volume);
                }
                WorldEvent::Dry => self.sound(6, volume),
                WorldEvent::Reward { .. } => {}
            }
        }
    }
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.pos += p.velocity * dt;
            p.velocity *= (-PARTICLE_DRAG * dt).exp();
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.);
        for r in &mut self.rings {
            r.life -= dt;
        }
        self.rings.retain(|r| r.life > 0.);
        self.shake *= (-SHAKE_DECAY * dt).exp();
        self.flash = (self.flash - dt * FLASH_DECAY).max(0.);
    }
    pub fn offset(&mut self, strength: f32) -> V2 {
        V2::new(self.rng.range(-1., 1.), self.rng.range(-1., 1.)) * self.shake * strength
    }
    pub fn draw(&self, camera: V2, flashes: bool) {
        for p in &self.particles {
            let point = screen(p.pos, camera);
            let tail = screen(p.pos - p.velocity * PARTICLE_TAIL_SECONDS, camera);
            let mut color = p.color;
            color.a = p.life / p.max;
            draw_line(tail.x, tail.y, point.x, point.y, p.size, color);
        }
        for r in &self.rings {
            let p = screen(r.pos, camera);
            let mut color = r.color;
            color.a = r.life * RING_SCALE;
            draw_circle_lines(p.x, p.y, (1. - r.life / RING_LIFE) * r.size, 2., color);
        }
        if flashes && self.flash > 0. {
            draw_rectangle(
                0.,
                0.,
                screen_width(),
                screen_height(),
                Color::new(
                    FLASH_COLOR[0],
                    FLASH_COLOR[1],
                    FLASH_COLOR[2],
                    self.flash * FLASH_COLOR[3],
                ),
            );
        }
    }
}
fn wave(frequency: f32, duration: f32, noise: f32) -> Vec<u8> {
    let samples = (AUDIO_SAMPLE_RATE as f32 * duration) as usize;
    let size = (samples * AUDIO_SAMPLE_BYTES as usize) as u32;
    let mut bytes = Vec::with_capacity(samples * AUDIO_SAMPLE_BYTES as usize + WAVE_HEADER_BYTES);
    bytes.extend(b"RIFF");
    bytes.extend((size + WAVE_HEADER_BYTES as u32 - 8).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(WAVE_FMT_BYTES.to_le_bytes());
    bytes.extend(WAVE_PCM_FORMAT.to_le_bytes());
    bytes.extend(WAVE_PCM_FORMAT.to_le_bytes());
    bytes.extend(AUDIO_SAMPLE_RATE.to_le_bytes());
    bytes.extend((AUDIO_SAMPLE_RATE * AUDIO_SAMPLE_BYTES as u32).to_le_bytes());
    bytes.extend(AUDIO_SAMPLE_BYTES.to_le_bytes());
    bytes.extend(AUDIO_BITS.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(size.to_le_bytes());
    let mut rng = Rng(WAVE_RNG_SEED);
    let mut phase = 0.;
    for i in 0..samples {
        let t = i as f32 / samples as f32;
        phase += std::f32::consts::TAU * frequency * (1. - t * AUDIO_PITCH_DECAY)
            / AUDIO_SAMPLE_RATE as f32;
        let envelope = (1. - t).powf(2.) * (t * AUDIO_ATTACK).min(1.);
        let sample = ((phase.sin() * (1. - noise) + rng.range(-1., 1.) * noise)
            * envelope
            * AUDIO_AMPLITUDE) as i16;
        bytes.extend(sample.to_le_bytes());
    }
    bytes
}
