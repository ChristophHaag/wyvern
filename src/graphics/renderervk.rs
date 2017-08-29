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
use std::sync::*;
use std::str;
use std::ffi::*;
use std::os::raw::*;
use std::ptr;
use std::mem;
use std::any::Any;

use semver::Version;

use vk::vulkan::*;
use vk::manual::*;

use glfw;
use glfw::*;

use graphics::renderer::*;
use graphics::shader::*;
use graphics::shaderspirv::*;
use graphics::texture::*;
use graphics::texturevk::*;
use graphics::rendertarget::*;
use graphics::rendertargetvk::*;
use graphics::resources::*;
use algebra::matrix::Mat4;
use algebra::vector::*;

// TODO: Improve the performance by improving the synchronisation

///
/// Note: allow(dead_code) is used where objects need to be kept alive for as long as
/// the owner, but where the field is otherwise unused.
///

macro_rules! matches(
    ($e:expr, $p:pat) => (
        match $e {
            $p => true,
            _ => false
        }
    )
);

macro_rules! check_result(
    ($n:expr, $e:expr) => {
        let res = $e;
        if !matches!(res, VkResult::VK_SUCCESS) {
            println!("Unexpected result from {}: {}", $n, res);
            panic!("Vulkan API call failed");
        }
    }
);

// Why is everything back-to-front here?  Because Rust's destruction order
// is the opposite of C++.  In fact, Rust does not define the destruction
// order of fields, and this is just the order that the compiler implements.
//
// The drop_in_place compiler intrisic can be used to invoke a destructor,
// but the lifetime of an object is absolutely set in stone, and the destructor
// will be invoked again at the end of the object's life.  Unlike the very
// specific lifetime semantics of an object, absolutely no guarantee is
// placed on the relative lifetimes of objects within the same compound
// (struct, tuple, array, etc.)
pub struct RendererVk {
    current_pass_identifier: u32,
    current_depth_target: Option<VkImage>,
    current_render_target: Option<VkFramebuffer>,
    vertex_array_type: VertexArrayType,
    shader_name: &'static str,
    image_index: usize,

    prepresent_command_buffers: Vec<RendererVkCommandBuffer>,
    cleardepth_command_buffers: Vec<RendererVkCommandBuffer>,
    command_buffers: Vec<Vec<RendererVkCommandBuffer>>,
    command_pools: Vec<RendererVkCommandPool>,
    render_pipelines: HashMap<&'static str, RendererVkPipeline>,
    framebuffers: Vec<RendererVkFramebuffer>,
    pub render_passes: Vec<RendererVkRenderPass>,
    uniform_buffers: HashMap<&'static str, RendererVkUniformBuffer>,
    descriptor_sets: HashMap<&'static str, RendererVkDescriptorSet>,
    pipeline_layouts: HashMap<&'static str, RendererVkPipelineLayout>,
    descriptor_set_layouts: HashMap<&'static str, RendererVkDescriptorSetLayouts>,
    descriptor_pools: Vec<RendererVkDescriptorPool>,

    vertex_buffer_index: Vec<Vec<Vec<i32>>>,
    vertex_buffer: Vec<Vec<Vec<Vec<RendererVkVertexBuffer>>>>,

    resource_manager: Arc<Mutex<Box<ResourceManager>>>,

    // Prior to this point, the members are set up once the requisite data is made available to
    // the renderer.  The following items are created by the constructor.
    pub aux_command_pool: RendererVkCommandPool,

    render_finished_semaphore: RendererVkSemaphore,
    image_available_semaphore: RendererVkSemaphore,

    swapchain: RendererVkSwapchain,
    surface: RendererVkSurface,
    pub device: RendererVkDevice,
    queue_families: RendererVkQueueFamilies,
    pub physical_device: RendererVkPhysicalDevice,
    #[allow(dead_code)]
    debug_callback: Option<RendererVkDebugCallback>,
    #[allow(dead_code)]
    instance: RendererVkInstance,

    max_threads: usize,
    threaddata_arcs: Vec<Arc<Mutex<Box<ThreadData>>>>,
}
unsafe impl Send for RendererVk {}
unsafe impl Sync for RendererVk {}

impl RendererVk {
    /// Initialise Vulkan to the point where we have a device and a swapchain
    ///
    /// TODO: Add proper error handling
    ///
    /// max_threads: The maximum number of rendering threads
    /// application_name: String containing the application name
    /// application_version: String containing the application version
    /// engine_version: String containing the engine version
    /// debug_level: The debug level
    /// vk_debug_mask: A bitmask of Vulkan log messages
    /// glfw: The main GLFW object
    /// window: The GLFW Window object
    /// resource_manager: The shader resource manager
    /// threaddata_arcs: A vector of Arcs encapsulating ThreadData structures
    pub fn new(application_name: &str,
               application_version: &str,
               engine_version: &str,
               max_threads: usize,
               debug_level: u32,
               vk_debug_mask: u32,
               glfw: &mut Glfw,
               window: &mut Window,
               resource_manager: &Arc<Mutex<Box<ResourceManager>>>,
               threaddata_arcs: Vec<Arc<Mutex<Box<ThreadData>>>>)
               -> RendererVk {
        // Build collections of layer and extension names required by the application
        let mut instance_layers: Vec<String> = vec![];
        let mut instance_extensions: Vec<String> = vec![];
        let mut device_extensions: Vec<String> = vec![];

        // Check GLFW for its requirements
        if glfw.vulkan_supported() {
            // List required Vulkan instance extension
            let reqd_vk_extensions = glfw.get_required_instance_extensions().unwrap_or(vec![]);
            for ext in reqd_vk_extensions.iter() {
                instance_extensions.push(ext.clone());
            }
        } else {
            panic!("Vulkan is not supported");
        }

        // Now add explicit layers and extensions as desired
        instance_layers.push("VK_LAYER_LUNARG_monitor".to_string());
        if vk_debug_mask != 0 {
            instance_layers.push("VK_LAYER_LUNARG_standard_validation".to_string());
            if debug_level > 1 {
                instance_layers.push("VK_LAYER_LUNARG_api_dump".to_string());
            }
            instance_extensions.push("VK_EXT_debug_report".to_string());
        }
        device_extensions.push("VK_KHR_swapchain".to_string());
        if debug_level > 0 {
            println!("Requested instance layers {:?}", instance_layers);
            println!("Requested instance extensions {:?}", instance_extensions);
            println!("Requested device extensions {:?}", device_extensions);
        }

        // Now start creating all the Vulkan objects required
        let instance = RendererVkInstance::new(application_name,
                                               application_version,
                                               engine_version,
                                               &instance_layers,
                                               &instance_extensions);

        let debug_callback;
        if vk_debug_mask != 0 {
            debug_callback = Some(RendererVkDebugCallback::new(&instance, vk_debug_mask));
        } else {
            debug_callback = None;
        }

        let physical_device = RendererVkPhysicalDevice::new(&instance, debug_level);

        let queue_families = RendererVkQueueFamilies::new(&physical_device);

        let surface = RendererVkSurface::new(window,
                                             &instance,
                                             &physical_device,
                                             queue_families.index,
                                             debug_level);

        let device = RendererVkDevice::new(&physical_device,
                                           queue_families.index,
                                           &instance_layers,
                                           &device_extensions);

        let swapchain = RendererVkSwapchain::new(&device, &surface, 2 /* swapchain image count */);

        let image_available_semaphore = RendererVkSemaphore::new(&device);
        let render_finished_semaphore = RendererVkSemaphore::new(&device);

        let aux_command_pool = RendererVkCommandPool::new(&device, queue_families.index);

        let mut vertex_buffer = vec![];
        let mut vertex_buffer_index = vec![];

        for i in 0..swapchain.image_count {
            vertex_buffer.push(vec![]);
            vertex_buffer_index.push(vec![]);

            for ty in VERTEX_ARRAY_TYPE_BEGIN_RANGE..VERTEX_ARRAY_TYPE_END_RANGE + 1 {
                vertex_buffer[i as usize].push(vec![]);
                vertex_buffer_index[i as usize].push(vec![]);

                for _ in 0..max_threads {
                    vertex_buffer[i as usize][ty as usize].push(vec![]);
                    vertex_buffer_index[i as usize][ty as usize].push(-1);
                }
            }
        }

        // Now construct the RendererVk object containing all of these good things
        RendererVk {
            max_threads: max_threads,
            threaddata_arcs: threaddata_arcs,

            instance: instance,
            debug_callback: debug_callback,
            physical_device: physical_device,
            queue_families: queue_families,
            device: device,
            surface: surface,
            swapchain: swapchain,

            image_available_semaphore: image_available_semaphore,
            render_finished_semaphore: render_finished_semaphore,

            aux_command_pool: aux_command_pool,

            resource_manager: resource_manager.clone(),

            vertex_buffer: vertex_buffer,
            vertex_buffer_index: vertex_buffer_index,

            descriptor_pools: vec![],
            descriptor_set_layouts: HashMap::new(),
            pipeline_layouts: HashMap::new(),
            descriptor_sets: HashMap::new(),
            uniform_buffers: HashMap::new(),
            render_passes: vec![],
            framebuffers: vec![],
            render_pipelines: HashMap::new(),
            command_pools: vec![],
            command_buffers: vec![],
            cleardepth_command_buffers: vec![],
            prepresent_command_buffers: vec![],

            image_index: u32::max_value() as usize,
            shader_name: "",
            vertex_array_type: VertexArrayType::F3F3F3,
            current_render_target: None,
            current_depth_target: None,
            current_pass_identifier: u32::max_value(),
        }
    }

    /// Return the raw Vulkan device
    pub fn get_raw_device(&self) -> VkDevice {
        self.device.raw
    }

    /// Continue initialising Vulkan structures to the point where stuff can be rendered
    ///
    /// The goal is that the specifics of the renderer setup go in here or in the trait
    /// implementation, and the general Vulkan code elsewhere.  At some point in the
    /// future the specific and the general will be separated entirely.
    ///
    /// shaders: The shaders to continue setting up
    /// textures: The textures to continue setting up
    pub fn setup(&mut self, shaders: &HashMap<&'static str, &ShaderSpirv>, textures: &HashMap<&'static str, &TextureVk>) {
        let res_manager = self.resource_manager.lock().unwrap();

        // Create initial vertex buffers
        //
        for i in 0..self.swapchain.image_count {
            for ty in VERTEX_ARRAY_TYPE_BEGIN_RANGE..VERTEX_ARRAY_TYPE_END_RANGE + 1 {
                for thr in 0..self.max_threads {
                    self.vertex_buffer[i as usize][ty as usize][thr]
                        .push(RendererVkVertexBuffer::new(&self.device,
                                                          &self.physical_device,
                                                          VertexArrayType::from_u32(ty)));
                }
            }
        }

        // Generate a uniform buffer for each uniform block
        //
        for block in res_manager.uniform_block_specs.iter() {
            let (block_name, block_spec) = block;
            self.uniform_buffers.insert(block_name,
                                        RendererVkUniformBuffer::new(&self.device, &self.physical_device, block_spec));
        }

        // Create a descriptor pool with space for necessary resources
        //
        // This is all very conservative (e.g. number of uniform buffers = num shaders * num blocks)
        // until a better method can be arranged after the renderer is fully operational
        let num_uniform_blocks = res_manager.uniform_block_specs.len();
        let num_shaders = res_manager.shader_specs.len();
        self.descriptor_pools.push(RendererVkDescriptorPool::new(&self.device,
                                                                 num_shaders * num_uniform_blocks, // Maximum uniform buffers
                                                                 num_shaders, // Maximum combined image samplers
                                                                 num_shaders * num_uniform_blocks /* Maximum desc sets */));

        // Generate a descriptor set layout and a descriptor for each shader
        //
        for (shader_name, shader_spec) in res_manager.shader_specs.iter() {
            let descriptor_set_layouts = RendererVkDescriptorSetLayouts::new(&self.device, &res_manager, &shader_spec);

            let pipeline_layout = RendererVkPipelineLayout::new(&self.device, &descriptor_set_layouts);

            let descriptor_set = RendererVkDescriptorSet::new(&self.device,
                                                              &self.descriptor_pools[0],
                                                              &descriptor_set_layouts,
                                                              &shader_spec,
                                                              &self.uniform_buffers,
                                                              textures);

            self.descriptor_set_layouts.insert(shader_name, descriptor_set_layouts);
            self.pipeline_layouts.insert(shader_name, pipeline_layout);
            self.descriptor_sets.insert(shader_name, descriptor_set);
        }

        // The render pass with identifier zero is reserved for the final render to the swapchain.
        // So the final shader that performs the post-processing will specify zero as the pass identifier.
        // The other shaders will specify other render pass identifiers used for offscreen rendering.
        //
        let depth_format = self.choose_depth_format();
        self.render_passes.push(RendererVkRenderPass::new(&self.device,
                                                          self.surface.format.format,
                                                          None /* Depth format */));
        self.render_passes.push(RendererVkRenderPass::new(&self.device,
                                                          VkFormat::VK_FORMAT_R32G32B32A32_SFLOAT,
                                                          Some(depth_format)));

        // Create a framebuffer for each swapchain image
        //
        for i in 0..self.swapchain.image_count {
            self.framebuffers.push(RendererVkFramebuffer::new(&self.device,
                                                              self.swapchain.views[i as usize],
                                                              None, // Depth image view
                                                              &self.render_passes[0],
                                                              self.surface.capabilities.currentExtent.width,
                                                              self.surface.capabilities.currentExtent.height));
        }

        // Create a render pipeline for each shader
        //
        for (shader_name, _) in shaders {
            let ref shader_spec = res_manager.shader_specs[shader_name];

            self.render_pipelines
                .insert(shader_name,
                        RendererVkPipeline::new(&self.device,
                                                &self.render_passes[shader_spec.pass_identifier as usize],
                                                &shader_spec,
                                                shaders[shader_name],
                                                &self.pipeline_layouts[shader_name],
                                                self.surface.capabilities.currentExtent.width,
                                                self.surface.capabilities.currentExtent.height));
        }

        // Create a command pool for each thread
        //
        for _ in 0..self.max_threads {
            self.command_pools.push(RendererVkCommandPool::new(&self.device, self.queue_families.index));
        }

        // Create command buffers for each swapchain image for each thread
        //
        for i in 0..self.swapchain.image_count {
            self.command_buffers.push(vec![]);
            for thr in 0..self.max_threads {
                self.command_buffers[i as usize].push(RendererVkCommandBuffer::new(&self.device,
                                                                                   &self.command_pools[thr],
                                                                                   true /* primary */));
            }

        }

        // Create additional command buffers per swap chain image
        //
        for _ in 0..self.swapchain.image_count {
            self.cleardepth_command_buffers.push(RendererVkCommandBuffer::new(&self.device,
                                                                              &self.aux_command_pool,
                                                                              true /* primary */));
            self.prepresent_command_buffers.push(RendererVkCommandBuffer::new(&self.device,
                                                                              &self.aux_command_pool,
                                                                              true /* primary */));
        }

    }

    /// Find an available memory that suits the requirements
    ///
    ///
    pub fn find_suitable_memory(physical_device: &RendererVkPhysicalDevice,
                                type_filter: u32,
                                properties: VkMemoryPropertyFlags)
                                -> u32 {
        let mut memory_properties = VkPhysicalDeviceMemoryProperties::default();
        unsafe {
            vkGetPhysicalDeviceMemoryProperties(physical_device.raw, &mut memory_properties);
        }

        for i in 0..memory_properties.memoryTypeCount {
            if ((type_filter as u32) & (1 << i) != 0) &&
               ((memory_properties.memoryTypes[i as usize].propertyFlags as u32) & (properties as u32)) == (properties as u32) {
                return i;
            }
        }

        // Could not find a suitable memory type
        u32::max_value()
    }

    /// Choose the first format of a list that fits the desired features
    ///
    ///
    fn choose_supported_format(&self,
                               candidates: &Vec<VkFormat>,
                               tiling: VkImageTiling,
                               features: VkFormatFeatureFlags)
                               -> VkFormat {
        for format in candidates {
            let mut props = VkFormatProperties::default();
            unsafe {
                vkGetPhysicalDeviceFormatProperties(self.physical_device.raw, *format, &mut props);
            }

            if tiling as u32 == VkImageTiling::VK_IMAGE_TILING_LINEAR as u32 &&
               (props.linearTilingFeatures as u32 & features as u32) == features as u32 {
                return *format;
            } else if tiling as u32 == VkImageTiling::VK_IMAGE_TILING_OPTIMAL as u32 &&
                      (props.optimalTilingFeatures as u32 & features as u32) == features as u32 {
                return *format;
            }
        }

        panic!("Failed to find supported format");
    }

