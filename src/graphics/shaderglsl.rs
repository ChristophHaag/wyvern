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
use std::mem;
use std::ptr;
use std::str;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw;
use std::os::raw::*;
use std::time::SystemTime;
use std::any::Any;
use regex::Regex;

use gl;
use gl::types::*;

use graphics::renderer::Renderer;
use graphics::renderergl::*;
use graphics::shader::*;
use graphics::resources::*;
use misc::fileutils::*;
use misc::embeddedresources::*;

pub struct UniformBlockDesc {
    pub size: usize,
    pub index: GLuint,
    pub handle: GLuint,
    pub binding: GLuint,
    pub offsets: HashMap<&'static str, usize>,
    pub strides: HashMap<&'static str, usize>,
}

pub struct ShaderGlsl {
    use_autos: bool,
    generate_warnings: bool,
    old_driver: bool,

    shader_name: &'static str,
    lib_files: Vec<&'static str>,
    shader_files: Vec<ShaderFilesSpecification>,
    uniform_block_names: Vec<&'static str>,
    uniform_specs: Vec<UniformSpec>,
    attribute_names: Vec<&'static str>,
    fragment_out: &'static str,
    depth_test_enabled: bool,
    alpha_blending_enabled: bool,

    file_mod_times: HashMap<&'static str, SystemTime>,

    program: GLint,
    vao: GLuint,
    vbo: GLuint,

    shaders: Vec<GLuint>,

    uniform_block_descs: HashMap<&'static str, UniformBlockDesc>,
    uniforms: HashMap<&'static str, GLint>,
    attributes: HashMap<&'static str, GLint>,
}
unsafe impl Send for ShaderGlsl {}
unsafe impl Sync for ShaderGlsl {}

