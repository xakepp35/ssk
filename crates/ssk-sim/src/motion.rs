//! Bounded visual dead reckoning. Never used for ownership, damage or rewards.
use crate::*;

#[derive(Clone, Copy, Default)]
pub(crate) struct MotionCorrection {
    position: V2,
    angle: f32,
}
impl MotionCorrection {
    pub fn between(previous: (V2, f32), position: V2, angle: f32) -> Self {
        let error = previous.0 - position;
        if error.length_squared() > SHARED_VISUAL_SNAP_DISTANCE.powi(2) {
            return Self::default();
        }
        Self {
            position: error,
            angle: (previous.1 - angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI,
        }
    }
    pub fn sample(
        self,
        position: V2,
        velocity: V2,
        angle: f32,
        spin: f32,
        age: f64,
        horizon: f64,
    ) -> (V2, f32) {
        let age = age.max(0.);
        let prediction = age.min(horizon) as f32;
        let decay = (-age / SHARED_CORRECTION_SECONDS).exp() as f32;
        (
            position + velocity * prediction + self.position * decay,
            angle + spin * prediction + self.angle * decay,
        )
    }
}
