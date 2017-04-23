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

use std::collections::HashMap;
use std::str::FromStr;
use num::Zero;

use graphics::shader::*;
use graphics::renderer::*;
use misc::fileutils::*;

pub enum RenderTargetId {
    Swapchain = 0,
    Offscreen = 1,
}

pub struct ShaderFilesSpecification {
    pub filename: &'static str,
    pub shader_stage: ShaderStage,
    pub spirv_out: &'static str,
    pub reflect_out: &'static str,
}

impl Clone for ShaderFilesSpecification {
    fn clone(&self) -> ShaderFilesSpecification {
        ShaderFilesSpecification {
            filename: self.filename,
            shader_stage: self.shader_stage,
            spirv_out: self.spirv_out,
            reflect_out: self.reflect_out,
        }
    }
}

/// The possible types of uniform blocks
///
/// This matches all of the types in VkDescriptorType on the
/// assumption that this covers all the OpenGL types too.  It
/// should not need to exactly concur with the Vulkan types.
#[derive(Clone, Copy, PartialEq)]
pub enum UniformType {
    Sampler = 0,
    CombinedImageSampler = 1,
    SampledImage = 2,
    StorageImage = 3,
    UniformTexelBuffer = 4,
    StorageTexelBuffer = 5,
    UniformBuffer = 6,
    StorageBuffer = 7,
    UniformBufferDynamic = 8,
    StorageBufferDynamic = 9,
    InputAttachment = 10,
    RangeSize = 11,
}

// A specifier for a uniform block
pub struct UniformBlockSpec {
    pub name: &'static str,
    pub size: usize,
    pub set: u32,
    pub binding: u32,
    pub block_type: UniformType,
    pub uniforms: Vec<BlockUniformSpec>,
}

impl Clone for UniformBlockSpec {
    fn clone(&self) -> UniformBlockSpec {
        UniformBlockSpec {
            name: self.name,
            size: self.size,
            set: self.set,
            binding: self.binding,
            block_type: self.block_type,
            uniforms: self.uniforms.clone(),
        }
    }
}

impl Default for UniformBlockSpec {
    fn default() -> UniformBlockSpec {
        UniformBlockSpec {
            name: "none",
            size: 0,
            set: u32::max_value(),
            binding: u32::max_value(),
            block_type: UniformType::RangeSize,
            uniforms: vec![],
        }
    }
}

// A specifier for a uniform outside a block
pub struct UniformSpec {
    pub name: &'static str,
    pub set: u32, // Used by Vulkan
    pub binding: u32, // Used by Vulkan
    pub uniform_type: UniformType,
}

impl Clone for UniformSpec {
    fn clone(&self) -> UniformSpec {
        UniformSpec {
            name: self.name,
            set: self.set,
            binding: self.binding,
            uniform_type: self.uniform_type,
        }
    }
}

impl Default for UniformSpec {
    fn default() -> UniformSpec {
        UniformSpec {
            name: "none",
            set: u32::max_value(),
            binding: u32::max_value(),
            uniform_type: UniformType::RangeSize,
        }
    }
}

// A specifier for a uniform within block
pub struct BlockUniformSpec {
    pub name: &'static str,
    pub offset: usize, // Used by Vulkan
    pub stride: usize, // Used by Vulkan
}

impl Clone for BlockUniformSpec {
    fn clone(&self) -> BlockUniformSpec {
        BlockUniformSpec {
            name: self.name,
            offset: self.offset,
            stride: self.stride,
        }
    }
}

impl Default for BlockUniformSpec {
    fn default() -> BlockUniformSpec {
        BlockUniformSpec {
            name: "none",
            offset: 0,
            stride: 0,
        }
    }
}

// A specifier for a shader
pub struct ShaderSpec {
    pub name: &'static str,
    pub library_files: Vec<&'static str>,
    pub shader_files: Vec<ShaderFilesSpecification>,
    pub uniform_block_names: Vec<&'static str>,
    pub uniform_specs: Vec<UniformSpec>,
    pub vertex_array_type: VertexArrayType,
    pub attributes: Vec<&'static str>,
    pub fragment_out: &'static str,
    pub depth_test_enabled: bool,
    pub alpha_blending_enabled: bool,
    pub pass_identifier: u32,
}