impl ShaderGlsl {
    pub fn new() -> ShaderGlsl {
        ShaderGlsl {
            use_autos: false,
            generate_warnings: true,
            old_driver: false,

            shader_name: "",
            lib_files: vec![],
            shader_files: vec![],
            uniform_block_names: vec![],
            uniform_specs: vec![],
            attribute_names: vec![],
            fragment_out: "",
            depth_test_enabled: false,
            alpha_blending_enabled: false,

            file_mod_times: HashMap::new(),

            program: -1,
            vao: 0,
            vbo: 0,

            shaders: vec![],

            uniform_block_descs: HashMap::new(),
            uniforms: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

    /// Convert a ShaderStage to an OpenGL shader stage type
    pub fn internal_shader_stage(shader_stage: ShaderStage) -> GLenum {
        match shader_stage {
            ShaderStage::VertexShader => gl::VERTEX_SHADER,
            ShaderStage::TessControlShader => gl::TESS_CONTROL_SHADER,
            ShaderStage::TessEvalShader => gl::TESS_EVALUATION_SHADER,
            ShaderStage::GeometryShader => gl::GEOMETRY_SHADER,
            ShaderStage::FragmentShader => gl::FRAGMENT_SHADER,
        }
    }

    /// Main portion of loading, compiling, and linking shaders into a shader program
    ///
    /// The shader level must be defined before the preprocessor directives,
    /// so it is explicitly output here and not declared in the sources.
    /// This forces all shaders to be written for the same GLSL level.
    ///
    /// autos: The automatically generated resources object
    /// renderer: The renderer object
    /// resource_manager: The shader resource manager
    pub fn build_shader_helper(&mut self,
                               autos: Option<&EmbeddedResources>,
                               renderer: &Box<Renderer>,
                               resource_manager: &Arc<Mutex<Box<ResourceManager>>>) {
        let renderer_gl = match renderer.as_any().downcast_ref::<RendererGl>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        unsafe {
            let mut vao: GLuint = 0;
            let mut vbo: GLuint = 0;

            // Create a Vertex Array Object
            gl::GenVertexArrays(1, &mut vao);

            // Create a Vertex Buffer Object
            gl::GenBuffers(1, &mut vbo);

            // Bind them
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            // Read the library source file
            let mut source_names = vec![];
            let mut lib_source = String::new();
            for filename in self.lib_files.iter() {
                let now = SystemTime::now();
                self.file_mod_times.insert(filename, now);
                let source = read_text_file(autos, filename);
                lib_source = lib_source.to_string() + &source + &"\n#line 1\n";
            }

            // Load and compile GLSL shader sources
            let mut shaders = vec![];
            for shader_file in self.shader_files.iter() {
                let now = SystemTime::now();
                self.file_mod_times.insert(shader_file.filename, now);
                let source = read_text_file(autos, &shader_file.filename);
                let full_source = lib_source.to_string() + &source;
                let compiled = compile_glsl(&self.shader_name,
                                            &source_names,
                                            &full_source,
                                            shader_file.shader_stage,
                                            self.old_driver);
                if compiled < 0 {
                    gl::DeleteVertexArrays(1, &vao);
                    gl::DeleteBuffers(1, &vbo);
                    return;
                }
                shaders.push(compiled as GLuint);
                source_names.pop();
            }

            // Link the shader program
            let program = link_program(&self.shader_name, &shaders);
            if program < 0 {
                gl::DeleteVertexArrays(1, &vao);
                gl::DeleteBuffers(1, &vbo);
                return;
            }

            // Select the shader program
            gl::UseProgram(program as GLuint);

            // Find all the required uniform block information and store it
            let mut uniform_block_descs = HashMap::new();
            for block_name in self.uniform_block_names.iter() {
                let ref block = resource_manager.lock().unwrap().uniform_block_specs[block_name];

                let block_index = gl::GetUniformBlockIndex(program as GLuint,
                                                           CString::new(block_name.clone()).unwrap().as_ptr());
                if block_index == gl::INVALID_INDEX {
                    println!("build_shader could not find uniform block {} for {}",
                             block_name,
                             self.shader_name);
                } else {
                    let mut block_size: GLint = 0;
                    gl::GetActiveUniformBlockiv(program as GLuint,
                                                block_index,
                                                gl::UNIFORM_BLOCK_DATA_SIZE,
                                                &mut block_size);

                    // Get and the buffer handle corresponding to the block name
                    let ubo_handle = renderer_gl.get_uniform_buffer_handle(block_name);
                    gl::BindBuffer(gl::UNIFORM_BUFFER, ubo_handle);

                    // Initialise our record of the uniform block
                    let mut uniform_block_entries = UniformBlockDesc {
                        size: block_size as usize,
                        index: block_index,
                        handle: ubo_handle,
                        binding: block.binding,
                        offsets: HashMap::new(),
                        strides: HashMap::new(),
                    };

                    // Initialise the uniform buffer with a disposable vector
                    let mut bytes = Vec::with_capacity(block_size as usize);
                    bytes.resize(block_size as usize, 0);
                    gl::BufferData(gl::UNIFORM_BUFFER,
                                   block_size as isize,
                                   ptr::null(),
                                   gl::DYNAMIC_DRAW);
                    gl::BindBufferBase(gl::UNIFORM_BUFFER, block.binding, ubo_handle);
                    gl::UniformBlockBinding(program as GLuint, block_index, block.binding);

                    // Obtain an array of uniform indices from GL
                    let num_uniforms = block.uniforms.len();
                    let mut indices: Vec<GLuint> = Vec::with_capacity(num_uniforms);
                    indices.resize(num_uniforms, 0);
                    let uniform_cstring_array: Vec<CString> =
                        block.uniforms.iter().map(|uniform| CString::new(uniform.name.clone()).unwrap()).collect();
                    let uniform_carray: Vec<*const c_char> = uniform_cstring_array.iter().map(|str| str.as_ptr()).collect();
                    gl::GetUniformIndices(program as GLuint,
                                          num_uniforms as i32,
                                          uniform_carray.as_ptr(),
                                          indices.as_mut_ptr());

                    // Obtain an array of uniform offsets corresponding to the indices
                    let mut offsets: Vec<GLint> = Vec::with_capacity(num_uniforms);
                    offsets.resize(num_uniforms, 0);
                    gl::GetActiveUniformsiv(program as GLuint,
                                            num_uniforms as i32,
                                            indices.as_ptr(),
                                            gl::UNIFORM_OFFSET,
                                            offsets.as_mut_ptr());

                    // Obtain an array of uniform strides corresponding to the indices
                    let mut strides: Vec<GLint> = Vec::with_capacity(num_uniforms);
                    strides.resize(num_uniforms, 0);
                    gl::GetActiveUniformsiv(program as GLuint,
                                            num_uniforms as i32,
                                            indices.as_ptr(),
                                            gl::UNIFORM_ARRAY_STRIDE,
                                            strides.as_mut_ptr());

                    // Now check each and store the valid offsets for each uniform
                    for (offset, uniform) in offsets.iter().zip(block.uniforms.iter()) {
                        if *offset == -1 {
                            println!("Failed to find uniform block {} name {}",
                                     block_name,
                                     uniform.name);
                        } else {
                            // println!("build_shader {} uniform block {} size {} uniform {}, offset {}",
                            //          self.shader_name,
                            //          block_name,
                            //          block_size,
                            //          uniform.name,
                            //          *offset);
                            uniform_block_entries.offsets.insert(uniform.name, *offset as usize);
                        }
                    }

                    // Now check each and store the valid strides for each uniform
                    for (stride, uniform) in strides.iter().zip(block.uniforms.iter()) {
                        if *stride != -1 {
                            // println!("build_shader {} uniform block {} size {} uniform {}, stride {}",
                            //          self.shader_name,
                            //          block_name,
                            //          block_size,
                            //          uniform.name,
                            //          *stride);
                            uniform_block_entries.strides.insert(uniform.name, *stride as usize);
                        }
                    }

                    // And store the block into the collection of blocks for this shader
                    uniform_block_descs.insert(*block_name, uniform_block_entries);

                    // Unbind the uniform buffer now that we are done with it
                    gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
                }
            }

            // Find all the required uniform locations and store them
            let mut uniforms = HashMap::new();
            for spec in self.uniform_specs.iter() {
                let uniform = gl::GetUniformLocation(program as GLuint,
                                                     CString::new(spec.name.clone()).unwrap().as_ptr());
                if uniform == -1 {
                    println!("build_shader could not find uniform {} for {}",
                             spec.name,
                             self.shader_name);
                } else {
                    uniforms.insert(spec.name.clone(), uniform);
                }
            }

            // Find all the required attribute locations and store them
            let mut attributes = HashMap::new();
            for name in self.attribute_names.iter() {
                let attribute = gl::GetAttribLocation(program as GLuint,
                                                      CString::new((*name).clone()).unwrap().as_ptr());
                if attribute == -1 {
                    println!("build_shader could not find attribute {} for {}",
                             name,
                             self.shader_name);
                } else {
                    attributes.insert((*name).clone(), attribute);
                }
            }

            // At this point we can update things
            self.program = program;
            self.vao = vao;
            self.vbo = vbo;
            self.shaders = shaders;
            self.uniform_block_descs = uniform_block_descs;
            self.uniforms = uniforms;
            self.attributes = attributes;

            // Define the fragment output variable
            gl::BindFragDataLocation(self.program as GLuint,
                                     0,
                                     CString::new(self.fragment_out.clone()).unwrap().as_ptr());
        }

        self.generate_warnings = true;
    }

    /// Get the uniform buffer layout from a uniform block
    ///
    /// block_name: The name of the block to return the buffer information for
    ///
    /// Returns: The uniform descriptor for the named block
    pub fn get_uniform_buffer_descriptor_from_uniform_block(&self, block_name: &str) -> UniformBufferDesc {
        if !self.uniform_block_descs.contains_key(block_name) {
            panic!("Could not find uniform buffer for block named {}", block_name);
        }
        let ref block = self.uniform_block_descs[block_name];

        let mut descriptor = UniformBufferDesc {
            size: block.size,
            bytes: Vec::with_capacity(block.size),
            offsets: block.offsets.clone(),
            strides: block.strides.clone(),
        };
        descriptor.bytes.resize(block.size, 0);

        descriptor
    }

    /// Get the ID of the named uniform
    ///
    /// This prints a warning if the name is not known.  This permits configuring
    /// uniforms that are not used and therefore optimised away.  The Rust is
    /// expensive to recompile, whereas the GLSL is very cheap, so modifications
    /// to the GLSL should not force changes in the Rust.
    ///
    /// str: The name of the uniform to return
    ///
    /// Returns the identifier of the uniform
    fn get_uniform(&self, name: &str) -> GLint {
        if !self.uniforms.contains_key(name) {
            if self.generate_warnings {
                println!("get_uniform could not find {} for {}",
                         name,
                         self.shader_name);
            }
            return -1;
        }
        self.uniforms[name]
    }

    /// Get the ID of the named attribute
    ///
    /// This prints a warning if the name is not known.  This permits configuring
    /// attributes that are not used and therefore optimised away.  The Rust is
    /// expensive to recompile, whereas the GLSL is very cheap, so modifications
    /// to the GLSL should not force changes in the Rust.
    ///
    /// str: The name of the attribute to return
    ///
    /// Returns the identifier of the attribute
    fn get_attribute(&self, name: &str) -> GLint {
        if !self.attributes.contains_key(name) {
            if self.generate_warnings {
                println!("get_attribute could not find {} for {}",
                         name,
                         self.shader_name);
            }
            return -1;
        }
        self.attributes[name]
    }
}

impl Shader for ShaderGlsl {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any {
        self
    }

    /// Set the generate-warnings flag
    fn set_generate_warnings(&mut self, gen: bool) {
        self.generate_warnings = gen;
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
                    autos: Option<&EmbeddedResources>,
                    renderer: &Box<Renderer>,
                    resource_manager: &Arc<Mutex<Box<ResourceManager>>>,
                    shader_spec: &ShaderSpec,
                    old_driver: bool) {
        match autos {
            Some(ref autos) => self.use_autos = autos.use_me(),
            None => (),
        };

        self.old_driver = old_driver;

        self.shader_name = shader_spec.name;
        self.lib_files = shader_spec.library_files.clone();
        self.shader_files = shader_spec.shader_files.clone();
        self.uniform_block_names = shader_spec.uniform_block_names.clone();
        self.uniform_specs = shader_spec.uniform_specs.clone();
        self.attribute_names = shader_spec.attributes.clone();
        self.fragment_out = shader_spec.fragment_out.clone();
        self.depth_test_enabled = shader_spec.depth_test_enabled;
        self.alpha_blending_enabled = shader_spec.alpha_blending_enabled;

        self.build_shader_helper(autos, renderer, resource_manager);
    }

    /// Check whether the shader needs to be recompiled
    ///
    /// autos: The automatically generated resources object, only used for
    ///     checking whther the resources have been "baked in"
    /// renderer: The renderer object
    /// resource_manager: The shader resource manager
    fn check_for_rebuild(&mut self,
                         autos: Option<&EmbeddedResources>,
                         renderer: &Box<Renderer>,
                         resource_manager: &Arc<Mutex<Box<ResourceManager>>>)
                         -> bool {
        if self.use_autos {
            // Don't rebuild, as the resources are not on disk
            return false;
        }

        let mut recompile = false;
        for filename in self.lib_files.iter() {
            if get_last_modification_timestamp(filename).unwrap() > self.file_mod_times[filename] {
                recompile = true;
            }
        }
        for shader_file in self.shader_files.iter() {
            if get_last_modification_timestamp(shader_file.filename).unwrap() > self.file_mod_times[shader_file.filename] {
                recompile = true;
            }
        }

        if recompile {
            println!("Recompiling {}", self.shader_name);
            self.build_shader_helper(autos, renderer, resource_manager);
        }

        recompile
    }

    /// Tell the renderer to use the shader
    fn select(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::UseProgram(self.program as GLuint);
        }

        if self.depth_test_enabled {
            unsafe {
                gl::DepthFunc(gl::LESS);
                gl::Enable(gl::DEPTH_TEST);
            }
        } else {
            unsafe {
                gl::Disable(gl::DEPTH_TEST);
            }
        }

        if self.alpha_blending_enabled {
            unsafe {
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::Enable(gl::BLEND);
            }
        } else {
            unsafe {
                gl::Disable(gl::BLEND);
            }
        }
    }

    /// Set a single integer value for the named uniform, used only for identifying opaques, e.g. texture names
    ///
    /// uniform_name: The name of the uniform whose value should be set
    /// value: The value to set the uniform to
    fn set_uniform_int(&self, uniform_name: &str, value: i32) {
        let u = self.get_uniform(uniform_name);
        if u >= 0 {
            unsafe {
                gl::Uniform1i(u, value);
            }
        }
    }

    /// Configure a vertex attribute pointer
    ///
    /// This configures an attribute so that it can point at interleaved data
    /// in the vertex array when rendering.  It also enables the pointer in
    /// the Vertex Array Object.
    ///
    /// Note that this is quite restrictive at the moment, in that it assumes
    /// floating point data with stride and offset that are multiples of a
    /// floating point type.
    ///
    /// attribute_name: The name of the attribute whose pointer should be configured
    /// components: The number of components in each entity (e.g. 3 for a vertex, or
    ///    2 for a texture u/v coordinate)
    /// stride: The distance (in single-precision floating points) from one entity
    ///    to the next
    /// offset: The offset within the vertex array of the first entity
    fn setup_float_attribute_pointer(&self, attribute_name: &str, components: usize, stride: usize, offset: usize) {
        let attribute = self.get_attribute(attribute_name);
        if attribute > -1 {
            unsafe {
                gl::VertexAttribPointer(attribute as GLuint,
                                        components as GLint,
                                        gl::FLOAT,
                                        gl::FALSE as GLboolean, // Whether normalised
                                        (stride * mem::size_of::<GLfloat>()) as GLsizei,
                                        (offset * mem::size_of::<GLfloat>()) as *const raw::c_void);
                gl::EnableVertexAttribArray(attribute as GLuint);
            }
        }
    }
}

impl Drop for ShaderGlsl {
    fn drop(&mut self) {
        if self.program != -1 {
            unsafe {
                gl::DeleteProgram(self.program as GLuint);
                for shader in self.shaders.iter() {
                    gl::DeleteShader(*shader);
                }
                gl::DeleteBuffers(1, &self.vbo);
                gl::DeleteVertexArrays(1, &self.vao);
            }
        }
    }
}

/// Compile the GLSL passed in as a string
///
/// Based on the C code at:
/// https://www.opengl.org/wiki/Shader_Compilation
///
/// name: The name of the shader
/// source_names: The names of the source files
/// glsl: The source to compile
/// shadertype: The type of shader being compiled
/// old_driver: true to avoid GL_KHR_vulkan_glsl
fn compile_glsl(name: &str, source_names: &Vec<String>, glsl: &str, shader_stage: ShaderStage, old_driver: bool) -> GLint {
    let shader;

    let mut preamble = "#version 450 core\n".to_string();

    let preprocessed;
    if old_driver {
        let re = Regex::new(r"layout\s*\(set\s+=\s+\d+,").unwrap();
        preprocessed = re.replace_all(glsl, "layout(").to_string();
    } else {
        preprocessed = glsl.to_string();
        preamble = preamble + &"#extension GL_KHR_vulkan_glsl : enable\n"
    }

    unsafe {
        shader = gl::CreateShader(ShaderGlsl::internal_shader_stage(shader_stage));

        let full_source = CString::new((preamble + preprocessed.as_str()).as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &full_source.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Note the different functions here: gl*Shader* instead of gl*Program*
        let mut successful: GLint = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut successful);

        let mut max_length: GLint = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut max_length);

        if successful == (gl::FALSE as GLint) {
            println!("Failed to compile {} shader for {}",
                     shader_stage_name(shader_stage),
                     name);
            for source in source_names {
                println!("The source includes: {}", source);
            }
        }

        if max_length > 1 {
            // The max_length includes the NULL character
            let mut info_log: Vec<u8> = vec![];
            info_log.reserve(max_length as usize - 1);
            info_log.set_len(max_length as usize - 1);
            gl::GetShaderInfoLog(shader,
                                 max_length,
                                 ptr::null_mut(),
                                 info_log.as_mut_ptr() as *mut GLchar);

            println!("Compilation log:\n{}", str::from_utf8(&info_log).unwrap());
        }

        if successful == (gl::FALSE as GLint) {
            return -1 as GLint;
        }
    }

