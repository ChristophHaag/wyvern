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
//   notice, this list of conditions and the following disclaimer in the
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

use std::f32;
use std::fmt;
use std::mem;
use std::ops::*;
use num::*;

use algebra::vector::Vec3;
use misc::conversions::degrees_to_radians;

#[derive(Clone, Copy)]
pub struct Quaternion<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl<T: Zero + One> Quaternion<T> {
    pub fn identity() -> Quaternion<T> {
        Quaternion {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
            w: T::one(),
        }
    }
}

impl<T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Copy> Quaternion<T> {
    /// Multiply two quaternions
    ///
    /// other: The RHS of the multiplication
    pub fn multiply(self, other: &Quaternion<T>) -> Quaternion<T> {
        let v1 = self.vector();
        let v2 = other.vector();
        let c = Vec3::cross(&v1, &v2);
        let d = Vec3::dot(&v1, &v2);

        Quaternion::<T> {
            x: c.x + self.w * v2.x + other.w * v1.x,
            y: c.y + self.w * v2.y + other.w * v1.y,
            z: c.z + self.w * v2.z + other.w * v1.z,
            w: self.w * other.w - d,
        }
    }
}

impl<T: Neg<Output = T> + Copy> Quaternion<T> {
    /// Calculate the conjugation of a quaternion
    pub fn conjugate(self) -> Quaternion<T> {
        Quaternion::<T> {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }
}

impl<T> Quaternion<T> {
    /// Obtain the vector part of a quaternion
    pub fn vector(self) -> Vec3<T> {
        Vec3::<T> {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

impl<T: Zero> Vec3<T> {
    /// Convert a vector to a quaternion with a zero w component
    pub fn to_quaternion(self) -> Quaternion<T> {
        Quaternion::<T> {
            x: self.x,
            y: self.y,
            z: self.z,
            w: T::zero(),
        }
    }
}

impl Quaternion<f32> {
    /// Convert an axis and an angle (in degrees) to a quaternion
    ///
    /// axis: The axis of rotation
    /// angle: The angle of rotation, in degrees
    pub fn axis_and_angle_to_quaternion(axis: &Vec3<f32>, angle: f32) -> Quaternion<f32> {
        let unit = axis.normalise();
        let s = (degrees_to_radians(angle * 0.5f32)).sin();
        let c = (degrees_to_radians(angle * 0.5f32)).cos();
        Quaternion::<f32> {
            x: s * unit.x,
            y: s * unit.y,
            z: s * unit.z,
            w: c,
        }
    }

    /// Convert an axis and an angle (in radians) to a quaternion
    ///
    /// axis: The axis of rotation
    /// angle: The angle of rotation, in degrees
    pub fn axis_and_angle_in_radians_to_quaternion(axis: &Vec3<f32>, angle: f32) -> Quaternion<f32> {
        let unit = axis.normalise();
        let s = (angle * 0.5f32).sin();
        let c = (angle * 0.5f32).cos();
        Quaternion::<f32> {
            x: s * unit.x,
            y: s * unit.y,
            z: s * unit.z,
            w: c,
        }
    }

    /// Convert a location and point to look at into a quaternion
    ///
    /// source_position: Location of the intended camera
    /// destination_position: Location of the point to look at
    pub fn look_at(source_position: &Vec3<f32>, destination_position: &Vec3<f32>) -> Quaternion<f32> {
        const FORWARD: Vec3<f32> = Vec3 {
            x: 0.0f32,
            y: 0.0f32,
            z: -1.0f32,
        };
        const UP: Vec3<f32> = Vec3 {
            x: 0.0f32,
            y: 1.0f32,
            z: 0.0f32,
        };

        let look_vector = (*destination_position - *source_position).normalise();

        // Be aware of the negation
        let dot = -Vec3::dot(&look_vector, &FORWARD);

        if (dot + 1.0f32).abs() < 1.0e-6f32 {
            return Quaternion {
                x: UP.x,
                y: UP.y,
                z: UP.z,
                w: f32::consts::PI, // 180 degrees
            };
        }
        if (dot - 1.0f32).abs() < 1.0e-6f32 {
            return Quaternion::identity();
        }

        let rotate_axis = Vec3::cross(&look_vector, &FORWARD);
        let rotate_angle = dot.acos();

        Quaternion::axis_and_angle_in_radians_to_quaternion(&rotate_axis, rotate_angle)
    }

    /// Rotate a vector by the specified quaternion
    ///
    /// vector: The vector to be rotated
    /// rotation: The quaternion specifying the rotation to be applied
    pub fn rotate(vector: &Vec3<f32>, rotation: &Quaternion<f32>) -> Vec3<f32> {
        rotation.multiply(&vector.to_quaternion()).multiply(&rotation.conjugate()).vector()
    }
}

/// Equivalence operator for quaternions
///
/// other: Vector for comparison
impl<T: PartialEq> PartialEq for Quaternion<T> {
    fn eq(&self, other: &Quaternion<T>) -> bool {
        (self.x == other.x) && (self.y == other.y) && (self.z == other.z) && (self.w == other.w)
    }
}

/// Approximate equivalence for quaternions
///
/// other: Vector for comparison
/// ulps: How many units in the last place to compare to (approximately)
impl Quaternion<f32> {
    pub fn approx_eq_ulps(&self, other: &Quaternion<f32>, ulps: i32) -> bool {
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


/// How to display a quaternion
impl<T: fmt::Display> fmt::Display for Quaternion<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

/// How to display a quaternion of f32 for debugging purposes
impl fmt::Debug for Quaternion<f32> {
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
