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

use std::f32;

use algebra::quaternion::Quaternion;
use algebra::vector::*;
use misc::conversions::radians_to_degrees;

#[test]
fn rotation_low_level() {
    // Tests conjugation, multiplication, and conversion

    let q1 = Quaternion {
        x: 1.0f32,
        y: 0.0f32,
        z: 0.0f32,
        w: 0.0f32,
    };
    let q2 = Quaternion::axis_and_angle_to_quaternion(&Vec3 {
                                                          x: 0.0f32,
                                                          y: 1.0f32,
                                                          z: 0.0f32,
                                                      },
                                                      90.0f32);

    let v3 = q2.multiply(&q1).multiply(&q2.conjugate()).vector();

    let e = Vec3 {
        x: 0.0f32,
        y: 0.0f32,
        z: -1.0f32,
    };
    println!("result is {}", v3);
    println!("expected is {}", e);
    assert!(v3.approx_eq_ulps(&e, 2));
}

#[test]
fn axis_and_angle_to_quaternion() {
    let v1 = Vec3 {
        x: 2.0f32,
        y: 2.0f32,
        z: 3.0f32,
    };
    let v2 = v1.normalise();
    let a1 = f32::consts::PI * 0.5f32;
    let q1 = Quaternion::axis_and_angle_to_quaternion(&v1, radians_to_degrees(a1));
    let e1 = Quaternion {
        x: v2.x * (a1 * 0.5f32).sin(),
        y: v2.y * (a1 * 0.5f32).sin(),
        z: v2.z * (a1 * 0.5f32).sin(),
        w: (a1 * 0.5f32).cos(),
    };
    println!("result is {}", q1);
    println!("expected is {}", e1);
    assert!(q1.approx_eq_ulps(&e1, 2));
}

#[test]
fn axis_and_angle_in_radians_to_quaternion() {
    let v1 = Vec3 {
        x: 2.0f32,
        y: 2.0f32,
        z: 3.0f32,
    };
    let v2 = v1.normalise();
    let a1 = f32::consts::PI * 0.5f32;
    let q1 = Quaternion::axis_and_angle_in_radians_to_quaternion(&v1, a1);
    let e1 = Quaternion {
        x: v2.x * (a1 * 0.5f32).sin(),
        y: v2.y * (a1 * 0.5f32).sin(),
        z: v2.z * (a1 * 0.5f32).sin(),
        w: (a1 * 0.5f32).cos(),
    };
    println!("result is {}", q1);
    println!("expected is {}", e1);
    assert!(q1.approx_eq_ulps(&e1, 2));
}

#[test]
fn look_at() {
    let v1 = Vec3 {
        x: 2.0f32,
        y: 2.0f32,
        z: 2.0f32,
    };
    let v2 = Vec3 {
        x: 3.0f32,
        y: 3.0f32,
        z: 3.0f32,
    };
    let v3 = (&v2 - &v1).normalise();
    let v4 = Vec3 {
        x: 0.0f32,
        y: 0.0f32,
        z: -1.0f32,
    };
    let c = -Vec3::dot(&v3, &v4);
    let th = c.acos() * 0.5f32;
    let s = th.sin();
    let c = th.cos();
    let q1 = Quaternion::look_at(&v1, &v2);
    let v5 = Vec3::cross(&v3, &v4).normalise();
    let e1 = Quaternion {
        x: s * v5.x,
        y: s * v5.y,
        z: s * v5.z,
        w: c,
    };
    println!("result is {}", q1);
    println!("expected is {}", e1);
    assert!(q1.approx_eq_ulps(&e1, 2));
}

#[test]
fn rotation() {
    let v1 = Vec3 {
        x: 1.0f32,
        y: 0.0f32,
        z: 0.0f32,
    };

    let q1 = Quaternion::axis_and_angle_to_quaternion(&Vec3 {
                                                          x: 0.0f32,
                                                          y: 1.0f32,
                                                          z: 0.0f32,
                                                      },
                                                      90.0f32);

    let v2 = Quaternion::rotate(&v1, &q1);

    let e1 = Vec3 {
        x: 0.0f32,
        y: 0.0f32,
        z: -1.0f32,
    };

    println!("result is {}", v2);
    println!("expected is {}", e1);

    assert!(v2.approx_eq_ulps(&e1, 2));
}
