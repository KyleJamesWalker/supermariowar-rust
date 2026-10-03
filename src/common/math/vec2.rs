//! Port of src/common/math/Vec2.h

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

pub type Vec2f = Vec2<f32>;
pub type Vec2s = Vec2<i16>;

impl<T: Default> Vec2<T> {
    pub fn zero() -> Self {
        Vec2::default()
    }
}

impl<T> Vec2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Vec2 { x, y }
    }
}

impl<T: AddAssign> AddAssign for Vec2<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl<T: SubAssign> SubAssign for Vec2<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
impl<T: MulAssign + Copy> MulAssign<T> for Vec2<T> {
    fn mul_assign(&mut self, val: T) {
        self.x *= val;
        self.y *= val;
    }
}
impl<T: DivAssign + Copy> DivAssign<T> for Vec2<T> {
    fn div_assign(&mut self, val: T) {
        self.x /= val;
        self.y /= val;
    }
}
impl<T: Add<Output = T>> Add for Vec2<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}
impl<T: Sub<Output = T>> Sub for Vec2<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Vec2 { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}
impl<T: Mul<Output = T>> Mul for Vec2<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Vec2 { x: self.x * rhs.x, y: self.y * rhs.y }
    }
}
impl<T: Div<Output = T>> Div for Vec2<T> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Vec2 { x: self.x / rhs.x, y: self.y / rhs.y }
    }
}
impl<T: Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Self;
    fn mul(self, s: T) -> Self {
        Vec2 { x: self.x * s, y: self.y * s }
    }
}
impl<T: Div<Output = T> + Copy> Div<T> for Vec2<T> {
    type Output = Self;
    fn div(self, s: T) -> Self {
        Vec2 { x: self.x / s, y: self.y / s }
    }
}
