use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}
impl V2 {
    pub const ZERO: Self = Self { x: 0., y: 0. };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }
    pub fn normalized(self) -> Self {
        self / self.length().max(crate::NORMAL_EPSILON)
    }
    pub fn clamp_length(self, max: f32) -> Self {
        if self.length_squared() > max * max {
            self.normalized() * max
        } else {
            self
        }
    }
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }
    pub fn from_angle(a: f32) -> Self {
        Self::new(a.cos(), a.sin())
    }
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
impl Add for V2 {
    type Output = Self;
    fn add(self, r: Self) -> Self {
        Self::new(self.x + r.x, self.y + r.y)
    }
}
impl Sub for V2 {
    type Output = Self;
    fn sub(self, r: Self) -> Self {
        Self::new(self.x - r.x, self.y - r.y)
    }
}
impl Mul<f32> for V2 {
    type Output = Self;
    fn mul(self, r: f32) -> Self {
        Self::new(self.x * r, self.y * r)
    }
}
impl Div<f32> for V2 {
    type Output = Self;
    fn div(self, r: f32) -> Self {
        self * (1. / r)
    }
}
impl Neg for V2 {
    type Output = Self;
    fn neg(self) -> Self {
        self * -1.
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, r: Self) {
        *self = *self + r;
    }
}
impl SubAssign for V2 {
    fn sub_assign(&mut self, r: Self) {
        *self = *self - r;
    }
}
impl MulAssign<f32> for V2 {
    fn mul_assign(&mut self, r: f32) {
        *self = *self * r;
    }
}
/// Stable PRNG: content and weapon spreads do not depend on rendering.
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(crate::RNG_INCREMENT);
        let mut z = self.0;
        z = (z ^ (z >> crate::RNG_MIX_SHIFTS[0])).wrapping_mul(crate::RNG_MULTIPLIERS[0]);
        z = (z ^ (z >> crate::RNG_MIX_SHIFTS[1])).wrapping_mul(crate::RNG_MULTIPLIERS[1]);
        z ^ (z >> crate::RNG_MIX_SHIFTS[2])
    }
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> crate::RNG_FLOAT_SHIFT) as f32 / crate::RNG_FLOAT_SCALE
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }
}
