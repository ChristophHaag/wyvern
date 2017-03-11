// Copyright (c) 2016 Bruce Stenning. All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
// FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
// COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
// INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
// OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
// AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF
// THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH
// DAMAGE.

#![allow(dead_code)]

use std::ops::*;
use std::mem;
use num::*;
use std::cmp::PartialEq;
use std::fmt;

#[derive(Clone, Copy)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T: Add<T, Output = T>> Add for Vec2<T> {
    type Output = Vec2<T>;

    /// 2-component vector add
    ///
    /// other: The RHS of the addition
    fn add(self, other: Vec2<T>) -> Vec2<T> {
        Vec2::<T> {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T: AddAssign<T> + Copy> AddAssign<T> for Vec2<T> {
    /// 2-component vector add-and-assign
    ///
    /// other: The RHS of the addition
    fn add_assign(&mut self, addition: T) {
        self.x += addition;
        self.y += addition;
    }
}

impl<T: Sub<T, Output = T>> Sub for Vec2<T> {
    type Output = Vec2<T>;

    /// 2-component vector subtraction
    ///
    /// other: The RHS of the subtraction
    fn sub(self, other: Vec2<T>) -> Vec2<T> {
        Vec2::<T> {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<'a, T: Sub<T, Output = T> + Copy> Sub for &'a Vec2<T> {
    type Output = Vec2<T>;

    /// 2-component vector subtraction, by reference
    ///
    /// other: The RHS of the subtraction
    fn sub(self, other: &'a Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

/// 2-component vector subtraction-and-assign
///
/// other: The RHS of the subtraction
impl<T: SubAssign<T> + Copy> SubAssign<T> for Vec2<T> {
    fn sub_assign(&mut self, subtraction: T) {
        self.x -= subtraction;
        self.y -= subtraction;
    }
}

/// Equivalence operator for 2-component vectors
///
/// other: Vector for comparison
impl<T: PartialEq> PartialEq for Vec2<T> {
    fn eq(&self, other: &Vec2<T>) -> bool {
        (self.x == other.x) && (self.y == other.y)
    }
}

/// Approximate equivalence for 2-component vectors
///
/// other: Vector for comparison
/// ulps: How many units in the last place to compare to (approximately)
impl Vec2<f32> {
    pub fn approx_eq_ulps(&self, other: &Vec2<f32>, ulps: i32) -> bool {
        let pe = 10.0f32.powf((ulps - 7) as f32);
        if self.x < other.x - pe {
            return false;
        };
        if self.x > other.x + pe {
            return false;
        };
        if self.y < other.y - pe {
            return false;
        };
        if self.y > other.y + pe {
            return false;
        };
        return true;
    }
}

/// How to display a 2-component vector
impl<T: fmt::Display> fmt::Display for Vec2<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// How to display a 2-component vector of f32 for debugging purposes
impl fmt::Debug for Vec2<f32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            write!(f,
                   "({:x}, {:x})",
                   mem::transmute::<f32, i32>(self.x),
                   mem::transmute::<f32, i32>(self.y))
        }
    }
}

#[derive(Clone, Copy)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: Add<T, Output = T>> Add for Vec3<T> {
    type Output = Vec3<T>;

    /// 3-component vector add
    ///
    /// other: The RHS of the addition
    fn add(self, other: Vec3<T>) -> Vec3<T> {
        Vec3::<T> {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl<T: AddAssign<T> + Copy> AddAssign<T> for Vec3<T> {
    /// 3-component vector add-and-assign
    ///
    /// other: The RHS of the addition
    fn add_assign(&mut self, addition: T) {
        self.x += addition;
        self.y += addition;
        self.z += addition;
    }
}

impl<T: Sub<T, Output = T>> Sub for Vec3<T> {
    type Output = Vec3<T>;

    /// 3-component vector subtraction
    ///
    /// other: The RHS of the subtraction
    fn sub(self, other: Vec3<T>) -> Vec3<T> {
        Vec3::<T> {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl<'a, T: Sub<T, Output = T> + Copy> Sub for &'a Vec3<T> {
    type Output = Vec3<T>;

    /// 3-component vector subtraction, by reference
    ///
    /// other: The RHS of the subtraction
    fn sub(self, other: &'a Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// 3-component vector subtraction-and-assign
///
/// other: The RHS of the subtraction
impl<T: SubAssign<T> + Copy> SubAssign<T> for Vec3<T> {
    fn sub_assign(&mut self, subtraction: T) {
        self.x -= subtraction;
        self.y -= subtraction;
        self.z -= subtraction;
    }
}

impl<T: Div<T, Output = T> + Copy> Div<T> for Vec3<T> {
    type Output = Vec3<T>;

    /// 3-component vector divide by scalar
    ///
    /// divisor: The divisor
    fn div(self, divisor: T) -> Vec3<T> {
        Vec3::<T> {
            x: self.x / divisor,
            y: self.y / divisor,
            z: self.z / divisor,
        }
    }
}

impl<T: DivAssign<T> + Copy> DivAssign<T> for Vec3<T> {
    /// 3-component vector divide by scalar and assign
    ///
    /// divisor: The divisor
    fn div_assign(&mut self, divisor: T) {
        self.x /= divisor;
        self.y /= divisor;
        self.z /= divisor;
    }
}

impl<T: Mul<T, Output = T> + Copy> Mul<T> for Vec3<T> {
    type Output = Vec3<T>;

    /// 3-component vector multiply by scalar
    ///
    /// multiplicand: The multiplicand
    fn mul(self, multiplicand: T) -> Vec3<T> {
        Vec3 {
            x: self.x * multiplicand,
            y: self.y * multiplicand,
            z: self.z * multiplicand,
        }
    }
}

impl<'a, T: Mul<T, Output = T> + Copy> Mul<T> for &'a Vec3<T> {
    type Output = Vec3<T>;

    /// 3-component vector multiply by scalar, by reference
    ///
    /// multiplicand: The multiplicand
    fn mul(self, multiplicand: T) -> Vec3<T> {
        Vec3 {
            x: self.x * multiplicand,
            y: self.y * multiplicand,
            z: self.z * multiplicand,
        }
    }
}

/// 3-component vector multiply by scalar and asssign
///
/// multiplicand: The multiplicand
impl<T: MulAssign<T> + Copy> MulAssign<T> for Vec3<T> {
    fn mul_assign(&mut self, multiplicand: T) {
        self.x *= multiplicand;
        self.y *= multiplicand;
        self.z *= multiplicand;
    }
}

/// Construct new zero 3-component vector
impl<T: Zero> Vec3<T> {
    pub fn new() -> Vec3<T> {
        Vec3 {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
        }
    }
}

impl Vec3<f32> {
    /// Calculate the magnitude of a 3-component vector
    pub fn magnitude(&self) -> f32 {
        let magsq = self.x * self.x + self.y * self.y + self.z * self.z;
        magsq.sqrt()
    }

    /// Calculate the magnitude squared of a 3-component vector
    pub fn magnitude_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Normalise a 3-component vector
    pub fn normalise(&self) -> Vec3<f32> {
        let magsq = self.x * self.x + self.y * self.y + self.z * self.z;
        let invmag = 1.0f32 / magsq.sqrt();

        Vec3::<f32> {
            x: self.x * invmag,
            y: self.y * invmag,
            z: self.z * invmag,
        }
    }

    /// Normalise a 3-component vector using an evil hack
    ///
    /// https://en.wikipedia.org/wiki/Fast_inverse_square_root
    pub fn normalise_evil(&self) -> Vec3<f32> {
        let magsq = self.x * self.x + self.y * self.y + self.z * self.z;

        unsafe {
            let x2 = magsq * 0.5f32;
            let mut y = magsq;
            let mut i = mem::transmute::<f32, i32>(y);
            i = 0x5f3759df - (i >> 1);
            y = mem::transmute::<i32, f32>(i);
            y = y * (1.5f32 - (x2 * y * y));
            let invmag = y;

            Vec3::<f32> {
                x: self.x * invmag,
                y: self.y * invmag,
                z: self.z * invmag,
            }
        }
    }
}

impl<T: AddAssign<T> + Copy> Vec3<T> {
    /// 3-component vector add to 3-component vector
    ///
    /// other: The RHS of the addition
    pub fn add_assign_vec(&mut self, other: &Vec3<T>) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl<T: Add<T, Output = T> + Mul<T, Output = T> + Copy> Vec3<T> {
    /// Calculate the dot product of two 3-component vectors
    ///
    /// other: The RHS of the dot-product
    pub fn dot(&self, other: &Vec3<T>) -> T {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

impl<T: Sub<T, Output = T> + Mul<T, Output = T> + Copy> Vec3<T> {
    /// Calculate the cross product of two 3-component vectors
    ///
    /// other: The RHS of the cross-product
    pub fn cross(a: &Vec3<T>, b: &Vec3<T>) -> Vec3<T> {
        Vec3::<T> {
            x: a.y * b.z - b.y * a.z,
            y: a.z * b.x - b.z * a.x,
            z: a.x * b.y - b.x * a.y,
        }
    }
}

impl<T: One + Copy> Vec3<T> {
    /// Convert a 3-component vector to a 4-component vector in homogeneous coordinates
    pub fn to_homogeneous(&self) -> Vec4<T> {
        Vec4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w: T::one(),
        }
    }
}

/// Linear interpolation of two vectors
///
/// This version should guarantee a at t = 0.0 and b at t = 1.0
///
/// a: First vector
/// b: Second vector
/// t: Interpolation value in range [0, 1]
impl<T: One + Mul<T, Output = T> + Sub<T, Output = T> + Add<T, Output = T> + Copy> Vec3<T> {
    pub fn lerp(a: &Vec3<T>, b: &Vec3<T>, t: T) -> Vec3<T> {
        Vec3 {
            x: a.x * (T::one() - t) + b.x * t,
            y: a.y * (T::one() - t) + b.y * t,
            z: a.z * (T::one() - t) + b.z * t,
        }
    }
}

/// Equivalence operator for 3-component vectors
///
/// other: Vector for comparison
impl<T: PartialEq> PartialEq for Vec3<T> {
    fn eq(&self, other: &Vec3<T>) -> bool {
        (self.x == other.x) && (self.y == other.y) && (self.z == other.z)
    }
}

/// Approximate equivalence for 3-component vectors
///
/// other: Vector for comparison
/// ulps: How many units in the last place to compare to (approximately)
impl Vec3<f32> {
    pub fn approx_eq_ulps(&self, other: &Vec3<f32>, ulps: i32) -> bool {
        let pe = 10.0f32.powf((ulps - 7) as f32);
        if self.x < other.x - pe {
            return false;
        };
        if self.x > other.x + pe {
            return false;
        };
        if self.y < other.y - pe {
            return false;
        };
        if self.y > other.y + pe {
            return false;
        };
        if self.z < other.z - pe {
            return false;
        };
        if self.z > other.z + pe {
            return false;
        };
        return true;
    }
}

/// How to display a 3-component vector
impl<T: fmt::Display> fmt::Display for Vec3<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// How to display a 3-component vector of f32 for debugging purposes
impl fmt::Debug for Vec3<f32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            write!(f,
                   "({:x}, {:x}, {:x})",
                   mem::transmute::<f32, i32>(self.x),
                   mem::transmute::<f32, i32>(self.y),
                   mem::transmute::<f32, i32>(self.z))
        }
    }
}

#[derive(Clone, Copy)]
pub struct Vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

/// Construct a 4-component zero vector
impl<T: Zero> Vec4<T> {
    pub fn new() -> Vec4<T> {
        Vec4 {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
            w: T::zero(),
        }
    }
}

/// Project a 4-component vector in homogeneous coordinates to 3-space
impl<T: Div<T, Output = T> + Copy> Vec4<T> {
    pub fn project(&self) -> Vec3<T> {
        Vec3 {
            x: self.x / self.w,
            y: self.y / self.w,
            z: self.z / self.w,
        }
    }
}

impl<T: Add<T, Output = T>> Add for Vec4<T> {
    type Output = Vec4<T>;

    /// 4-component vector addition
    ///
    /// other: The RHS of the addition
    fn add(self, other: Vec4<T>) -> Vec4<T> {
        Vec4::<T> {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

/// Equivalence operator for 4-component vectors
///
/// other: Vector for comparison
impl<T: PartialEq> PartialEq for Vec4<T> {
    fn eq(&self, other: &Vec4<T>) -> bool {
        (self.x == other.x) && (self.y == other.y) && (self.z == other.z) && (self.w == other.w)
    }
}

/// Approximate equivalence for 4-component vectors
///
/// other: Vector for comparison
/// ulps: How many units in the last place to compare to (approximately)
impl Vec4<f32> {
    pub fn approx_eq_ulps(&self, other: &Vec4<f32>, ulps: i32) -> bool {
        let pe = 10.0f32.powf((ulps - 7) as f32);
        if self.x < other.x - pe {
            return false;
        };
        if self.x > other.x + pe {
            return false;
        };
        if self.y < other.y - pe {
            return false;
        };
        if self.y > other.y + pe {
            return false;
        };
        if self.z < other.z - pe {
            return false;
        };
        if self.z > other.z + pe {
            return false;
        };
        if self.w < other.w - pe {
            return false;
        };
        if self.w > other.w + pe {
            return false;
        };
        return true;
    }
}


/// How to display a 4-component vector
impl<T: fmt::Display> fmt::Display for Vec4<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

/// How to display a 4-component vector of f32 for debugging purposes
impl fmt::Debug for Vec4<f32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            write!(f,
                   "({:x}, {:x}, {:x}, {:x})",
                   mem::transmute::<f32, i32>(self.x),
                   mem::transmute::<f32, i32>(self.y),
                   mem::transmute::<f32, i32>(self.z),
                   mem::transmute::<f32, i32>(self.w))
        }
    }
}
