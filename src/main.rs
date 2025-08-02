//! Vulkan Application with the goal to render a triangle.
#![allow(
    dead_code,
    unused_variables,
    unsafe_op_in_unsafe_fn,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use std::{
    collections::HashSet,
    ffi::{c_void, CStr},
};

use anyhow::{anyhow, Context, Result};
use log::*;
use thiserror::Error;
use vulkanalia::{
    bytecode::Bytecode, loader::{LibloadingLoader, LIBRARY}, vk, vk::{
        DeviceV1_0, EntryV1_0, ExtDebugUtilsExtension, Handle, HasBuilder, InstanceV1_0,
        KhrSurfaceExtension, KhrSwapchainExtension, PhysicalDeviceType, SurfaceKHR,
    },
    window as vk_window,
    Device,
    Entry,
    Instance,
    Version,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

// ---------------------------------------------
// Macros
// ---------------------------------------------

/// Include a `.spv` SPIR-V bytecode file from the build script's OUT_DIR at compile time.
macro_rules! include_spirv {
    ($name:expr) => {
        include_bytes!(concat!(env!("SHADER_OUT_DIR"), "/", $name, ".spv"))
    };
}

// ---------------------------------------------
// Global Constants
// ---------------------------------------------

/// Whether Vulkan validation layers should be enabled or not.
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

/// The Vulkan validation layer name as a safe CString.
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(c"VK_LAYER_KHRONOS_validation".to_bytes_with_nul());

/// The Vulkan SDK version that started requiring the portability subset extension for macOS.
const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

/// The Vulkan physical device level extensions. Here, we list the "VK_KHR_swapchain" extension,
/// later useful for checking if the GPU has swap chain support.
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

/// Contains the vertex shader's compiled SPIR-V bytecode contents
const VERTEX_BYTECODE: &[u8] = include_spirv!("triangle.vert");

/// Contains the fragment shader's compiled SPIR-V bytecode contents
const FRAGMENT_BYTECODE: &[u8] = include_spirv!("triangle.frag");

// ---------------------------------------------
// Entrypoint
// ---------------------------------------------

fn main() -> Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "trace");
    }
    pretty_env_logger::init();

    // Create the event loop.
    let event_loop = EventLoop::new().context("Failed to create event loop.")?;

    // Create the window.
    let window = WindowBuilder::new()
        .with_title("Vulkan App")
        .with_inner_size(LogicalSize::new(600, 400))
        .with_active(false)
        .build(&event_loop)
        .context("Failed to create window.")?;

    // Create the Vulkan app
    let mut app = unsafe { App::create(&window).context("Failed to create Vulkan App.")? };

    // Run the event loop
    event_loop.run(move |event, window_target| {
        match event {
            // Request a redraw when all events have been processed (so we can render continuously
            // even when there is no input from the OS)
            Event::AboutToWait => window.request_redraw(),

            // Check for window events (e.g. keyboard input, window close, etc.)
            Event::WindowEvent { event, .. } => match event {
                // Render a frame if the App is not being destroyed.
                WindowEvent::RedrawRequested if !window_target.exiting() => {
                    unsafe { app.render(&window) }.unwrap();
                }

                // Destroy the App.
                WindowEvent::CloseRequested => {
                    window_target.exit();
                    unsafe {
                        app.destroy();
                    }
                }

                _ => {}
            },

            _ => {}
        }
    })?;

    Ok(())
}

// ---------------------------------------------
// Structs
// ---------------------------------------------

