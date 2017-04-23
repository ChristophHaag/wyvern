// Copyright (c) 2016-2017 Bruce Stenning. All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// 1. Redistributions of source code must retain the above copyright
//   notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//   notice, this list of conditions and the following disclaimer in the
//   documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//   contributors may be used to endorse or promote products derived
//   from this software without specific prior written permission.
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

use std::ops::Add;
use std::ops::Mul;
use std::ops::AddAssign;
use num::One;
use num::Zero;
use std::f32;
use std::fmt;

use algebra::quaternion::Quaternion;
use algebra::vector::*;
use misc::conversions::degrees_to_radians;

#[derive(Clone, Copy)]
pub struct Mat4<T: Copy> {
    pub m: [[T; 4]; 4],
}

impl<T: Add<T, Output = T> + AddAssign<T> + Mul<T, Output = T> + Zero + Copy> Mul for Mat4<T> {
    type Output = Mat4<T>;

    /// 4x4 matrix by 4x4 matrix multiply
    ///
    /// other: The RHS matrix
    fn mul(self, other: Mat4<T>) -> Mat4<T> {
        let mut output: Mat4<T> = Mat4::<T>::new();

        for y in 0..4 {
            for x in 0..4 {
                let mut sum = T::zero();
                for w in 0..4 {
                    sum += other.m[x][w] * self.m[w][y];
                }
                output.m[x][y] = sum;
            }
        }

        output
    }
}

impl<'a, T: Add<T, Output = T> + AddAssign<T> + Mul<T, Output = T> + Zero + Copy> Mul for &'a Mat4<T> {
    type Output = Mat4<T>;

    /// 4x4 matrix by 4x4 matrix multiply, by reference
    ///
    /// other: The RHS matrix
    fn mul(self, other: &Mat4<T>) -> Mat4<T> {
        let mut output: Mat4<T> = Mat4::<T>::new();

        for y in 0..4 {
            for x in 0..4 {
                let mut sum = T::zero();
                for w in 0..4 {
                    sum += other.m[x][w] * self.m[w][y];
                }
                output.m[x][y] = sum;
            }
        }

        output
    }
}

impl<T: Zero + Copy> Mat4<T> {
    /// Constructor for a 4x4 matrix, all zeros
    pub fn new() -> Self {
        Mat4 { m: [[T::zero(); 4]; 4] }
    }
}

impl<T: Zero + One + Copy> Mat4<T> {
    /// Constructor for a 4x4 identity matrix
    pub fn newidentity() -> Self {
        let mut matrix = Mat4::new();

        for y in 0..4 {
            for x in 0..4 {
                if x == y {
                    matrix.m[x][y] = T::one();
                } else {
                    matrix.m[x][y] = T::zero();
                }
            }
        }

        matrix
    }

    /// Set a matrix to identity
    pub fn identity(&mut self) -> &mut Self {
        for y in 0..4 {
            for x in 0..4 {
                if x == y {
                    self.m[x][y] = T::one();
                } else {
                    self.m[x][y] = T::zero();
                }
            }
        }

        self
    }
}

impl Mat4<f32> {
    /// 4x4 matrix by 4-component vector multiply
    ///
    /// The input vector is implicitly promoted to homogeneous coordinates.
    ///
    /// In Rust it is not possible to overload the multiplication operator
    /// based on the other type, sadly.
    ///
    /// vector: The RHS of the multiplication, a 3-component vector
    pub fn mul_by_vec3(self, vector: Vec3<f32>) -> Vec4<f32> {
        let mut output: Vec4<f32> = Vec4::<f32>::new();

        let mut out = [0.0f32; 4];
        let inp = [vector.x, vector.y, vector.z, 1.0f32];

        for y in 0..4 {
            let mut sum = 0.0f32;

            for x in 0..4 {
                sum += self.m[x][y] * inp[x];
            }
            out[y] = sum;
        }

        output.x = out[0];
        output.y = out[1];
        output.z = out[2];
        output.w = out[3];

        output
    }

    /// 4x4 matrix by 4-component vector multiply
    ///
    /// The input vector is implicitly promoted to homogeneous coordinates.
    ///
    /// In Rust it is not possible to overload the multiplication operator
    /// based on the other type, sadly.
    ///
    /// vector: The RHS of the multiplication, a 3-component vector
    pub fn mul_by_vec3_byref(self, vector: &Vec3<f32>) -> Vec4<f32> {
        let mut output: Vec4<f32> = Vec4::<f32>::new();

        let mut out = [0.0f32; 4];
        let inp = [vector.x, vector.y, vector.z, 1.0f32];

        for y in 0..4 {
            let mut sum = 0.0f32;

            for x in 0..4 {
                sum += self.m[x][y] * inp[x];
            }
            out[y] = sum;
        }

        output.x = out[0];
        output.y = out[1];
        output.z = out[2];
        output.w = out[3];

        output
    }

