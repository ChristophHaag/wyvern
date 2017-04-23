// Copyright (c) 2016-2017 Bruce Stenning. All rights reserved.
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

use std::ptr;
use std::mem;
use std::os::raw;
use std::any::Any;

use gl;
use gl::types::*;

use graphics::texture::Texture;
use graphics::renderer::Renderer;

#[derive(Clone)]
pub struct TextureGl {
    pub texture_name: GLuint,
}

impl TextureGl {
    /// Set up a new 4-component float texture of the specified dimensions and the specified contents
    ///
    /// renderer: The renderer object
    /// width: The width of the texture
    /// height: The height of the texture
    /// data: The image data, empty if just defining the texture not populating it
    pub fn new_float_rgba(_: &mut Box<Renderer>, width: u32, height: u32, data: &Vec<u8>) -> TextureGl {
        TextureGl::new_specific(gl::RGBA,
                                gl::RGBA,
                                gl::FLOAT,
                                width as GLuint,
                                height as GLuint,
                                data)
    }

    /// Set up a new 3-component byte texture of the specified dimensions and the specified contents
    ///
    /// renderer: The renderer object
    /// width: The width of the texture
    /// height: The height of the texture
    /// data: The image data, empty if just defining the texture not populating it
    pub fn new_ubyte_rgba(_: &mut Box<Renderer>, width: u32, height: u32, data: &Vec<u8>) -> TextureGl {
        TextureGl::new_specific(gl::RGBA,
                                gl::RGBA,
                                gl::UNSIGNED_BYTE,
                                width as GLuint,
                                height as GLuint,
                                data)
    }

    /// Bind the texture as the specified active texture number
    ///
    /// num: The texture number to bind the texture to
    pub fn bind(&self, num: i32) {
        unsafe {
            gl::ActiveTexture(match num {
                1 => gl::TEXTURE1,
                _ => gl::TEXTURE0,
            });

            gl::BindTexture(gl::TEXTURE_2D, self.texture_name);
        }
    }
}

impl Texture for TextureGl {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut Any {
        self
    }

    /// Bind the texture as the specified active texture number
    ///
    /// num: The texture number to bind the texture to
    fn bind(&self, num: i32) {
        let texture_gl: &TextureGl = match self.as_any().downcast_ref::<TextureGl>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        texture_gl.bind(num);
    }
}

impl TextureGl {
    /// Set up a new texture of the specified format
    ///
    /// internal_format: The interal format of the texture
    /// data_format: The data format of the pixel data
    /// data_type: The data type of the pixel data
    /// width: The width of the texture
    /// height: The height of the texture
    /// data: The image data, empty if just defining the texture not populating it
    pub fn new_specific(internal_format: GLuint,
                        data_format: GLuint,
                        data_type: GLuint,
                        width: GLuint,
                        height: GLuint,
                        data: &Vec<u8>)
                        -> TextureGl {
        let mut texture_name: GLuint = 0;

        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::GenTextures(1, &mut texture_name);

            gl::BindTexture(gl::TEXTURE_2D, texture_name);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);

            let mut ptr: *const raw::c_void = ptr::null();
            if data.len() != 0 {
                ptr = mem::transmute(data.as_ptr());
            };

            gl::TexImage2D(gl::TEXTURE_2D,
                           0, // Level
                           internal_format as GLint,
                           width as GLint,
                           height as GLint,
                           0, // Border
                           data_format,
                           data_type,
                           ptr);
        }

        TextureGl { texture_name: texture_name }
    }
}