    shader as GLint
}

/// Link the compiled shaders to produce a glsl program
///
/// Based on the C code at:
/// https://www.opengl.org/wiki/Shader_Compilation
///
/// name: The name of the shader
/// shaders: Collection of identifiers of compiled shader programs to link
fn link_program(name: &str, shaders: &Vec<GLuint>) -> GLint {
    let program;

    unsafe {
        program = gl::CreateProgram();
        for shader in shaders {
            gl::AttachShader(program, *shader);
        }
        gl::LinkProgram(program);

        // Note the different functions here: gl*Program* instead of gl*Shader*
        let mut successful: GLint = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut successful);

        let mut max_length: GLint = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut max_length);

        if successful == (gl::FALSE as GLint) {
            println!("Failed to link shader {}", name);
        }

        if max_length > 1 {
            // The max_length includes the NULL character
            let mut info_log: Vec<u8> = vec![];
            info_log.reserve(max_length as usize - 1);
            info_log.set_len(max_length as usize - 1);
            gl::GetProgramInfoLog(program,
                                  max_length,
                                  ptr::null_mut(),
                                  info_log.as_mut_ptr() as *mut GLchar);

            println!("Link log:\n{}", str::from_utf8(&info_log).unwrap());

            if successful == (gl::FALSE as GLint) {
                return -1 as GLint;
            }
        }
    }

    program as GLint
}