    /// Choose the most desirable depth format that is available
    ///
    ///
    pub fn choose_depth_format(&self) -> VkFormat {
        self.choose_supported_format(
            &vec![VkFormat::VK_FORMAT_D32_SFLOAT, VkFormat::VK_FORMAT_D32_SFLOAT_S8_UINT, VkFormat::VK_FORMAT_D24_UNORM_S8_UINT],
            VkImageTiling::VK_IMAGE_TILING_OPTIMAL,
            VkFormatFeatureFlagBits::VK_FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT as VkFormatFeatureFlags
        )
    }
}

impl Drop for RendererVk {
    fn drop(&mut self) {
        unsafe {
            check_result!("vkQueueWaitIdle",
                          vkQueueWaitIdle(self.device.graphics_queue));

            check_result!("vkDeviceWaitIdle", vkDeviceWaitIdle(self.device.raw));
        }

        self.descriptor_sets.clear();
        self.pipeline_layouts.clear();
        self.descriptor_set_layouts.clear();
        self.descriptor_pools.clear();
        self.uniform_buffers.clear();

        self.render_pipelines.clear();
        self.command_pools.clear();
        self.framebuffers.clear();
        self.render_passes.clear();
    }
}

pub struct RendererVkInstance {
    raw: VkInstance,
}

impl RendererVkInstance {
    /// Create a Vulkan instance
    ///
    ///
    fn new(application_name: &str,
           application_version: &str,
           engine_version: &str,
           instance_layers: &Vec<String>,
           instance_extensions: &Vec<String>)
           -> RendererVkInstance {
        let app_name = CString::new(application_name.to_owned()).unwrap().into_raw();
        let app_version = Version::parse(application_version).unwrap();
        let eng_version = Version::parse(engine_version).unwrap();
        let application_info = VkApplicationInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_APPLICATION_INFO,
            apiVersion: VK_MAKE_VERSION(1, 0, 0),
            applicationVersion: VK_MAKE_VERSION(app_version.major as u32,
                                                app_version.minor as u32,
                                                app_version.patch as u32),
            engineVersion: VK_MAKE_VERSION(eng_version.major as u32,
                                           eng_version.minor as u32,
                                           eng_version.patch as u32),
            pApplicationName: app_name,
            pEngineName: app_name,
            pNext: ptr::null(),
        };

        let il: Vec<*mut c_char> = instance_layers.iter().map(|x| CString::new(x.to_owned()).unwrap().into_raw()).collect();
        let ie: Vec<*mut c_char> = instance_extensions.iter().map(|x| CString::new(x.to_owned()).unwrap().into_raw()).collect();

        let instance_create_info = VkInstanceCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            pApplicationInfo: &application_info,
            enabledLayerCount: instance_layers.len() as u32,
            ppEnabledLayerNames: il.as_ptr() as *mut _,
            enabledExtensionCount: instance_extensions.len() as u32,
            ppEnabledExtensionNames: ie.as_ptr() as *mut _,
            flags: 0,
            pNext: ptr::null(),
        };

        let mut instance: VkInstance = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateInstance",
                          vkCreateInstance(&instance_create_info, ptr::null(), &mut instance));
        }

        let _: CString = unsafe { CString::from_raw(app_name) };
        let _: Vec<CString> = il.iter().map(|x| unsafe { CString::from_raw(*x) }).collect();
        let _: Vec<CString> = ie.iter().map(|x| unsafe { CString::from_raw(*x) }).collect();

        RendererVkInstance { raw: instance }
    }
}

impl Clone for RendererVkInstance {
    fn clone(&self) -> RendererVkInstance {
        unimplemented!();
    }
}

impl Drop for RendererVkInstance {
    fn drop(&mut self) {
        unsafe {
            vkDestroyInstance(self.raw, ptr::null());
        }
    }
}

pub struct RendererVkDebugCallback {
    instance: VkInstance,
    raw: VkDebugReportCallbackEXT,
}

impl RendererVkDebugCallback {
    /// Install a Vulkan debug callback function
    ///
    ///
    fn new(instance: &RendererVkInstance, debug_mask: u32) -> RendererVkDebugCallback {
        // The following directly follows the bitmask values defined for Vulkan, and
        // could just be collapsed, but it seems worth the application explicitly
        // coding these levels
        let mut flags: u32 = 0;
        if (debug_mask & 1) != 0 {
            flags |= VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_INFORMATION_BIT_EXT as VkDebugReportFlagsEXT
        };
        if (debug_mask & 2) != 0 {
            flags |= VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_WARNING_BIT_EXT as VkDebugReportFlagsEXT
        };
        if (debug_mask & 4) != 0 {
            flags |= VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT as VkDebugReportFlagsEXT
        };
        if (debug_mask & 8) != 0 {
            flags |= VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_ERROR_BIT_EXT as VkDebugReportFlagsEXT
        };
        if (debug_mask & 16) != 0 {
            flags |= VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_DEBUG_BIT_EXT as VkDebugReportFlagsEXT
        };

        let debug_callback_create_info = VkDebugReportCallbackCreateInfoEXT {
            sType: VkStructureType::VK_STRUCTURE_TYPE_DEBUG_REPORT_CALLBACK_CREATE_INFO_EXT,
            pfnCallback: Some(RendererVkDebugCallback::debug_callback),
            flags: flags,
            pNext: ptr::null(),
            pUserData: ptr::null_mut(),
        };

        let entrypoint_name = CString::new("vkCreateDebugReportCallbackEXT").unwrap();
        let opt_void_ptr = unsafe { vkGetInstanceProcAddr(instance.raw, entrypoint_name.as_ptr()) };
        let mut callback_handle: VkDebugReportCallbackEXT = VK_NULL_HANDLE_MUT();
        if opt_void_ptr.is_some() {
            let void_fn_ptr = opt_void_ptr.expect("to_be_found");
            type SrcType = unsafe extern "C" fn();
            #[allow(non_snake_case)]
            type DstType = extern "C" fn(instance: VkInstance,
                                         pCreateInfo: *const VkDebugReportCallbackCreateInfoEXT,
                                         pAllocator: *const VkAllocationCallbacks,
                                         pCallback: *mut VkDebugReportCallbackEXT)
                                         -> VkResult;
            #[allow(non_snake_case)]
            let pfn_vkCreateDebugReportCallbackEXT = unsafe { mem::transmute::<SrcType, DstType>(void_fn_ptr) };

            pfn_vkCreateDebugReportCallbackEXT(instance.raw,
                                               &debug_callback_create_info,
                                               ptr::null_mut(),
                                               &mut callback_handle);
        }

        RendererVkDebugCallback {
            instance: instance.raw,
            raw: callback_handle,
        }
    }

    /// A Vulkan debug callback function
    ///
    ///
    #[allow(unused_variables)]
    unsafe extern "C" fn debug_callback(flags: VkDebugReportFlagsEXT,
                                        obj_type: VkDebugReportObjectTypeEXT,
                                        src_obj: u64,
                                        location: usize,
                                        msg_code: i32,
                                        layer_prefix: *const c_char,
                                        msg: *const c_char,
                                        user_data: *mut c_void)
                                        -> u32 {
        let f = flags as u32;

        if (f & VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_INFORMATION_BIT_EXT as VkDebugReportFlagsEXT) != 0 {
            print!("INFO: ");
        }
        if (f & VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_WARNING_BIT_EXT as VkDebugReportFlagsEXT) != 0 {
            print!("WARN: ");
        }
        if (f & VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT as VkDebugReportFlagsEXT) != 0 {
            print!("PERF: ");
        }
        if (f & VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_ERROR_BIT_EXT as VkDebugReportFlagsEXT) != 0 {
            print!("ERRR: ");
        }
        if (f & VkDebugReportFlagBitsEXT::VK_DEBUG_REPORT_DEBUG_BIT_EXT as VkDebugReportFlagsEXT) != 0 {
            print!("DEBG: ");
        }
        let prefix = CStr::from_ptr(layer_prefix as *const i8)
            .to_string_lossy()
            .into_owned();
        let message = CStr::from_ptr(msg as *const i8)
            .to_string_lossy()
            .into_owned();
        println!("@[{}]: {}", prefix, message);

        false as u32
    }
}

impl Drop for RendererVkDebugCallback {
    fn drop(&mut self) {
        unsafe {
            let entrypoint_name = CString::new("vkDestroyDebugReportCallbackEXT").unwrap();
            let opt_void_ptr = vkGetInstanceProcAddr(self.instance, entrypoint_name.as_ptr());
            if opt_void_ptr.is_some() {
                let void_fn_ptr = opt_void_ptr.expect("to_be_found");
                type SrcType = unsafe extern "C" fn();
                #[allow(non_snake_case)]
                type DstType = extern "C" fn(instance: VkInstance,
                                             pCallback: VkDebugReportCallbackEXT,
                                             pAllocator: *const VkAllocationCallbacks)
                                             -> VkResult;
    			#[allow(non_snake_case)]
                let pfn_vkDestroyDebugReportCallbackEXT = mem::transmute::<SrcType, DstType>(void_fn_ptr);

                pfn_vkDestroyDebugReportCallbackEXT(self.instance, self.raw, ptr::null());
            }
        }
    }
}

pub struct RendererVkPhysicalDevice {
    raw: VkPhysicalDevice,
    features: VkPhysicalDeviceFeatures,
}

impl RendererVkPhysicalDevice {
    /// Enumerate physical devices and choose one
    ///
    ///
    fn new(instance: &RendererVkInstance, debug_level: u32) -> RendererVkPhysicalDevice {
        let mut physical_device_count: u32 = 0;
        unsafe {
            check_result!("vkEnumeratePhysicalDevices",
                          vkEnumeratePhysicalDevices(instance.raw, &mut physical_device_count, ptr::null_mut()));
        }

        // Enumerate the physical devices supported by Vulkan
        //
        let mut physical_devices: Vec<VkPhysicalDevice> = vec![];
        physical_devices.resize(physical_device_count as usize, VK_NULL_HANDLE_MUT());
        unsafe {
            // Now enumerate the physical devices
            check_result!("vkEnumeratePhysicalDevices",
                          vkEnumeratePhysicalDevices(instance.raw,
                                                     &mut physical_device_count,
                                                     physical_devices.as_mut_ptr()));
        }

        let mut best_score = 0;
        let mut chosen_device = 0;
        let mut device_properties = VkPhysicalDeviceProperties::default();
        let mut device_features = VkPhysicalDeviceFeatures::default();
        for i in 0..physical_device_count {
            unsafe {
                vkGetPhysicalDeviceProperties(physical_devices[i as usize], &mut device_properties);
                vkGetPhysicalDeviceFeatures(physical_devices[i as usize], &mut device_features);
            }

            let mut score = 0;
            if device_properties.deviceType as u32 == VkPhysicalDeviceType::VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU as u32 {
                if debug_level > 0 {
                    println!("Device {} scores 10 for being a discrete GPU", i);
                }
                score += 10
            };
            if device_features.tessellationShader == 0 {
                if debug_level > 0 {
                    println!("Device {} scores -100 for not supporting tessellation shaders",
                             i);
                }
                score -= 100;
            }
            if device_features.geometryShader == 0 {
                if debug_level > 0 {
                    println!("Device {} scores -100 for not supporting geometry shaders",
                             i);
                }
                score -= 100;
            }

            if debug_level > 0 {
                let slice = unsafe { CStr::from_ptr(&device_properties.deviceName as *const c_char) };
                println!("Physical device number {} is {} with score {}",
                         i,
                         CStr::to_string_lossy(slice),
                         score);
            }

            if score > best_score {
                chosen_device = i;
                best_score = score;
            }
        }
        if debug_level > 0 {
            println!("Chose physical device: {}", chosen_device);
        }
        if best_score < 0 {
            panic!("Device does not support required features");
        }

        RendererVkPhysicalDevice {
            raw: physical_devices[chosen_device as usize],
            features: device_features,
        }
    }
}

pub struct RendererVkQueueFamilies {
    #[allow(dead_code)]
    raw: Vec<VkQueueFamilyProperties>,
    index: u32,
}

impl RendererVkQueueFamilies {
    /// Enumerate queue families and select one that supports graphics
    ///
    ///
    fn new(physical_device: &RendererVkPhysicalDevice) -> RendererVkQueueFamilies {
        let mut queue_family_count: u32 = 0;
        unsafe {
            vkGetPhysicalDeviceQueueFamilyProperties(physical_device.raw,
                                                     &mut queue_family_count,
                                                     ptr::null_mut());
        }

        let mut queue_properties_array: Vec<VkQueueFamilyProperties> = vec![];
        queue_properties_array.resize(queue_family_count as usize,
                                      VkQueueFamilyProperties::default());
        unsafe {
            vkGetPhysicalDeviceQueueFamilyProperties(physical_device.raw,
                                                     &mut queue_family_count,
                                                     queue_properties_array.as_mut_ptr());
        }

        let mut graphics_queue_index: u32 = u32::max_value();
        for i in 0..queue_family_count {
            if queue_properties_array[i as usize].queueFlags & (VkQueueFlagBits::VK_QUEUE_GRAPHICS_BIT as VkQueueFlags) != 0 {
                graphics_queue_index = i;
            }
        }
        if graphics_queue_index == u32::max_value() {
            panic!("Suitable queue family not found");
        }

        RendererVkQueueFamilies {
            raw: queue_properties_array,
            index: graphics_queue_index,
        }
    }
}

pub struct RendererVkSurface {
    instance: VkInstance,
    raw: VkSurfaceKHR,
    format: VkSurfaceFormatKHR,
    capabilities: VkSurfaceCapabilitiesKHR,
    presentation: VkPresentModeKHR,
}

impl RendererVkSurface {
    /// Create Vulkan window surface
    ///
    ///
    fn new(window: &Window,
           instance: &RendererVkInstance,
           physical_device: &RendererVkPhysicalDevice,
           queue_family_index: u32,
           debug_level: u32)
           -> RendererVkSurface {
        // The following test is probably the same as the vkGetPhysicalDeviceSurfaceSupportKHR
        // call in query_surface_capabilities(), but just make sure GLFW thinks it is happy too
        if !window.get_physical_device_presentation_support(instance.raw as usize,
                                                            physical_device.raw as usize,
                                                            queue_family_index) {
            panic!("Queue family does not support image presentation");
        }

        let mut surface: VkSurfaceKHR = VK_NULL_HANDLE_MUT();
        unsafe {
            let res = glfw::ffi::glfwCreateWindowSurface(mem::transmute(instance.raw),
                                                         window.window_ptr(),
                                                         ptr::null(),
                                                         mem::transmute(&mut surface));
            if res != VkResult::VK_SUCCESS as u32 {
                panic!("Unable to create Vulkan surface");
            }
        }

        RendererVkSurface {
            instance: instance.raw,
            raw: surface,
            format: RendererVkSurface::choose_surface_format(physical_device, surface),
            capabilities: RendererVkSurface::determine_surface_capabilities(physical_device, queue_family_index, surface),
            presentation: RendererVkSurface::choose_presentation_mode(physical_device, surface, debug_level),
        }
    }

    /// Choose Vulkan window surface format
    ///
    ///
    fn choose_surface_format(physical_device: &RendererVkPhysicalDevice, raw_surface: VkSurfaceKHR) -> VkSurfaceFormatKHR {
        let mut format_count: u32 = 0;
        unsafe {
            check_result!("vkGetPhysicalDeviceSurfaceFormatsKHR",
                          vkGetPhysicalDeviceSurfaceFormatsKHR(physical_device.raw,
                                                               raw_surface,
                                                               &mut format_count,
                                                               ptr::null_mut()));
        }
        if format_count == 0 {
            panic!("Surface formats missing");
        }

        // Enumerate the surface formats available and pick one
        //
        let mut formats: Vec<VkSurfaceFormatKHR> = vec![];
        let default_format = VkSurfaceFormatKHR {
            format: VkFormat::VK_FORMAT_R8G8B8A8_UNORM,
            colorSpace: VkColorSpaceKHR::VK_COLOR_SPACE_SRGB_NONLINEAR_KHR,
        };
        formats.resize(format_count as usize, default_format);
        unsafe {
            check_result!("vkGetPhysicalDeviceSurfaceFormatsKHR",
                          vkGetPhysicalDeviceSurfaceFormatsKHR(physical_device.raw,
                                                               raw_surface,
                                                               &mut format_count,
                                                               formats.as_mut_ptr()));
        }

        // Arbitrarily pick the first available format
        let mut format = formats[0];

        if format_count == 1 && matches!(formats[0].format, VkFormat::VK_FORMAT_UNDEFINED) {
            // The driver has indicated that no format is preferred
            format = default_format;
        }

        format
    }

