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

use std::mem;
use std::any::Any;

use gl;
use gl::types::*;

use graphics::rendertarget::RenderTarget;
use graphics::renderer::Renderer;
use graphics::texturegl::TextureGl;
use graphics::image::Image;

pub struct RenderTargetGl {
    texture: TextureGl,
    width: u32,
    height: u32,
    fbo: GLuint,
}

impl RenderTargetGl {
    /// Return the OpenGL framebuffer object identifier for this render target
    pub fn get_fbo(&self) -> GLuint {
        self.fbo
    }

    /// Return the texture object for this render target
    pub fn get_texture(&self) -> TextureGl {
        self.texture.clone()
    }

    /// Configure texture as a render-to-texture target
    ///
    /// width: Texture width
    /// height: Texture height
    pub fn new(renderer: &mut Box<Renderer>, width: u32, height: u32) -> RenderTargetGl {
        let texture_gl = TextureGl::new_float_rgba(renderer, width, height, &vec![]);

        let mut fbo: GLuint = 0;
        let mut drb: GLuint = 0;

        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                     gl::COLOR_ATTACHMENT0,
                                     gl::TEXTURE_2D,
                                     texture_gl.texture_name,
                                     0); // Level

            gl::GenRenderbuffers(1, &mut drb);
            gl::BindRenderbuffer(gl::RENDERBUFFER, drb);
            gl::RenderbufferStorage(gl::RENDERBUFFER,
                                    gl::DEPTH_COMPONENT24,
                                    width as GLint,
                                    height as GLint);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::RENDERBUFFER, drb);

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            debug_assert!(status == gl::FRAMEBUFFER_COMPLETE);

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        RenderTargetGl {
            texture: texture_gl,
            width: width,
            height: height,
            fbo: fbo,
        }
    }
}

impl RenderTarget for RenderTargetGl {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut Any {
        self
    }

    /// Bind the associated texture as the specified active texture number
    ///
    /// num: The texture number to bind the texture to
    fn bind_texture(&self, num: i32) {
        self.texture.bind(num);
    }

    /// Take a snapshot to disk
    ///
    /// renderer: The renderer object
    /// filename: The filename to save the snapshot to
    fn snapshot(&self, _: &Box<Renderer>, filename: &str) {
        let mut data: Vec<u8> = vec![];
        data.resize((self.width * self.height * 3) as usize, 0);

        unsafe {
            gl::ReadPixels(0,
                           0,
                           self.width as i32,
                           self.height as i32,
                           gl::RGB,
                           gl::UNSIGNED_BYTE,
                           mem::transmute(data.as_ptr()));
        }

        let image = Image::create_from_raw_data(self.width, self.height, &data);
        image.save_to(filename);
    }
}
