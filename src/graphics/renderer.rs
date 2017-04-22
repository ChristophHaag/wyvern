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
use crossbeam;

use glfw;

use graphics::renderergl::*;
use graphics::renderervk::*;
use graphics::rendertarget::*;
use algebra::matrix::Mat4;
use algebra::vector::*;

// Triangle buffer maximum size (in triangles)
pub const TRIANGLE_ARRAY_SIZE: usize = 512;

// Number of components per vertex: 3 dimensions by 3 attributes
pub const VERTEX_MAX_COMPONENTS: usize = 3 * 3;

// Number of components per triangle
// TODO: Use indexed arrays instead of three explicit vertices per triangle
pub const TRIANGLE_MAX_COMPONENTS: usize = VERTEX_MAX_COMPONENTS * 3;

// Number of individual components in a full component array
pub const TRIANGLE_MAX_TOTAL_COMPONENTS: usize = TRIANGLE_ARRAY_SIZE * TRIANGLE_MAX_COMPONENTS;

#[derive(PartialEq)]
pub enum RendererType {
    RendererGl,
    RendererVk,
}

#[derive(Clone, Copy, PartialEq)]
pub enum VertexArrayType {
    F3,
    F3F3F3,
    F3F3,
    F2F2,
}
pub const VERTEX_ARRAY_TYPE_BEGIN_RANGE: u32 = VertexArrayType::F3 as u32;
pub const VERTEX_ARRAY_TYPE_END_RANGE: u32 = VertexArrayType::F2F2 as u32;

#[derive(Clone, Copy)]
pub enum PrimitiveType {
    PrimitiveTriangles,
    PrimitivePatches,
}

impl VertexArrayType {
    pub fn components_per_vertex(ty: VertexArrayType) -> usize {
        match ty {
            VertexArrayType::F3 => 3,
            VertexArrayType::F3F3F3 => 9,
            VertexArrayType::F3F3 => 6,
            VertexArrayType::F2F2 => 4,
        }
    }

    pub fn from_u32(ty: u32) -> VertexArrayType {
        match ty {
            0 => VertexArrayType::F3,
            1 => VertexArrayType::F3F3F3,
            2 => VertexArrayType::F3F3,
            3 => VertexArrayType::F2F2,
            _ => panic!("Unexpected vertex array type"),
        }
    }
}

pub struct ThreadData {
    pub thr: i32,
    pub vertex_array_type: VertexArrayType,
    pub index: usize,
    pub primitive: PrimitiveType,
    pub finished: bool,

    pub data: Vec<f32>,
}

// ThreadData needs to be cloneable to permit sending from a worker GL rendering
// thread back to the master thread for flushing
impl Clone for ThreadData {
    fn clone(&self) -> ThreadData {
        let mut td = ThreadData::new(-1);

        td.thr = self.thr;
        td.vertex_array_type = self.vertex_array_type;
        td.index = self.index;
        td.primitive = self.primitive;
        td.finished = self.finished;

        td.data = self.data.iter().map(|x| *x).collect();

        td
    }
}

impl ThreadData {
    pub fn new(thr: i32) -> ThreadData {
        let mut td = ThreadData {
            thr: thr,
            vertex_array_type: VertexArrayType::F3F3F3,
            index: 0 as usize,
            primitive: PrimitiveType::PrimitiveTriangles,
            finished: false,

            data: vec![],
        };

        td.data.reserve(TRIANGLE_MAX_TOTAL_COMPONENTS);

        for _ in 0..TRIANGLE_MAX_TOTAL_COMPONENTS {
            td.data.push(0.0f32);
        }

        td
    }

