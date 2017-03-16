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

use std::any::Any;

use vk::vulkan::*;

use graphics::rendertarget::RenderTarget;
use graphics::renderer::Renderer;
use graphics::renderervk::*;
use graphics::texturevk::TextureVk;
use graphics::image::Image;

// This will likely all change as Vulkan renderer work progresses!

pub struct RenderTargetVk {
    width: u32,
    height: u32,
    pub texture: TextureVk,
    depth_image_view: RendererVkImageView,
    depth_image: RendererVkImage,
    framebuffer: Option<RendererVkFramebuffer>,
}

impl RenderTargetVk {
    /// Return the raw framebuffer handle
    pub fn get_framebuffer_raw(&mut self) -> VkFramebuffer {
        let fb = self.framebuffer.take().unwrap();
        let raw = fb.get_framebuffer_raw();
        self.framebuffer = Some(fb);

        raw
    }

    /// Return the raw depth image handle
    pub fn get_depth_image_raw(&self) -> VkImage {
        self.depth_image.get_image_raw()
    }

    /// Configure texture as a render-to-texture target
    ///
    /// texture: The texture the render target will use as storage
    /// width: Texture width
    /// height: Texture height
    pub fn new(renderer: &mut Box<Renderer>, width: u32, height: u32) -> RenderTargetVk {
        let texture_vk = TextureVk::new_float_rgba(renderer, width, height, &vec![]);

        let renderer_vk = match renderer.as_any_mut().downcast_mut::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        let depth_format = renderer_vk.choose_depth_format();

        let depth_image =
            RendererVkImage::new(&renderer_vk.device,
                                 &renderer_vk.physical_device,
                                 &renderer_vk.aux_command_pool,
                                 width,
                                 height,
                                 depth_format,
                                 VkImageTiling::VK_IMAGE_TILING_OPTIMAL,
                                 VkImageUsageFlagBits::VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT as VkImageUsageFlags,
                                 VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT as VkMemoryPropertyFlags,
                                 VkImageLayout::VK_IMAGE_LAYOUT_UNDEFINED,
                                 VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let depth_image_view = RendererVkImageView::new(&renderer_vk.device,
                                                        &depth_image,
                                                        depth_format,
                                                        VkImageAspectFlagBits::VK_IMAGE_ASPECT_DEPTH_BIT as VkImageAspectFlags);

        RenderTargetVk {
            width: width,
            height: height,
            texture: texture_vk,
            depth_image: depth_image,
            depth_image_view: depth_image_view,
            framebuffer: None,
        }
    }

    /// Continue configuration of the framebuffer object
    ///
    /// renderer_vk: The Vulkan renderer object
    pub fn setup(&mut self, renderer: &Box<Renderer>, pass_identifier: u32) {
        let renderer_vk = match renderer.as_any().downcast_ref::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        self.framebuffer = Some(RendererVkFramebuffer::new(&renderer_vk.device,
                                                           self.texture.texture.get_view_raw(),
                                                           Some(self.depth_image_view.get_view_raw()),
                                                           &renderer_vk.render_passes[pass_identifier as usize],
                                                           self.width,
                                                           self.height));
    }
}

impl RenderTarget for RenderTargetVk {
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
    fn snapshot(&self, renderer: &Box<Renderer>, filename: &str) {
        let renderer_vk = match renderer.as_any().downcast_ref::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        let data: Vec<u8> = self.texture.texture.read_pixels(renderer_vk);
        let image = Image::create_from_raw_data(self.width, self.height, &data);
        image.save_to(filename);
    }
}