    /// Determine Vulkan surface capabilities
    ///
    ///
    fn determine_surface_capabilities(physical_device: &RendererVkPhysicalDevice,
                                      graphics_queue_family_index: u32,
                                      raw_surface: VkSurfaceKHR)
                                      -> VkSurfaceCapabilitiesKHR {
        // Determine whether the surface is supported or not
        let mut supported: VkBool32 = false as VkBool32;
        unsafe {
            check_result!("vkGetPhysicalDeviceSurfaceSupportKHR",
                          vkGetPhysicalDeviceSurfaceSupportKHR(physical_device.raw,
                                                               graphics_queue_family_index,
                                                               raw_surface,
                                                               &mut supported));
        }
        if supported != true as VkBool32 {
            panic!("The surface is not supported");
        }

        // Query the surface capabilities
        //
        let mut surface_capabilities = VkSurfaceCapabilitiesKHR::default();
        unsafe {
            check_result!("vkGetPhysicalDeviceSurfaceCapabilitiesKHR",
                          vkGetPhysicalDeviceSurfaceCapabilitiesKHR(physical_device.raw, raw_surface, &mut surface_capabilities));
        }

        if surface_capabilities.currentExtent.width == u32::max_value() {
            panic!("Unable to get surface dimensions");
        }

        debug_assert!(surface_capabilities.currentExtent.width >= surface_capabilities.minImageExtent.width);
        debug_assert!(surface_capabilities.currentExtent.width <= surface_capabilities.maxImageExtent.width);
        debug_assert!(surface_capabilities.currentExtent.height >= surface_capabilities.minImageExtent.height);
        debug_assert!(surface_capabilities.currentExtent.height <= surface_capabilities.maxImageExtent.height);

        surface_capabilities
    }

    /// Choose a Vulkan presentation mode
    ///
    ///
    fn choose_presentation_mode(physical_device: &RendererVkPhysicalDevice,
                                raw_surface: VkSurfaceKHR,
                                debug_level: u32)
                                -> VkPresentModeKHR {
        let mut presentation_mode_count: u32 = 0;
        unsafe {
            check_result!("vkGetPhysicalDeviceSurfacePresentModesKHR",
                          vkGetPhysicalDeviceSurfacePresentModesKHR(physical_device.raw,
                                                                    raw_surface,
                                                                    &mut presentation_mode_count,
                                                                    ptr::null_mut()));
        }

        let mut presentation_mode_list: Vec<VkPresentModeKHR> = vec![];
        presentation_mode_list.resize(presentation_mode_count as usize,
                                      VkPresentModeKHR::VK_PRESENT_MODE_MAX_ENUM_KHR);
        unsafe {
            check_result!("vkGetPhysicalDeviceSurfacePresentModesKHR",
                          vkGetPhysicalDeviceSurfacePresentModesKHR(physical_device.raw,
                                                                    raw_surface,
                                                                    &mut presentation_mode_count,
                                                                    presentation_mode_list.as_mut_ptr()));
        }

        let mut presentation_mode: VkPresentModeKHR = VkPresentModeKHR::VK_PRESENT_MODE_MAX_ENUM_KHR;
        for mode in presentation_mode_list.iter() {
            if debug_level > 0 {
                println!("Available presentation mode: {} {}", *mode as i32, *mode);
            }
            if matches!(*mode, VkPresentModeKHR::VK_PRESENT_MODE_FIFO_KHR) {
                presentation_mode = *mode;
            }
        }
        if matches!(presentation_mode,
                    VkPresentModeKHR::VK_PRESENT_MODE_MAX_ENUM_KHR) {
            presentation_mode = presentation_mode_list[0];
        }
        if debug_level > 0 {
            println!("Selected presentation mode is: {} {}",
                     presentation_mode as i32,
                     presentation_mode);
        }

        presentation_mode
    }
}

impl Drop for RendererVkSurface {
    fn drop(&mut self) {
        unsafe {
            vkDestroySurfaceKHR(self.instance, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkDevice {
    raw: VkDevice,
    graphics_queue: VkQueue,
}

impl RendererVkDevice {
    /// Set up structures required for creating a Vulkan device
    ///
    ///
    fn new(physical_device: &RendererVkPhysicalDevice,
           queue_family_index: u32,
           instance_layers: &Vec<String>,
           device_extensions: &Vec<String>)
           -> RendererVkDevice {
        let priorities: Vec<f32> = vec![1.0f32];
        let queue_create_info = VkDeviceQueueCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
            queueCount: 1,
            queueFamilyIndex: queue_family_index,
            pQueuePriorities: priorities.as_ptr(),
            flags: 0,
            pNext: ptr::null(),
        };

        let il: Vec<*mut c_char> = instance_layers.iter().map(|x| CString::new(x.to_owned()).unwrap().into_raw()).collect();
        let de: Vec<*mut c_char> = device_extensions.iter().map(|x| CString::new(x.to_owned()).unwrap().into_raw()).collect();

        let device_create_info = VkDeviceCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
            queueCreateInfoCount: 1,
            pQueueCreateInfos: &queue_create_info,
            // Enabling all features seems like cheating: We have the missed
            // opportunity to request all the features the application needs and
            // have the API reject device creation if they are not available.
            pEnabledFeatures: &physical_device.features,
            enabledLayerCount: instance_layers.len() as u32,
            ppEnabledLayerNames: il.as_ptr() as *mut _,
            enabledExtensionCount: device_extensions.len() as u32,
            ppEnabledExtensionNames: de.as_ptr() as *mut _,
            flags: 0,
            pNext: ptr::null(),
        };

        // Create a Vulkan device
        //
        let mut device: VkDevice = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateDevice",
                          vkCreateDevice(physical_device.raw,
                                         &device_create_info,
                                         ptr::null(),
                                         &mut device));
        }

        let _: Vec<CString> = il.iter().map(|x| unsafe { CString::from_raw(*x) }).collect();
        let _: Vec<CString> = de.iter().map(|x| unsafe { CString::from_raw(*x) }).collect();

        let mut graphics_queue: VkQueue = VK_NULL_HANDLE_MUT();
        unsafe {
            vkGetDeviceQueue(device, queue_family_index, 0, &mut graphics_queue);
        };

        RendererVkDevice {
            raw: device,
            graphics_queue: graphics_queue,
        }
    }
}

impl Drop for RendererVkDevice {
    fn drop(&mut self) {
        unsafe {
            vkDestroyDevice(self.raw, ptr::null());
        }
    }
}

pub struct RendererVkSwapchain {
    device: VkDevice,
    raw: VkSwapchainKHR,
    image_count: u32,
    images: Vec<VkImage>,
    views: Vec<VkImageView>,
}

impl RendererVkSwapchain {
    /// Create a swapchain
    ///
    ///
    fn new(device: &RendererVkDevice, surface: &RendererVkSurface, image_count: u32) -> RendererVkSwapchain {
        debug_assert!(image_count >= surface.capabilities.minImageCount);
        debug_assert!(image_count <= surface.capabilities.maxImageCount || surface.capabilities.maxImageCount == 0);

        let layers = 1; // Non-stereoscopic
        debug_assert!(layers <= surface.capabilities.maxImageArrayLayers);

        let usage = VkImageUsageFlagBits::VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT as VkImageUsageFlags;
        debug_assert!(((surface.capabilities.supportedUsageFlags as u32) & (usage as u32)) == (usage as u32));

        let transform = VkSurfaceTransformFlagBitsKHR::VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
        debug_assert!(((surface.capabilities.supportedTransforms as u32) & (transform as u32)) == (transform as u32));

        let alpha = VkCompositeAlphaFlagBitsKHR::VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
        debug_assert!(((surface.capabilities.supportedCompositeAlpha as u32) & (alpha as u32)) == (alpha as u32));

        let swapchain_create_info = VkSwapchainCreateInfoKHR {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
            surface: surface.raw,
            minImageCount: image_count,
            imageFormat: surface.format.format,
            imageColorSpace: surface.format.colorSpace,
            imageExtent: VkExtent2D {
                width: surface.capabilities.currentExtent.width,
                height: surface.capabilities.currentExtent.height,
            },
            imageArrayLayers: layers,
            imageUsage: usage,
            imageSharingMode: VkSharingMode::VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0, // None due to VK_SHARING_MODE_EXCLUSIVE
            pQueueFamilyIndices: ptr::null_mut(), // Null due to VK_SHARING_MODE_EXCLUSIVE
            preTransform: transform,
            compositeAlpha: alpha,
            presentMode: surface.presentation,
            clipped: true as VkBool32,
            oldSwapchain: ptr::null_mut(),
            flags: 0,
            pNext: ptr::null(),
        };

        let mut swapchain: VkSwapchainKHR = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateSwapchainKHR",
                          vkCreateSwapchainKHR(device.raw,
                                               &swapchain_create_info,
                                               ptr::null_mut(),
                                               &mut swapchain));
        }

        let mut local_image_count: u32 = 0;
        unsafe {
            check_result!("vkGetSwapchainImagesKHR",
                          vkGetSwapchainImagesKHR(device.raw,
                                                  swapchain,
                                                  &mut local_image_count,
                                                  ptr::null_mut()));
        }
        debug_assert!(local_image_count == image_count);

        let mut swapchain_images: Vec<VkImage> = vec![];
        swapchain_images.resize(local_image_count as usize, VK_NULL_HANDLE_MUT());
        unsafe {
            vkGetSwapchainImagesKHR(device.raw,
                                    swapchain,
                                    &mut local_image_count,
                                    swapchain_images.as_mut_ptr());
        }
        debug_assert!(local_image_count == image_count);

        let mut swapchain_image_views: Vec<VkImageView> = vec![];
        swapchain_image_views.resize(image_count as usize, VK_NULL_HANDLE_MUT());

        for i in 0..image_count {
            let create_info = VkImageViewCreateInfo {
                sType: VkStructureType::VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                image: swapchain_images[i as usize],
                viewType: VkImageViewType::VK_IMAGE_VIEW_TYPE_2D,
                format: surface.format.format,
                components: VkComponentMapping {
                    r: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                    g: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                    b: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                    a: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                },
                subresourceRange: VkImageSubresourceRange {
                    aspectMask: VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                    baseMipLevel: 0,
                    levelCount: 1,
                    baseArrayLayer: 0,
                    layerCount: 1,
                },
                flags: 0,
                pNext: ptr::null(),
            };

            unsafe {
                check_result!("vkCreateImageView",
                              vkCreateImageView(device.raw,
                                                &create_info,
                                                ptr::null_mut(),
                                                &mut swapchain_image_views[i as usize]));
            }
        }

        RendererVkSwapchain {
            device: device.raw,
            raw: swapchain,
            image_count: image_count,
            images: swapchain_images,
            views: swapchain_image_views,
        }
    }
}

impl Drop for RendererVkSwapchain {
    fn drop(&mut self) {
        for view in self.views.iter() {
            unsafe {
                vkDestroyImageView(self.device, *view, ptr::null());
            }
        }

        unsafe {
            vkDestroySwapchainKHR(self.device, self.raw, ptr::null());
        }
    }
}

struct OneTimeCommandBuffer {
    queue: VkQueue,
    buffer: RendererVkCommandBuffer,
}

impl OneTimeCommandBuffer {
    /// Create a single-use command buffer
    ///
    ///
    fn new(device: &RendererVkDevice, command_pool: &RendererVkCommandPool) -> OneTimeCommandBuffer {
        let command_buffer = RendererVkCommandBuffer::new(device, command_pool, true /* primary */);

        command_buffer.begin_primary(true, // one_time_submit
                                     false, // render_pass_continue
                                     false /* simultaneous_use */);

        OneTimeCommandBuffer {
            queue: device.graphics_queue,
            buffer: command_buffer,
        }
    }

    /// Execute the single-use command buffer
    ///
    ///
    fn execute(&mut self) {
        self.buffer.end();

        let command_buffers = vec![self.buffer.raw];
        let submit_info = VkSubmitInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SUBMIT_INFO,
            commandBufferCount: command_buffers.len() as u32,
            pCommandBuffers: command_buffers.as_ptr(),
            waitSemaphoreCount: 0,
            pWaitSemaphores: ptr::null(),
            pWaitDstStageMask: ptr::null(),
            signalSemaphoreCount: 0,
            pSignalSemaphores: ptr::null(),
            pNext: ptr::null(),
        };

        unsafe {
            check_result!("vkQueueSubmit",
                          vkQueueSubmit(self.queue, 1, &submit_info, VK_NULL_HANDLE_MUT()));
            check_result!("vkQueueWaitIdle", vkQueueWaitIdle(self.queue));
        }
    }
}

pub struct RendererVkImage {
    device: VkDevice,
    raw: VkImage,
    memory: VkDeviceMemory,
}

impl RendererVkImage {
    /// Return the raw image handle
    pub fn get_image_raw(&self) -> VkImage {
        self.raw
    }

    /// Create an image
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               physical_device: &RendererVkPhysicalDevice,
               command_pool: &RendererVkCommandPool,
               width: u32,
               height: u32,
               format: VkFormat,
               tiling: VkImageTiling,
               usage: VkImageUsageFlags,
               memory_properties: VkMemoryPropertyFlags,
               initial_layout: VkImageLayout,
               final_layout: VkImageLayout)
               -> RendererVkImage {
        let image_info = VkImageCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            imageType: VkImageType::VK_IMAGE_TYPE_2D,
            extent: VkExtent3D {
                width: width,
                height: height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: 1,
            format: format,
            tiling: tiling,
            initialLayout: VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED,
            usage: usage,
            samples: VkSampleCountFlagBits::VK_SAMPLE_COUNT_1_BIT,
            sharingMode: VkSharingMode::VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: VK_NULL_HANDLE_MUT(),
            flags: 0,
            pNext: ptr::null(),
        };

        let mut image: VkImage = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateImage",
                          vkCreateImage(device.raw, &image_info, ptr::null_mut(), &mut image));
        }

        let mut memory_requirements = VkMemoryRequirements::default();
        unsafe {
            vkGetImageMemoryRequirements(device.raw, image, &mut memory_requirements);
        }

        let alloc_info = VkMemoryAllocateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            allocationSize: memory_requirements.size,
            memoryTypeIndex: RendererVk::find_suitable_memory(physical_device,
                                                              memory_requirements.memoryTypeBits,
                                                              memory_properties),
            pNext: ptr::null(),
        };

        let mut image_memory: VkDeviceMemory = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkAllocateMemory",
                          vkAllocateMemory(device.raw, &alloc_info, ptr::null(), &mut image_memory));

            check_result!("vkBindBufferMemory",
                          vkBindImageMemory(device.raw, image, image_memory, 0));
        }

        let image = RendererVkImage {
            device: device.raw,
            raw: image,
            memory: image_memory,
        };

        let mut aspect_mask = 0;
        if final_layout as u32 == VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL as u32 {
            aspect_mask |= VkImageAspectFlagBits::VK_IMAGE_ASPECT_DEPTH_BIT as VkImageAspectFlags;

            if RendererVkImage::has_stencil_component(format) {
                aspect_mask |= VkImageAspectFlagBits::VK_IMAGE_ASPECT_STENCIL_BIT as VkImageAspectFlags;
            }
        } else {
            aspect_mask |= VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags;
        };

        if initial_layout as u32 != final_layout as u32 {
            RendererVkImage::transition_layout_immediate(image.raw,
                                               device,
                                               command_pool,
                                               aspect_mask,
                                               initial_layout,
                                               final_layout,
                                               VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags,
                                               VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags);
        }

        image
    }

    /// Does the specified format support a stencil component?
    ///
    ///
    fn has_stencil_component(format: VkFormat) -> bool {
        format as u32 == VkFormat::VK_FORMAT_D32_SFLOAT_S8_UINT as u32 ||
        format as u32 == VkFormat::VK_FORMAT_D24_UNORM_S8_UINT as u32
    }

    /// Add a command to the specified command buffer to transition an image layout into a new layout
    ///
    ///
    fn transition_layout(image: VkImage,
                         command_buffer: &RendererVkCommandBuffer,
                         aspect_mask: VkImageAspectFlags,
                         old_layout: VkImageLayout,
                         new_layout: VkImageLayout,
                         src_stage_mask: VkPipelineStageFlags,
                         dst_stage_mask: VkPipelineStageFlags) {
        let mut barrier = VkImageMemoryBarrier {
            sType: VkStructureType::VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            oldLayout: old_layout,
            newLayout: new_layout,
            srcQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED as u32,
            dstQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED as u32,
            image: image,
            subresourceRange: VkImageSubresourceRange::default(), // Filled in below
            srcAccessMask: 0, // Filled in below
            dstAccessMask: 0, // Filled in below
            pNext: ptr::null(),
        };

        barrier.subresourceRange.aspectMask = aspect_mask;
        barrier.subresourceRange.baseMipLevel = 0;
        barrier.subresourceRange.levelCount = 1;
        barrier.subresourceRange.baseArrayLayer = 0;
        barrier.subresourceRange.layerCount = 1;

        match old_layout {
            VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED => {
                barrier.srcAccessMask = VkAccessFlagBits::VK_ACCESS_HOST_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL => {
                barrier.srcAccessMask = VkAccessFlagBits::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL => {
                barrier.srcAccessMask = VkAccessFlagBits::VK_ACCESS_TRANSFER_READ_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL => {
                barrier.srcAccessMask = VkAccessFlagBits::VK_ACCESS_TRANSFER_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_UNDEFINED => {
                barrier.srcAccessMask = 0;
            }
            _ => {
                panic!("Unsupported transition source layout");
            }
        };

        match new_layout {
            VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_HOST_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_TRANSFER_READ_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_TRANSFER_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_SHADER_READ_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_PRESENT_SRC_KHR => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_MEMORY_READ_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_GENERAL => {
                barrier.dstAccessMask = VkAccessFlagBits::VK_ACCESS_HOST_READ_BIT as VkAccessFlags;
            }
            VkImageLayout::VK_IMAGE_LAYOUT_UNDEFINED => {
                barrier.dstAccessMask = 0;
            }
            _ => {
                panic!("Unsupported transition destination layout");
            }
        };

        unsafe {
            vkCmdPipelineBarrier(command_buffer.raw,
                                 src_stage_mask,
                                 dst_stage_mask,
                                 0, // Dependency flags
                                 0, // Memory barrier count
                                 ptr::null(), // Memory barriers
                                 0, // Buffer memory barrier count
                                 ptr::null(), // Buffer memory barriers
                                 1, // Image barrier count
                                 &barrier); // Image barriers
        }
    }

    /// Transition an image layout into a new layout and execute the transition immediately
    ///
    ///
    fn transition_layout_immediate(image: VkImage,
                                   device: &RendererVkDevice,
                                   command_pool: &RendererVkCommandPool,
                                   aspect_mask: VkImageAspectFlags,
                                   old_layout: VkImageLayout,
                                   new_layout: VkImageLayout,
                                   src_stages: VkPipelineStageFlags,
                                   dst_stages: VkPipelineStageFlags) {
        let mut one_time = OneTimeCommandBuffer::new(device, command_pool);

        RendererVkImage::transition_layout(image,
                                           &one_time.buffer,
                                           aspect_mask,
                                           old_layout,
                                           new_layout,
                                           src_stages,
                                           dst_stages);

        one_time.execute();
    }

    /// Copy the contents of an image to another image, e.g. for staging
    ///
    /// TODO: Split this up as for transition_layout and transition_layout_immediate
    ///
    pub fn copy(device: &RendererVkDevice,
                command_pool: &RendererVkCommandPool,
                source_image: VkImage,
                destination_image: VkImage,
                width: u32,
                height: u32) {
        let mut one_time = OneTimeCommandBuffer::new(device, command_pool);

        // Now copy the staging image to its final destination
        //
        let sub_resource = VkImageSubresourceLayers {
            aspectMask: VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
            baseArrayLayer: 0,
            mipLevel: 0,
            layerCount: 1,
        };

        let region = VkImageCopy {
            srcSubresource: sub_resource,
            dstSubresource: sub_resource,
            srcOffset: VkOffset3D { x: 0, y: 0, z: 0 },
            dstOffset: VkOffset3D { x: 0, y: 0, z: 0 },
            extent: VkExtent3D {
                width: width,
                height: height,
                depth: 1,
            },
        };

        unsafe {
            vkCmdCopyImage(one_time.buffer.raw,
                           source_image,
                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                           destination_image,
                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                           1,
                           &region);
        }

        one_time.execute();
    }
}

impl Drop for RendererVkImage {
    fn drop(&mut self) {
        unsafe {
            vkDestroyImage(self.device, self.raw, ptr::null());
            vkFreeMemory(self.device, self.memory, ptr::null());
        }
    }
}

pub struct RendererVkImageView {
    device: VkDevice,
    raw: VkImageView,
}

impl RendererVkImageView {
    /// Return the raw image view for this image
    pub fn get_view_raw(&self) -> VkImageView {
        self.raw
    }

    /// Create an image view
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               image: &RendererVkImage,
               format: VkFormat,
               aspect_flags: VkImageAspectFlags)
               -> RendererVkImageView {
        let image_view_info = VkImageViewCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            image: image.raw,
            viewType: VkImageViewType::VK_IMAGE_VIEW_TYPE_2D,
            format: format,
            subresourceRange: VkImageSubresourceRange {
                aspectMask: aspect_flags,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            components: VkComponentMapping {
                r: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                g: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                b: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                a: VkComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
            },
            flags: 0,
            pNext: ptr::null(),
        };

        let mut image_view: VkImageView = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateImageView",
                          vkCreateImageView(device.raw,
                                            &image_view_info,
                                            ptr::null_mut(),
                                            &mut image_view));
        }

        RendererVkImageView {
            device: device.raw,
            raw: image_view,
        }
    }
}