    /// Add the specified raw triangle data to the thread data array, with no flush-check
    ///
    /// This is for the Normal-only case, with three components
    ///
    /// This is used primarily for the single-threaded case, but is
    /// also used for the multi-threaded case where the rest of the
    /// task is done by the caller.
    ///
    /// ni: Vector for normal at ith vertex of the triangle
    pub fn add_triangle_st_n3(&mut self, n1: &Vec3<f32>, n2: &Vec3<f32>, n3: &Vec3<f32>) {
        let i = self.index * VertexArrayType::components_per_vertex(self.vertex_array_type) * 3;

        self.data[i + 00] = n1.x;
        self.data[i + 01] = n1.y;
        self.data[i + 02] = n1.z;
        self.data[i + 03] = n2.x;
        self.data[i + 04] = n2.y;
        self.data[i + 05] = n2.z;
        self.data[i + 06] = n3.x;
        self.data[i + 07] = n3.y;
        self.data[i + 08] = n3.z;

        self.index += 1;
    }

    /// Add the specified raw triangle data to the thread data array, with no flush-check
    ///
    /// This is for the Vertex + Normal + Colour case, with three
    /// components each
    ///
    /// This is used primarily for the single-threaded case, but is
    /// also used for the multi-threaded case where the rest of the
    /// task is done by the caller.
    ///
    /// vi: Vector for ith vertex of the triangle
    /// ni: Vector for normal at ith vertex of the triangle
    /// ci: Shader colour inputs for normal at ith vertex of the triangle
    pub fn add_triangle_st_f3f3f3(&mut self,
                                  v1: &Vec3<f32>,
                                  n1: &Vec3<f32>,
                                  c1: &Vec3<f32>,
                                  v2: &Vec3<f32>,
                                  n2: &Vec3<f32>,
                                  c2: &Vec3<f32>,
                                  v3: &Vec3<f32>,
                                  n3: &Vec3<f32>,
                                  c3: &Vec3<f32>) {
        let i = self.index * VertexArrayType::components_per_vertex(self.vertex_array_type) * 3;

        self.data[i + 00] = v1.x;
        self.data[i + 01] = v1.y;
        self.data[i + 02] = v1.z;
        self.data[i + 03] = n1.x;
        self.data[i + 04] = n1.y;
        self.data[i + 05] = n1.z;
        self.data[i + 06] = c1.x;
        self.data[i + 07] = c1.y;
        self.data[i + 08] = c1.z;
        self.data[i + 09] = v2.x;
        self.data[i + 10] = v2.y;
        self.data[i + 11] = v2.z;
        self.data[i + 12] = n2.x;
        self.data[i + 13] = n2.y;
        self.data[i + 14] = n2.z;
        self.data[i + 15] = c2.x;
        self.data[i + 16] = c2.y;
        self.data[i + 17] = c2.z;
        self.data[i + 18] = v3.x;
        self.data[i + 19] = v3.y;
        self.data[i + 20] = v3.z;
        self.data[i + 21] = n3.x;
        self.data[i + 22] = n3.y;
        self.data[i + 23] = n3.z;
        self.data[i + 24] = c3.x;
        self.data[i + 25] = c3.y;
        self.data[i + 26] = c3.z;

        self.index += 1;
    }

    /// Add the specified raw triangle data to the thread data array, with no flush-check
    ///
    /// This is for the Vertex + Normal case, with three components
    /// each
    ///
    /// This is used primarily for the single-threaded case, but is
    /// also used for the multi-threaded case where the rest of the
    /// task is done by the caller.
    ///
    /// vi: Vector for ith vertex of the triangle
    /// ni: Vector for normal at ith vertex of the triangle
    pub fn add_triangle_st_f3f3(&mut self,
                                v1: &Vec3<f32>,
                                n1: &Vec3<f32>,
                                v2: &Vec3<f32>,
                                n2: &Vec3<f32>,
                                v3: &Vec3<f32>,
                                n3: &Vec3<f32>) {
        let i = self.index * VertexArrayType::components_per_vertex(self.vertex_array_type) * 3;

        self.data[i + 00] = v1.x;
        self.data[i + 01] = v1.y;
        self.data[i + 02] = v1.z;
        self.data[i + 03] = n1.x;
        self.data[i + 04] = n1.y;
        self.data[i + 05] = n1.z;
        self.data[i + 06] = v2.x;
        self.data[i + 07] = v2.y;
        self.data[i + 08] = v2.z;
        self.data[i + 09] = n2.x;
        self.data[i + 10] = n2.y;
        self.data[i + 11] = n2.z;
        self.data[i + 12] = v3.x;
        self.data[i + 13] = v3.y;
        self.data[i + 14] = v3.z;
        self.data[i + 15] = n3.x;
        self.data[i + 16] = n3.y;
        self.data[i + 17] = n3.z;

        self.index += 1;
    }

