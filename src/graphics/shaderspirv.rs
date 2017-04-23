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

use std::sync::*;
use std::boxed::Box;
use std::any::Any;
use std::process::Command;
use std::fs::remove_file;
use std::time::UNIX_EPOCH;

use vk::vulkan::*;

use graphics::shader::*;
use graphics::renderer::*;
use graphics::renderervk::*;
use graphics::resources::*;
use misc::fileutils::*;
use misc::embeddedresources::*;

pub struct ShaderSpirv {
    device: VkDevice,
    shader_name: &'static str,
    lib_files: Vec<&'static str>,
    shader_files: Vec<ShaderFilesSpecification>,
    attribute_names: Vec<&'static str>,
    fragment_out: &'static str,

    shader_modules: Vec<RendererVkShaderModule>,
    shader_modules_raw: Vec<(ShaderStage, VkShaderModule)>,
}
unsafe impl Send for ShaderSpirv {}
unsafe impl Sync for ShaderSpirv {}

impl ShaderSpirv {
    pub fn new(renderer: &Box<Renderer>) -> ShaderSpirv {
        let renderer_vk: &RendererVk = match renderer.as_any().downcast_ref::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        ShaderSpirv {
            device: renderer_vk.get_raw_device(),
            shader_name: "",
            lib_files: vec![],
            shader_files: vec![],
            attribute_names: vec![],
            fragment_out: "",

            shader_modules: vec![],
            shader_modules_raw: vec![],
        }
    }

    /// Convert a ShaderStage to an Vulkan shader stage enum
    ///
    /// shader_stage: The shader stage to convert to a Vulkan enum
    ///
    /// Returns: The Vulkan shader stage flag bit corresponding to the shader stage
    pub fn internal_shader_stage(shader_stage: ShaderStage) -> VkShaderStageFlagBits {
        match shader_stage {
            ShaderStage::VertexShader => VkShaderStageFlagBits::VK_SHADER_STAGE_VERTEX_BIT,
            ShaderStage::TessControlShader => VkShaderStageFlagBits::VK_SHADER_STAGE_TESSELLATION_CONTROL_BIT,
            ShaderStage::TessEvalShader => VkShaderStageFlagBits::VK_SHADER_STAGE_TESSELLATION_EVALUATION_BIT,
            ShaderStage::GeometryShader => VkShaderStageFlagBits::VK_SHADER_STAGE_GEOMETRY_BIT,
            ShaderStage::FragmentShader => VkShaderStageFlagBits::VK_SHADER_STAGE_FRAGMENT_BIT,
        }
    }

    /// Convert a ShaderStage to short string for use as an extension name or for passing to glslangValidator
    ///
    /// shader_stage: The shader stage to convert to a string
    ///
    /// Returns: The short string name of the shader stage
    pub fn shader_extension_name(shader_stage: ShaderStage) -> &'static str {
        match shader_stage {
            ShaderStage::VertexShader => "vert",
            ShaderStage::TessControlShader => "tesc",
            ShaderStage::TessEvalShader => "tese",
            ShaderStage::GeometryShader => "geom",
            ShaderStage::FragmentShader => "frag",
        }
    }

    /// Compile all of the files used by a specific shader
    ///
    /// spec: The specification of the shader resource to build
    /// conditionally: When true, compare the timestamps of the input and
    ///     output to decide whether to compile or not
    /// debug_output_level: Debug output level (0 = silent, 1 = minimal, 2 = full)
    /// all_succeeded: Set this flag to false whenever any of the files
    ///     failed to compile
    pub fn compile_shader_resource(spec: &ShaderSpec, conditionally: bool, debug_output_level: u32, all_succeeded: &mut bool) {
        for shader_file in spec.shader_files.iter() {
            let extension = ShaderSpirv::shader_extension_name(shader_file.shader_stage);
            let stage_name = shader_stage_name(shader_file.shader_stage);

            let mut output_timestamp = match get_last_modification_timestamp(shader_file.spirv_out) {
                Err(_) => UNIX_EPOCH,
                Ok(t) => t,
            };
            let output_timestamp_rfl = match get_last_modification_timestamp(shader_file.reflect_out) {
                Err(_) => UNIX_EPOCH,
                Ok(t) => t,
            };
            if output_timestamp_rfl < output_timestamp {
                output_timestamp = output_timestamp_rfl;
            }

            let mut rebuild = false;
            for lib_filename in spec.library_files.iter() {
                let input_timestamp = match get_last_modification_timestamp(lib_filename) {
                    Err(e) => panic!("{}", e),
                    Ok(t) => t,
                };
                if input_timestamp > output_timestamp {
                    rebuild = true;
                }
            }
            let input_timestamp = match get_last_modification_timestamp(shader_file.filename) {
                Err(e) => panic!("{}", e),
                Ok(t) => t,
            };
            if input_timestamp > output_timestamp {
                rebuild = true;
            }
            if conditionally && !rebuild {
                if debug_output_level > 0 {
                    println!("Skipping compilation of SPIR-V for {}, for {} stage",
                             spec.name,
                             extension);
                }
                continue;
            }

            if debug_output_level > 0 {
                println!("Compiling SPIR-V for {}, stage {}", spec.name, stage_name);
            }

            let mut lib_source = "#version 450 core\n\n".to_string();
            for lib_filename in spec.library_files.iter() {
                if debug_output_level > 1 {
                    println!("Incorporating library file {}", lib_filename);
                }
                let source = read_text_file(None, lib_filename);
                lib_source = lib_source + &source + &"\n#line 1\n";
            }

            if debug_output_level > 1 {
                println!("Incorporating source file {}", shader_file.filename);
            }
            let source = read_text_file(None, &shader_file.filename);
            let full_source = lib_source.clone() + &source;
            write_entire_file(&full_source, &("temp.".to_string() + &extension)).expect("Failed to write shader temporary file");

            // Build the SPIR-V
            //
            let mut command;
            command = Command::new("glslangValidator");
            command.arg("-V") // SPIR-V output with Vulkan semantics
                    .arg("-q") // Build reflection data
                    .arg("-o") // Specify output file
                    .arg(shader_file.spirv_out)
                    .arg("temp.".to_string() + &extension);

            if debug_output_level > 1 {
                println!("Running glslangValidator:");
            }
            let output = command.output().expect("Failed to invoke GLSL to SPIR-V compiler");
            if debug_output_level > 1 || !output.status.success() {
                println!("Status: {}", output.status);
                println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            }

            if !output.status.success() {
                *all_succeeded = false;
            } else {
                write_entire_file(&String::from_utf8_lossy(&output.stdout),
                                  shader_file.reflect_out)
                    .expect("Failed to write shader reflection file");
            }

            // Remove temporary file
            remove_file("temp.".to_string() + &extension).expect("Failed to remove temporary file");

            if debug_output_level > 1 {
                println!("");
            }
        }
    }

    /// Return the raw Vulkan shader modules
    pub fn get_shader_modules(&self) -> Vec<(ShaderStage, VkShaderModule)> {
        // Clone the vector of raw shader module handles
        self.shader_modules_raw.clone()
    }
}

