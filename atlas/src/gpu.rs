use std::{
    ops::{self},
    sync::Arc,
};

use crate::egui_state;

use paste::paste;

pub enum Primitive {
    U32,
    Vec2U32,
    Vec3U32,
    Vec4U32,
    Mat3x3U32,
    Mat4x4U32,

    F32,
    Vec2F32,
    Vec3F32,
    Vec4F32,
    Mat3x3F32,
    Mat4x4F32,

    F64,
    Vec2F64,
    Vec3F64,
    Vec4F64,
    Mat3x3F64,
    Mat4x4F64,
}

impl Primitive {
    const fn to_vertex_format(&self) -> wgpu::VertexFormat {
        use Primitive as P;
        use wgpu::VertexFormat as VF;
        match self {
            P::U32 => VF::Uint32,
            P::Vec2U32 => VF::Uint32x2,
            P::Vec3U32 => VF::Uint32x3,
            P::Vec4U32 => VF::Uint32x4,
            P::F32 => VF::Float32,
            P::Vec2F32 => VF::Float32x2,
            P::Vec3F32 => VF::Float32x3,
            P::Vec4F32 => VF::Float32x4,
            P::F64 => VF::Float64,
            P::Vec2F64 => VF::Float64x2,
            P::Vec3F64 => VF::Float64x3,
            P::Vec4F64 => VF::Float64x4,
            P::Mat3x3U32
            | P::Mat4x4U32
            | P::Mat3x3F64
            | P::Mat4x4F64
            | P::Mat3x3F32
            | P::Mat4x4F32 => panic!(),
        }
    }

    const fn as_wgpu_ty(&self) -> &'static str {
        use Primitive as P;
        match self {
            P::U32 => "u32",
            P::Vec2U32 => "vec2<u32>",
            P::Vec3U32 => "vec3<u32>",
            P::Vec4U32 => "vec4<u32>",
            P::Mat3x3U32 => "mat3x3<u32>",
            P::Mat4x4U32 => "mat4x4<u32>",
            P::F32 => "f32",
            P::Vec2F32 => "vec2<f32>",
            P::Vec3F32 => "vec3<f32>",
            P::Vec4F32 => "vec4<f32>",
            P::Mat3x3F32 => "mat3x3<f32>",
            P::Mat4x4F32 => "mat4x4<f32>",
            P::F64 => "f64",
            P::Vec2F64 => "vec2<f64>",
            P::Vec3F64 => "vec3<f64>",
            P::Vec4F64 => "vec4<f64>",
            P::Mat3x3F64 => "mat3x3<f64>",
            P::Mat4x4F64 => "mat4x4<f64>",
        }
    }

    const fn size(&self) -> usize {
        use Primitive as P;
        match self {
            P::U32 => 4,
            P::Vec2U32 => 4 * 2,
            P::Vec3U32 => 4 * 3,
            P::Vec4U32 => 4 * 4,
            P::Mat3x3U32 => 4 * 3 * 3,
            P::Mat4x4U32 => 4 * 4 * 4,
            P::F32 => 4,
            P::Vec2F32 => 4 * 2,
            P::Vec3F32 => 4 * 3,
            P::Vec4F32 => 4 * 4,
            P::Mat3x3F32 => 4 * 3 * 3,
            P::Mat4x4F32 => 4 * 4 * 4,
            P::F64 => 8,
            P::Vec2F64 => 8 * 2,
            P::Vec3F64 => 8 * 3,
            P::Vec4F64 => 8 * 4,
            P::Mat3x3F64 => 8 * 3 * 3,
            P::Mat4x4F64 => 8 * 4 * 4,
        }
    }
}

pub trait GpuPrimitive: Copy {
    const GPU_PRIMITIVE: Primitive;
}

macro_rules! impl_gpu_primitive {
    ($ty:ty, $prim:ident) => {
        impl GpuPrimitive for $ty {
            const GPU_PRIMITIVE: Primitive = paste! { Primitive::$prim };
        }
    };
}

impl_gpu_primitive!(u32, U32);
impl_gpu_primitive!(glam::UVec2, Vec2U32);
impl_gpu_primitive!(glam::UVec3, Vec3U32);
impl_gpu_primitive!(glam::UVec4, Vec4U32);