impl Drop for RendererVkImageView {
    fn drop(&mut self) {
        unsafe {
            vkDestroyImageView(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkRenderPass {
    device: VkDevice,
    raw: VkRenderPass,
}

impl RendererVkRenderPass {
    /// Create a render pass with some default rendering choices
    ///
    ///
    fn new(device: &RendererVkDevice, colour_format: VkFormat, depth_format: Option<VkFormat>) -> RendererVkRenderPass {
        // Define the colour and depth attachment references
        //
        let color_attachment_refs = vec![VkAttachmentReference {
                                             attachment: 0,
                                             layout: VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                         }];

        let depth_attachment_ref = VkAttachmentReference {
            attachment: 1,
            layout: VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        // Create subpasses and subpass dependencies.  Note that the wait stages on the queue
        // submission has been set to be VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT in order to avoid
        // an explicit dependency between VK_SUBPASS_EXTERNAL and subpass 0.
        //
        let mut sub_passes: Vec<VkSubpassDescription> = vec![];
        let dependencies: Vec<VkSubpassDependency> = vec![];

        sub_passes.push(VkSubpassDescription {
            pipelineBindPoint: VkPipelineBindPoint::VK_PIPELINE_BIND_POINT_GRAPHICS,
            colorAttachmentCount: color_attachment_refs.len() as u32,
            pColorAttachments: color_attachment_refs.as_ptr(),
            inputAttachmentCount: 0,
            pInputAttachments: ptr::null(),
            pResolveAttachments: ptr::null(),
            pDepthStencilAttachment: if depth_format.is_some() {
                &depth_attachment_ref
            } else {
                ptr::null()
            },
            preserveAttachmentCount: 0,
            pPreserveAttachments: ptr::null(),
            flags: 0,
        });

        // Now define the renderpass
        //
        let mut attachments = vec![VkAttachmentDescription {
                                       format: colour_format,
                                       samples: VkSampleCountFlagBits::VK_SAMPLE_COUNT_1_BIT,
                                       loadOp: VkAttachmentLoadOp::VK_ATTACHMENT_LOAD_OP_DONT_CARE,
                                       storeOp: VkAttachmentStoreOp::VK_ATTACHMENT_STORE_OP_STORE,
                                       stencilLoadOp: VkAttachmentLoadOp::VK_ATTACHMENT_LOAD_OP_DONT_CARE,
                                       stencilStoreOp: VkAttachmentStoreOp::VK_ATTACHMENT_STORE_OP_DONT_CARE,
                                       initialLayout: VkImageLayout::VK_IMAGE_LAYOUT_UNDEFINED,
                                       finalLayout: VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                       flags: 0,
                                   }];
        if depth_format.is_some() {
            attachments.push(VkAttachmentDescription {
                format: depth_format.unwrap(),
                samples: VkSampleCountFlagBits::VK_SAMPLE_COUNT_1_BIT,
                loadOp: VkAttachmentLoadOp::VK_ATTACHMENT_LOAD_OP_LOAD,
                storeOp: VkAttachmentStoreOp::VK_ATTACHMENT_STORE_OP_STORE,
                stencilLoadOp: VkAttachmentLoadOp::VK_ATTACHMENT_LOAD_OP_DONT_CARE,
                stencilStoreOp: VkAttachmentStoreOp::VK_ATTACHMENT_STORE_OP_DONT_CARE,
                initialLayout: VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                finalLayout: VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                flags: 0,
            });
        }

        let render_pass_info = VkRenderPassCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            attachmentCount: attachments.len() as u32,
            pAttachments: attachments.as_ptr(),
            subpassCount: sub_passes.len() as u32,
            pSubpasses: sub_passes.as_ptr(),
            dependencyCount: dependencies.len() as u32,
            pDependencies: dependencies.as_ptr(),
            flags: 0,
            pNext: ptr::null(),
        };

        let mut render_pass: VkRenderPass = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateRenderPass",
                          vkCreateRenderPass(device.raw, &render_pass_info, ptr::null(), &mut render_pass));
        }

        RendererVkRenderPass {
            device: device.raw,
            raw: render_pass,
        }
    }

    /// Begin a render pass
    ///
    ///
    pub fn begin(&self, raw_command_buffer: VkCommandBuffer, raw_framebuffer: VkFramebuffer, width: u32, height: u32) {
        let render_pass_begin_info = VkRenderPassBeginInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            renderPass: self.raw,
            framebuffer: raw_framebuffer,
            renderArea: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: VkExtent2D {
                    width: width,
                    height: height,
                },
            },
            clearValueCount: 0,
            pClearValues: ptr::null(),
            pNext: ptr::null(),
        };

        unsafe {
            vkCmdBeginRenderPass(raw_command_buffer,
                                 &render_pass_begin_info,
                                 VkSubpassContents::VK_SUBPASS_CONTENTS_INLINE);
        }
    }

    /// End render pass
    ///
    ///
    pub fn end(&self, raw_command_buffer: VkCommandBuffer) {
        unsafe {
            vkCmdEndRenderPass(raw_command_buffer);
        }
    }
}

impl Drop for RendererVkRenderPass {
    fn drop(&mut self) {
        unsafe {
            vkDestroyRenderPass(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkFramebuffer {
    device: VkDevice,
    raw: VkFramebuffer,
}

impl RendererVkFramebuffer {
    /// Return the raw framebuffer handle
    pub fn get_framebuffer_raw(&self) -> VkFramebuffer {
        self.raw
    }

    /// Create a framebuffer for a colour image view and a depth image view
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               raw_colour_image_view: VkImageView,
               raw_depth_image_view: Option<VkImageView>,
               render_pass: &RendererVkRenderPass,
               width: u32,
               height: u32)
               -> RendererVkFramebuffer {
        let mut framebuffer: VkFramebuffer = VK_NULL_HANDLE_MUT();

        let mut attachments: Vec<VkImageView> = vec![raw_colour_image_view];
        if raw_depth_image_view.is_some() {
            attachments.push(raw_depth_image_view.unwrap());
        }

        let framebuffer_info = VkFramebufferCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            renderPass: render_pass.raw,
            attachmentCount: attachments.len() as u32,
            pAttachments: attachments.as_ptr(),
            width: width,
            height: height,
            layers: 1,
            flags: 0,
            pNext: ptr::null(),
        };

        unsafe {
            check_result!("vkCreateFramebuffer",
                          vkCreateFramebuffer(device.raw,
                                              &framebuffer_info,
                                              ptr::null_mut(),
                                              &mut framebuffer));
        }

        RendererVkFramebuffer {
            device: device.raw,
            raw: framebuffer,
        }
    }
}

impl Drop for RendererVkFramebuffer {
    fn drop(&mut self) {
        unsafe {
            vkDestroyFramebuffer(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkShaderModule {
    device: VkDevice,
    raw: VkShaderModule,
}

impl RendererVkShaderModule {
    /// Create a shader module
    ///
    /// device: The Vulkan device
    /// binary: The shader SPIR-V bytecode
    pub fn new(raw_device: VkDevice, binary: &Vec<u8>) -> RendererVkShaderModule {
        let create_info = VkShaderModuleCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
            codeSize: binary.len(), // codeSize is the length in bytes
            pCode: binary.as_ptr() as *const u32,
            flags: 0,
            pNext: ptr::null(),
        };
        let mut shader_module: VkShaderModule = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateShaderModule",
                          vkCreateShaderModule(raw_device, &create_info, ptr::null(), &mut shader_module));
        }

        RendererVkShaderModule {
            device: raw_device,
            raw: shader_module,
        }
    }

    pub fn get_raw(&self) -> VkShaderModule {
        self.raw
    }
}

impl Drop for RendererVkShaderModule {
    fn drop(&mut self) {
        unsafe {
            vkDestroyShaderModule(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkVertexBuffer {
    buffer: RendererVkBuffer,
}

impl RendererVkVertexBuffer {
    /// Create a Vulkan vertex buffer
    ///
    /// TODO: Using only host-visible-and-coherent won't be the fastest
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               physical_device: &RendererVkPhysicalDevice,
               array_type: VertexArrayType)
               -> RendererVkVertexBuffer {
        let size = match array_type {
            VertexArrayType::F3 => 3 * mem::size_of::<f32>() * 3 * TRIANGLE_ARRAY_SIZE,
            VertexArrayType::F3F3F3 => 9 * mem::size_of::<f32>() * 3 * TRIANGLE_ARRAY_SIZE,
            VertexArrayType::F3F3 => 6 * mem::size_of::<f32>() * 3 * TRIANGLE_ARRAY_SIZE,
            VertexArrayType::F2F2 => 4 * mem::size_of::<f32>() * 3 * TRIANGLE_ARRAY_SIZE,
        };

        let properties = unsafe {
            mem::transmute(VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT as VkMemoryPropertyFlags |
                           VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT as VkMemoryPropertyFlags)
        };

        RendererVkVertexBuffer {
            buffer: RendererVkBuffer::new(device,
                                          physical_device,
                                          VkBufferUsageFlagBits::VK_BUFFER_USAGE_VERTEX_BUFFER_BIT as VkBufferUsageFlags,
                                          properties,
                                          size),
        }
    }
}

pub struct RendererVkUniformBuffer {
    buffer: RendererVkBuffer,
    binding: u32,
    bytes: Vec<u8>,
    offsets: HashMap<&'static str, usize>,
    strides: HashMap<&'static str, usize>,
}

impl RendererVkUniformBuffer {
    /// Create a Vulkan uniform buffer
    ///
    /// TODO: Using only host-visible-and-coherent won't be the fastest
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               physical_device: &RendererVkPhysicalDevice,
               spec: &UniformBlockSpec)
               -> RendererVkUniformBuffer {
        let properties = unsafe {
            mem::transmute(VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT as VkMemoryPropertyFlags |
                           VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT as VkMemoryPropertyFlags)
        };

        let buffer = RendererVkBuffer::new(device,
                                           physical_device,
                                           VkBufferUsageFlagBits::VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT as VkBufferUsageFlags,
                                           properties,
                                           spec.size);

        let mut bytes = Vec::with_capacity(spec.size);
        bytes.resize(spec.size, 0);
        let mut offsets: HashMap<&'static str, usize> = HashMap::new();
        for uniform in spec.uniforms.iter() {
            offsets.insert(uniform.name, uniform.offset);
        }
        let mut strides: HashMap<&'static str, usize> = HashMap::new();
        for uniform in spec.uniforms.iter() {
            strides.insert(uniform.name, uniform.stride);
        }

        RendererVkUniformBuffer {
            buffer: buffer,
            binding: spec.binding,
            bytes: bytes,
            offsets: offsets,
            strides: strides,
        }
    }
}

pub struct RendererVkBuffer {
    device: VkDevice,
    raw: VkBuffer,
    memory: VkDeviceMemory,
    size: usize, // Size of requested buffer: actual allocation may be bigger
}

impl RendererVkBuffer {
    /// Create a Vulkan buffer of various types
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               physical_device: &RendererVkPhysicalDevice,
               usage_bits: VkBufferUsageFlags,
               memory_type: VkMemoryPropertyFlags,
               size: usize)
               -> RendererVkBuffer {
        debug_assert!(size != 0);
        let buffer_info = VkBufferCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            size: size as u64,
            usage: usage_bits as u32,
            sharingMode: VkSharingMode::VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0, // No sharing
            pQueueFamilyIndices: ptr::null(), // No sharing
            flags: 0,
            pNext: ptr::null_mut(),
        };

        let mut buffer: VkBuffer = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateBuffer",
                          vkCreateBuffer(device.raw, &buffer_info, ptr::null(), &mut buffer));
        }

        let mut memory_requirements = VkMemoryRequirements::default();
        unsafe {
            vkGetBufferMemoryRequirements(device.raw, buffer, &mut memory_requirements);
        }

        let memory_type = RendererVk::find_suitable_memory(physical_device,
                                                           memory_requirements.memoryTypeBits,
                                                           memory_type);

        if memory_type == u32::max_value() {
            panic!("Unable to identify suitable memory for buffer");
        }

        let alloc_info = VkMemoryAllocateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            allocationSize: memory_requirements.size,
            memoryTypeIndex: memory_type as u32,
            pNext: ptr::null(),
        };

        let mut buffer_memory: VkDeviceMemory = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkAllocateMemory",
                          vkAllocateMemory(device.raw, &alloc_info, ptr::null(), &mut buffer_memory));

            check_result!("vkBindBufferMemory",
                          vkBindBufferMemory(device.raw, buffer, buffer_memory, 0));
        }

        RendererVkBuffer {
            device: device.raw,
            raw: buffer,
            memory: buffer_memory,
            size: size,
        }
    }
}

