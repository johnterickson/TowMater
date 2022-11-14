#![allow(dead_code)]

use std::{f32::consts::PI, ops::Sub};

// [-180, 180)
pub struct Angle {
    radians: f32,
}

impl Angle {
    const DEGREES_PER_RADIAN: f32 = 180.0 / PI;
    const RADIANS_PER_DEGREE: f32 = PI / 180.0;

    pub fn from_radians(radians: f32) -> Self {
        Self { radians }.normalized()
    }

    pub fn from_degrees(degrees: f32) -> Self {
        Self {
            radians: degrees * Self::RADIANS_PER_DEGREE,
        }
        .normalized()
    }

    pub fn from_atan2(y: f32, x: f32) -> Self {
        Self::from_radians(f32::atan2(y, x))
    }

    pub fn normalized(&self) -> Self {
        Self {
            radians: if self.radians < -PI {
                self.radians + (PI * 2.0)
            } else if self.radians >= PI {
                self.radians - (PI * 2.0)
            } else {
                self.radians
            },
        }
    }

    pub fn degrees(&self) -> f32 {
        self.radians * Self::DEGREES_PER_RADIAN
    }

    pub fn radians(&self) -> f32 {
        self.radians
    }
}

impl Sub for Angle {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Angle::from_radians(self.radians - rhs.radians)
    }
}