    /// Add the specified raw triangle data to the thread data array, with no flush-check
    ///
    /// This is for the Vertex + TexCoord case, with two components
    /// each
    ///
    /// This is used primarily for the single-threaded case, but is
    /// also used for the multi-threaded case where the rest of the
    /// task is done by the caller.
    ///
    /// vi: Vector for ith vertex of the triangle
    /// ni: Vector for texture coordinates at ith vertex of the triangle
    pub fn add_triangle_st_f2f2(&mut self,
                                v1: &Vec2<f32>,
                                t1: &Vec2<f32>,
                                v2: &Vec2<f32>,
                                t2: &Vec2<f32>,
                                v3: &Vec2<f32>,
                                t3: &Vec2<f32>) {
        let i = self.index * VertexArrayType::components_per_vertex(self.vertex_array_type) * 3;

        self.data[i + 00] = v1.x;
        self.data[i + 01] = v1.y;
        self.data[i + 02] = t1.x;
        self.data[i + 03] = t1.y;
        self.data[i + 04] = v2.x;
        self.data[i + 05] = v2.y;
        self.data[i + 06] = t2.x;
        self.data[i + 07] = t2.y;
        self.data[i + 08] = v3.x;
        self.data[i + 09] = v3.y;
        self.data[i + 10] = t3.x;
        self.data[i + 11] = t3.y;

        self.index += 1;
    }

    /// Add the specified raw triangle data to the thread data array
    ///
    /// This is for the Normal-only case, with three components
    ///
    /// single_threaded: True when only using one thread, false when
    ///     multi-threading
    /// renderer_arc: Atomic reference counted lockable reference to the
    ///     renderer, only used when single_threaded
    /// thread_data: Container for vertex data etc for this thread
    /// datatx: A reference to the mpsc sender object for sending the computed
    ///     vertex data to the main thread to be flushed as GL commands
    /// backrx: A reference to the mpsc receiver object for the main thread
    ///     to communicate back that the GL commands have been executed
    /// vi: Vector for ith vertex of the triangle
    /// ni: Vector for normal at ith vertex of the triangle
    /// ci: Shader colour inputs for normal at first vertex of the triangle
    pub fn add_triangle_n3<Rend: Renderer + ?Sized>(&mut self,
                                                    single_threaded: bool,
                                                    renderer_arc: Arc<Mutex<&mut Rend>>,
                                                    datatx: &mpsc::Sender<ThreadData>,
                                                    backrx: &mpsc::Receiver<i32>,
                                                    n1: &Vec3<f32>,
                                                    n2: &Vec3<f32>,
                                                    n3: &Vec3<f32>) {
        self.check_flush(false, // force
                         single_threaded,
                         renderer_arc,
                         datatx,
                         backrx);
        self.add_triangle_st_n3(n1, n2, n3);
    }