/// Intermediate Vulkan handles which get bundled into `App`.
#[derive(Clone, Debug, Default)]
struct AppData {
    messenger: vk::DebugUtilsMessengerEXT,
    surface: SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

/// Main `App` which implements Vulkan boilerplate functionality.
#[derive(Clone, Debug)]
struct App {
    entry: Entry,
    instance: Instance,
    device: Device,
    data: AppData,
}

/// Obtains information about Vulkan swapchain availability.
#[derive(Clone, Debug)]
struct SwapchainSupport {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

/// A family queue contains different operations available on a physical device, (e.g. graphics,
/// compute, transfer). These 'families' are identified by their index, starting from 0. The
/// most important queue is the graphics queue, as this gives us the capability to render.
#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
    present: u32,
}

/// Custom error handling for checking suitability of Vulkan components.
#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);

// ---------------------------------------------
// Vulkan Application
// ---------------------------------------------

impl App {
    /// Initializes Vulkan resources tied to App.
    unsafe fn create(window: &Window) -> Result<Self> {
        let (entry, instance, data, device) = unsafe {
            let loader = LibloadingLoader::new(LIBRARY)?;
            let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
            let mut data = AppData::default();
            let instance = create_instance(window, &entry, &mut data)?;
            data.surface = create_surface(&instance, &window)?;
            pick_physical_device(&instance, &mut data)?;
            let device = create_logical_device(&entry, &instance, &mut data)?;
            create_swapchain(window, &instance, &device, &mut data)?;
            create_swapchain_image_views(&device, &mut data)?;
            create_render_pass(&instance, &device, &mut data)?;
            create_pipeline(&device, &mut data)?;

            (entry, instance, data, device)
        };

        Ok(Self {
            entry,
            instance,
            device,
            data,
        })
    }

    /// Renders a frame for our Vulkan App.
    unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// Destroys Vulkan resources tied to App.
    unsafe fn destroy(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.data.pipeline, None);
            trace!("Destroyed Vulkan pipeline object.");

            self.device
                .destroy_pipeline_layout(self.data.pipeline_layout, None);
            trace!("Destroyed Vulkan pipeline layout.");

            self.device.destroy_render_pass(self.data.render_pass, None);
            trace!("Destroyed Vulkan render pass.");

            self.data.swapchain_image_views.iter().for_each(|v| {
                self.device.destroy_image_view(*v, None);
            });

            self.device.destroy_swapchain_khr(self.data.swapchain, None);
            trace!("Destroyed Vulkan swapchain.");

            self.device.destroy_device(None);
            trace!("Destroyed Vulkan logical device.");

            self.instance.destroy_surface_khr(self.data.surface, None);
            trace!("Destroyed Vulkan surface.");

            if VALIDATION_ENABLED {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.data.messenger, None);
                trace!("Destroyed Vulkan debug messenger.")
            }

            self.instance.destroy_instance(None);
            trace!("Destroyed Vulkan instance.")
        }
    }
}

// ---------------------------------------------
// Graphics Pipeline
// ---------------------------------------------

unsafe fn create_pipeline(device: &Device, data: &mut AppData) -> Result<()> {
    let vertex_shader_module = create_shader_module(device, VERTEX_BYTECODE)?;
    let fragment_shader_module = create_shader_module(device, FRAGMENT_BYTECODE)?;

    let vertex_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader_module)
        .name(b"main\0");

    let fragment_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader_module)
        .name(b"main\0");

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(data.swapchain_extent.width as f32)
        .height(data.swapchain_extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(data.swapchain_extent);

    let viewports = &[viewport];
    let scissors = &[scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(viewports)
        .scissors(scissors);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::_1);

    let attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::all())
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD);

    let attachments = &[attachment];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    let layout_info = vk::PipelineLayoutCreateInfo::builder();
    data.pipeline_layout = device.create_pipeline_layout(&layout_info, None)?;
    trace!("Created Vulkan pipeline layout.");

    let stages = &[vertex_stage, fragment_stage];
    let info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .layout(data.pipeline_layout)
        .render_pass(data.render_pass)
        .subpass(0);

    data.pipeline = device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
        .0[0];
    trace!("Created Vulkan pipeline object.");

    // Once the pipeline is created, SPIR-V bytecode is compiled into binary for execution by the
    // GPU, at which point the shader modules have no purpose and should be immediately terminated.
    // This is why shader modules are local to this function rather than fields in AppData.
    device.destroy_shader_module(vertex_shader_module, None);
    device.destroy_shader_module(fragment_shader_module, None);

    Ok(())
}

/// A Vulkan shader module is a container for the SPIR-V bytecode to be explicitly validated and
/// formatted internally by the implementation until it is linked to the Vulkan graphics pipeline.
/// If the same shader module is used for multiple pipelines, caching will improve performance.