impl Shader for ShaderSpirv {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any {
        self
    }

    /// Set the generate-warnings flag
    fn set_generate_warnings(&mut self, _: bool) {
        // Ignore this request
    }

    /// Load, compile, and link shaders into a shader program
    ///
    /// autos: The automatically generated resources object
    /// renderer: The renderer object
    /// resource_manager: The shader resource manager
    /// resources: The resources required for the shader
    /// old_driver: true if this is being run on an older driver that doesn't understand
    ///     some Vulkan GLSL enhancements
    fn build_shader(&mut self,
                    _: Option<&EmbeddedResources>,
                    _: &Box<Renderer>,
                    _: &Arc<Mutex<Box<ResourceManager>>>,
                    resources: &ShaderSpec,
                    _: bool) {
        self.shader_name = resources.name.clone();
        self.lib_files = resources.library_files.clone();
        self.shader_files = resources.shader_files.clone();
        self.attribute_names = resources.attributes.clone();
        self.fragment_out = resources.fragment_out.clone();

        for shader_file in self.shader_files.iter() {
            let bytecode = read_binary_file(shader_file.spirv_out, false /* debug */).expect("Unable to read SPIR-V");
            let shader_module = RendererVkShaderModule::new(self.device, &bytecode);
            self.shader_modules_raw.push((shader_file.shader_stage, shader_module.get_raw()));
            self.shader_modules.push(shader_module);
        }
    }

    /// Check whether the shader needs to be recompiled
    ///
    /// autos: The automatically generated resources object, only used for
    ///     checking whther the resources have been "baked in"
    /// renderer: The renderer object
    /// resource_manager: The shader resource manager
    fn check_for_rebuild(&mut self,
                         _: Option<&EmbeddedResources>,
                         _: &Box<Renderer>,
                         _: &Arc<Mutex<Box<ResourceManager>>>)
                         -> bool {
        // There is no shader recompilation at runtime for Vulkan as the moment
        false
    }

    /// Tell the renderer to use the shader
    fn select(&self) {
        // The shader is selected via pipeline selection in Vulkan
    }

    /// Set a single integer value for the named uniform, used only for identifying opaques, e.g. texture names
    ///
    /// uniform_name: The name of the uniform whose value should be set
    /// value: The value to set the uniform to
    #[allow(unused_variables)]
    fn set_uniform_int(&self, uniform_name: &str, value: i32) {
        // Nothing needs to be done here for Vulkan rendering
    }

    /// Configure a vertex attribute pointer
    ///
    /// attribute_name: The name of the attribute whose pointer should be configured
    /// components: The number of components in each entity (e.g. 3 for a vertex, or
    ///    2 for a texture u/v coordinate)
    /// stride: The distance (in single-precision floating points) from one entity
    ///    to the next
    /// offset: The offset within the vertex array of the first entity
    #[allow(unused_variables)]
    fn setup_float_attribute_pointer(&self, attribute_name: &str, components: usize, stride: usize, offset: usize) {}
}

impl Drop for ShaderSpirv {
    fn drop(&mut self) {}
}
