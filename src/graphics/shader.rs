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

use std::sync::*;
use std::boxed::Box;
use std::any::Any;

use graphics::renderer::Renderer;
use graphics::resources::*;
use misc::embeddedresources::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ShaderStage {
    VertexShader,
    TessControlShader,
    TessEvalShader,
    GeometryShader,
    FragmentShader,
}

/// Convert a ShaderStage to string for debug
pub fn shader_stage_name(shader_stage: ShaderStage) -> &'static str {
    match shader_stage {
        ShaderStage::VertexShader => "vertex",
        ShaderStage::TessControlShader => "tesselation control",
        ShaderStage::TessEvalShader => "tesselation evaluation",
        ShaderStage::GeometryShader => "geometry",
        ShaderStage::FragmentShader => "fragment",
    }
}

pub trait Shader: Send + Sync {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any;

    fn set_generate_warnings(&mut self, gen: bool);

    fn build_shader(&mut self,
                    autos: Option<&EmbeddedResources>,
                    renderer: &Box<Renderer>,
                    resource_manager: &Arc<Mutex<Box<ResourceManager>>>,
                    resources: &ShaderSpec,
                    old_driver: bool);

    fn check_for_rebuild(&mut self,
                         autos: Option<&EmbeddedResources>,
                         renderer: &Box<Renderer>,
                         resource_manager: &Arc<Mutex<Box<ResourceManager>>>)
                         -> bool;

    fn select(&self);

    fn set_uniform_int(&self, uniform_name: &str, value: i32);

    fn setup_float_attribute_pointer(&self, attribute_name: &str, components: usize, stride: usize, offset: usize);
}