    /// Add the specified raw triangle data to the thread data array
    ///
    /// This is for the Vertex + Normal + Colour case, with three
    /// components each
    ///
    /// single_threaded: True when only using one thread, false when
    ///     multi-threading
    /// renderer_arc: Atomic reference counted lockable reference to the
    ///     renderer, only used when single_threaded
    /// thread_data: Container for vertex data etc for this thread
    /// datatx: A reference to the mpsc sender object for sending the computed
    ///     vertex data to the main thread to be flushed as GL commands
    /// backrx: A reference to the mpsc receiver object for the main thread
    ///     to communicate back that the GL commands have been executed
    /// vi: Vector for ith vertex of the triangle
    /// ni: Vector for normal at ith vertex of the triangle
    /// ci: Shader colour inputs for normal at first vertex of the triangle
    pub fn add_triangle_f3f3f3<Rend: Renderer + ?Sized>(&mut self,
                                                        single_threaded: bool,
                                                        renderer_arc: Arc<Mutex<&mut Rend>>,
                                                        datatx: &mpsc::Sender<ThreadData>,
                                                        backrx: &mpsc::Receiver<i32>,
                                                        v1: &Vec3<f32>,
                                                        n1: &Vec3<f32>,
                                                        c1: &Vec3<f32>,
                                                        v2: &Vec3<f32>,
                                                        n2: &Vec3<f32>,
                                                        c2: &Vec3<f32>,
                                                        v3: &Vec3<f32>,
                                                        n3: &Vec3<f32>,
                                                        c3: &Vec3<f32>) {
        self.check_flush(false, // force
                         single_threaded,
                         renderer_arc,
                         datatx,
                         backrx);
        self.add_triangle_st_f3f3f3(v1, n1, c1, v2, n2, c2, v3, n3, c3);
    }

    /// Add the specified raw triangle data to the thread data array
    ///
    /// This is for the Vertex + Normal case, with three components each
    ///
    /// single_threaded: True when only using one thread, false when
    ///     multi-threading
    /// renderer_arc: Atomic reference counted lockable reference to the
    ///     renderer, only used when single_threaded
    /// thread_data: Container for vertex data etc for this thread
    /// datatx: A reference to the mpsc sender object for sending the computed
    ///     vertex data to the main thread to be flushed as GL commands
    /// backrx: A reference to the mpsc receiver object for the main thread
    ///     to communicate back that the GL commands have been executed
    /// vi: Vector for ith vertex of the triangle
    /// ni: Vector for normal at ith vertex of the triangle
    pub fn add_triangle_f3f3<Rend: Renderer + ?Sized>(&mut self,
                                                      single_threaded: bool,
                                                      renderer_arc: Arc<Mutex<&mut Rend>>,
                                                      datatx: &mpsc::Sender<ThreadData>,
                                                      backrx: &mpsc::Receiver<i32>,
                                                      v1: &Vec3<f32>,
                                                      n1: &Vec3<f32>,
                                                      v2: &Vec3<f32>,
                                                      n2: &Vec3<f32>,
                                                      v3: &Vec3<f32>,
                                                      n3: &Vec3<f32>) {
        self.check_flush(false, // force
                         single_threaded,
                         renderer_arc,
                         datatx,
                         backrx);
        self.add_triangle_st_f3f3(v1, n1, v2, n2, v3, n3);
    }

    /// This checks whether a flush is required and actions it when necessary
    ///
    /// This is run from the main thread when single-threaded rendering.  Because the renderer's
    /// flush method is designed to lock as required, it is necessary to construct a new
    /// Arc<Mutex<>> here.
    ///
    /// force: true if the flush should be forced, and false if the buffer should only be
    ///     flushed when full
    pub fn check_flush_st<Rend: Renderer + ?Sized>(&mut self, force: bool, renderer: &mut Rend) {
        if force || self.index == TRIANGLE_ARRAY_SIZE {
            // We can flush directly from the main thread
            //
            match renderer.renderer_type() {
                RendererType::RendererGl => RendererGl::flush(Arc::new(Mutex::new(renderer)), self),
                RendererType::RendererVk => RendererVk::flush(Arc::new(Mutex::new(renderer)), self),
            }

            // Reset the triangle index
            self.index = 0;
        }
    }