impl Drop for RendererVkBuffer {
    fn drop(&mut self) {
        unsafe {
            vkDestroyBuffer(self.device, self.raw, ptr::null());
            vkFreeMemory(self.device, self.memory, ptr::null());
        }
    }
}

pub struct RendererVkDescriptorSetLayouts {
    device: VkDevice,
    raw: Vec<VkDescriptorSetLayout>,
}

impl RendererVkDescriptorSetLayouts {
    /// Create a set of descriptor set layouts for a given shader
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               resource_manager: &ResourceManager,
               shader_spec: &ShaderSpec)
               -> RendererVkDescriptorSetLayouts {
        let max_set = RendererVkDescriptorSetLayouts::number_of_sets(resource_manager, shader_spec);

        // Then iterate through them all and identify the elements belonging to each set
        //
        let mut descriptor_set_layouts: Vec<VkDescriptorSetLayout> = vec![];
        for for_set in 0..(max_set + 1) {
            let mut set_layout_bindings: Vec<VkDescriptorSetLayoutBinding> = vec![];

            for block_name in shader_spec.uniform_block_names.iter() {
                let ref block = resource_manager.uniform_block_specs[block_name];

                if block.set == for_set {
                    // println!("Creating descriptor set layout binding for uniform block {} at {}.{}",
                    //          block_name,
                    //          block.set,
                    //          block.binding);

                    set_layout_bindings.push(VkDescriptorSetLayoutBinding {
                        descriptorType: RendererVkDescriptorSetLayouts::internal_descriptor_type(block.block_type),
                        // stageFlags set to 'all' is overkill, but does it harm performance?
                        stageFlags: VkShaderStageFlagBits::VK_SHADER_STAGE_ALL as VkShaderStageFlags,
                        binding: block.binding,
                        descriptorCount: 1,
                        pImmutableSamplers: ptr::null(),
                    });
                }
            }

            for uniform in shader_spec.uniform_specs.iter() {
                if uniform.set == for_set && uniform.binding != u32::max_value() {
                    // println!("Creating descriptor set layout binding for uniform {} at {}.{}",
                    //          uniform.name,
                    //          uniform.set,
                    //          uniform.binding);

                    set_layout_bindings.push(VkDescriptorSetLayoutBinding {
                        descriptorType: RendererVkDescriptorSetLayouts::internal_descriptor_type(uniform.uniform_type),
                        // stageFlags set to 'all' is overkill, but does it harm performance?
                        stageFlags: VkShaderStageFlagBits::VK_SHADER_STAGE_ALL as VkShaderStageFlags,
                        binding: uniform.binding,
                        descriptorCount: 1,
                        pImmutableSamplers: ptr::null(), // Optional
                    });
                }
            }

            // If there were items in the set, create a descriptor set layout object
            //
            if set_layout_bindings.len() > 0 {
                let set_layout_create_info = VkDescriptorSetLayoutCreateInfo {
                    sType: VkStructureType::VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                    bindingCount: set_layout_bindings.len() as u32,
                    pBindings: set_layout_bindings.as_ptr(),
                    flags: 0,
                    pNext: ptr::null(),
                };

                let mut descriptor_set_layout: VkDescriptorSetLayout = VK_NULL_HANDLE_MUT();
                unsafe {
                    check_result!("vkCreateDescriptorSetLayout",
                                  vkCreateDescriptorSetLayout(device.raw,
                                                              &set_layout_create_info,
                                                              ptr::null(), // Allocator
                                                              &mut descriptor_set_layout));
                }

                descriptor_set_layouts.push(descriptor_set_layout);
            }
        }

        RendererVkDescriptorSetLayouts {
            device: device.raw,
            raw: descriptor_set_layouts,
        }
    }

    /// Convert a resources general descriptor type to a Vulkan descriptor type
    ///
    ///
    pub fn internal_descriptor_type(descriptor_type: UniformType) -> VkDescriptorType {
        match descriptor_type {
            UniformType::Sampler => VkDescriptorType::VK_DESCRIPTOR_TYPE_SAMPLER,
            UniformType::CombinedImageSampler => VkDescriptorType::VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
            UniformType::SampledImage => VkDescriptorType::VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE,
            UniformType::StorageImage => VkDescriptorType::VK_DESCRIPTOR_TYPE_STORAGE_IMAGE,
            UniformType::UniformTexelBuffer => VkDescriptorType::VK_DESCRIPTOR_TYPE_UNIFORM_TEXEL_BUFFER,
            UniformType::StorageTexelBuffer => VkDescriptorType::VK_DESCRIPTOR_TYPE_STORAGE_TEXEL_BUFFER,
            UniformType::UniformBuffer => VkDescriptorType::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER,
            UniformType::StorageBuffer => VkDescriptorType::VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
            UniformType::UniformBufferDynamic => VkDescriptorType::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER_DYNAMIC,
            UniformType::StorageBufferDynamic => VkDescriptorType::VK_DESCRIPTOR_TYPE_STORAGE_BUFFER_DYNAMIC,
            UniformType::InputAttachment => VkDescriptorType::VK_DESCRIPTOR_TYPE_INPUT_ATTACHMENT,
            UniformType::RangeSize => VkDescriptorType::VK_DESCRIPTOR_TYPE_RANGE_SIZE,
        }
    }

    /// Find out the number of sets
    ///
    ///
    pub fn number_of_sets(resource_manager: &ResourceManager, shader_spec: &ShaderSpec) -> u32 {
        let mut max_set: u32 = 0;

        for block_name in shader_spec.uniform_block_names.iter() {
            let ref block = resource_manager.uniform_block_specs[block_name];
            if block.set > max_set {
                max_set = block.set;
            }
        }

        for uniform in shader_spec.uniform_specs.iter() {
            if uniform.set > max_set {
                max_set = uniform.set;
            }
        }

        max_set
    }
}

impl Drop for RendererVkDescriptorSetLayouts {
    fn drop(&mut self) {
        for dsl in self.raw.iter() {
            unsafe {
                vkDestroyDescriptorSetLayout(self.device, *dsl, ptr::null());
            }
        }
    }
}

pub struct RendererVkDescriptorPool {
    device: VkDevice,
    raw: VkDescriptorPool,
}

impl RendererVkDescriptorPool {
    /// Create a Vulkan descriptor pool
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               max_uniform_buffers: usize,
               max_combined_image_samplers: usize,
               max_sets: usize)
               -> RendererVkDescriptorPool {
        let uniform_buffer_pool_size = VkDescriptorPoolSize {
            type_: VkDescriptorType::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER,
            descriptorCount: max_uniform_buffers as u32,
        };

        let combined_image_samplers_pool_size = VkDescriptorPoolSize {
            type_: VkDescriptorType::VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
            descriptorCount: max_combined_image_samplers as u32,
        };

        let buffer_pool_sizes = vec![uniform_buffer_pool_size, combined_image_samplers_pool_size];

        let pool_info = VkDescriptorPoolCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
            poolSizeCount: buffer_pool_sizes.len() as u32,
            pPoolSizes: buffer_pool_sizes.as_ptr(),
            maxSets: max_sets as u32,
            flags: 0,
            pNext: ptr::null(),
        };

        let mut descriptor_pool: VkDescriptorPool = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateDescriptorPool",
                          vkCreateDescriptorPool(device.raw, &pool_info, ptr::null(), &mut descriptor_pool));
        }

        RendererVkDescriptorPool {
            device: device.raw,
            raw: descriptor_pool,
        }
    }
}

impl Drop for RendererVkDescriptorPool {
    fn drop(&mut self) {
        unsafe {
            vkDestroyDescriptorPool(self.device, self.raw, ptr::null());
        }
    }
}

// Note: There is no Drop implementation for RendererVkDescriptorSet as
// VkDescriptorSet objects are destroyed when the VkDescriptorPool they belong
// to is destroyed
pub struct RendererVkDescriptorSet {
    raw: VkDescriptorSet,
}

impl RendererVkDescriptorSet {
    /// Create a Vulkan descriptor set
    ///
    ///
    pub fn new(device: &RendererVkDevice,
               descriptor_pool: &RendererVkDescriptorPool,
               descriptor_set_layouts: &RendererVkDescriptorSetLayouts,
               resource: &ShaderSpec,
               uniform_buffers: &HashMap<&'static str, RendererVkUniformBuffer>,
               textures: &HashMap<&'static str, &TextureVk>)
               -> RendererVkDescriptorSet {
        let alloc_info = VkDescriptorSetAllocateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
            descriptorPool: descriptor_pool.raw,
            descriptorSetCount: descriptor_set_layouts.raw.len() as u32,
            pSetLayouts: descriptor_set_layouts.raw.as_ptr(),
            pNext: ptr::null(),
        };

        let mut descriptor_set: VkDescriptorSet = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkAllocateDescriptorSets",
                          vkAllocateDescriptorSets(device.raw, &alloc_info, &mut descriptor_set));
        }

        // Iterate through all the uniform blocks and generate descriptor set writes for each
        //
        let n = resource.uniform_block_names.len();
        let mut buffer_infos = Vec::with_capacity(n);
        let mut descriptor_writes = Vec::with_capacity(n);
        let mut i = 0;
        for uniform_block_name in resource.uniform_block_names.iter() {
            let ref uniform_buffer = uniform_buffers[uniform_block_name];

            buffer_infos.push(VkDescriptorBufferInfo {
                buffer: uniform_buffer.buffer.raw,
                offset: 0,
                range: uniform_buffer.buffer.size as u64,
            });

            descriptor_writes.push(VkWriteDescriptorSet {
                sType: VkStructureType::VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                dstSet: descriptor_set,
                dstBinding: uniform_buffer.binding,
                dstArrayElement: 0,
                descriptorType: VkDescriptorType::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER,
                descriptorCount: 1,
                pBufferInfo: &buffer_infos[i],
                pImageInfo: ptr::null(), // Optional
                pTexelBufferView: ptr::null(), // Optional
                pNext: ptr::null(),
            });

            i += 1;
        }

        // Iterate through all the combined image and sampler uniforms and generate descriptor
        // set writes for each
        //
        i = 0;
        let mut image_infos = vec![];
        for uniform_spec in resource.uniform_specs.iter() {
            if uniform_spec.uniform_type == UniformType::CombinedImageSampler && textures.contains_key(uniform_spec.name) {
                image_infos.push(VkDescriptorImageInfo {
                    imageLayout: VkImageLayout::VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                    imageView: textures[uniform_spec.name].texture.view.raw,
                    sampler: textures[uniform_spec.name].texture.sampler,
                });

                descriptor_writes.push(VkWriteDescriptorSet {
                    sType: VkStructureType::VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                    dstSet: descriptor_set,
                    dstBinding: uniform_spec.binding,
                    dstArrayElement: 0,
                    descriptorType: VkDescriptorType::VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                    descriptorCount: 1,
                    pBufferInfo: ptr::null(), // Optional
                    pImageInfo: &image_infos[i],
                    pTexelBufferView: ptr::null(), // Optional
                    pNext: ptr::null(),
                });

                i += 1;
            }
        }

        unsafe {
            vkUpdateDescriptorSets(device.raw,
                                   descriptor_writes.len() as u32,
                                   descriptor_writes.as_ptr(),
                                   0, // Copy count
                                   ptr::null() /* Descriptor copies */);
        }

        RendererVkDescriptorSet { raw: descriptor_set }
    }
}

pub struct RendererVkPipelineLayout {
    device: VkDevice,
    raw: VkPipelineLayout,
}

impl RendererVkPipelineLayout {
    /// Set up pipeline layout with the specified descriptor set layouts
    ///
    ///
    pub fn new(device: &RendererVkDevice, descriptor_set_layouts: &RendererVkDescriptorSetLayouts) -> RendererVkPipelineLayout {
        let pipeline_layout_info = VkPipelineLayoutCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
            setLayoutCount: descriptor_set_layouts.raw.len() as u32,
            pSetLayouts: descriptor_set_layouts.raw.as_ptr(),
            pushConstantRangeCount: 0, // Optional
            pPushConstantRanges: ptr::null(), // Optional
            flags: 0,
            pNext: ptr::null(),
        };

        let mut pipeline_layout: VkPipelineLayout = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreatePipelineLayout",
                          vkCreatePipelineLayout(device.raw,
                                                 &pipeline_layout_info,
                                                 ptr::null(),
                                                 &mut pipeline_layout));
        }

        RendererVkPipelineLayout {
            device: device.raw,
            raw: pipeline_layout,
        }
    }
}

impl Drop for RendererVkPipelineLayout {
    fn drop(&mut self) {
        unsafe {
            vkDestroyPipelineLayout(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkPipeline {
    device: VkDevice,
    raw: VkPipeline,
}

impl RendererVkPipeline {
    /// Create a graphics pipeline with some default rendering choices
    ///
    ///
    fn new(device: &RendererVkDevice,
           render_pass: &RendererVkRenderPass,
           shader_spec: &ShaderSpec,
           shader: &ShaderSpirv,
           pipeline_layout: &RendererVkPipelineLayout,
           width: u32,
           height: u32)
           -> RendererVkPipeline {
        let viewport = VkViewport {
            x: 0.0f32,
            y: 0.0f32,
            width: width as f32,
            height: height as f32,
            minDepth: 0.0f32,
            maxDepth: 1.0f32,
        };

        let scissor = VkRect2D {
            offset: VkOffset2D { x: 0, y: 0 },
            extent: VkExtent2D {
                width: width,
                height: height,
            },
        };

        let viewport_state = VkPipelineViewportStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            viewportCount: 1,
            pViewports: &viewport,
            scissorCount: 1,
            pScissors: &scissor,
            flags: 0,
            pNext: ptr::null(),
        };

        let rasterizer = VkPipelineRasterizationStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            depthClampEnable: false as VkBool32,
            rasterizerDiscardEnable: false as VkBool32,
            polygonMode: VkPolygonMode::VK_POLYGON_MODE_FILL,
            lineWidth: 1.0f32,
            cullMode: VkCullModeFlagBits::VK_CULL_MODE_NONE as VkCullModeFlags,
            frontFace: VkFrontFace::VK_FRONT_FACE_COUNTER_CLOCKWISE,
            depthBiasEnable: false as VkBool32,
            depthBiasConstantFactor: 0.0f32, // Optional
            depthBiasClamp: 0.0f32, // Optional
            depthBiasSlopeFactor: 0.0f32, // Optional
            flags: 0,
            pNext: ptr::null(),
        };

        let multisampling = VkPipelineMultisampleStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            sampleShadingEnable: false as VkBool32,
            rasterizationSamples: VkSampleCountFlagBits::VK_SAMPLE_COUNT_1_BIT,
            minSampleShading: 1.0f32, // Optional
            pSampleMask: ptr::null(), // Optional
            alphaToCoverageEnable: false as VkBool32, // Optional
            alphaToOneEnable: false as VkBool32, // Optional
            flags: 0,
            pNext: ptr::null(),
        };

        let color_blend_attachment = if shader_spec.alpha_blending_enabled {
            VkPipelineColorBlendAttachmentState {
                colorWriteMask: VkColorComponentFlagBits::VK_COLOR_COMPONENT_R_BIT as VkColorComponentFlags |
                                VkColorComponentFlagBits::VK_COLOR_COMPONENT_G_BIT as VkColorComponentFlags |
                                VkColorComponentFlagBits::VK_COLOR_COMPONENT_B_BIT as VkColorComponentFlags |
                                VkColorComponentFlagBits::VK_COLOR_COMPONENT_A_BIT as VkColorComponentFlags,
                blendEnable: true as VkBool32,
                srcColorBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_SRC_ALPHA,
                dstColorBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                colorBlendOp: VkBlendOp::VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ZERO,
                alphaBlendOp: VkBlendOp::VK_BLEND_OP_ADD, // Optional
            }
        } else {
            VkPipelineColorBlendAttachmentState {
                colorWriteMask: VkColorComponentFlagBits::VK_COLOR_COMPONENT_R_BIT as VkColorComponentFlags |
                                VkColorComponentFlagBits::VK_COLOR_COMPONENT_G_BIT as VkColorComponentFlags |
                                VkColorComponentFlagBits::VK_COLOR_COMPONENT_B_BIT as VkColorComponentFlags |
                                VkColorComponentFlagBits::VK_COLOR_COMPONENT_A_BIT as VkColorComponentFlags,
                blendEnable: true as VkBool32,
                srcColorBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ONE,
                dstColorBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ZERO,
                colorBlendOp: VkBlendOp::VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VkBlendFactor::VK_BLEND_FACTOR_ZERO,
                alphaBlendOp: VkBlendOp::VK_BLEND_OP_ADD,
            }
        };

        let color_blending = if shader_spec.alpha_blending_enabled {
            VkPipelineColorBlendStateCreateInfo {
                sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                logicOpEnable: false as VkBool32,
                logicOp: VkLogicOp::VK_LOGIC_OP_COPY,
                attachmentCount: 1,
                pAttachments: &color_blend_attachment,
                blendConstants: [0.0f32, 0.0f32, 0.0f32, 0.0f32],
                flags: 0,
                pNext: ptr::null(),
            }
        } else {
            VkPipelineColorBlendStateCreateInfo {
                sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                logicOpEnable: false as VkBool32,
                logicOp: VkLogicOp::VK_LOGIC_OP_COPY,
                attachmentCount: 1,
                pAttachments: &color_blend_attachment,
                blendConstants: [0.0f32, 0.0f32, 0.0f32, 0.0f32],
                flags: 0,
                pNext: ptr::null(),
            }
        };

        let tessellation_state_create_info = VkPipelineTessellationStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_TESSELLATION_STATE_CREATE_INFO,
            patchControlPoints: 3, // To agree with the tessellation control and evaluation shaders
            flags: 0,
            pNext: ptr::null(),
        };

        let entrypoint_name = CString::new("main").unwrap().into_raw();

        let maximum_shader_stages = 5;
        let modules = shader.get_shader_modules();

        let mut shader_stage_infos: Vec<VkPipelineShaderStageCreateInfo> = Vec::with_capacity(maximum_shader_stages);
        let mut has_tessellation = false;
        for module in modules.iter() {
            let (shader_stage, shader_module) = *module;

            if shader_stage == ShaderStage::TessControlShader || shader_stage == ShaderStage::TessEvalShader {
                has_tessellation = true;
            }

            shader_stage_infos.push(VkPipelineShaderStageCreateInfo {
                sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                stage: ShaderSpirv::internal_shader_stage(shader_stage),
                module: shader_module,
                pName: entrypoint_name,
                pSpecializationInfo: ptr::null(),
                flags: 0,
                pNext: ptr::null(),
            });
        }

        let input_assembly_info = VkPipelineInputAssemblyStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            topology: if has_tessellation {
                VkPrimitiveTopology::VK_PRIMITIVE_TOPOLOGY_PATCH_LIST
            } else {
                VkPrimitiveTopology::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST
            },
            primitiveRestartEnable: false as VkBool32,
            flags: 0,
            pNext: ptr::null(),
        };