impl Clone for ShaderSpec {
    fn clone(&self) -> ShaderSpec {
        ShaderSpec {
            name: self.name,
            library_files: self.library_files.clone(),
            shader_files: self.shader_files.clone(),
            uniform_block_names: self.uniform_block_names.clone(),
            uniform_specs: self.uniform_specs.clone(),
            vertex_array_type: self.vertex_array_type.clone(),
            attributes: self.attributes.clone(),
            fragment_out: self.fragment_out.clone(),
            depth_test_enabled: self.depth_test_enabled,
            alpha_blending_enabled: self.alpha_blending_enabled,
            pass_identifier: self.pass_identifier,
        }
    }
}

pub struct ResourceManager {
    pub uniform_block_specs: HashMap<&'static str, UniformBlockSpec>,
    pub shader_specs: HashMap<&'static str, ShaderSpec>,
}

impl ResourceManager {
    /// Construct a new shader resource manager, populated with the specified data
    ///
    /// uniform_block_specs: The uniform block specifications to use
    /// shader_specs: The shader specifications to use
    pub fn new(uniform_block_specs: HashMap<&'static str, UniformBlockSpec>,
               shader_specs: HashMap<&'static str, ShaderSpec>)
               -> ResourceManager {
        ResourceManager {
            uniform_block_specs: uniform_block_specs,
            shader_specs: shader_specs,
        }
    }

    /// Read in the data from the SPIR-V reflection files
    ///
    /// Note, this makes idempotent updates to various fields when they have multiple uses.
    /// For example for the binding points of uniform blocks that are used in multiple shaders.
    ///
    /// debug: true if debug statements should be dumped, false otherwise
    pub fn read_reflection_data(&mut self, debug: bool) {
        for ref mut shader in self.shader_specs.iter_mut() {
            let (_, ref mut shader_spec) = *shader;

            // Read all the offsets from .rfl files for this shader spec
            //
            let mut offsets: HashMap<String, usize> = HashMap::new();
            let mut block_sizes: HashMap<String, usize> = HashMap::new();
            let mut block_bindings: HashMap<String, u32> = HashMap::new();
            let mut uniform_bindings: HashMap<String, u32> = HashMap::new();

            for shader_file in shader_spec.shader_files.iter() {
                if debug {
                    println!("Reading {}", shader_file.reflect_out);
                }

                let contents = read_text_file(None, shader_file.reflect_out);
                let lines: Vec<&str> = contents.split("\n").collect();

                #[derive(PartialEq)]
                enum ParseMode {
                    Start = 0,
                    Uniforms = 1,
                    UniformBlocks = 2,
                    VertexAttribute = 3,
                }

                fn convert<T: FromStr + Zero>(arg: &String) -> T {
                    match arg.parse::<T>() {
                        Ok(r) => r,
                        Err(_) => {
                            println!("Unable to parse '{}'", arg);
                            T::zero()
                        }
                    }
                }

                let mut mode = ParseMode::Start;

                for l in lines {
                    let mut line = l.to_string();
                    line.pop();

                    if line.contains("Uniform reflection:") {
                        mode = ParseMode::Uniforms;
                    } else if line.contains("Uniform block reflection:") {
                        mode = ParseMode::UniformBlocks;
                    } else if line.contains("Vertex attribute reflection:") {
                        mode = ParseMode::VertexAttribute;
                    } else {
                        if line.contains(":") {
                            let uniform_line: Vec<&str> = line.split(":").collect();
                            let fields: Vec<&str> = uniform_line[1].split(",").collect();
                            for field in fields.iter() {
                                let field_bits: Vec<&str> = field.split(" ").collect();
                                // Note the space after each comma means that fields_bits[0] will be empty
                                if mode == ParseMode::Uniforms && field_bits[1] == "offset" && field_bits[2] != "-1" {
                                    if debug {
                                        println!("offset {} {}", uniform_line[0], field_bits[2]);
                                    }
                                    let offset = convert::<i32>(&field_bits[2].to_owned());
                                    offsets.insert(uniform_line[0].to_owned(), offset as usize);
                                } else if mode == ParseMode::Uniforms && field_bits[1] == "binding" && field_bits[2] != "-1" {
                                    if debug {
                                        println!("uniform binding {} {}", uniform_line[0], field_bits[2]);
                                    }
                                    let binding = convert::<u32>(&field_bits[2].to_owned());
                                    uniform_bindings.insert(uniform_line[0].to_owned(), binding);
                                } else if mode == ParseMode::UniformBlocks && field_bits[1] == "size" && field_bits[2] != "-1" {
                                    if debug {
                                        println!("block size {} {}", uniform_line[0], field_bits[2]);
                                    }
                                    let size = convert::<u32>(&field_bits[2].to_owned());
                                    block_sizes.insert(uniform_line[0].to_owned(), size as usize);
                                } else if mode == ParseMode::UniformBlocks && field_bits[1] == "binding" && field_bits[2] != "-1" {
                                    if debug {
                                        println!("block binding {} {}", uniform_line[0], field_bits[2]);
                                    }
                                    let binding = convert::<u32>(&field_bits[2].to_owned());
                                    block_bindings.insert(uniform_line[0].to_owned(), binding);
                                }
                            }
                        }
                    }
                }
            }

            // Update:
            // - Offsets for all of the non-opaque uniforms
            // - Binding points for all uniform blocks
            //
            for uniform_block_name in shader_spec.uniform_block_names.iter() {
                if !self.uniform_block_specs.contains_key(uniform_block_name) {
                    println!("Failed to find uniform block {} for shader {}", uniform_block_name, shader_spec.name);
                    panic!("Check the resource definitions");
                }
                let ref mut uniform_block_spec = self.uniform_block_specs.get_mut(uniform_block_name).unwrap();

                for ref mut uniform in uniform_block_spec.uniforms.iter_mut() {
                    if offsets.contains_key(uniform.name) {
                        if debug {
                            println!("Updating offset for {} to {}",
                                     uniform.name,
                                     offsets[uniform.name]);
                        }
                        uniform.offset = offsets[uniform.name];
                    } else {
                        // If the key has not been found then it indicates that the uniform is
                        // not used, so not output in the reflection data
                        if debug {
                            println!("Ignoring offset for {}", uniform.name);
                        }
                    }
                }

                if block_bindings.contains_key(*uniform_block_name) {
                    if debug {
                        println!("Updating size for block {} to {}",
                                 uniform_block_name,
                                 block_sizes[*uniform_block_name]);
                    }
                    uniform_block_spec.size = block_sizes[*uniform_block_name];

                    if debug {
                        println!("Updating binding point for block {} to {}",
                                 uniform_block_name,
                                 block_bindings[*uniform_block_name]);
                    }
                    uniform_block_spec.binding = block_bindings[*uniform_block_name];
                } else {
                    if debug {
                        println!("Ignoring size and binding point for block {}",
                                 *uniform_block_name);
                    }
                }
            }

            // Update the binding points for all the opaque uniforms
            //
            for ref mut uniform_spec in shader_spec.uniform_specs.iter_mut() {
                if uniform_bindings.contains_key(uniform_spec.name) {
                    if debug {
                        println!("Updating binding point for uniform {} to {}",
                                 uniform_spec.name,
                                 uniform_bindings[uniform_spec.name]);
                    }
                    uniform_spec.binding = uniform_bindings[uniform_spec.name];
                } else {
                    if debug {
                        println!("Ignoring binding point for uniform {}", uniform_spec.name);
                    }
                }
            }
        }
    }

    /// Return the shader resource specification object for the given shader name
    ///
    /// name: The name of the desired shader
    ///
    /// Returns the shader resource object corresponding to the specified name
    pub fn get_shader_spec(&self, name: &str) -> ShaderSpec {
        self.shader_specs[name].clone()
    }
}