impl_gpu_primitive!(f32, F32);
impl_gpu_primitive!(glam::Vec2, Vec2F32);
impl_gpu_primitive!(glam::Vec3, Vec3F32);
impl_gpu_primitive!(glam::Vec4, Vec4F32);
impl_gpu_primitive!(glam::Mat3, Mat3x3F32);
impl_gpu_primitive!(glam::Mat4, Mat4x4F32);

impl_gpu_primitive!(f64, F64);
impl_gpu_primitive!(glam::DVec2, Vec2F64);
impl_gpu_primitive!(glam::DVec3, Vec3F64);
impl_gpu_primitive!(glam::DVec4, Vec4F64);
impl_gpu_primitive!(glam::DMat3, Mat3x3F64);
impl_gpu_primitive!(glam::DMat4, Mat4x4F64);

pub trait VertexFormat {
    const VERT_FORMAT: wgpu::VertexFormat;
}

pub trait ShaderStruct {
    const FIELDS: &'static [(&'static str, Primitive)];

    fn wgpu_struct(name: &str) -> String {
        let mut fields = String::new();

        for (name, prim) in Self::FIELDS {
            let ty = prim.as_wgpu_ty();
            fields.push_str(&format!("{name}: {ty},\n"));
        }

        format! {"struct {name} {{\n {fields} }} "}
    }
}

pub const fn new_vert_attrib_array<const N: usize>(
    formats: [wgpu::VertexFormat; N],
) -> [wgpu::VertexAttribute; N] {
    let uninit_attrib = wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Uint8x4,
        offset: 0,
        shader_location: 0,
    };

    let mut attribs: [wgpu::VertexAttribute; N] = [uninit_attrib; N];

    let mut offset = 0;
    let mut i = 0;
    while i < N {
        attribs[i] = wgpu::VertexAttribute {
            offset,
            format: formats[i],
            shader_location: i as u32,
        };
        offset += formats[i].size();
        i += 1;
    }

    attribs
}

#[derive(Debug, Clone)]
pub struct Texture {
    texture: Arc<wgpu::Texture>,
    view: Arc<wgpu::TextureView>,
    sampler: Arc<wgpu::Sampler>,

    size: wgpu::Extent3d,
    egui_id: Option<egui::TextureId>,
}

