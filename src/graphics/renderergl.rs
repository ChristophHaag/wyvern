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

use std::collections::HashMap;
use std::vec::Vec;
use std::ffi::CStr;
use std::sync::*;
use std::boxed::Box;
use std::any::Any;
use std::mem;
use std::ptr;
use std::os::raw::*;

use glfw;

use gl;
use gl::types::*;

use graphics::resources::ResourceManager;
use graphics::renderer::*;
use graphics::shaderglsl::ShaderGlsl;
use graphics::rendertarget::*;
use graphics::rendertargetgl::*;
use algebra::matrix::Mat4;
use algebra::vector::Vec3;

macro_rules! gl_check {
    () => {{
        let e = unsafe { gl::GetError() };
        if e != 0 {
            println!("gl::GetError returned {:?}", e);
        }
        debug_assert!(e == 0);
    }}
}

macro_rules! gl_check_no_assert {
    () => {{
        let e = unsafe { gl::GetError() };
        if e != 0 {
            println!("gl::GetError returned {:?}", e);
        }
    }}
}

pub struct UniformBufferDesc {
    pub size: usize,
    pub bytes: Vec<u8>,
    pub offsets: HashMap<&'static str, usize>,
    pub strides: HashMap<&'static str, usize>,
}

// Note: Fields are public for testing
pub struct RendererGl {
    vertex_array_type: VertexArrayType,

    resource_manager: Arc<Mutex<Box<ResourceManager>>>,

    uniform_buffer_descs: HashMap<&'static str, UniformBufferDesc>,
    uniform_buffer_natives: HashMap<&'static str, GLuint>,

    max_threads: usize,
    threaddata_arcs: Vec<Arc<Mutex<Box<ThreadData>>>>,
}

impl RendererGl {
    /// Initialise the OpenGL renderer
    ///
    /// debug_level: The debug level
    /// max_threads: The maximum number of threads
    /// window: The GLFW Window object
    /// resource_manager: The shader resource manager object
    /// threaddata_arcs: A vector of Arcs encapsulating ThreadData structures
    pub fn new(debug_level: u32,
               max_threads: usize,
               window: &mut glfw::Window,
               resource_manager: &Arc<Mutex<Box<ResourceManager>>>,
               threaddata_arcs: Vec<Arc<Mutex<Box<ThreadData>>>>)
               -> RendererGl {
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

        if debug_level > 0 {
            unsafe {
                let glven = CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8)
                    .to_string_lossy()
                    .into_owned();
                println!("GL_VENDOR: {}", glven);
                let glren = CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8)
                    .to_string_lossy()
                    .into_owned();
                println!("GL_RENDERER: {}", glren);
                let glver = CStr::from_ptr(gl::GetString(gl::VERSION) as *const i8)
                    .to_string_lossy()
                    .into_owned();
                println!("GL_VERSION: {}", glver);
                let slver =
                    CStr::from_ptr(gl::GetString(gl::SHADING_LANGUAGE_VERSION) as *const i8)
                        .to_string_lossy()
                        .into_owned();
                println!("GL_SHADING_LANGUAGE_VERSION: {}", slver);

                let mut val: GLint = -1;
                gl::GetIntegerv(gl::MAX_UNIFORM_BUFFER_BINDINGS, &mut val);
                println!("GL_MAX_UNIFORM_BUFFER_BINDINGS: {}", val);
                gl::GetIntegerv(gl::MAX_UNIFORM_BLOCK_SIZE, &mut val);
                println!("GL_MAX_UNIFORM_BLOCK_SIZE: {}", val);
                gl::GetIntegerv(gl::MAX_VERTEX_UNIFORM_BLOCKS, &mut val);
                println!("GL_MAX_VERTEX_UNIFORM_BLOCKS: {}", val);
                gl::GetIntegerv(gl::MAX_FRAGMENT_UNIFORM_BLOCKS, &mut val);
                println!("GL_MAX_FRAGMENT_UNIFORM_BLOCKS: {}", val);
                gl::GetIntegerv(gl::MAX_GEOMETRY_UNIFORM_BLOCKS, &mut val);
                println!("GL_MAX_GEOMETRY_UNIFORM_BLOCKS: {}", val);
            }
        }

        // Generate UBO handles for each uniform buffer
        // This is performed early so that the shaders can be created using these buffer handles
        let mut uniform_buffer_natives: HashMap<&'static str, GLuint> = HashMap::new();
        for block_spec in resource_manager.lock().unwrap().uniform_block_specs.iter() {
            let (block_name, _) = block_spec;

            let mut ubo_handle: GLuint = 0;
            unsafe {
                gl::GenBuffers(1, &mut ubo_handle);
            }

            uniform_buffer_natives.insert(block_name, ubo_handle);
        }

        RendererGl {
            max_threads: max_threads,
            threaddata_arcs: threaddata_arcs,

            uniform_buffer_descs: HashMap::new(),
            uniform_buffer_natives: uniform_buffer_natives,

            resource_manager: resource_manager.clone(),

            vertex_array_type: VertexArrayType::F3F3F3,
        }
    }

    /// Return the uniform buffer object handle for the named uniform buffer
    ///
    /// buffer_name: The name of the buffer to return the handle for
    pub fn get_uniform_buffer_handle(&self, buffer_name: &str) -> GLuint {
        self.uniform_buffer_natives[buffer_name]
    }

    /// Continue initialising OpenGL structures to the point where stuff can be rendered
    ///
    /// shaders: The shaders to continue setting up
    pub fn setup(&mut self, shaders: &HashMap<&'static str, &ShaderGlsl>) {
        let res_manager = self.resource_manager.lock().unwrap();

        for sh in shaders.iter() {
            let (shader_name, shader) = sh;
            let ref shader_spec = res_manager.shader_specs[shader_name];
            for uniform_block_name in shader_spec.uniform_block_names.iter() {
                let buffer_descriptor =
                    shader.get_uniform_buffer_descriptor_from_uniform_block(uniform_block_name);
                self.uniform_buffer_descs.insert(uniform_block_name, buffer_descriptor);
            }
        }
    }
}