/// Before creating the `Shader Module`, it is required to convert the shader bytecode from `u8` to
/// `u32` because Vulkan expects it that way.
unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    let bytecode = Bytecode::new(bytecode)?;

    let info = vk::ShaderModuleCreateInfo::builder()
        .code(bytecode.code())
        .code_size(bytecode.code_size());

    Ok(device.create_shader_module(&info, None)?)
}

unsafe fn create_render_pass(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let color_attachment = vk::AttachmentDescription::builder()
        .format(data.swapchain_format)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = &[color_attachment_ref];
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments);

    let attachments = &[color_attachment];
    let subpasses = &[subpass];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(attachments)
        .subpasses(subpasses);

    data.render_pass = device.create_render_pass(&info, None)?;
    trace!("Created the Vulkan render pass.");

    Ok(())
}

// ---------------------------------------------
// Swapchain
// ---------------------------------------------

/// In Vulkan, the `Swapchain` is essentially a queue of images waiting be presented to the screen.
/// The conditions of how the queue works and how images are presented/synchronized can be
/// configured explicitly, but the general purpose remains the same.
///
/// The swapchain works with the GPU obtaining an image from the queue, rendering
/// to it, and returning it back to the queue to be presented. The swapchain uses synchronization to
/// ensure that an image was fully rendered to before presenting to the screen.
///
/// Because Vulkan is platform and hardware agnostic, presentation is handled
/// via an extension rather than a default framebuffer, which must be obtained. This extension is
/// not guaranteed to be available, although most modern GPUs support swap chain functionality.

impl SwapchainSupport {
    /// Gets Vulkan swapchain capabilities, surface formats, and presentation modes.
    unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        Ok(Self {
            capabilities: instance
                .get_physical_device_surface_capabilities_khr(physical_device, data.surface)?,
            formats: instance
                .get_physical_device_surface_formats_khr(physical_device, data.surface)?,
            present_modes: instance
                .get_physical_device_surface_present_modes_khr(physical_device, data.surface)?,
        })
    }
}

unsafe fn create_swapchain(
    window: &Window,
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
    let support = SwapchainSupport::get(instance, data, data.physical_device)?;

    let surface_format = get_swapchain_surface_format(&support.formats);
    let present_mode = get_swapchain_present_mode(&support.present_modes);
    let extent = get_swapchain_extent(window, support.capabilities);

    let mut image_count = support.capabilities.min_image_count + 1;
    // `0` here is a special value that means there is no maximum
    if support.capabilities.max_image_count != 0
        && image_count > support.capabilities.max_image_count
    {
        image_count = support.capabilities.max_image_count;
    }

    let mut queue_family_indices = vec![];
    let image_sharing_mode = if indices.graphics != indices.present {
        queue_family_indices.push(indices.graphics);
        queue_family_indices.push(indices.present);
        vk::SharingMode::CONCURRENT
    } else {
        vk::SharingMode::EXCLUSIVE
    };

    let info = vk::SwapchainCreateInfoKHR::builder()
        .surface(data.surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(image_sharing_mode)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null());

    data.swapchain = device.create_swapchain_khr(&info, None)?;
    trace!("Created Vulkan swapchain.");

    data.swapchain_images = device.get_swapchain_images_khr(data.swapchain)?;
    data.swapchain_format = surface_format.format;
    data.swapchain_extent = extent;

    Ok(())
}

fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .cloned()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| formats[0])
}

fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

fn get_swapchain_extent(window: &Window, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D::builder()
            .width(window.inner_size().width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ))
            .height(window.inner_size().height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ))
            .build()
    }
}