        let (bindings, attributes) = RendererVkPipeline::vertex_input_bindings_and_attributes_info(shader_spec.vertex_array_type);
        let vertex_input_info = VkPipelineVertexInputStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            vertexBindingDescriptionCount: bindings.len() as u32,
            pVertexBindingDescriptions: bindings.as_ptr(),
            vertexAttributeDescriptionCount: attributes.len() as u32,
            pVertexAttributeDescriptions: attributes.as_ptr(),
            flags: 0,
            pNext: ptr::null(),
        };

        let depth_stencil_info = VkPipelineDepthStencilStateCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            depthTestEnable: shader_spec.depth_test_enabled as VkBool32,
            depthWriteEnable: shader_spec.depth_test_enabled as VkBool32,
            depthCompareOp: VkCompareOp::VK_COMPARE_OP_LESS,
            depthBoundsTestEnable: false as VkBool32,
            minDepthBounds: 0.0f32, // Optional
            maxDepthBounds: 1.0f32, // Optional
            stencilTestEnable: false as VkBool32,
            front: VkStencilOpState::default(), // Optional
            back: VkStencilOpState::default(), // Optional
            flags: 0,
            pNext: ptr::null(),
        };

        let pipeline_info = VkGraphicsPipelineCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            stageCount: modules.len() as u32,
            pStages: shader_stage_infos.as_ptr(),
            pVertexInputState: &vertex_input_info,
            pInputAssemblyState: &input_assembly_info,
            pViewportState: &viewport_state,
            pRasterizationState: &rasterizer,
            pMultisampleState: &multisampling,
            pDepthStencilState: &depth_stencil_info,
            pColorBlendState: &color_blending,
            pDynamicState: ptr::null(), // Optional
            layout: pipeline_layout.raw,
            renderPass: render_pass.raw,
            subpass: 0,
            basePipelineIndex: -1, // Optional: This indicates that the new pipeline is not derived
            basePipelineHandle: VK_NULL_HANDLE_MUT(), // Optional: This indicates that the new pipeline is not derived
            pTessellationState: if has_tessellation {
                &tessellation_state_create_info
            } else {
                ptr::null()
            },
            flags: 0,
            pNext: ptr::null(),
        };

        let mut render_pipeline: VkPipeline = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateGraphicsPipelines",
                          vkCreateGraphicsPipelines(device.raw,
                                                    VK_NULL_HANDLE_MUT(), // Optional pipeline cache
                                                    1, // Number of pipelines to create
                                                    &pipeline_info,
                                                    ptr::null(),
                                                    &mut render_pipeline));
        }

        let _: CString = unsafe { CString::from_raw(entrypoint_name) };

        RendererVkPipeline {
            device: device.raw,
            raw: render_pipeline,
        }
    }

    /// Return pipeline vertex input state bindings and attributes structures for a given vertex array type
    ///
    ///
    fn vertex_input_bindings_and_attributes_info
        (array_type: VertexArrayType)
         -> (Vec<VkVertexInputBindingDescription>, Vec<VkVertexInputAttributeDescription>) {
        let bindings;
        let attributes;
        match array_type {
            VertexArrayType::F3 => {
                bindings = vec![VkVertexInputBindingDescription {
                                    binding: 0,
                                    stride: 3 * mem::size_of::<f32>() as u32,
                                    inputRate: VkVertexInputRate::VK_VERTEX_INPUT_RATE_VERTEX,
                                }];
                attributes = vec![VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 0,
                                      format: VkFormat::VK_FORMAT_R32G32B32_SFLOAT,
                                      offset: 0,
                                  }];
            }
            VertexArrayType::F3F3F3 => {
                bindings = vec![VkVertexInputBindingDescription {
                                    binding: 0,
                                    stride: 9 * mem::size_of::<f32>() as u32,
                                    inputRate: VkVertexInputRate::VK_VERTEX_INPUT_RATE_VERTEX,
                                }];
                attributes = vec![VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 0,
                                      format: VkFormat::VK_FORMAT_R32G32B32_SFLOAT,
                                      offset: 0,
                                  },
                                  VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 1,
                                      format: VkFormat::VK_FORMAT_R32G32B32_SFLOAT,
                                      offset: 3 * mem::size_of::<f32>() as u32,
                                  },
                                  VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 2,
                                      format: VkFormat::VK_FORMAT_R32G32B32_SFLOAT,
                                      offset: 6 * mem::size_of::<f32>() as u32,
                                  }];
            }
            VertexArrayType::F3F3 => {
                bindings = vec![VkVertexInputBindingDescription {
                                    binding: 0,
                                    stride: 6 * mem::size_of::<f32>() as u32,
                                    inputRate: VkVertexInputRate::VK_VERTEX_INPUT_RATE_VERTEX,
                                }];
                attributes = vec![VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 0,
                                      format: VkFormat::VK_FORMAT_R32G32B32_SFLOAT,
                                      offset: 0,
                                  },
                                  VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 1,
                                      format: VkFormat::VK_FORMAT_R32G32B32_SFLOAT,
                                      offset: 3 * mem::size_of::<f32>() as u32,
                                  }];
            }
            VertexArrayType::F2F2 => {
                bindings = vec![VkVertexInputBindingDescription {
                                    binding: 0,
                                    stride: 4 * mem::size_of::<f32>() as u32,
                                    inputRate: VkVertexInputRate::VK_VERTEX_INPUT_RATE_VERTEX,
                                }];
                attributes = vec![VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 0,
                                      format: VkFormat::VK_FORMAT_R32G32_SFLOAT,
                                      offset: 0,
                                  },
                                  VkVertexInputAttributeDescription {
                                      binding: 0,
                                      location: 1,
                                      format: VkFormat::VK_FORMAT_R32G32_SFLOAT,
                                      offset: 2 * mem::size_of::<f32>() as u32,
                                  }];
            }
        }

        (bindings, attributes)
    }
}

impl Drop for RendererVkPipeline {
    fn drop(&mut self) {
        unsafe {
            vkDestroyPipeline(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkCommandPool {
    device: VkDevice,
    raw: VkCommandPool,
}

impl RendererVkCommandPool {
    /// Create a command pool
    ///
    /// device: The Vulkan device
    pub fn new(device: &RendererVkDevice, queue_family_index: u32) -> RendererVkCommandPool {
        let pool_info = VkCommandPoolCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            queueFamilyIndex: queue_family_index,
            flags: VkCommandPoolCreateFlagBits::VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT as VkCommandPoolCreateFlags,
            pNext: ptr::null(),
        };

        let mut command_pool: VkCommandPool = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateCommandPool",
                          vkCreateCommandPool(device.raw, &pool_info, ptr::null(), &mut command_pool));
        }

        RendererVkCommandPool {
            device: device.raw,
            raw: command_pool,
        }
    }
}

impl Drop for RendererVkCommandPool {
    fn drop(&mut self) {
        unsafe {
            vkDestroyCommandPool(self.device, self.raw, ptr::null());
        }
    }
}

// Note: There is no Drop implementation for RendererVkCommandBuffer as
// VkCommandBuffer objects are destroyed when the VkCommandPool they belong
// to is destroyed
pub struct RendererVkCommandBuffer {
    raw: VkCommandBuffer,
    primary: bool,
}

impl RendererVkCommandBuffer {
    /// Create a command buffer
    ///
    /// device: The Vulkan device
    /// primary: true if this is to be a primary command buffers, false if it is to be
    ///     a secondary command buffer
    pub fn new(device: &RendererVkDevice, command_pool: &RendererVkCommandPool, primary: bool) -> RendererVkCommandBuffer {
        let allocate_info = VkCommandBufferAllocateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            commandPool: command_pool.raw,
            level: if primary {
                VkCommandBufferLevel::VK_COMMAND_BUFFER_LEVEL_PRIMARY
            } else {
                VkCommandBufferLevel::VK_COMMAND_BUFFER_LEVEL_SECONDARY
            },
            commandBufferCount: 1,
            pNext: ptr::null(),
        };

        let mut command_buffer: VkCommandBuffer = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkAllocateCommandBuffers",
                          vkAllocateCommandBuffers(device.raw, &allocate_info, &mut command_buffer));
        }

        RendererVkCommandBuffer {
            raw: command_buffer,
            primary: primary,
        }
    }

    /// Begin recording to the primary command buffer
    ///
    ///
    pub fn begin_primary(&self, one_time_submit: bool, render_pass_continue: bool, simultaneous_use: bool) {
        debug_assert!(self.primary);

        let mut flags: u32 = 0;
        if one_time_submit {
            flags |= VkCommandBufferUsageFlagBits::VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT as VkCommandBufferUsageFlags;
        };
        if render_pass_continue {
            flags |= VkCommandBufferUsageFlagBits::VK_COMMAND_BUFFER_USAGE_RENDER_PASS_CONTINUE_BIT as VkCommandBufferUsageFlags;
        };
        if simultaneous_use {
            flags |= VkCommandBufferUsageFlagBits::VK_COMMAND_BUFFER_USAGE_SIMULTANEOUS_USE_BIT as VkCommandBufferUsageFlags;
        };

        let begin_info = VkCommandBufferBeginInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            flags: flags,
            pInheritanceInfo: ptr::null(),
            pNext: ptr::null(),
        };

        unsafe {
            vkBeginCommandBuffer(self.raw, &begin_info);
        }
    }

    /// Begin recording to the secondary command buffer using the supplied inheritance info
    ///
    ///
    pub fn begin_secondary(&self, renderpass: &RendererVkRenderPass, subpass: u32, framebuffer: &RendererVkFramebuffer) {
        debug_assert!(!self.primary);

        let inheritance_info = VkCommandBufferInheritanceInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_COMMAND_BUFFER_INHERITANCE_INFO,
            pNext: ptr::null(),
            renderPass: renderpass.raw,
            subpass: subpass,
            framebuffer: framebuffer.raw,
            occlusionQueryEnable: true as VkBool32,
            queryFlags: VkQueryControlFlagBits::VK_QUERY_CONTROL_PRECISE_BIT as VkQueryControlFlags,
            pipelineStatistics: 0,
        };

        let begin_info = VkCommandBufferBeginInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            flags: VkCommandBufferUsageFlagBits::VK_COMMAND_BUFFER_USAGE_SIMULTANEOUS_USE_BIT as VkCommandBufferUsageFlags |
                   VkCommandBufferUsageFlagBits::VK_COMMAND_BUFFER_USAGE_RENDER_PASS_CONTINUE_BIT as VkCommandBufferUsageFlags,
            pInheritanceInfo: &inheritance_info,
            pNext: ptr::null(),
        };

        unsafe {
            vkBeginCommandBuffer(self.raw, &begin_info);
        }
    }

    /// End recording to the command buffer
    ///
    ///
    pub fn end(&self) {
        unsafe {
            check_result!("vkEndCommandBuffer", vkEndCommandBuffer(self.raw));
        }
    }

    /// Emit an explicit pipeline image memory barrier, raw
    ///
    ///
    pub fn image_memory_barrier_raw(command_buffer: VkCommandBuffer,
                                    image: VkImage,
                                    src_access_mask: VkAccessFlags,
                                    dst_access_mask: VkAccessFlags,
                                    old_layout: VkImageLayout,
                                    new_layout: VkImageLayout,
                                    aspect_mask: VkImageAspectFlags,
                                    src_stage_mask: VkPipelineStageFlags,
                                    dst_stage_mask: VkPipelineStageFlags) {
        let barrier = VkImageMemoryBarrier {
            sType: VkStructureType::VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            srcAccessMask: src_access_mask,
            dstAccessMask: dst_access_mask,
            oldLayout: old_layout,
            newLayout: new_layout,
            srcQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED as u32,
            dstQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED as u32,
            image: image,
            subresourceRange: VkImageSubresourceRange {
                aspectMask: aspect_mask,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            pNext: ptr::null(),
        };

        unsafe {
            vkCmdPipelineBarrier(command_buffer,
                                 src_stage_mask,
                                 dst_stage_mask,
                                 0, // Dependency flags
                                 0, // Memory barrier count
                                 ptr::null(), // Memory barriers
                                 0, // Buffer memory barrier count
                                 ptr::null(), // Buffer memory barriers
                                 1, // Image barrier count
                                 &barrier); // Image barriers
        }
    }

    /// Emit an explicit pipeline image memory barrier
    ///
    ///
    pub fn image_memory_barrier(&self,
                                image: VkImage,
                                src_access_mask: VkAccessFlags,
                                dst_access_mask: VkAccessFlags,
                                old_layout: VkImageLayout,
                                new_layout: VkImageLayout,
                                aspect_mask: VkImageAspectFlags,
                                src_stage_mask: VkPipelineStageFlags,
                                dst_stage_mask: VkPipelineStageFlags) {
        RendererVkCommandBuffer::image_memory_barrier_raw(self.raw,
                                                          image,
                                                          src_access_mask,
                                                          dst_access_mask,
                                                          old_layout,
                                                          new_layout,
                                                          aspect_mask,
                                                          src_stage_mask,
                                                          dst_stage_mask);
    }

    /// Emit an explicit pipeline buffer memory barrier, raw
    ///
    ///
    pub fn buffer_memory_barrier_raw(command_buffer: VkCommandBuffer,
                                     buffer: VkBuffer,
                                     src_access_mask: VkAccessFlags,
                                     dst_access_mask: VkAccessFlags,
                                     src_stage_mask: VkPipelineStageFlags,
                                     dst_stage_mask: VkPipelineStageFlags) {
        let barrier = VkBufferMemoryBarrier {
            sType: VkStructureType::VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,
            srcAccessMask: src_access_mask,
            dstAccessMask: dst_access_mask,
            srcQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED as u32,
            dstQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED as u32,
            buffer: buffer,
            offset: 0,
            size: VK_WHOLE_SIZE as u64,
            pNext: ptr::null(),
        };

        unsafe {
            vkCmdPipelineBarrier(command_buffer,
                                 src_stage_mask,
                                 dst_stage_mask,
                                 0, // Dependency flags
                                 0, // Memory barrier count
                                 ptr::null(), // Memory barriers
                                 1, // Buffer memory barrier count
                                 &barrier, // Buffer memory barriers
                                 0, // Image barrier count
                                 ptr::null()); // Image barriers
        }
    }

    /// Emit an explicit pipeline buffer memory barrier
    ///
    ///
    pub fn buffer_memory_barrier(&self,
                                 buffer: VkBuffer,
                                 src_access_mask: VkAccessFlags,
                                 dst_access_mask: VkAccessFlags,
                                 src_stage_mask: VkPipelineStageFlags,
                                 dst_stage_mask: VkPipelineStageFlags) {
        RendererVkCommandBuffer::buffer_memory_barrier_raw(self.raw,
                                                           buffer,
                                                           src_access_mask,
                                                           dst_access_mask,
                                                           src_stage_mask,
                                                           dst_stage_mask);
    }

    /// Emit an explicit pipeline memory barrier, raw
    ///
    ///
    pub fn memory_barrier_raw(command_buffer: VkCommandBuffer,
                              src_access_mask: VkAccessFlags,
                              dst_access_mask: VkAccessFlags,
                              src_stage_mask: VkPipelineStageFlags,
                              dst_stage_mask: VkPipelineStageFlags) {
        let barrier = VkMemoryBarrier {
            sType: VkStructureType::VK_STRUCTURE_TYPE_MEMORY_BARRIER,
            srcAccessMask: src_access_mask,
            dstAccessMask: dst_access_mask,
            pNext: ptr::null(),
        };

        unsafe {
            vkCmdPipelineBarrier(command_buffer,
                                 src_stage_mask,
                                 dst_stage_mask,
                                 0, // Dependency flags
                                 1, // Memory barrier count
                                 &barrier, // Memory barriers
                                 0, // Buffer memory barrier count
                                 ptr::null(), // Buffer memory barriers
                                 0, // Image barrier count
                                 ptr::null()); // Image barriers
        }
    }

    /// Emit an explicit pipeline memory barrier
    ///
    ///
    pub fn memory_barrier(&self,
                          src_access_mask: VkAccessFlags,
                          dst_access_mask: VkAccessFlags,
                          src_stage_mask: VkPipelineStageFlags,
                          dst_stage_mask: VkPipelineStageFlags) {
        RendererVkCommandBuffer::memory_barrier_raw(self.raw,
                                                    src_access_mask,
                                                    dst_access_mask,
                                                    src_stage_mask,
                                                    dst_stage_mask);
    }
}

