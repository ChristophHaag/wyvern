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

use std::any::Any;

use graphics::texture::Texture;
use graphics::renderer::Renderer;
use graphics::renderervk::*;

use vk::vulkan::*;

pub struct TextureVk {
    pub texture: RendererVkTexture,
}

impl TextureVk {
    /// Set up a new 4-component float texture of the specified dimensions and the specified contents
    ///
    /// renderer: The renderer object
    /// width: The width of the texture
    /// height: The height of the texture
    /// data: The image data, empty if just defining the texture not populating it
    pub fn new_float_rgba(renderer: &mut Box<Renderer>, width: u32, height: u32, data: &Vec<u8>) -> TextureVk {
        let renderer_vk = match renderer.as_any_mut().downcast_mut::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        let texture = RendererVkTexture::new(renderer_vk,
                                             width,
                                             height,
                                             VkFormat::VK_FORMAT_R32G32B32A32_SFLOAT,
                                             16, // Four single-precision floats
                                             data);

        TextureVk { texture: texture }
    }

    /// Set up a new 3-component byte texture of the specified dimensions and the specified contents
    ///
    /// renderer: The renderer object
    /// width: The width of the texture
    /// height: The height of the texture
    /// data: The image data, empty if just defining the texture not populating it
    pub fn new_ubyte_rgba(renderer: &mut Box<Renderer>, width: u32, height: u32, data: &Vec<u8>) -> TextureVk {
        let renderer_vk = match renderer.as_any_mut().downcast_mut::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        let texture = RendererVkTexture::new(renderer_vk,
                                             width,
                                             height,
                                             VkFormat::VK_FORMAT_R8G8B8A8_UNORM,
                                             4,
                                             data);

        TextureVk { texture: texture }
    }

    /// Bind the texture as the specified active texture number
    ///
    /// num: The texture number to bind the texture to
    pub fn bind(&self, _: i32) {}
}

impl Texture for TextureVk {
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
    fn bind(&self, _: i32) {}
}