unsafe fn create_swapchain_image_views(device: &Device, data: &mut AppData) -> Result<()> {
    data.swapchain_image_views = data
        .swapchain_images
        .iter()
        .map(|i| {
            let components = vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY);

            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let info = vk::ImageViewCreateInfo::builder()
                .image(*i)
                .view_type(vk::ImageViewType::_2D)
                .format(data.swapchain_format)
                .components(components)
                .subresource_range(subresource_range);

            device.create_image_view(&info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(())
}

// ---------------------------------------------
// Window Surface
// ---------------------------------------------

/// The Vulkan `Surface` is simply an *abstraction* of the native windowing or display target you
/// want to present images to. It serves as the "glue" between Vulkan's generic presentation API
/// and whatever the OS's underlying window-library actually provides.
///
/// Vulkan by itself doesn't know about windows or screens due to its platform-agnostic nature.
/// It relies on `WSI` (window system integration) KHR extensions to tell it where to show
/// rendered images to, which is essentially what the surface does.
unsafe fn create_surface(instance: &Instance, window: &Window) -> Result<SurfaceKHR> {
    let surface_raw = unsafe { vk_window::create_surface(instance, window, window) };

    let surface = surface_raw.map_err(|e| {
        error!("Failed to create Vulkan KHR surface: {}", e);
        e
    })?;

    trace!("Created Vulkan KHR surface.");

    Ok(surface)
}

// ---------------------------------------------
// Logical Device
// ---------------------------------------------

/// The Vulkan physical device is just a description of the **GPU**. The Vulkan physical device
/// never talks to the GPU directly. The `Logical Device` is simply a handle to that physical device
/// to do all the *real* work. The logical device is a configured interface to the GPU and allows us
/// to allocate memory or submit any work to Vulkan queues.
///
/// Multiple logical devices can be created from one physical device if we wanted different
/// *contexts* with different enabled features or queue priorities. Also, multiple physical
/// devices can be created if there are multiple GPUs detected, each with their own logical device
/// for a multi-GPU setup.
unsafe fn create_logical_device(
    entry: &Entry,
    instance: &Instance,
    data: &mut AppData,
) -> Result<Device> {
    let indices = unsafe { QueueFamilyIndices::get(instance, data, data.physical_device)? };

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.present);

    // Set the queue priority to highest and provide the queue create info with the family index
    // and priority float.
    let queue_priorities = &[1.0];

    // A single logical device can be created with multiple queues from different queue families,
    // each with their own capabilities and priority. (in this case we only are using graphics
    // queue)
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    // Give the device level layer validation layer support (for compatibility of older
    // vulkan implementations).
    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // Device-level extension for the swapchain KHR functionality.
    let mut extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|n| n.as_ptr())
        .collect::<Vec<_>>();

    // Give the device level extensions portability support if the host is running macOS.
    if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    // Get the physical device's supported features.
    let features = vk::PhysicalDeviceFeatures::builder();

    // Setup logical device create info with all queue info structs, layers, extensions, and
    // features.
    let device_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    // Create the Vulkan logical device.

    let device = unsafe {
        match instance.create_device(data.physical_device, &device_info, None) {
            Ok(device) => {
                trace!("Created Vulkan logical device.");
                device
            }

            Err(e) => {
                error!("Failed to create Vulkan logical device.");
                return Err(e.into());
            }
        }
    };

    // Set the graphics queue to the graphics family index.
    data.graphics_queue = unsafe { device.get_device_queue(indices.graphics, 0) };
    data.present_queue = unsafe { device.get_device_queue(indices.present, 0) };

    Ok(device)
}

// ---------------------------------------------
// Queue Families
// ---------------------------------------------

impl QueueFamilyIndices {
    /// Get the index of the queue family which supports the "graphics" operation and
    /// return it as an u32. If no family is found with graphics support, the function returns
    /// with an error, otherwise the u32 index is returned (usually 0).
    unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        // Iterate through the physical device queue family properties for the graphics queue flags.
        // If the graphics flag exists, convert it to u32 (index).
        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        let mut present = None;
        for (index, properties) in properties.iter().enumerate() {
            let present_support = unsafe {
                instance.get_physical_device_surface_support_khr(
                    physical_device,
                    index as u32,
                    data.surface,
                )?
            };

            if present_support {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            Err(anyhow!(SuitabilityError(
                "Missing required queue families."
            )))
        }
    }
}

// ---------------------------------------------
// Physical Device
// ---------------------------------------------