impl ops::Deref for Texture {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl Texture {
    /// The width of the texture in pixels.
    pub fn width(&self) -> u32 {
        self.size.width
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// The height of the texture in pixels.
    pub fn height(&self) -> u32 {
        self.size.height
    }

    /// The depth of the texture.
    pub fn depth(&self) -> u32 {
        self.size.depth_or_array_layers
    }

    /// The size of the texture in pixels.
    pub fn size(&self) -> wgpu::Extent3d {
        self.size
    }

    /// The underlying `wgpu::Texture`.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Returns the format of this `Texture`.
    ///
    /// This is always equal to the `format` that was specified when creating the texture.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }

    /// Returns the msaa sample_count of this `Texture`.
    ///
    /// This is always equal to the `sample_count` that was specified when creating the texture.
    pub fn msaa_samples(&self) -> u32 {
        self.texture.sample_count()
    }

    /// The `wgpu::TextureView` of the underlying texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns the egui_id of this `Texture`.
    ///
    /// This will panic if the texture was not registered for egui during creation
    pub fn egui_id(&self) -> egui::TextureId {
        match self.egui_id {
            Some(id) => id,
            None => {
                panic!("texture was not registered for use with egui")
            }
        }
    }
}

/// Config for creating a texture.
///
/// Uses the builder pattern.
#[derive(derive_setters::Setters)]
#[setters(strip_option)]
pub struct TextureConfig<'a> {
    /// The size of the texture.
    pub size: wgpu::Extent3d,
    /// An optional label for the texture used for debugging.
    pub label: Option<&'a str>,
    /// The format of the texture, if not set uses the format from the renderer.
    pub format: wgpu::TextureFormat,
    /// The usage of the texture.
    #[setters(skip)]
    pub usage: wgpu::TextureUsages,
    /// The mip level of the texture.
    pub mip_level_count: u32,
    /// The sample count of the texture.
    pub msaa_samples: u32,
    /// The dimension of the texture.
    pub dimension: wgpu::TextureDimension,
    // The sampler descriptor of the texture.
    pub sampler_desc: wgpu::SamplerDescriptor<'a>,
    pub use_with_egui: Option<&'a mut egui_state::EguiState>,
    pub filter_mode: wgpu::FilterMode,
}

impl TextureConfig<'_> {
    pub fn d2(size: wgpu::Extent3d, format: wgpu::TextureFormat) -> Self {
        TextureConfig {
            size,
            label: Some("default 2d"),
            format,
            usage: wgpu::TextureUsages::empty(),
            mip_level_count: 1,
            msaa_samples: 1,
            dimension: wgpu::TextureDimension::D2,
            filter_mode: wgpu::FilterMode::Linear,
            sampler_desc: wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 32.0,
                compare: None,
                anisotropy_clamp: 1,
                border_color: None,
            },
            use_with_egui: None,
        }
    }

    pub fn depthf32(size: wgpu::Extent3d) -> Self {
        TextureConfig {
            label: Some("default depth"),
            sampler_desc: wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.,
                ..Default::default()
            },
            ..Self::d2(size, wgpu::TextureFormat::Depth32Float)
        }
    }

    #[inline(always)]
    pub fn usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage |= usage;
        self
    }

    pub fn as_copy_src(self) -> Self {
        self.usage(wgpu::TextureUsages::COPY_SRC)
    }
    pub fn as_copy_dst(self) -> Self {
        self.usage(wgpu::TextureUsages::COPY_DST)
    }
    pub fn as_copy_src_dst(self) -> Self {
        self.as_copy_src().as_copy_dst()
    }

    pub fn as_texture_binding(self) -> Self {
        self.usage(wgpu::TextureUsages::TEXTURE_BINDING)
    }
    pub fn as_storage_binding(self) -> Self {
        self.usage(wgpu::TextureUsages::STORAGE_BINDING)
    }
    pub fn as_binding(self) -> Self {
        self.as_texture_binding().as_storage_binding()
    }

    pub fn as_render_attachment(self) -> Self {
        self.usage(wgpu::TextureUsages::RENDER_ATTACHMENT)
    }

    pub fn build(&self, device: &wgpu::Device) -> Texture {
        let texture = Arc::new(device.create_texture(&wgpu::TextureDescriptor {
            label: self.label,
            size: self.size,
            mip_level_count: self.mip_level_count,
            sample_count: self.msaa_samples,
            dimension: self.dimension,
            format: self.format,
            usage: self.usage,
            view_formats: &[self.format],
        }));

        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));

        let mut sampler_desc = self.sampler_desc.clone();
        if self.msaa_samples != 1 {
            sampler_desc.mag_filter = wgpu::FilterMode::Nearest;
            sampler_desc.min_filter = wgpu::FilterMode::Nearest;
        } else {
            sampler_desc.mag_filter = self.filter_mode;
            sampler_desc.min_filter = self.filter_mode;
        }

        let sampler = device.create_sampler(&sampler_desc).into();

        let egui_id = if let Some(egui_state) = &self.use_with_egui {
            let mut wgpu_state = egui_state.wgpu_state.write().unwrap();
            Some(wgpu_state.register_native_texture_with_sampler_options(
                device,
                &view,
                sampler_desc,
            ))
        } else {
            None
        };

        Texture {
            texture,
            view,
            size: self.size,
            egui_id,
            sampler,
        }
    }
}

pub fn default_surface_config(
    size: winit::dpi::PhysicalSize<u32>,
    capabilities: wgpu::SurfaceCapabilities,
) -> wgpu::SurfaceConfiguration {
    let surface_format = capabilities
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(capabilities.formats[0]);

    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width.max(1u32),
        height: size.height.max(1u32),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

pub struct WgpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    // drop last
    pub window: Arc<winit::window::Window>,
}

impl WgpuContext {
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        pollster::block_on(Self::new_async(window))
    }

    pub async fn new_async(window: Arc<winit::window::Window>) -> Self {
        log::debug!("initializing wgpu context:");
        let window_size = window.inner_size();
        let instance = init_instance();
        log::debug!("ATLAS: instance: {instance:?}");

        let surface = instance.create_surface(window.clone()).unwrap();
        log::debug!("ATLAS: surface: {surface:?}");

        let adapter = init_adapter_async(instance, &surface).await;
        log::debug!("ATLAS: adapter: {adapter:?}");

        let (device, queue) = init_device_async(&adapter).await;
        log::debug!("ATLAS: device: {device:?}");

        let surface_caps = surface.get_capabilities(&adapter);
        let config = default_surface_config(window_size, surface_caps);
        surface.configure(&device, &config);

        let adapter_info = adapter.get_info();
        log::info!("{adapter_info:#?}");

        Self {
            surface,
            device,
            queue,
            config,
            window,
        }
    }
}

