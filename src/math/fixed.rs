/// 16.16 fixed-point number, matching the original C source.
///
/// In the original, fixed-point values were `long` (32-bit) with the integer
/// part in the high 16 bits and the fractional part in the low 16 bits.
/// Operations must stay in this format to match original game behavior.
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed(pub i32);

impl Fixed {
    pub const ONE: Fixed = Fixed(1 << 16);
    pub const ZERO: Fixed = Fixed(0);

    pub fn from_int(i: i32) -> Self {
        Fixed(i << 16)
    }

    pub fn from_f32(f: f32) -> Self {
        Fixed((f * 65536.0) as i32)
    }

    pub fn to_f32(self) -> f32 {
        self.0 as f32 / 65536.0
    }

    pub fn to_int(self) -> i32 {
        self.0 >> 16
    }

    pub fn frac(self) -> i32 {
        self.0 & 0xFFFF
    }

    pub fn abs(self) -> Self {
        Fixed(self.0.abs())
    }
}

impl Add for Fixed {
    type Output = Fixed;
    fn add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0 + rhs.0)
    }
}

impl Sub for Fixed {
    type Output = Fixed;
    fn sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0 - rhs.0)
    }
}

impl Neg for Fixed {
    type Output = Fixed;
    fn neg(self) -> Fixed {
        Fixed(-self.0)
    }
}

impl Mul for Fixed {
    type Output = Fixed;
    fn mul(self, rhs: Fixed) -> Fixed {
        // Use i64 intermediate to avoid overflow
        Fixed(((self.0 as i64 * rhs.0 as i64) >> 16) as i32)
    }
}

impl Div for Fixed {
    type Output = Fixed;
    fn div(self, rhs: Fixed) -> Fixed {
        Fixed((((self.0 as i64) << 16) / rhs.0 as i64) as i32)
    }
}

impl From<i32> for Fixed {
    fn from(i: i32) -> Self {
        Fixed::from_int(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_int() {
        assert_eq!(Fixed::from_int(5).to_int(), 5);
        assert_eq!(Fixed::from_int(-3).to_int(), -3);
    }

    #[test]
    fn mul_fixed() {
        let a = Fixed::from_int(2);
        let b = Fixed::from_f32(1.5);
        let result = (a * b).to_f32();
        assert!((result - 3.0).abs() < 0.001);
    }

    #[test]
    fn add_fixed() {
        let a = Fixed::from_int(1);
        let b = Fixed::from_int(2);
        assert_eq!((a + b).to_int(), 3);
    }
}