    /// This checks whether a flush is required and actions it when necessary
    ///
    /// Note: Only call this version from a worker thread!
    ///
    /// force: true if the flush should be forced, and false if the
    ///     buffer should only be flushed when full
    /// single_threaded: True when only using one thread, false when
    ///     multi-threading
    /// renderer_arc: Atomic reference counted lockable reference to the
    ///     renderer, only used when single_threaded
    /// tx: A reference to the mpsc sender object for requesting of the main
    ///     thread that it flushes the data as GL commands
    /// backrx: A reference to the mpsc receiver object for the main thread
    ///     indicating flush completion
    pub fn check_flush<Rend: Renderer + ?Sized>(&mut self,
                                                force: bool,
                                                single_threaded: bool,
                                                renderer_arc: Arc<Mutex<&mut Rend>>,
                                                tx: &mpsc::Sender<ThreadData>,
                                                backrx: &mpsc::Receiver<i32>) {
        if force || self.index == TRIANGLE_ARRAY_SIZE {
            let renderer_type;
            {
                let renderer = renderer_arc.lock().unwrap();
                renderer_type = renderer.renderer_type();
            }

            match renderer_type {
                RendererType::RendererGl => {
                    if single_threaded {
                        // We can flush directly from the main thread
                        RendererGl::flush(renderer_arc.clone(), self);
                    } else {
                        // TODO: Is it possible to avoid transferring all of the data every time?
                        let mut td: ThreadData = self.clone();
                        td.finished = force;

                        // Send the data to the main thread
                        tx.send(td).unwrap();

                        // Wait for and discard the message from the main thread indicating
                        // that the rendering calls are complete
                        let _ = backrx.recv();
                    }
                }
                RendererType::RendererVk => RendererVk::flush(renderer_arc.clone(), self),
            }

            // Reset the triangle index
            self.index = 0;
        }
    }
}

/// Types must implement this trait in order to be able to use the MT harness
pub trait WorkerThread {
    /// Perform one thread's worth of work for rendering
    fn render_thread<Rend: Renderer + ?Sized>(&self,
                                              renderer_arc: Arc<Mutex<&mut Rend>>,
                                              threaddata_arc: Arc<Mutex<Box<ThreadData>>>,
                                              thr: usize,
                                              max_threads: usize,
                                              datatx: &mpsc::Sender<ThreadData>,
                                              backrx: &mpsc::Receiver<i32>);
}