pub async fn init_device_async(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    use wgpu::Features;
    log::info!("features: {:#?}", adapter.features());
    //POLYGON_MODE_LINE
    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: Features::POLYGON_MODE_LINE,
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .unwrap()
}

pub async fn init_adapter_async(
    instance: wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> wgpu::Adapter {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap()
}

pub fn init_instance() -> wgpu::Instance {
    wgpu::Instance::new(&wgpu::InstanceDescriptor {
        #[cfg(any(target_os = "linux"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_os = "macos")]
        backends: wgpu::Backends::METAL,
        #[cfg(target_os = "windows")]
        backends: wgpu::Backends::DX12 | wgpu::Backends::GL,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::GL,
        ..Default::default()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

pub struct ShaderConfig<'a> {
    pub src: String,
    pub label: Option<&'a str>,
}

#[derive(Debug)]
pub struct ShaderModule {
    pub wgpu_module: wgpu::ShaderModule,
    pub entries: Vec<(ShaderStage, String)>,
}

impl ShaderModule {
    pub fn vs_entry(&self) -> Result<&str, Vec<&str>> {
        self.stage_entry(ShaderStage::Vertex)
    }
    pub fn fs_entry(&self) -> Result<&str, Vec<&str>> {
        self.stage_entry(ShaderStage::Fragment)
    }
    pub fn cs_entry(&self) -> Result<&str, Vec<&str>> {
        self.stage_entry(ShaderStage::Compute)
    }
    pub fn stage_entry(&self, shader_stage: ShaderStage) -> Result<&str, Vec<&str>> {
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(stage, entry)| {
                if *stage == shader_stage {
                    Some(entry.as_str())
                } else {
                    None
                }
            })
            .collect();

        if entries.len() == 1 {
            Ok(entries.pop().unwrap())
        } else {
            Err(entries)
        }
    }
}

impl ShaderConfig<'_> {
    pub fn from_wgsl(src: &str) -> Self {
        Self {
            src: src.into(),
            label: None,
        }
    }

    pub fn with_struct<T: ShaderStruct>(mut self, strct_name: &str) -> Self {
        let strct_src = T::wgpu_struct(strct_name);
        let placeholder = format!("@rust(struct {strct_name})");
        self.src = self.src.replace(&placeholder, &strct_src);
        self
    }

    pub fn build(self, device: &wgpu::Device) -> ShaderModule {
        let re = regex::Regex::new(r"@(\w+)\s+fn\s+(\w+)").unwrap();
        let mut entries = Vec::new();

        for cap in re.captures_iter(&self.src) {
            let shader_type = &cap[1]; // e.g., vertex, fragment, compute
            let function_name = &cap[2]; // e.g., main_vertex, main_fragment

            let stage = match shader_type {
                "vertex" => ShaderStage::Vertex,
                "fragment" => ShaderStage::Fragment,
                "compute" => ShaderStage::Compute,
                _ => continue,
            };

            entries.push((stage, function_name.to_string()));
        }

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label,
            source: wgpu::ShaderSource::Wgsl(self.src.into()),
        });

        ShaderModule {
            wgpu_module: module,
            entries,
        }
    }
}

pub trait VertexDescription: Sized {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }

    fn instance_desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug, derive_setters::Setters)]
#[setters(generate_delegates(ty = "PipelineConfig<'_, RenderPipelineConfig<'_>>", field = "data"))]
#[setters(strip_option)]
pub struct RenderPipelineConfig<'a> {
    pub vertex_desc: wgpu::VertexBufferLayout<'static>,
    pub instance_desc: Option<wgpu::VertexBufferLayout<'static>>,
    pub blend: Option<wgpu::BlendState>,
    pub msaa_samples: u32,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub polygon_mode: wgpu::PolygonMode,
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub cull_mode: Option<wgpu::Face>,
    #[setters(skip)]
    pub vs_entry: Option<&'a str>,
    #[setters(skip)]
    pub fs_entry: Option<&'a str>,
}

#[derive(Debug, derive_setters::Setters)]
#[setters(generate_delegates(ty = "PipelineConfig<'_, ComputePipelineConfig>", field = "data"))]
pub struct ComputePipelineConfig {
    pub entry: Option<String>,
}

