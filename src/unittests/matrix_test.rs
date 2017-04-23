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

#![allow(unused_imports)]

use algebra::vector::Vec3;
use algebra::matrix::Mat4;
use misc::conversions::degrees_to_radians;

#[test]
fn mat4_identity_multiplied_by_vec3_and_project_leaves_vec3_unchanged() {
    let m = Mat4::newidentity();
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3(v1).project();
    println!("result is {}", v2);
    println!("expected is {}", v1);
    assert!(v1 == v2);
}

#[test]
fn mat4_translation() {
    let m = Mat4::translate(2.0f32, 3.0f32, 4.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 2.0f32,
        z: 1.0f32,
    };
    let v2 = m.mul_by_vec3(v1).project();
    let e = Vec3 {
        x: 5.0f32,
        y: 5.0f32,
        z: 5.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2 == e);
}

#[test]
fn mat4_rotation_x() {
    let m = Mat4::rotatex(90.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3(v1).project();
    let e = Vec3 {
        x: 3.0f32,
        y: -5.0f32,
        z: 4.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2.approx_eq_ulps(&e, 2));
}

#[test]
fn mat4_rotation_y() {
    let m = Mat4::rotatey(90.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3(v1).project();
    let e = Vec3 {
        x: 5.0f32,
        y: 4.0f32,
        z: -3.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2.approx_eq_ulps(&e, 2));
}

#[test]
fn mat4_rotation_z() {
    let m = Mat4::rotatez(90.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3(v1).project();
    let e = Vec3 {
        x: -4.0f32,
        y: 3.0f32,
        z: 5.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2.approx_eq_ulps(&e, 2));
}

#[test]
fn mat4_identity_multiplied_by_vec3_and_project_leaves_vec3_unchanged_byref() {
    let m = Mat4::newidentity();
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3_byref(&v1).project();
    println!("result is {}", v2);
    println!("expected is {}", v1);
    assert!(v1 == v2);
}

#[test]
fn mat4_translation_byref() {
    let m = Mat4::translate(2.0f32, 3.0f32, 4.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 2.0f32,
        z: 1.0f32,
    };
    let v2 = m.mul_by_vec3_byref(&v1).project();
    let e = Vec3 {
        x: 5.0f32,
        y: 5.0f32,
        z: 5.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2 == e);
}

#[test]
fn mat4_rotation_x_byref() {
    let m = Mat4::rotatex(90.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3_byref(&v1).project();
    let e = Vec3 {
        x: 3.0f32,
        y: -5.0f32,
        z: 4.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2.approx_eq_ulps(&e, 2));
}

#[test]
fn mat4_rotation_y_byref() {
    let m = Mat4::rotatey(90.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3_byref(&v1).project();
    let e = Vec3 {
        x: 5.0f32,
        y: 4.0f32,
        z: -3.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2.approx_eq_ulps(&e, 2));
}

#[test]
fn mat4_rotation_z_byref() {
    let m = Mat4::rotatez(90.0f32);
    let v1 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: 5.0f32,
    };
    let v2 = m.mul_by_vec3_byref(&v1).project();
    let e = Vec3 {
        x: -4.0f32,
        y: 3.0f32,
        z: 5.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2.approx_eq_ulps(&e, 2));
}

#[test]
fn mat4_multiply_mat4() {
    let m1 = Mat4::rotatez(90.0f32);
    let m2 = Mat4::translate(3.0f32, 4.0f32, 5.0f32);
    let v1 = Vec3 {
        x: 6.0f32,
        y: 7.0f32,
        z: 8.0f32,
    };
    let v2 = m1.mul_by_vec3(v1).project();
    let e1 = Vec3 {
        x: -7.0f32,
        y: 6.0f32,
        z: 8.0f32,
    };
    println!("result 1 is {}", v2);
    println!("expected 1 is {}", e1);
    assert!(v2.approx_eq_ulps(&e1, 2));
    let v3 = m2.mul_by_vec3(v1).project();
    let e2 = Vec3 {
        x: 9.0f32,
        y: 11.0f32,
        z: 13.0f32,
    };
    println!("result 2 is {}", v3);
    println!("expected 2 is {}", e2);
    assert!(v3.approx_eq_ulps(&e2, 2));
    let v4 = m1.mul_by_vec3(v1).project();
    let v5 = m2.mul_by_vec3(v4).project();
    let m3 = m2 * m1;
    let v6 = m3.mul_by_vec3(v1).project();
    println!("result 3 is {}", v5);
    println!("result 4 is {}", v6);
    assert!(v5.approx_eq_ulps(&v6, 2));
}

#[test]
fn mat4_multiply_mat4_byref() {
    let m1 = Mat4::rotatez(90.0f32);
    let m2 = Mat4::translate(3.0f32, 4.0f32, 5.0f32);
    let v1 = Vec3 {
        x: 6.0f32,
        y: 7.0f32,
        z: 8.0f32,
    };
    let v2 = m1.mul_by_vec3(v1).project();
    let e1 = Vec3 {
        x: -7.0f32,
        y: 6.0f32,
        z: 8.0f32,
    };
    println!("result 1 is {}", v2);
    println!("expected 1 is {}", e1);
    assert!(v2.approx_eq_ulps(&e1, 2));
    let v3 = m2.mul_by_vec3(v1).project();
    let e2 = Vec3 {
        x: 9.0f32,
        y: 11.0f32,
        z: 13.0f32,
    };
    println!("result 2 is {}", v3);
    println!("expected 2 is {}", e2);
    assert!(v3.approx_eq_ulps(&e2, 2));
    let v4 = m1.mul_by_vec3(v1).project();
    let v5 = m2.mul_by_vec3(v4).project();
    let m3 = &m2 * &m1;
    let v6 = m3.mul_by_vec3(v1).project();
    println!("result 3 is {}", v5);
    println!("result 4 is {}", v6);
    assert!(v5.approx_eq_ulps(&v6, 2));
}

#[test]
fn mat4_rotate_vec() {
    let v1 = Vec3 {
            x: 1.0f32,
            y: 1.0f32,
            z: 1.0f32,
        }
        .normalise();
    let v2 = Vec3 {
        x: 2.0f32,
        y: 3.0f32,
        z: 4.0f32,
    };
    let m1 = Mat4::rotate_vec(&v1, 180.0f32);
    let v3 = m1.mul_by_vec3(v2).project();
    let e = Vec3 {
        x: 4.0f32,
        y: 3.0f32,
        z: 2.0f32,
    };
    println!("result is {}", v3);
    println!("expected 2 is {}", e);
    assert!(v3.approx_eq_ulps(&e, 2));
}