    /// 4x4 matrix by 4-component vector multiply
    ///
    /// In Rust it is not possible to overload the multiplication operator
    /// based on the other type, sadly.
    ///
    /// vector: The RHS of the multiplication, a 4-component vector
    pub fn mul_by_vec4(self, vector: Vec4<f32>) -> Vec4<f32> {
        let mut output: Vec4<f32> = Vec4::<f32>::new();

        let mut out = [0.0f32; 4];
        let inp = [vector.x, vector.y, vector.z, vector.w];

        for y in 0..4 {
            let mut sum = 0.0f32;

            for x in 0..4 {
                sum += self.m[x][y] * inp[x];
            }
            out[y] = sum;
        }

        output.x = out[0];
        output.y = out[1];
        output.z = out[2];
        output.w = out[3];

        output
    }

    /// Construct a new translation matrix
    ///
    /// x: The offset of the translation in the X axis
    /// y: The offset of the translation in the Y axis
    /// z: The offset of the translation in the Z axis
    pub fn translate(x: f32, y: f32, z: f32) -> Self {
        let mut matrix = Mat4::newidentity();

        matrix.m[3][0] = x;
        matrix.m[3][1] = y;
        matrix.m[3][2] = z;

        matrix
    }

    /// Construct a new rotation matrix about the X axis
    ///
    /// degrees: The angle, in degrees, of the rotation
    pub fn rotatex(degrees: f32) -> Self {
        let mut matrix = Mat4::newidentity();

        let radians = degrees_to_radians(degrees);
        let c: f32 = radians.cos();
        let s: f32 = radians.sin();

        matrix.m[1][2] = s;
        matrix.m[2][1] = -s;
        matrix.m[1][1] = c;
        matrix.m[2][2] = c;

        matrix
    }

    /// Construct a new rotation matrix about the Y axis
    ///
    /// degrees: The angle, in degrees, of the rotation
    pub fn rotatey(degrees: f32) -> Self {
        let mut matrix = Mat4::newidentity();

        let radians = degrees_to_radians(degrees);
        let c: f32 = radians.cos();
        let s: f32 = radians.sin();

        matrix.m[2][0] = s;
        matrix.m[0][2] = -s;
        matrix.m[0][0] = c;
        matrix.m[2][2] = c;

        matrix
    }

    /// Construct a new rotation matrix about the Z axis
    ///
    /// degrees: The angle, in degrees, of the rotation
    pub fn rotatez(degrees: f32) -> Self {
        let mut matrix = Mat4::newidentity();

        let radians = degrees_to_radians(degrees);
        let c: f32 = radians.cos();
        let s: f32 = radians.sin();

        matrix.m[0][1] = s;
        matrix.m[1][0] = -s;
        matrix.m[0][0] = c;
        matrix.m[1][1] = c;

        matrix
    }

    /// Construct a new transformation matrix for rotation around the specified vector
    ///
    /// vector: The axis of rotation
    /// radians: The angle, in degrees, of rotation
    pub fn rotate_vec(vector: &Vec3<f32>, degrees: f32) -> Self {
        let mut matrix = Mat4::newidentity();

        let axis = vector.normalise();

        let radians = degrees_to_radians(degrees);
        let c: f32 = radians.cos();
        let s: f32 = radians.sin();

        let omc = 1.0 - c;

        matrix.m[0][0] = omc * axis.x * axis.x + c;
        matrix.m[1][1] = omc * axis.y * axis.y + c;
        matrix.m[2][2] = omc * axis.z * axis.z + c;

        matrix.m[0][1] = omc * axis.x * axis.y + axis.z * s;
        matrix.m[1][0] = omc * axis.x * axis.y - axis.z * s;

        matrix.m[0][2] = omc * axis.z * axis.x - axis.y * s;
        matrix.m[2][0] = omc * axis.z * axis.x + axis.y * s;

        matrix.m[1][2] = omc * axis.y * axis.z + axis.x * s;
        matrix.m[2][1] = omc * axis.y * axis.z - axis.x * s;

        matrix
    }