#[derive(Debug, derive_setters::Setters)]
#[setters(strip_option)]
pub struct PipelineConfig<'a, T> {
    pub label: Option<&'a str>,
    pub bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    pub module: &'a ShaderModule,
    pub data: T,
}

impl<'a> PipelineConfig<'a, ()> {
    pub fn new(module: &'a ShaderModule) -> Self {
        Self {
            module,
            label: None,
            bind_group_layouts: &[],
            data: (),
        }
    }

    pub fn color_depth<V: VertexDescription>(
        self,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> PipelineConfig<'a, RenderPipelineConfig<'a>> {
        PipelineConfig {
            label: self.label,
            bind_group_layouts: self.bind_group_layouts,
            module: self.module,
            data: RenderPipelineConfig {
                vertex_desc: V::desc(),
                instance_desc: None,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                msaa_samples: 1,
                color_format,
                polygon_mode: wgpu::PolygonMode::Fill,
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                depth_format: Some(depth_format),
                vs_entry: None,
                fs_entry: None,
            },
        }
    }

    pub fn color<V: VertexDescription>(
        self,
        color_format: wgpu::TextureFormat,
    ) -> PipelineConfig<'a, RenderPipelineConfig<'a>> {
        PipelineConfig {
            label: self.label,
            bind_group_layouts: self.bind_group_layouts,
            module: self.module,
            data: RenderPipelineConfig {
                vertex_desc: V::desc(),
                instance_desc: None,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                msaa_samples: 1,
                color_format,
                polygon_mode: wgpu::PolygonMode::Fill,
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                depth_format: None,
                vs_entry: None,
                fs_entry: None,
            },
        }
    }

    pub fn compute(self) -> PipelineConfig<'a, ComputePipelineConfig> {
        PipelineConfig {
            label: self.label,
            bind_group_layouts: self.bind_group_layouts,
            module: self.module,
            data: ComputePipelineConfig { entry: None },
        }
    }
}

impl<T> PipelineConfig<'_, T> {
    pub fn build_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label.map(|label| format!("{label} layout")).as_deref(),
            bind_group_layouts: self.bind_group_layouts,
            push_constant_ranges: &[],
        })
    }

    pub fn set_if(self, cond: bool, setter: impl Fn(Self) -> Self) -> Self {
        if cond { setter(self) } else { self }
    }
}

impl PipelineConfig<'_, ComputePipelineConfig> {
    pub fn build(self, device: &wgpu::Device) -> wgpu::ComputePipeline {
        let layout = self.build_layout(device);

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: self.label,
            layout: Some(&layout),
            module: &self.module.wgpu_module,
            entry_point: Some(self.data.entry.as_deref().unwrap_or_else(|| {
                self.module
                    .cs_entry()
                    .expect("could not infer compute entry")
            })),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }
}

impl<'a> PipelineConfig<'a, RenderPipelineConfig<'a>> {
    pub fn vs_entry(mut self, entry: &'a str) -> Self {
        self.data.vs_entry = Some(entry);
        self
    }

    pub fn fs_entry(mut self, entry: &'a str) -> Self {
        self.data.fs_entry = Some(entry);
        self
    }

    pub fn set_cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.data.cull_mode = cull_mode;
        self
    }

    pub fn with_instances<I: VertexDescription>(mut self) -> Self {
        self.data.instance_desc = Some(I::instance_desc());
        self
    }

    pub fn build(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        let layout = self.build_layout(device);

        let buffers = if let Some(instance_desc) = &self.data.instance_desc {
            vec![self.data.vertex_desc.clone(), instance_desc.clone()]
        } else {
            vec![self.data.vertex_desc.clone()]
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &self.module.wgpu_module,
                entry_point: Some(self.data.vs_entry.unwrap_or_else(|| {
                    self.module
                        .vs_entry()
                        .expect("could not infer vertex entry")
                })),
                buffers: &buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.module.wgpu_module,
                entry_point: Some(self.data.fs_entry.unwrap_or_else(|| {
                    self.module
                        .fs_entry()
                        .expect("could not infer fragment entry")
                })),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.data.color_format,
                    blend: self.data.blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.data.primitive_topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: self.data.cull_mode,
                polygon_mode: self.data.polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: self
                .data
                .depth_format
                .map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
            multisample: wgpu::MultisampleState {
                count: self.data.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }
}

#[derive(derive_setters::Setters)]
#[setters(strip_option)]
pub struct RenderPass<'a> {
    pub clear_color: Option<wgpu::Color>,
    pub color_target: &'a wgpu::TextureView,
    pub depth_target: Option<&'a wgpu::TextureView>,
    pub resolve_target: Option<&'a wgpu::TextureView>,
    pub label: Option<&'a str>,
    //pub render_pipeline: Option<&'a wgpu::RenderPipeline>,
    //pub bind_group: Option<&'a wgpu::BindGroup>,
    //pub vertex_buffer: Option<wgpu::BufferSlice<'a>>,
    //#[setters(skip)]
    //pub index_buffer: Option<wgpu::BufferSlice<'a>>,
    //#[setters(skip)]
    //pub index_format: Option<wgpu::IndexFormat>,
    //#[setters(skip)]
    //pub indices: Range<u32>,
}