pub struct RendererVkSemaphore {
    device: VkDevice,
    raw: VkSemaphore,
}

impl RendererVkSemaphore {
    /// Create a semaphore
    ///
    ///
    pub fn new(device: &RendererVkDevice) -> RendererVkSemaphore {
        let semaphore_create_info = VkSemaphoreCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
            flags: 0,
            pNext: ptr::null_mut(),
        };

        let mut semaphore: VkSemaphore = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateSemaphore",
                          vkCreateSemaphore(device.raw,
                                            &semaphore_create_info,
                                            ptr::null_mut(),
                                            &mut semaphore));
        }

        RendererVkSemaphore {
            device: device.raw,
            raw: semaphore,
        }
    }
}

impl Drop for RendererVkSemaphore {
    fn drop(&mut self) {
        unsafe {
            vkDestroySemaphore(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkFence {
    device: VkDevice,
    raw: VkFence,
}

impl RendererVkFence {
    /// Create a fence
    ///
    ///
    pub fn new(device: &RendererVkDevice) -> RendererVkFence {
        let fence_create_info = VkFenceCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
            flags: VkFenceCreateFlagBits::VK_FENCE_CREATE_SIGNALED_BIT as VkFenceCreateFlags,
            pNext: ptr::null(),
        };

        let mut fence: VkFence = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateFence",
                          vkCreateFence(device.raw, &fence_create_info, ptr::null_mut(), &mut fence));
        }

        RendererVkFence {
            device: device.raw,
            raw: fence,
        }
    }
}

impl Drop for RendererVkFence {
    fn drop(&mut self) {
        unsafe {
            vkDestroyFence(self.device, self.raw, ptr::null());
        }
    }
}

pub struct RendererVkTexture {
    device: VkDevice,
    queue: VkQueue,
    #[allow(dead_code)]
    image: RendererVkImage,
    view: RendererVkImageView,
    sampler: VkSampler,
    width: u32,
    height: u32,
    format: VkFormat,
    row_pitch: u64,
}

impl RendererVkTexture {
    /// Return the raw image view for this texture
    pub fn get_view_raw(&self) -> VkImageView {
        self.view.raw
    }

    /// Constructor for a Vulkan texture object
    ///
    ///
    pub fn new(renderer: &RendererVk,
               width: u32,
               height: u32,
               format: VkFormat,
               bytes_per_pixel: u32,
               data: &Vec<u8>)
               -> RendererVkTexture {
        // Create a new host-accessible staging image to format the image data into
        //
        let props = VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT as VkMemoryPropertyFlags |
                    VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT as VkMemoryPropertyFlags;

        let staging_image = RendererVkImage::new(&renderer.device,
                                                 &renderer.physical_device,
                                                 &renderer.aux_command_pool,
                                                 width,
                                                 height,
                                                 format,
                                                 VkImageTiling::VK_IMAGE_TILING_LINEAR,
                                                 VkImageUsageFlagBits::VK_IMAGE_USAGE_TRANSFER_SRC_BIT as VkImageUsageFlags,
                                                 props,
                                                 VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED,
                                                 VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED);

        // Query the subresource layout information
        //
        let subresource = VkImageSubresource {
            aspectMask: VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
            mipLevel: 0,
            arrayLayer: 0,
        };

        let mut staging_image_layout = VkSubresourceLayout::default();
        unsafe {
            vkGetImageSubresourceLayout(renderer.device.raw,
                                        staging_image.raw,
                                        &subresource,
                                        &mut staging_image_layout);
        }

        if data.len() > 0 {
            // Map the image into host-addressable memory and then reformat the raw image data into it
            //
            let mut raw: *mut c_void = VK_NULL_HANDLE_MUT();
            unsafe {
                check_result!("vkMapMemory",
                              vkMapMemory(renderer.device.raw,
                                          staging_image.memory,
                                          0, // Offset
                                          VK_WHOLE_SIZE as u64,
                                          0, // Flags
                                          &mut raw));
            }

            // TODO: Optimise this when the image layout contains no padding
            let raw_u8 = raw as *mut u8;

            unsafe {
                for y in 0..height {
                    ptr::copy_nonoverlapping(&data[(y * width * bytes_per_pixel) as usize],
                                             raw_u8.offset(y as isize * staging_image_layout.rowPitch as isize),
                                             (width * bytes_per_pixel) as usize);
                }
            }

            unsafe {
                vkUnmapMemory(renderer.device.raw, staging_image.memory);
            }
        }

        // The staging image needs to be in a layout suitable for being the source of a copy
        //
        RendererVkImage::transition_layout_immediate(staging_image.raw,
                                           &renderer.device,
                                           &renderer.aux_command_pool,
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags);

        // Create the device-local image to copy into
        //
        let image = RendererVkImage::new(&renderer.device,
                                         &renderer.physical_device,
                                         &renderer.aux_command_pool,
                                         width,
                                         height,
                                         format,
                                         VkImageTiling::VK_IMAGE_TILING_OPTIMAL,
                                         VkImageUsageFlagBits::VK_IMAGE_USAGE_TRANSFER_SRC_BIT as VkImageUsageFlags |
                                         VkImageUsageFlagBits::VK_IMAGE_USAGE_TRANSFER_DST_BIT as VkImageUsageFlags |
                                         VkImageUsageFlagBits::VK_IMAGE_USAGE_SAMPLED_BIT as VkImageUsageFlags |
                                         VkImageUsageFlagBits::VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT as VkImageUsageFlags,
                                         VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT as VkMemoryPropertyFlags,
                                         VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED,
                                         VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL);

        // Now copy from the staging image to its final location
        //
        if data.len() > 0 {
            RendererVkImage::copy(&renderer.device,
                                  &renderer.aux_command_pool,
                                  staging_image.raw,
                                  image.raw,
                                  width,
                                  height);
        }

        // The final image needs to be in a layout suitable for being used in the shader
        //
        RendererVkImage::transition_layout_immediate(image.raw,
                                           &renderer.device,
                                           &renderer.aux_command_pool,
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                           VkImageLayout::VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags);

        // Create an image view for the texture in its final location
        //
        let view = RendererVkImageView::new(&renderer.device,
                                            &image,
                                            format,
                                            VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags);

        // Create a sampler for the texture
        //
        let sampler_info = VkSamplerCreateInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
            magFilter: VkFilter::VK_FILTER_LINEAR,
            minFilter: VkFilter::VK_FILTER_LINEAR,
            addressModeU: VkSamplerAddressMode::VK_SAMPLER_ADDRESS_MODE_REPEAT,
            addressModeV: VkSamplerAddressMode::VK_SAMPLER_ADDRESS_MODE_REPEAT,
            addressModeW: VkSamplerAddressMode::VK_SAMPLER_ADDRESS_MODE_REPEAT,
            anisotropyEnable: true as VkBool32,
            maxAnisotropy: 16.0,
            borderColor: VkBorderColor::VK_BORDER_COLOR_INT_OPAQUE_BLACK,
            unnormalizedCoordinates: false as VkBool32,
            compareEnable: false as VkBool32,
            compareOp: VkCompareOp::VK_COMPARE_OP_ALWAYS,
            mipmapMode: VkSamplerMipmapMode::VK_SAMPLER_MIPMAP_MODE_LINEAR,
            mipLodBias: 0.0f32,
            minLod: 0.0f32,
            maxLod: 0.0f32,
            flags: 0,
            pNext: ptr::null_mut(),
        };

        let mut sampler: VkSampler = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkCreateSampler",
                          vkCreateSampler(renderer.device.raw,
                                          &sampler_info,
                                          ptr::null(),
                                          &mut sampler));
        }

        RendererVkTexture {
            device: renderer.device.raw,
            queue: renderer.device.graphics_queue,
            image: image,
            view: view,
            sampler: sampler,
            width: width,
            height: height,
            format: format,
            row_pitch: staging_image_layout.rowPitch,
        }
    }


    /// Obtain the pixel contents of a Vulkan texture object
    ///
    /// TODO: Optimise this, maybe
    ///
    ///
    pub fn read_pixels(&self, renderer: &RendererVk) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];
        let num_bytes: usize = (self.height * self.width * 3) as usize;
        data.resize(num_bytes, 0);

        // Create a new host-accessible staging image to format the image data into
        //
        let props = VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT as VkMemoryPropertyFlags |
                    VkMemoryPropertyFlagBits::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT as VkMemoryPropertyFlags;

        let staging_image = RendererVkImage::new(&renderer.device,
                                                 &renderer.physical_device,
                                                 &renderer.aux_command_pool,
                                                 self.width,
                                                 self.height,
                                                 self.format, // VkFormat::VK_FORMAT_R8G8B8A8_UNORM,
                                                 VkImageTiling::VK_IMAGE_TILING_LINEAR,
                                                 VkImageUsageFlagBits::VK_IMAGE_USAGE_TRANSFER_DST_BIT as VkImageUsageFlags,
                                                 props,
                                                 VkImageLayout::VK_IMAGE_LAYOUT_PREINITIALIZED,
                                                 VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL);

        // Transition the render target to something that we can transfer from
        //
        RendererVkImage::transition_layout_immediate(self.image.raw,
                                           &renderer.device,
                                           &renderer.aux_command_pool,
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags);

        // Now copy from the device local memory to the staging image
        //
        RendererVkImage::copy(&renderer.device,
                              &renderer.aux_command_pool,
                              self.image.raw,
                              staging_image.raw,
                              self.width,
                              self.height);

        // Transition the render target back to something that we can render to
        //
        RendererVkImage::transition_layout_immediate(self.image.raw,
                                           &renderer.device,
                                           &renderer.aux_command_pool,
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                           VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags);

        // Transition the staging image to something that we can map
        //
        RendererVkImage::transition_layout_immediate(staging_image.raw,
                                           &renderer.device,
                                           &renderer.aux_command_pool,
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                           VkImageLayout::VK_IMAGE_LAYOUT_GENERAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags);

        // Map the staging image data into memory
        //
        let mut raw: *mut c_void = VK_NULL_HANDLE_MUT();
        unsafe {
            check_result!("vkMapMemory",
                          vkMapMemory(renderer.device.raw,
                                      staging_image.memory,
                                      0, // Offset
                                      VK_WHOLE_SIZE as u64,
                                      0, // Flags
                                      &mut raw));
        }

        let raw_f32 = raw as *const f32;

        unsafe {
            for y in 0..self.height {
                for x in 0..self.width {
                    for i in 0..3 {
                        let f = *raw_f32.offset(((y as u64 * (self.row_pitch >> 2) + 4 * x as u64) + i as u64) as isize);
                        data[(((self.height - 1 - y) * self.width + x) * 3 + i) as usize] = (f * 255.0) as u8;
                    }
                }
            }
        }

        unsafe {
            vkUnmapMemory(renderer.device.raw, staging_image.memory);
        }

        data
    }
}

impl Drop for RendererVkTexture {
    fn drop(&mut self) {
        unsafe {
            // This is possibly using a sledgehammer to crack a nut, but the sampler
            // must not be in use when we try to destroy it
            check_result!("vkQueueWaitIdle", vkQueueWaitIdle(self.queue));
            vkDestroySampler(self.device, self.sampler, ptr::null());
        }
    }
}

impl Renderer for RendererVk {
    /// To facilitate downcasting back to a concrete type
    fn as_any(&self) -> &Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut Any {
        self
    }

    /// Return the renderer type
    fn renderer_type(&self) -> RendererType {
        RendererType::RendererVk
    }

    /// Obtain an Arc for the ThreadData structure for the specified thread
    fn get_threaddata(&self, thr: usize) -> Arc<Mutex<Box<ThreadData>>> {
        self.threaddata_arcs[thr].clone()
    }

    /// Return the maximum number of threads allowed
    fn get_maxthreads(&self) -> usize {
        return self.max_threads;
    }

    /// Finish initialisation of resources
    ///
    /// shaders: A map of the shaders to set up, keyed by name
    /// textures: A map of the textures to set up, keyed by name
    fn finish_resource_initialisation(&mut self,
                                      shaders: &HashMap<&'static str, &Box<Shader>>,
                                      textures: &HashMap<&'static str, &Box<Texture>>) {
        let mut renderer_vk: &mut RendererVk = match self.as_any_mut().downcast_mut::<RendererVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        let mut shaders_vk = HashMap::new();
        for shader in shaders.iter() {
            let (nm, sh) = shader;
            match sh.as_any().downcast_ref::<ShaderSpirv>() {
                Some(s) => shaders_vk.insert(*nm, s),
                None => panic!("Unexpected runtime type"),
            };
        }

        let mut textures_vk = HashMap::new();
        for texture in textures.iter() {
            let (nm, tx) = texture;
            match tx.as_any().downcast_ref::<TextureVk>() {
                Some(t) => textures_vk.insert(*nm, t),
                None => panic!("Unexpected runtime type"),
            };
        }

        renderer_vk.setup(&shaders_vk, &textures_vk);
    }