impl Renderer for RendererGl {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut Any {
        self
    }

    /// Return the renderer type
    fn renderer_type(&self) -> RendererType {
        RendererType::RendererGl
    }

    /// Obtain an Arc for the ThreadData structure for the specified thread
    fn get_threaddata(&self, thr: usize) -> Arc<Mutex<Box<ThreadData>>> {
        self.threaddata_arcs[thr].clone()
    }

    /// Return the maximum number of threads allowed
    fn get_maxthreads(&self) -> usize {
        return self.max_threads;
    }

    /// Clear the depth buffer before starting rendering
    fn clear_depth_buffer(&self) {
        unsafe {
            gl::Clear(gl::DEPTH_BUFFER_BIT);
        }
    }

    /// Convert a renderer primitive type to an OpenGL primitive type
    fn primitive(&self, primitive_type: PrimitiveType) -> GLuint {
        match primitive_type {
            PrimitiveType::PrimitiveTriangles => gl::TRIANGLES,
            PrimitiveType::PrimitivePatches => gl::PATCHES,
        }
    }

    /// Set a integer in part of the memory put aside for the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to contain the new value
    /// uniform_name: The name of the uniform whose value should be set
    /// value: The value to set for the uniform
    fn set_uniform_buffer_int(&self, buffer_name: &str, uniform_name: &str, value: i32) {
        let ref buffer = self.uniform_buffer_descs[buffer_name];
        let offset = buffer.offsets[uniform_name];
        unsafe {
            let dst: *const u8 = buffer.bytes.as_ptr().offset(offset as isize);
            let dst_i32 = dst as *mut i32;
            *dst_i32 = value;
        }
    }

    /// Set a floating point in part of the memory put aside for the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to contain the new value
    /// uniform_name: The name of the uniform whose value should be set
    /// value: The value to set for the uniform
    fn set_uniform_buffer_float(&self, buffer_name: &str, uniform_name: &str, value: f32) {
        let ref buffer = self.uniform_buffer_descs[buffer_name];
        let offset = buffer.offsets[uniform_name];
        unsafe {
            let dst: *const u8 = buffer.bytes.as_ptr().offset(offset as isize);
            let dst_f32 = dst as *mut f32;
            *dst_f32 = value;
        }
    }

    /// Set a 3-component vector in part of the memory put aside for the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to contain the new value
    /// uniform_name: The name of the uniform whose value should be set
    /// value: The value to set for the uniform
    fn set_uniform_buffer_vec3(&self, buffer_name: &str, uniform_name: &str, value: &Vec3<f32>) {
        let ref buffer = self.uniform_buffer_descs[buffer_name];
        let offset = buffer.offsets[uniform_name];
        debug_assert!((offset + 3 * mem::size_of::<f32>()) <= buffer.size);
        unsafe {
            let dst: *const u8 = buffer.bytes.as_ptr().offset(offset as isize);
            let dst_f32 = dst as *mut f32;
            let src: *const f32 = mem::transmute(value);
            ptr::copy_nonoverlapping(src, dst_f32, 3);
        }
    }

    /// Set a 4x4-component matrix in part of the memory put aside for the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to contain the new value
    /// uniform_name: The name of the uniform whose value should be set
    /// matrix: The value to set for the uniform
    fn set_uniform_buffer_matrix(&self,
                                 buffer_name: &str,
                                 uniform_name: &str,
                                 matrix: &Mat4<f32>) {
        let ref buffer = self.uniform_buffer_descs[buffer_name];
        let offset = buffer.offsets[uniform_name];
        debug_assert!((offset + 16 * mem::size_of::<f32>()) <= buffer.size);
        unsafe {
            let dst: *const u8 = buffer.bytes.as_ptr().offset(offset as isize);
            let dst_f32 = dst as *mut f32;
            let src: *const f32 = mem::transmute(matrix);
            ptr::copy_nonoverlapping(src, dst_f32, 16);
        }
    }