/// Check whether the Vulkan physical device contains a geometry shader feature, and make
/// sure the GPU is either integrated or discrete. If any of these are not detected, we return
/// with an error.
unsafe fn check_physical_device(
    instance: &Instance,
    data: &AppData,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let features = unsafe { instance.get_physical_device_features(physical_device) };

    if properties.device_type != PhysicalDeviceType::DISCRETE_GPU
        && properties.device_type != PhysicalDeviceType::INTEGRATED_GPU
    {
        return Err(anyhow!(SuitabilityError(
            "Only discrete and integrated GPUs are supported."
        )));
    } else if features.geometry_shader != vk::TRUE {
        return Err(anyhow!(SuitabilityError(
            "Missing geometry shader support."
        )));
    } else {
        trace!("Found physical device: {}", properties.device_name);
    }

    unsafe { QueueFamilyIndices::get(instance, data, physical_device) }?;
    unsafe { check_physical_device_extensions(instance, physical_device)? };

    let support = SwapchainSupport::get(instance, data, physical_device)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        return Err(anyhow!(SuitabilityError("Insufficient swapchain support.")));
    }

    Ok(())
}

/// Ensure the physical device supports the required swap chain extension.
unsafe fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device, None)?
        .iter()
        .map(|ext| ext.extension_name)
        .collect::<HashSet<_>>();

    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        Err(anyhow!(SuitabilityError(
            "Missing required device extensions."
        )))
    }
}

/// Select a suitable Vulkan physical device (GPU) based on its properties and features.
fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()> {
    for physical_device in unsafe { instance.enumerate_physical_devices()? } {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };

        match unsafe { check_physical_device(instance, data, physical_device) } {
            Ok(_) => {
                data.physical_device = physical_device;
                return Ok(());
            }
            Err(_) => {
                warn!("Skipping physical device (`{}`)", properties.device_name);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------
// Debug Messenger
// ---------------------------------------------

/// Vulkan debug callback for explicit control over what the validation layers will print to
/// standard output.
extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message).to_string_lossy() };

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
    }

    vk::FALSE
}

// ---------------------------------------------
// Instance
// ---------------------------------------------

/// Create the Vulkan instance.
unsafe fn create_instance(window: &Window, entry: &Entry, data: &mut AppData) -> Result<Instance> {
    // Optional information to specify to the Vulkan driver.
    let application_info = vk::ApplicationInfo::builder()
        .application_name(c"Vulkan App".to_bytes_with_nul())
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(c"No Engine".to_bytes_with_nul())
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 0, 0));

    // Retrieve the Vulkan available instance layers.
    let available_layers = unsafe {
        let available_layers = entry
            .enumerate_instance_layer_properties()?
            .iter()
            .map(|l| l.layer_name)
            .collect::<HashSet<_>>();

        available_layers
    };

    if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
        return Err(anyhow!(
            "Vulkan Validation layer \"{}\" requested, but not supported.",
            VALIDATION_LAYER
        ));
    }

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    // Retrieve required instance platform extensions.
    let mut extensions = vk_window::get_required_instance_extensions(window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    if VALIDATION_ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    for &ext in &extensions {
        let name = unsafe { CStr::from_ptr(ext) }.to_string_lossy();
    }

    // Required by Vulkan SDK on macOS since 1.3.216.
    let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        trace!("Enabling extensions for macOS portability.");
        extensions.push(
            vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                .name
                .as_ptr(),
        );
        extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::empty()
    };

    // Populate the CreateInfo struct with the ApplicationInfo struct, and the enabled layers,
    // extensions, and portability flags.
    let mut create_info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .flags(flags);

    // Populate DebugInfo struct with all severity flags, message types, and add
    // `debug_callback()` to the struct.
    let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .user_callback(Some(debug_callback));

    // Add DebugInfo struct to CreateInfo struct as an extension so the debug messenger can validate
    // instance creation and destruction.
    if VALIDATION_ENABLED {
        create_info = create_info.push_next(&mut debug_info)
    }

    // Create the Vulkan instance.
    // let instance = unsafe { entry.create_instance(&create_info, None)? };
    let instance = unsafe {
        match entry.create_instance(&create_info, None) {
            Ok(instance) => {
                trace!("Created Vulkan instance.");
                instance
            }
            Err(e) => {
                error!("Failed to created Vulkan instance: {:?}", e);
                return Err(e.into());
            }
        }
    };

    // Create the Vulkan debug messenger.
    if VALIDATION_ENABLED {
        data.messenger = unsafe { instance.create_debug_utils_messenger_ext(&debug_info, None)? };
        trace!("Created Vulkan debug messenger.");
    }

    Ok(instance)
}