    /// Clear the depth buffer before starting rendering
    fn clear_depth_buffer(&self) {
        // First check that there is a depth target bound
        assert!(self.current_depth_target.is_some());

        unsafe {
            check_result!("vkResetCommandBuffer",
                          vkResetCommandBuffer(self.cleardepth_command_buffers[self.image_index].raw,
                                               0 /* flags */));
        }

        // Clear the frame.  We need to do the clear here instead of as a
        // load op since all of the threads share the same pipelines and
        // render pass.
        //
        let clear_stencil = unsafe {
            mem::transmute_copy(&VkClearDepthStencilValue {
                depth: 1.0f32,
                stencil: 0,
            })
        };

        self.cleardepth_command_buffers[self.image_index].begin_primary(true, // one_time_submit
                                                                        false, // render_pass_continue
                                                                        true /* simultaneous_use */);

        RendererVkImage::transition_layout(self.current_depth_target.unwrap(),
                                           &self.cleardepth_command_buffers[self.image_index],
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_DEPTH_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_UNDEFINED,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT
                                               as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TRANSFER_BIT
                                               as VkPipelineStageFlags);

        let subresource_range = VkImageSubresourceRange {
            aspectMask: VkImageAspectFlagBits::VK_IMAGE_ASPECT_DEPTH_BIT as VkImageAspectFlags,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: 0,
            layerCount: 1,
        };

        unsafe {
            vkCmdClearDepthStencilImage(self.cleardepth_command_buffers[self.image_index].raw,
                                        self.current_depth_target.unwrap(),
                                        VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        &clear_stencil,
                                        1, // Subrange count
                                        &subresource_range);
        }

        RendererVkImage::transition_layout(self.current_depth_target.unwrap(),
                                           &self.cleardepth_command_buffers[self.image_index],
                                           VkImageAspectFlagBits::VK_IMAGE_ASPECT_DEPTH_BIT as VkImageAspectFlags,
                                           VkImageLayout::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                           VkImageLayout::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TRANSFER_BIT
                                               as VkPipelineStageFlags,
                                           VkPipelineStageFlagBits::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT
                                               as VkPipelineStageFlags);

        self.cleardepth_command_buffers[self.image_index].end();

        // Submit the command buffer to the queue
        //
        let mut command_buffers = Vec::with_capacity(1);
        command_buffers.push(self.cleardepth_command_buffers[self.image_index].raw);

        let wait_semaphores: Vec<VkSemaphore> = vec![];
        let wait_stages: Vec<VkPipelineStageFlags> =
            vec![VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags];
        let signal_semaphores = vec![];

        let submit_info = VkSubmitInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SUBMIT_INFO,
            waitSemaphoreCount: wait_semaphores.len() as u32,
            pWaitSemaphores: wait_semaphores.as_ptr(),
            pWaitDstStageMask: wait_stages.as_ptr(),
            commandBufferCount: command_buffers.len() as u32,
            pCommandBuffers: command_buffers.as_ptr(),
            signalSemaphoreCount: signal_semaphores.len() as u32,
            pSignalSemaphores: signal_semaphores.as_ptr(),
            pNext: ptr::null(),
        };
        unsafe {
            // TODO: Deal with VK_ERROR_DEVICE_LOST result
            check_result!("vkQueueSubmit",
                          vkQueueSubmit(self.device.graphics_queue,
                                        1,
                                        &submit_info,
                                        VK_NULL_HANDLE_MUT() /* Fence */));

            check_result!("vkQueueWaitIdle",
                          vkQueueWaitIdle(self.device.graphics_queue));
        }
    }

    /// Convert a renderer primitive type to an OpenGL primitive type
    fn primitive(&self, _: PrimitiveType) -> u32 {
        0
    }

    /// Set a integer in part of the memory put aside for the named uniform buffer
    ///
    /// buffer_name: The name of the uniform buffer to contain the new value
    /// uniform_name: The name of the uniform whose value should be set
    /// value: The value to set for the uniform
    fn set_uniform_buffer_int(&self, buffer_name: &str, uniform_name: &str, value: i32) {
        let ref buffer = self.uniform_buffers[buffer_name];
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
        let ref buffer = self.uniform_buffers[buffer_name];
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
        let ref buffer = self.uniform_buffers[buffer_name];
        let offset = buffer.offsets[uniform_name];
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
    fn set_uniform_buffer_matrix(&self, buffer_name: &str, uniform_name: &str, matrix: &Mat4<f32>) {
        let ref buffer = self.uniform_buffers[buffer_name];
        let offset = buffer.offsets[uniform_name];
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
    fn set_uniform_buffer_float_vector(&self, buffer_name: &str, uniform_name: &str, vector: &Vec<f32>) {
        let ref buffer = self.uniform_buffers[buffer_name];
        let offset = buffer.offsets[uniform_name];
        let stride = buffer.strides[uniform_name];
        if stride == 0 || stride == 4 {
            unsafe {
                let dst: *const u8 = buffer.bytes.as_ptr().offset(offset as isize);
                let dst_f32 = dst as *mut f32;
                let src: *const f32 = mem::transmute(vector.as_ptr());
                ptr::copy_nonoverlapping(src, dst_f32, vector.len());
            }
        } else {
            // This path requires observing the stride
            unsafe {
                for i in 0..vector.len() {
                    let dst: *const u8 = buffer.bytes.as_ptr().offset((i * stride + offset) as isize);
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
        let ref buffer = self.uniform_buffers[buffer_name];
        // println!("Synchronising {} ({} bytes, binding = {})",
        //          buffer_name,
        //          buffer.size,
        //          buffer.binding);
        // dump_byte_vector(&buffer.bytes);
        unsafe {
            let mut data: *mut c_void = VK_NULL_HANDLE_MUT();
            check_result!("vkMapMemory",
                          vkMapMemory(self.device.raw,
                                      buffer.buffer.memory,
                                      0, // Offset
                                      VK_WHOLE_SIZE as u64,
                                      0, // Flags
                                      &mut data));
            ptr::copy_nonoverlapping(buffer.bytes.as_ptr(), data as *mut u8, buffer.buffer.size);
            vkUnmapMemory(self.device.raw, buffer.buffer.memory);
        }
    }

    /// Flip the back buffer to the front
    ///
    /// context: The GLFW context, not used on Vulkan
    fn flip(&self, _: &mut Context) {
        let signal_semaphores = vec![self.render_finished_semaphore.raw];
        let swapchains = vec![self.swapchain.raw];
        let image_indices = vec![self.image_index as u32];

        let present_info = VkPresentInfoKHR {
            sType: VkStructureType::VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
            waitSemaphoreCount: signal_semaphores.len() as u32,
            pWaitSemaphores: signal_semaphores.as_ptr(),
            swapchainCount: swapchains.len() as u32,
            pSwapchains: swapchains.as_ptr(),
            pImageIndices: image_indices.as_ptr(),
            pResults: ptr::null_mut(), // Optional
            pNext: ptr::null(),
        };

        unsafe {
            check_result!("vkQueuePresentKHR",
                          vkQueuePresentKHR(self.device.graphics_queue, &present_info));

            check_result!("vkQueueWaitIdle",
                          vkQueueWaitIdle(self.device.graphics_queue));
        }
    }

    /// Begin rendering a new frame
    fn begin_frame(&mut self) {
        // Acquire the next image in the swapchain
        //
        let mut image_index: u32 = 0;
        unsafe {
            // TODO: Deal with VK_SUBOPTIMAL_KHR and VK_ERROR_OUT_OF_DATE_KHR results
            check_result!("vkAcquireNextImageKHR",
                          vkAcquireNextImageKHR(self.device.raw,
                                                self.swapchain.raw,
                                                u64::max_value(), // No timeout
                                                self.image_available_semaphore.raw, // Semaphore
                                                VK_NULL_HANDLE_MUT(), // Fence
                                                &mut image_index));
        }
        self.image_index = image_index as usize;

        // Set the default render target
        self.deselect_render_target();
    }

    /// Terminate rendering a frame
    fn end_frame(&mut self) {
        // Add a pipeline barrier to ensure all the thread command buffers have finished before presenting
        //
        unsafe {
            check_result!("vkResetCommandBuffer",
                          vkResetCommandBuffer(self.prepresent_command_buffers[self.image_index].raw,
                                               0 /* flags */));
        }

        self.prepresent_command_buffers[self.image_index].begin_primary(true, // one_time_submit
                                                                        false, // render_pass_continue
                                                                        true /* simultaneous_use */);

        self.prepresent_command_buffers[self.image_index]
            .image_memory_barrier(self.swapchain.images[self.image_index],
                                  VkAccessFlagBits::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT as VkAccessFlags,
                                  VkAccessFlagBits::VK_ACCESS_MEMORY_READ_BIT as VkAccessFlags,
                                  VkImageLayout::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                  VkImageLayout::VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
                                  VkImageAspectFlagBits::VK_IMAGE_ASPECT_COLOR_BIT as VkImageAspectFlags,
                                  VkPipelineStageFlagBits::VK_PIPELINE_STAGE_ALL_GRAPHICS_BIT as VkPipelineStageFlags,
                                  VkPipelineStageFlagBits::VK_PIPELINE_STAGE_ALL_GRAPHICS_BIT as VkPipelineStageFlags);

        self.prepresent_command_buffers[self.image_index].end();

        // Submit the command buffer to the queue
        //
        let mut command_buffers = Vec::with_capacity(1);
        command_buffers.push(self.prepresent_command_buffers[self.image_index].raw);

        let wait_semaphores: Vec<VkSemaphore> = vec![self.image_available_semaphore.raw];
        let wait_stages: Vec<VkPipelineStageFlags> =
            vec![VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags];
        let signal_semaphores = vec![self.render_finished_semaphore.raw];

        let submit_info = VkSubmitInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SUBMIT_INFO,
            waitSemaphoreCount: wait_semaphores.len() as u32,
            pWaitSemaphores: wait_semaphores.as_ptr(),
            pWaitDstStageMask: wait_stages.as_ptr(),
            commandBufferCount: command_buffers.len() as u32,
            pCommandBuffers: command_buffers.as_ptr(),
            signalSemaphoreCount: signal_semaphores.len() as u32,
            pSignalSemaphores: signal_semaphores.as_ptr(),
            pNext: ptr::null(),
        };
        unsafe {
            // TODO: Deal with VK_ERROR_DEVICE_LOST result
            check_result!("vkQueueSubmit",
                          vkQueueSubmit(self.device.graphics_queue,
                                        1,
                                        &submit_info,
                                        VK_NULL_HANDLE_MUT() /* Fence */));
        }
    }

    /// Begin a pass with the specified shader
    ///
    ///
    fn begin_pass(&mut self, shader_name: &'static str) {
        self.shader_name = shader_name;

        {
            let res_manager = self.resource_manager.lock().unwrap();
            let ref shader_spec = res_manager.shader_specs[shader_name];
            self.vertex_array_type = shader_spec.vertex_array_type;
            self.current_pass_identifier = shader_spec.pass_identifier;
        }

        for thr in 0..self.max_threads {
            unsafe {
                check_result!("vkResetCommandBuffer",
                              vkResetCommandBuffer(self.command_buffers[self.image_index][thr].raw,
                                                   0 /* flags */));
            }

            self.command_buffers[self.image_index][thr].begin_primary(true, // one_time_submit
                                                                      false, // render_pass_continue
                                                                      true /* simultaneous_use */);

            self.render_passes[self.current_pass_identifier as usize].begin(self.command_buffers[self.image_index][thr].raw,
                                                                            self.current_render_target.unwrap(),
                                                                            self.surface.capabilities.currentExtent.width,
                                                                            self.surface.capabilities.currentExtent.height);
        }

        for ty in VERTEX_ARRAY_TYPE_BEGIN_RANGE..VERTEX_ARRAY_TYPE_END_RANGE + 1 {
            for thr in 0..self.max_threads {
                self.vertex_buffer_index[self.image_index][ty as usize][thr] = -1;
            }
        }

        for thr in 0..self.max_threads {
            unsafe {
                vkCmdBindPipeline(self.command_buffers[self.image_index][thr].raw,
                                  VkPipelineBindPoint::VK_PIPELINE_BIND_POINT_GRAPHICS,
                                  self.render_pipelines[shader_name].raw);
            }

            let descriptor_sets = vec![self.descriptor_sets[self.shader_name].raw];
            unsafe {
                vkCmdBindDescriptorSets(self.command_buffers[self.image_index][thr].raw,
                                        VkPipelineBindPoint::VK_PIPELINE_BIND_POINT_GRAPHICS,
                                        self.pipeline_layouts[self.shader_name].raw,
                                        0, // First set
                                        descriptor_sets.len() as u32,
                                        descriptor_sets.as_ptr(),
                                        0, // Dynamic offset count
                                        ptr::null()); // Dynamic offsets
            }
        }
    }

    /// Finish a pass with the specified shader
    fn end_pass(&mut self) {
        // Finish the command buffers and render passes and store the command buffers in a
        // collection to be submitted
        //
        let mut command_buffers = Vec::with_capacity(self.max_threads);
        for thr in 0..self.max_threads {
            self.render_passes[self.current_pass_identifier as usize].end(self.command_buffers[self.image_index][thr].raw);

            self.command_buffers[self.image_index][thr].end();
            command_buffers.push(self.command_buffers[self.image_index][thr].raw);
        }

        // Submit the command buffers to the queue
        //
        let wait_semaphores: Vec<VkSemaphore> = vec![];
        let wait_stages: Vec<VkPipelineStageFlags> =
            vec![VkPipelineStageFlagBits::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as VkPipelineStageFlags];
        let signal_semaphores = vec![];

        let submit_info = VkSubmitInfo {
            sType: VkStructureType::VK_STRUCTURE_TYPE_SUBMIT_INFO,
            waitSemaphoreCount: wait_semaphores.len() as u32,
            pWaitSemaphores: wait_semaphores.as_ptr(),
            pWaitDstStageMask: wait_stages.as_ptr(),
            commandBufferCount: command_buffers.len() as u32,
            pCommandBuffers: command_buffers.as_ptr(),
            signalSemaphoreCount: signal_semaphores.len() as u32,
            pSignalSemaphores: signal_semaphores.as_ptr(),
            pNext: ptr::null(),
        };
        unsafe {
            // TODO: Deal with VK_ERROR_DEVICE_LOST result
            check_result!("vkQueueSubmit",
                          vkQueueSubmit(self.device.graphics_queue,
                                        1,
                                        &submit_info,
                                        VK_NULL_HANDLE_MUT() /* Fence */));

            check_result!("vkQueueWaitIdle",
                          vkQueueWaitIdle(self.device.graphics_queue));
        }
    }

    /// Select the render target so that renderpasses output there instead of the swapchain
    ///
    /// num: The texture number to bind the render target texture to
    /// render_target: The render target to select
    fn select_render_target(&mut self, _: i32, render_target: &mut RenderTarget) {
        let target_vk = match render_target.as_any_mut().downcast_mut::<RenderTargetVk>() {
            Some(r) => r,
            None => panic!("Unexpected runtime type"),
        };

        self.current_render_target = Some(target_vk.get_framebuffer_raw());
        self.current_depth_target = Some(target_vk.get_depth_image_raw());
    }

    /// Deselect the render target so that renderpasses output to the swapchain
    fn deselect_render_target(&mut self) {
        self.current_render_target = Some(self.framebuffers[self.image_index].raw);
        self.current_depth_target = None;
    }
}

impl RendererVk {
    /// Flush the calculated vertex data
    ///
    /// renderer_arc: Atomic reference counted lockable reference to the
    ///     renderer, only used when single_threaded
    /// thread_data: The structure containing the vertex data
    pub fn flush<Rend: Renderer + ?Sized>(renderer_arc: Arc<Mutex<&mut Rend>>, thread_data: &ThreadData) {
        if thread_data.index == 0 {
            return;
        }

        let thr = thread_data.thr as usize;

        let ty;
        let image_index;
        let device;
        let command_buffer_raw;
        let vertex_buffer_memory;
        let vertex_buffer_raw;
        {
            let mut renderer = renderer_arc.lock().unwrap();

            let renderer_vk: &mut RendererVk = match renderer.as_any_mut().downcast_mut::<RendererVk>() {
                Some(r) => r,
                None => panic!("Unexpected runtime type"),
            };

            ty = renderer_vk.vertex_array_type;
            image_index = renderer_vk.image_index;

            // Move on to the next vertex buffer of the current type, creating one if necessary
            //
            renderer_vk.vertex_buffer_index[image_index][ty as usize][thr] += 1;
            if renderer_vk.vertex_buffer_index[image_index][ty as usize][thr] ==
               renderer_vk.vertex_buffer[image_index][ty as usize][thr].len() as i32 {
                renderer_vk.vertex_buffer[image_index as usize][ty as usize][thr]
                    .push(RendererVkVertexBuffer::new(&renderer_vk.device, &renderer_vk.physical_device, ty));
            }

            device = renderer_vk.device.raw;
            command_buffer_raw = renderer_vk.command_buffers[image_index][thr].raw;

            let vb_index = renderer_vk.vertex_buffer_index[image_index][ty as usize][thr] as usize;
            vertex_buffer_memory = renderer_vk.vertex_buffer[image_index][ty as usize][thr][vb_index].buffer.memory;
            vertex_buffer_raw = renderer_vk.vertex_buffer[image_index][ty as usize][thr][vb_index].buffer.raw;
        }

        {
            let components_per_triangle = 3 * VertexArrayType::components_per_vertex(ty);

            // println!("---");
            // println!("{} triangles", thread_data.index);
            // use misc::fileutils;
            // fileutils::dump_float_vector(&thread_data.data, components_per_triangle * thread_data.index, 6 /* columns */);

            let mut raw_buffer: *mut c_void = ptr::null_mut();
            unsafe {
                // TODO: Could leave this mapped and then explicitly synchronise after the copy
                check_result!("vkMapMemory",
                              vkMapMemory(device,
                                          vertex_buffer_memory,
                                          0, // Offset
                                          (components_per_triangle * thread_data.index * mem::size_of::<f32>()) as u64,
                                          0, // Flags, reserved
                                          &mut raw_buffer));

                ptr::copy_nonoverlapping(thread_data.data.as_ptr(),
                                         raw_buffer as *mut f32,
                                         components_per_triangle * thread_data.index); // Words

                vkUnmapMemory(device, vertex_buffer_memory);
            }

            let vertex_buffers = vec![vertex_buffer_raw];
            let buffer_offsets: Vec<VkDeviceSize> = vec![0];
            unsafe {
                vkCmdBindVertexBuffers(command_buffer_raw,
                                       0, // First binding
                                       vertex_buffers.len() as u32,
                                       vertex_buffers.as_ptr(),
                                       buffer_offsets.as_ptr());

                // In fact, at the moment apart from the font, the geometry is entirely static if you
                // ignore CPU-based culling.  The tessellation and geometry shaders do almost all the
                // work, so constructing the command buffers could be done once up front and then just
                // execute them into the queue each frame.  However, due to the procedural nature of
                // the application, and where it may head in the future, it warrants keeping it
                // generating the command buffers from scratch every frame.
                vkCmdDraw(command_buffer_raw,
                          3 * thread_data.index as u32, // Vertex count
                          1, // Instance count
                          0, // First vertex
                          0); // First instance
            }
        }
    }
}