    /// Set a floating point vector in part of the memory put aside for the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to contain the new value
    /// uniform_name: The name of the uniform whose value should be set
    /// vector: The vector to set for the uniform
    fn set_uniform_buffer_float_vector(&self,
                                       buffer_name: &str,
                                       uniform_name: &str,
                                       vector: &Vec<f32>) {
        let ref buffer = self.uniform_buffer_descs[buffer_name];
        let offset = buffer.offsets[uniform_name];
        let stride = buffer.strides[uniform_name];
        if stride == 0 || stride == 4 {
            debug_assert!((offset + vector.len() * mem::size_of::<f32>()) <= buffer.size);
            unsafe {
                let dst: *const u8 = buffer.bytes.as_ptr().offset(offset as isize);
                let dst_f32 = dst as *mut f32;
                let src: *const f32 = mem::transmute(vector.as_ptr());
                ptr::copy_nonoverlapping(src, dst_f32, vector.len());
            }
        } else {
            // This path requires observing the stride
            debug_assert!((offset + vector.len() * stride) <= buffer.size);
            unsafe {
                for i in 0..vector.len() {
                    let dst: *const u8 =
                        buffer.bytes.as_ptr().offset((i * stride + offset) as isize);
                    let dst_f32 = dst as *mut f32;
                    let src: *const f32 = mem::transmute(vector.as_ptr().offset(i as isize));
                    *dst_f32 = *src;
                }
            }
        }
    }

    /// Update the accumulated contents to the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to be configuring
    fn synchronise_uniform_buffer(&self, buffer_name: &str) {
        let ref buffer = self.uniform_buffer_descs[buffer_name];
        unsafe {
            gl::BindBuffer(gl::UNIFORM_BUFFER, self.uniform_buffer_natives[buffer_name]);
            let src: *const c_void = mem::transmute(buffer.bytes.as_ptr());
            gl::BufferSubData(gl::UNIFORM_BUFFER, 0, buffer.bytes.len() as isize, src);
            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
        }
    }

    /// Flip the back buffer to the front
    ///
    /// context: The GLFW context
    fn flip(&self, context: &mut glfw::Context) {
        (*context).swap_buffers();
    }

    /// Begin rendering a new frame
    fn begin_frame(&mut self) {
        // Nada
    }

    /// Terminate rendering a new frame
    fn end_frame(&mut self) {
        gl_check_no_assert!();
    }

    /// Initiate a render pass
    fn begin_pass(&mut self, shader_name: &'static str) {
        {
            let res_manager = self.resource_manager.lock().unwrap();
            self.vertex_array_type = res_manager.shader_specs[shader_name].vertex_array_type;
        }
    }

    /// Terminate a render pass
    fn end_pass(&mut self) {
        // Nada
    }

    /// Select the specified render target to render to
    ///
    /// num: The texture number to bind the render target texture to
    /// render_target: The render target to select
    fn select_render_target(&mut self, num: i32, render_target: &mut RenderTarget) {
        let target_gl = match render_target.as_any_mut().downcast_mut::<RenderTargetGl>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, target_gl.get_fbo());

            gl::ActiveTexture(match num {
                1 => gl::TEXTURE1,
                _ => gl::TEXTURE0,
            });

            gl::BindTexture(gl::TEXTURE_2D, target_gl.get_texture().texture_name);
        }
    }

    /// Select no render target
    fn deselect_render_target(&mut self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

impl RendererGl {
    /// As the main thread, flush the buffers returned by a thread as GL calls
    ///
    /// renderer_arc: Atomic reference counted lockable reference to the
    ///     renderer, only used when single_threaded
    /// thread_data: The thread data, potentially supplied by the worker thread
    pub fn flush<Rend: Renderer + ?Sized>(renderer_arc: Arc<Mutex<&mut Rend>>,
                                          thread_data: &ThreadData) {
        if thread_data.index == 0 {
            return;
        }

        // For the OpenGL renderer we must lock for the duration of the draw calls
        //
        let mut renderer = renderer_arc.lock().unwrap();

        let renderer_gl: &mut RendererGl = match renderer.as_any_mut()
            .downcast_mut::<RendererGl>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        let components_per_triangle =
            3 * VertexArrayType::components_per_vertex(renderer_gl.vertex_array_type);

        unsafe {
            gl::BufferData(gl::ARRAY_BUFFER,
                           (thread_data.index * components_per_triangle *
                            mem::size_of::<GLfloat>()) as GLsizeiptr,
                           mem::transmute(thread_data.data.as_ptr()),
                           gl::DYNAMIC_DRAW);

            gl::DrawArrays(renderer_gl.primitive(thread_data.primitive),
                           0, // Starting index
                           (thread_data.index * 3) as GLint);
        }
    }
}
