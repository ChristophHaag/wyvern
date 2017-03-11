// Copyright (c) 2016 Bruce Stenning. All rights reserved.
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

use algebra::vector::*;

#[test]
fn vec3_add() {
    let v1 = Vec3 {
        x: 2.0f32,
        y: 4.0f32,
        z: 6.0f32,
    };
    let v2 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: -5.0f32,
    };
    let v3 = v1 + v2;
    let e = Vec3 {
        x: 5.0f32,
        y: 8.0f32,
        z: 1.0f32,
    };
    println!("result is {}", v3);
    println!("expected is {}", e);
    assert!(v3 == e);
}

#[test]
fn vec3_add_assign() {
    let mut v = Vec3 {
        x: 2.0f32,
        y: 4.0f32,
        z: 6.0f32,
    };
    v += 2.0f32;
    let e = Vec3 {
        x: 4.0f32,
        y: 6.0f32,
        z: 8.0f32,
    };
    println!("result is {}", v);
    println!("expected is {}", e);
    assert!(v == e);
}

#[test]
fn vec3_sub() {
    let v1 = Vec3 {
        x: 2.0f32,
        y: 4.0f32,
        z: 6.0f32,
    };
    let v2 = Vec3 {
        x: 3.0f32,
        y: 4.0f32,
        z: -5.0f32,
    };
    let v3 = v1 - v2;
    let e = Vec3 {
        x: -1.0f32,
        y: 0.0f32,
        z: 11.0f32,
    };
    println!("result is {}", v3);
    println!("expected is {}", e);
    assert!(v3 == e);
}

#[test]
fn vec3_sub_assign() {
    let mut v = Vec3 {
        x: 2.0f32,
        y: 4.0f32,
        z: 6.0f32,
    };
    v -= 2.0f32;
    let e = Vec3 {
        x: 0.0f32,
        y: 2.0f32,
        z: 4.0f32,
    };
    println!("result is {}", v);
    println!("expected is {}", e);
    assert!(v == e);
}

#[test]
fn vec4_add() {
    let v1 = Vec4 {
        x: 2.0f32,
        y: 4.0f32,
        z: 6.0f32,
        w: 8.0f32,
    };
    let v2 = Vec4 {
        x: 3.0f32,
        y: 4.0f32,
        z: -5.0f32,
        w: 2.0f32,
    };
    let v3 = v1 + v2;
    let e = Vec4 {
        x: 5.0f32,
        y: 8.0f32,
        z: 1.0f32,
        w: 10.0f32,
    };
    println!("result is {}", v3);
    println!("expected is {}", e);
    assert!(v3 == e);
}

#[test]
fn vec4_project_to_vec3() {
    let v1 = Vec4 {
        x: 2.0f32,
        y: 4.0f32,
        z: 6.0f32,
        w: 2.0f32,
    };
    let v2 = v1.project();
    let e = Vec3 {
        x: 1.0f32,
        y: 2.0f32,
        z: 3.0f32,
    };
    println!("result is {}", v2);
    println!("expected is {}", e);
    assert!(v2 == e);
}