impl<'a> RenderPass<'a> {
    pub fn set_if(self, cond: bool, setter: impl Fn(Self) -> Self) -> Self {
        if cond { setter(self) } else { self }
    }

    pub fn target_color(color_target: &'a wgpu::TextureView) -> Self {
        Self {
            color_target,
            depth_target: None,
            resolve_target: None,
            clear_color: None,
            label: None,
            //render_pipeline: None,
            //bind_group: None,
            //vertex_buffer: None,
            //index_buffer: None,
            //index_format: None,
            //indices: 0..0,
        }
    }

    pub fn target_color_depth(
        color_target: &'a wgpu::TextureView,
        depth_target: &'a wgpu::TextureView,
    ) -> Self {
        Self {
            depth_target: Some(depth_target),
            ..Self::target_color(color_target)
        }
    }

    pub fn clear_hex(self, hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let values: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();

        let (r, g, b, a) = match values.as_slice() {
            [r, g, b] => (*r, *g, *b, 255),
            [r, g, b, a] => (*r, *g, *b, *a),
            _ => panic!("Hex code must be 6 or 8 characters long"),
        };

        self.clear_rgba(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            a as f64 / 255.0,
        )
    }

    pub fn clear_rgb(self, r: f64, g: f64, b: f64) -> Self {
        self.clear_rgba(r, g, b, 1.0)
    }

    pub fn clear_rgba(self, r: f64, g: f64, b: f64, a: f64) -> Self {
        let r = ((r + 0.055) / 1.055).powf(2.4);
        let g = ((g + 0.055) / 1.055).powf(2.4);
        let b = ((b + 0.055) / 1.055).powf(2.4);
        let a = ((a + 0.055) / 1.055).powf(2.4);

        self.clear_color(wgpu::Color { r, g, b, a })
    }

    //pub fn index_buffer(
    //    mut self,
    //    index_buffer: wgpu::BufferSlice<'a>,
    //    index_format: wgpu::IndexFormat,
    //    indices: Range<u32>,
    //) -> Self {
    //    self.index_buffer = Some(index_buffer);
    //    self.index_format = Some(index_format);
    //    self.indices = indices;
    //    self
    //}

    pub fn draw(
        self,
        encoder: &'a mut wgpu::CommandEncoder,
        draw_fn: impl Fn(wgpu::RenderPass<'a>),
    ) {
        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.color_target,
                resolve_target: self.resolve_target,
                ops: wgpu::Operations {
                    load: self
                        .clear_color
                        .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: self.depth_target.map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        draw_fn(rpass);
    }

    /*
    pub fn finish(self, encoder: &'a mut wgpu::CommandEncoder) {
        let mut rpass = self.build_render_pass(encoder);

        let index_buffer = self.index_buffer.unwrap();
        let index_format = self.index_format.unwrap();
        let vertex_buffer = self.vertex_buffer.unwrap();

        rpass.set_vertex_buffer(0, vertex_buffer);
        rpass.set_index_buffer(index_buffer, index_format);

        if let Some(rp) = self.render_pipeline {
            rpass.set_pipeline(rp)
        }
        if let Some(bg) = self.bind_group {
            rpass.set_bind_group(0, bg, &[])
        }

        rpass.draw_indexed(self.indices.clone(), 0, 0..1);
    }

    fn build_render_pass(&self, encoder: &'a mut wgpu::CommandEncoder) -> wgpu::RenderPass {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.color_target,
                resolve_target: self.resolve_target,
                ops: wgpu::Operations {
                    load: self
                        .clear_color
                        .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: self.depth_target.map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        })
    }
    */
}