    /// Construct a new projection matrix
    ///
    /// This should be the same as GLU's gluPerspective function.
    ///
    /// Note that the near and far clip plane specifications are *distances*, so they do
    /// not conform to OpenGL's right-handed world coordinates system. Both znear and zfar
    /// must be positive, even though the camera sees down the negative Z axis.
    ///
    /// fovy: The field-of-view angle, in degrees, of the vertical axis of the viewport
    /// aspect: The aspect ratio (width divided by height) of the viewport
    /// znear: The distance from the camera to the near clip plane
    /// zfar: The distance from the camera to the far clip plane
    /// flip: true if the y axis should be flipped
    /// halfz: true if the Z clip coordinates should be [0, 1] instead of [-1, 1]
    pub fn projection(fovy: f32, aspect: f32, znear: f32, zfar: f32, flip: bool, halfz: bool) -> Self {
        let mut matrix = Mat4::newidentity();

        let d2 = fovy / 2.0;
        let f: f32 = d2.atan();

        matrix.m[0][0] = f / aspect;
        matrix.m[1][1] = if flip { -f } else { f };
        matrix.m[2][2] = (zfar + znear) / (znear - zfar);
        matrix.m[3][2] = (2.0 * zfar * znear) / (znear - zfar);
        matrix.m[3][3] = 0.0f32;
        matrix.m[2][3] = -1.0f32;

        if !halfz {
            matrix
        } else {
            let mut squeeze = Mat4::newidentity();
            squeeze.m[2][2] = 0.5f32;
            squeeze.m[3][2] = 0.5f32;
            squeeze * matrix
        }
    }

    /// Construct a new model view matrix from a set of unit basis vectors and a position vector
    ///
    /// The basis vectors must be unit length and mutually orthogonal.
    ///
    /// position: The camera's position vector
    /// forward: The vector pointing down the camera's lens
    /// up: The vector pointing up from the camera
    /// right: The vector pointing right from the camera
    pub fn modelview(position: &Vec3<f32>, forward: &Vec3<f32>, right: &Vec3<f32>, up: &Vec3<f32>) -> Self {
        let mut matrix = Mat4::newidentity();

        matrix.m[0][0] = right.x;
        matrix.m[1][0] = right.y;
        matrix.m[2][0] = right.z;
        matrix.m[0][1] = up.x;
        matrix.m[1][1] = up.y;
        matrix.m[2][1] = up.z;
        matrix.m[0][2] = forward.x;
        matrix.m[1][2] = forward.y;
        matrix.m[2][2] = forward.z;

        matrix * Mat4::translate(-position.x, -position.y, -position.z)
    }

    /// Construct a new model view matrix from a set of unit basis vectors and a position vector
    ///
    /// The basis vectors must be unit length and mutually orthogonal.
    ///
    /// position: The camera's position vector
    /// quaternion: The view orientation, as a quaternion
    pub fn modelview_quaternion(position: &Vec3<f32>, quaternion: &Quaternion<f32>) -> Self {
        let right = Quaternion::rotate(&Vec3 {
                                           x: 1.0f32,
                                           y: 0.0f32,
                                           z: 0.0f32,
                                       },
                                       &quaternion);
        let up = Quaternion::rotate(&Vec3 {
                                        x: 0.0f32,
                                        y: 1.0f32,
                                        z: 0.0f32,
                                    },
                                    &quaternion);
        let forward = Quaternion::rotate(&Vec3 {
                                             x: 0.0f32,
                                             y: 0.0f32,
                                             z: -1.0f32,
                                         },
                                         &quaternion);

        let mut basis = Mat4::newidentity();

        basis.m[0][0] = right.x;
        basis.m[1][0] = right.y;
        basis.m[2][0] = right.z;
        basis.m[0][1] = up.x;
        basis.m[1][1] = up.y;
        basis.m[2][1] = up.z;
        basis.m[0][2] = forward.x;
        basis.m[1][2] = forward.y;
        basis.m[2][2] = forward.z;

        // TODO: Optimise this
        let translation = Mat4::translate(-position.x, -position.y, -position.z);
        let result = basis * translation;
        // println!("{} basis\n", basis);
        // println!("{} translation\n", translation);
        // println!("{} result\n", result);

        result
    }
}

/// How to display a 4x4-component matrix
impl<T: Copy + fmt::Display> fmt::Display for Mat4<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "({:>14.*}, {:>14.*}, {:>14.*}, {:>14.*})\n\
                |{:>14.*}, {:>14.*}, {:>14.*}, {:>14.*}|\n\
                |{:>14.*}, {:>14.*}, {:>14.*}, {:>14.*}|\n\
                ({:>14.*}, {:>14.*}, {:>14.*}, {:>14.*})",
               4,
               self.m[0][0],
               4,
               self.m[0][1],
               4,
               self.m[0][2],
               4,
               self.m[0][3],
               4,
               self.m[1][0],
               4,
               self.m[1][1],
               4,
               self.m[1][2],
               4,
               self.m[1][3],
               4,
               self.m[2][0],
               4,
               self.m[2][1],
               4,
               self.m[2][2],
               4,
               self.m[2][3],
               4,
               self.m[3][0],
               4,
               self.m[3][1],
               4,
               self.m[3][2],
               4,
               self.m[3][3])
    }
}