/// Multi-threaded render harness
///
/// object: The object performing the rendering
/// renderer: A reference to the renderer object to use
#[allow(dead_code)]
pub fn mt_render_harness<Object: WorkerThread + Send + Sync, Rend: Renderer + Send + Sync + ?Sized>(object: &Object,
                                                                                                    renderer: &mut Rend) {
    let renderer_type = renderer.renderer_type();

    let max_threads = renderer.get_maxthreads();
    let renderer_arc = Arc::new(Mutex::new(renderer));

    if max_threads == 1 {
        // Single-threaded path

        let (datatx, _) = mpsc::channel::<ThreadData>();
        let (_, backrx) = mpsc::channel::<i32>();

        let threaddata_arc;
        {
            let renderer = renderer_arc.lock().unwrap();
            threaddata_arc = renderer.get_threaddata(0);
        }

        object.render_thread(renderer_arc.clone(),
                             threaddata_arc,
                             0, // thread
                             1, // max_threads
                             &datatx,
                             &backrx);
    } else {
        // Multi-threaded path

        // Code to dump the main thread handle on Windows for debugging:
        //
        // extern "C" {
        //     fn GetCurrentThreadId() -> u32;
        // }
        // unsafe {
        //     println!("{}", GetCurrentThreadId());
        // }

        // TODO: Is there any way to avoid avoid spawning *new* threads all the time?
        crossbeam::scope(|scope| {
            let (datatx, datarx) = mpsc::channel::<ThreadData>();
            let mut backtxs: Vec<mpsc::Sender<i32>> = vec![];

            for thr in 0..max_threads {
                let renderer_arc = renderer_arc.clone();
                let threaddata_arc;
                {
                    let renderer = renderer_arc.lock().unwrap();
                    threaddata_arc = renderer.get_threaddata(thr);
                }
                let datatx = datatx.clone();
                let (backtx, backrx) = mpsc::channel::<i32>();
                backtxs.push(backtx);

                scope.spawn(move || {
                    object.render_thread(renderer_arc,
                                         threaddata_arc,
                                         thr,
                                         max_threads,
                                         &datatx,
                                         &backrx)
                });
            }

            // This marshalls the transfer of render data from the worker threads to the master
            // thread so that the OpenGL renderer can emit draw calls.  This is followed by the
            // notification to the worker that it can continue.  For Vulkan this is a NOP, as
            // the worker threads submit their computed command buffers to the graphics queue
            // directly.
            //
            // Only the OpenGL renderer needs to receive the thread data and renderer
            // For Vulkan, just wait for all the threads to join
            //
            if renderer_type == RendererType::RendererGl {
                let mut threads_finished = 0;
                while threads_finished < max_threads {
                    let thread_data = datarx.recv().unwrap();

                    // Flush the data calculated by the worker thread as draw calls
                    RendererGl::flush(renderer_arc.clone(), &thread_data);

                    // If this thread indicated that it was complete, update our tally
                    //
                    if thread_data.finished {
                        threads_finished += 1;
                    }

                    // Inform the worker thread that its data has been flushed
                    let _ = backtxs[thread_data.thr as usize].send(0);
                }
            }
        });
    }
}

pub trait Renderer: Send + Sync {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any;
    fn as_any_mut(&mut self) -> &mut Any;

    /// Return the renderer type
    /// TODO: Replace uses with uses of as_any
    fn renderer_type(&self) -> RendererType;

    /// Obtain an Arc for the ThreadData structure for the specified thread
    fn get_threaddata(&self, thr: usize) -> Arc<Mutex<Box<ThreadData>>>;

    /// Return the maximum number of threads
    fn get_maxthreads(&self) -> usize;

    /// Clear the depth buffer before starting rendering
    fn clear_depth_buffer(&self);

    /// This converts the primitive type that will be rendered to the renderer's intrinsic type
    fn primitive(&self, primitive_type: PrimitiveType) -> u32;

    /// Uniform buffer configuration
    fn set_uniform_buffer_int(&self, buffer_name: &str, uniform_name: &str, value: i32);
    fn set_uniform_buffer_float(&self, buffer_name: &str, uniform_name: &str, value: f32);
    fn set_uniform_buffer_vec3(&self, buffer_name: &str, uniform_name: &str, value: &Vec3<f32>);
    fn set_uniform_buffer_matrix(&self, buffer_name: &str, uniform_name: &str, matrix: &Mat4<f32>);
    fn set_uniform_buffer_float_vector(&self, buffer_name: &str, uniform_name: &str, vector: &Vec<f32>);
    fn synchronise_uniform_buffer(&self, buffer_name: &str);

    /// Flip the back buffer to the front
    fn flip(&self, window: &mut glfw::Context);

    /// Begin rendering a new frame
    fn begin_frame(&mut self);

    /// Terminate rendering a new frame
    fn end_frame(&mut self);

    /// Initiate a render pass
    fn begin_pass(&mut self, shader_name: &'static str);

    /// Terminate a render pass
    fn end_pass(&mut self);

    /// Select the specified render target to render to
    ///
    /// num: The texture number to bind the render target texture to
    /// render_target: The render target to select
    fn select_render_target(&mut self, num: i32, render_target: &mut RenderTarget);

    /// Select no render target
    fn deselect_render_target(&mut self);
}
