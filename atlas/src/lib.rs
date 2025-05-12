mod camera;
mod egui_state;
mod gpu;
mod gui;

// pub mod debug_iso_2d;
// pub mod iso;
// pub mod iso2;
// pub mod iso3;
pub mod iso4;
// mod iso2;
mod ui;
// mod athena;

pub mod vm;

pub extern crate self as atlas;

use camera::Camera;
use facet::Facet;
use macros::ShaderStruct;

use egui::Rect;

use crossbeam::channel;
use egui_probe::EguiProbe;
use glam::{DVec3, Mat4, UVec2, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use std::rc::Rc;
use std::thread;
use std::{sync::{Arc, mpsc}};
use vm::op;
use wgpu::util::DeviceExt;
use web_time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    error::EventLoopError,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
const MULTISAMPLE: bool = true;
#[cfg(target_arch = "wasm32")]
const MULTISAMPLE: bool = false;

fn multisample_state() -> wgpu::MultisampleState {
    if MULTISAMPLE {
        wgpu::MultisampleState {
            mask: !0,
            alpha_to_coverage_enabled: false,
            count: 4,
        }
    } else {
        Default::default()
    }
}

#[derive(Debug, Clone)]
pub enum WindowHandle {
    UnInit,
    Init(Arc<Window>),
}

impl WindowHandle {
    fn get_handle(&self) -> &Arc<Window> {
        match self {
            WindowHandle::UnInit => panic!("window was not initialized"),
            WindowHandle::Init(window) => window,
        }
    }

    fn id(&self) -> winit::window::WindowId {
        self.get_handle().id()
    }

    fn request_redraw(&self) {
        self.get_handle().request_redraw();
    }

    fn set_mouse_pos(&self, pos: Vec2) {
        self.get_handle()
            .set_cursor_position(PhysicalPosition::new(pos.x, pos.y))
            .ok();
    }
}

impl From<Window> for WindowHandle {
    fn from(value: Window) -> Self {
        Self::Init(Arc::new(value))
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, ShaderStruct)]
#[repr(C)]
pub struct Vertex {
    #[wgsl(@location(0))]
    pub pos: Vec4,
    #[wgsl(@location(1))]
    pub col: Vec4,
}

#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct LineSegmentInst {
    pub a: Vec3,
    pub b: Vec3,
}

impl gpu::VertexDescription for LineSegmentInst {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![3 => Float32x3, 4 => Float32x3];
}

impl Vertex {
    pub fn new(pos: Vec3, col: Vec4) -> Self {
        Self {
            pos: pos.extend(0.0),
            col,
        }
    }
}

impl gpu::VertexDescription for Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
}

pub fn hex_to_col(hex: &str) -> wgpu::Color {
    fn to_linear(u: u8) -> f64 {
        let srgb = u as f64 / 255.0;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    }

    let hex = hex.trim_start_matches('#');
    let vals: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();

    let (r8, g8, b8, a8) = match vals.as_slice() {
        [r, g, b]     => (*r, *g, *b, 255),
        [r, g, b, a] => (*r, *g, *b, *a),
        _ => panic!("Hex code must be 6 or 8 characters long"),
    };

    wgpu::Color {
        r: to_linear(r8),
        g: to_linear(g8),
        b: to_linear(b8),
        a: a8 as f64 / 255.0, // alpha is linear already
    }
}




#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, ShaderStruct)]
#[repr(C)]
pub struct WorldUniform {
    pub light_pos: Vec3,
    //pub pad1: u32,
    pub line_thickness: f32,
    pub view_proj: Mat4,
}

impl WorldUniform {
    pub fn new(view_proj: Mat4, light_pos: Vec3) -> Self {
        Self {
            light_pos,
            line_thickness: 0.1,
            view_proj,
        }
    }
}

pub struct MeshConfig(iso4::Iso2DConfig);

pub struct Atlas {
    //window: AtlasApp,
    event_loop: EventLoop<()>,
    //window: Window,
}

impl Atlas {
    pub fn init() -> Self {
        let event_loop = EventLoop::new().unwrap();
        // event_loop.set_control_flow(ControlFlow::Poll);
        // event_loop.set_control_flow(ControlFlow::Wait);

        Self {
            event_loop,
            //window,
        }
    }

    pub fn run(self) -> Result<(), EventLoopError> {
        let mut app = AtlasApp::new();
        self.event_loop.run_app(&mut app)
    }
}

#[derive(Debug, Clone, Copy, EguiProbe)]
struct WindowData {
    #[egui_probe(with ui::label_probe)]
    mouse_pixel_pos: Vec2,
    #[egui_probe(with ui::label_probe)]
    mouse_delta: Vec2,
    viewport_dragged: bool,
    viewport_rect: Rect,

    ui_pixel_per_point: f32,

    #[egui_probe(with ui::duration_probe)]
    delta_time: Duration,

    mesh_gen_time: f64,

    #[egui_probe(skip)]
    prev_frame_time: Instant,
}

impl WindowData {
    fn vp_rect_min(&self) -> Vec2 {
        let min = self.viewport_rect.min;
        (min.x, min.y).into()
    }
    fn vp_rect_max(&self) -> Vec2 {
        let max = self.viewport_rect.max.to_vec2();
        (max.x, max.y).into()
    }
    fn viewport_dim(&self) -> Vec2 {
        self.vp_rect_max() - self.vp_rect_min()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
struct RenderConfig {
    cull_mode: CullMode,
    polygon_mode: PolygonMode,
    #[egui_probe(with ui::angle_probe_deg)]
    fov: f32,
    #[egui_probe(toggle_switch)]
    depthbuffer: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
pub enum MeshGenerator {
    Iso2D,
}

#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
pub enum CameraMode {
    Drag2D,
    Pan2D,
    Orbit3D,
}

#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
struct AtlasSettings {
    iso_2d_config: iso4::Iso2DConfig,
    // iso_3d_config: iso::Iso3DConfig,
    #[egui_probe(skip)]
    show_tree: bool,
    #[egui_probe(skip)]
    show_mesh: bool,

    camera_mode: CameraMode,

    // #[egui_probe(with ui::button_probe("rebuild"))]
    #[egui_probe(skip)]
    rebuild_mesh: bool,
    #[egui_probe(skip)]
    mesh_gen: MeshGenerator,
    #[egui_probe(skip)]
    render_config: RenderConfig,
}

impl Default for AtlasSettings {
    fn default() -> Self {
        Self {
            iso_2d_config: iso4::Iso2DConfig {
                min: [-10.0, -10.0].into(),
                max: [10.0, 10.0].into(),
                intrvl_depth: 4,
                subdiv_depth: 4,
                line_thickness: 1.5,
                ..Default::default()
            },
            // iso_3d_config: iso::Iso3DConfig {
            //     min: [-10.0, -10.0, -10.0].into(),
            //     max: [10.0, 10.0, 10.0].into(),
            //     tol: 0.0,
            //     depth: 4,
            //     shade_smooth: false,
            // },
            camera_mode: CameraMode::Pan2D,

            rebuild_mesh: false,
            show_tree: false,
            show_mesh: true,
            mesh_gen: MeshGenerator::Iso2D,
            render_config: RenderConfig {
                cull_mode: CullMode::None,
                polygon_mode: PolygonMode::Fill,
                fov: 90.0,
                depthbuffer: false,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DataTy {
    U32,
    Vec2U32,
    Vec3U32,
    Vec4U32,

    F32,
    Vec2F32,
    Vec3F32,
    Vec4F32,
}

impl DataTy {
    const fn as_wgpu_attrib(&self) -> wgpu::VertexFormat {
        use wgpu::VertexFormat as VF;
        match self {
            DataTy::U32 => VF::Uint32,
            DataTy::Vec2U32 => VF::Uint32x2,
            DataTy::Vec3U32 => VF::Uint32x3,
            DataTy::Vec4U32 => VF::Uint32x4,
            DataTy::F32 => VF::Float32,
            DataTy::Vec2F32 => VF::Float32x2,
            DataTy::Vec3F32 => VF::Float32x3,
            DataTy::Vec4F32 => VF::Float32x4,
        }
    }

    const fn size(&self) -> u64 {
        self.as_wgpu_attrib().size()
    }
}


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DataLayout {
    fields: Vec<DataTy>,
}

impl<T: AsRef<[DataTy]>> From<T> for DataLayout {
    fn from(value: T) -> Self {
        Self {
            fields: value.as_ref().to_vec(),
        }
    }
}

impl DataLayout {
    fn as_wgpu_attribs(&self, mut loc_offset: u32) -> Vec<wgpu::VertexAttribute> {
        let mut attribs = vec![];

        let mut byte_offset = 0;
        for f in &self.fields {
            attribs.push(wgpu::VertexAttribute {
                format: f.as_wgpu_attrib(),
                offset: byte_offset,
                shader_location: loc_offset,
            });

            loc_offset += 1;
            byte_offset += f.size();
        }

        attribs
    }

    fn n_bytes(&self) -> u64 {
        let mut size = 0;

        for f in &self.fields {
            size += f.size();
        }

        size
    }
}

struct BufferDesc {
    label: Option<String>,
    layout: DataLayout,
    usage: wgpu::BufferUsages,
}

impl BufferDesc {
    fn desc(layout: impl Into<DataLayout>) -> Self {
        let layout: DataLayout = layout.into();
        Self {
            label: None,
            layout,
            usage: wgpu::BufferUsages::empty(),
        }
    }

    fn vertex(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::VERTEX;
        self
    }

    fn index(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::INDEX;
        self
    }

    fn copy_dst(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_DST;
        self
    }

    fn copy_src(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_SRC;
        self
    }

    fn empty(self) -> Buffer {
        Buffer {
            label: self.label,
            layout: self.layout,
            usage: self.usage,
            n_bytes: 0,
            data: None,
        }
    }

    fn with_data(self, data: &[u8], wgpu: &WGPU) -> Buffer {
        let n_bytes = data.len() as u64;
        let data = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label.as_ref().map(|s| s.as_str()),
            usage: self.usage,
            contents: data,
        });

        Buffer {
            label: self.label,
            layout: self.layout,
            data: Some(data),
            n_bytes,
            usage: self.usage,
        }
    }
}

#[derive(Debug)]
struct Buffer {
    label: Option<String>,
    layout: DataLayout,
    data: Option<wgpu::Buffer>,
    n_bytes: u64,
    usage: wgpu::BufferUsages,
}


impl Buffer {
    fn alloc(&mut self, wgpu: &WGPU, n_bytes: u64) {
        let data = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_ref().map(|s| s.as_str()),
            size: n_bytes,
            usage: self.usage,
            mapped_at_creation: false,
        });
        self.data = Some(data);
    }

    fn alloc_w_data(&mut self, wgpu: &WGPU, data: &[u8]) {
        self.n_bytes = data.len() as u64;
        let data = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label.as_ref().map(|s| s.as_str()),
            usage: self.usage,
            contents: data,
        });
        self.data = Some(data);
    }

    fn upload(&mut self, wgpu: &WGPU, data: &[u8]) {
        assert!(self.n_bytes >= data.len() as u64);
        wgpu.queue.write_buffer(self.data.as_ref().unwrap(), 0, data);
    }

    fn upload_or_alloc(&mut self, wgpu: &WGPU, data: &[u8]) {
        if self.n_bytes < self.n_bytes {
            self.alloc_w_data(wgpu, data);
        } else {
            self.upload(wgpu, data)
        }
    }
}


#[derive(Debug)]
struct ModelInstance {
    pipeline: Rc<wgpu::RenderPipeline>,
    vertex: wgpu::Buffer,
    n_vertices: u64,
    instance: wgpu::Buffer,
    n_instances: u64,
    instance_size: u64,
    n_max_instances: u64,
}

impl ModelInstance {
    fn new_rect_inst(wgpu: &WGPU, pipeline: impl Into<Rc<wgpu::RenderPipeline>>, data: &[u8], instance_size: u64, n_max_instances: u64) -> Self {
        let unit_rect = [
            Vertex {
                pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
            Vertex {
                pos: Vec4::new(0.0, 1.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
            Vertex {
                pos: Vec4::new(1.0, 0.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
            Vertex {
                pos: Vec4::new(1.0, 1.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
        ];

        let vertex = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("unit_rect_vertex_buffer"),
            contents: bytemuck::cast_slice(&unit_rect),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: n_max_instances * instance_size as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline: pipeline.into(),
            vertex,
            n_vertices: 4,
            instance,
            n_instances: 0,
            instance_size,
            n_max_instances,
        }
    }

    fn upload_or_new(&mut self, wgpu: &WGPU, data: &[u8], n_instances: u64) {
        if data.len() as u64 > self.n_max_instances * self.instance_size {
            let instance = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: data,
            });

            self.instance = instance;
            self.n_max_instances = n_instances;
        } else {
            self.upload(wgpu, data, n_instances)
        }
    }

    fn upload(&mut self, wgpu: &WGPU, data: &[u8], n_instances: u64) {
        self.n_instances = n_instances;
        wgpu.queue.write_buffer(&self.instance, 0, data);
    }
}

struct AtlasApp {
    renderer: Option<AtlasRenderer>,

    camera: Camera,
    pos_3d: Vec3,
    pos_2d: Vec2,

    ui_state: ui::UiState,
    egui_state: Option<egui_winit::State>,

    data: WindowData,
    settings: AtlasSettings,

    mesh_2d: Option<ModelInstance>,
    // egui_state: Option<egui_state::EguiState>,
    last_size: UVec2,
    last_render_time: Option<Instant>,

    #[cfg(target_arch = "wasm32")]
    renderer_receiver: Option<futures::channel::oneshot::Receiver<AtlasRenderer>>,

    window: Option<Arc<winit::window::Window>>,
    initialized: bool,
}

fn load_line_shader(wgpu: &WGPU) -> wgpu::RenderPipeline {
    let world_bind_group_layout =
        wgpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: "world_bind_group_layout".into(),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let line_shader = gpu::ShaderConfig::from_wgsl(include_str!("line.wgsl"))
        .with_struct::<Vertex>("VertexInput")
        .with_struct::<WorldUniform>("WorldUniform")
        .build(&wgpu.device);

    let line_pipeline_layout = wgpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("line_pipeline_layout"),
        bind_group_layouts: &[&world_bind_group_layout],
        push_constant_ranges: &[],
    });

    let line_pipeline = wgpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: Some("line_pipeline"),
        layout: Some(&line_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &line_shader.wgpu_module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4]
                },
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineSegmentInst>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![2 => Float32x3, 3 => Float32x3]
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: &line_shader.wgpu_module,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu.surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            unclipped_depth: false,
            front_face: wgpu::FrontFace::Ccw,
            polygon_mode: wgpu::PolygonMode::Fill,
            strip_index_format: None,
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: AtlasRenderer::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: multisample_state(),
        multiview: None,
        cache: None,
    });

    // let line_pipeline = gpu::PipelineConfig::new(&line_shader)
    //     .color::<Vertex>(wgpu.surface_format)
    //     .depth_format(AtlasRenderer::DEPTH_FORMAT)
    //     .with_instances::<LineSegmentInst>()
    //     .msaa_samples(4)
    //     .set_cull_mode(CullMode::None.into())
    //     .primitive_topology(wgpu::PrimitiveTopology::TriangleStrip)
    //     .bind_group_layouts(&[&world_bind_group_layout])
    //     .label("line pipeline")
    //     .build(&wgpu.device);

    line_pipeline
}

impl AtlasApp {

    fn try_init(&mut self) -> bool {
        if self.initialized {
            return true
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut renderer_received = false;
            if let Some(receiver) = self.renderer_receiver.as_mut() {
                if let Ok(Some(renderer)) = receiver.try_recv() {
                    self.renderer = Some(renderer);
                    renderer_received = true;
                }
            }
            if renderer_received {
                self.renderer_receiver = None;

                let unit_rect = [
                    Vertex {
                        pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
                        col: Vec4::ONE,
                    },
                    Vertex {
                        pos: Vec4::new(0.0, 1.0, 0.0, 1.0),
                        col: Vec4::ONE,
                    },
                    Vertex {
                        pos: Vec4::new(1.0, 0.0, 0.0, 1.0),
                        col: Vec4::ONE,
                    },
                    Vertex {
                        pos: Vec4::new(1.0, 1.0, 0.0, 1.0),
                        col: Vec4::ONE,
                    },
                ];

                let wgpu = &self.renderer.as_ref().unwrap().wgpu;
                let vertex = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("unit_rect_vertex_buffer"),
                    contents: bytemuck::cast_slice(&unit_rect),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let n_max_instances = 1024;

                let instance = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("instance_buffer"),
                    size: n_max_instances * std::mem::size_of::<LineSegmentInst>() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });


                self.mesh_2d = Some(ModelInstance {
                    pipeline: load_line_shader(&wgpu).into(),
                    vertex,
                    n_vertices: 4,
                    instance,
                    n_instances: 0,
                    n_max_instances,
                });

                let window = self.window.as_ref().unwrap();
                let size = window.inner_size();
                self.renderer.as_mut().unwrap().resize(size.width, size.height);
                // self.egui_state.as_ref().unwrap().egui_ctx().set_pixels_per_point(window.scale_factor() as f32);

            }
            self.initialized = renderer_received;
            return renderer_received
        }

        true
    }

    fn new() -> Self {
        log::debug!("init atlas app");

        let settings = AtlasSettings::default();

        let pos_3d = Vec3::splat(2.0);
        let pos_2d = Vec2::ZERO;

        let camera = Camera::pan_2d(pos_2d, 1.0);

        let data = WindowData {
            mouse_pixel_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            viewport_dragged: false,
            viewport_rect: Rect::ZERO,
            ui_pixel_per_point: 0.0,
            delta_time: Duration::ZERO,
            mesh_gen_time: 0.0,
            prev_frame_time: Instant::now(),
        };

        Self {
            renderer: None,
            egui_state: None,
            mesh_2d: None,
            pos_3d,
            pos_2d,
            ui_state: ui::UiState::new(),
            data,
            settings,
            camera,
            window: None,
            last_size: UVec2::ZERO,
            last_render_time: None,
            initialized: false,

            #[cfg(target_arch = "wasm32")]
            renderer_receiver: None,
        }
    }

    //fn on_update(&mut self) {
    //    let prev_time = self.data.prev_frame_time;
    //    let curr_time = Instant::now();
    //    let dt = curr_time - prev_time;

    //    self.data.prev_frame_time = curr_time;
    //    self.data.delta_time = dt;

    //    self.camera.set_aspect(
    //        self.data.viewport_rect.width() as u32,
    //        self.data.viewport_rect.height() as u32,
    //    );
    //    self.camera.time_step(dt);

    //    if self.data.viewport_dragged {
    //        self.camera
    //            .process_mouse(self.data.mouse_delta.x, self.data.mouse_delta.y);
    //    }

    //    let renderer = self.renderer.as_mut().unwrap();
    //    // let prev_viewport_size = renderer.viewport_size;
    //    let prev_render_config = self.settings.render_config;
    //    let prev_camera_mode = self.settings.camera_mode;
    //    //let mut settings = self.settings;

    //    self.egui_state.as_mut().unwrap().update(self.window.get_handle(), |ctx| {
    //        self.data.ui_pixel_per_point = ctx.input(|i| i.pixels_per_point);

    //        let access = ui::UiAccess {
    //            vp_texture: &renderer.framebuffer_resolve,
    //            camera: &self.camera,
    //            window_info: &mut self.data,
    //            settings: &mut self.settings,
    //        };

    //        self.ui_state.ui(ctx, access);

    //        // renderer.viewport_size = wgpu::Extent3d {
    //        //     width: self.data.viewport_rect.width() as u32,
    //        //     height: self.data.viewport_rect.height() as u32,
    //        //     depth_or_array_layers: 1,
    //        // }
    //    });

    //    self.camera.config.fov_rad = self.settings.render_config.fov.to_radians();
    //    self.data.mouse_delta = Vec2::ZERO;

    //    if prev_camera_mode != self.settings.camera_mode {
    //        let mode = match self.settings.camera_mode {
    //            CameraMode::Pan2D => {
    //                let zoom = if let camera::CameraMode::Drag2D(drag_2d) = &self.camera.mode {
    //                    drag_2d.zoom
    //                } else {
    //                    1.0
    //                };
    //                camera::CameraMode::Pan2D(camera::Pan2D::new(self.pos_2d, 1.0))
    //            }
    //            CameraMode::Orbit3D => {
    //                camera::CameraMode::Orbit3D(camera::Orbit3D::new(self.pos_3d, Vec3::ZERO))
    //            }
    //            CameraMode::Drag2D => {
    //                let zoom = if let camera::CameraMode::Pan2D(pan_2d) = &self.camera.mode {
    //                    pan_2d.zoom
    //                } else {
    //                    1.0
    //                };
    //                camera::CameraMode::Drag2D(camera::Drag2D::new(self.pos_2d, zoom as f32))
    //            }
    //        };
    //        self.camera.switch_mode(mode);
    //    }

    //    // if self.settings.render_config != prev_render_config {
    //    //     renderer.rebuild_from_settings(&self.settings);
    //    // } else if prev_viewport_size != renderer.viewport_size {
    //    //     renderer.resize_viewport();
    //    // }

    //    if !self.camera.mode.is_drag_2d() {
    //        if let camera::CameraMode::Pan2D(c) = &mut self.camera.mode {
    //            let (min, max) = c.get_bounds(&self.camera.config);
    //            self.settings.iso_2d_config.min = min.into();
    //            self.settings.iso_2d_config.max = max.into();
    //            self.settings.rebuild_mesh = false;
    //            let start = time::Instant::now();
    //            renderer.rebuild_mesh(&self.settings);
    //            let end = time::Instant::now();
    //            self.data.mesh_gen_time = (end - start).as_secs_f64() * 1000.0;
    //        }
    //    } else if self.settings.rebuild_mesh {
    //        self.settings.rebuild_mesh = false;
    //        renderer.rebuild_mesh(&self.settings);
    //    }
    //}

    fn rebuild_mesh_2d(&mut self) -> Vec<LineSegmentInst> {
        let (_, mut lines) = build_mesh_2d(&self.settings);
        for l in &mut lines {
            l.a = l.a * 2.0;
            l.b = l.b * 2.0;
        }
        lines
        // let renderer = &self.renderer.as_ref().unwrap();
        // self.mesh_2d.as_mut().unwrap().upload(&renderer.wgpu, bytemuck::cast_slice(&lines), lines.len() as u64);
    }

    fn resize(&mut self, w: u32, h: u32) {
        if !self.try_init() {
            return
        }

        let renderer = self.renderer.as_mut().unwrap();
        let w = w.max(1);
        let h = h.max(1);
        renderer.resize(w, h);


        let vp_rect = self.data.viewport_rect;
        let vp_w = (vp_rect.width() as u32).max(1);
        let vp_h = (vp_rect.height() as u32).max(1);
        renderer.resize_viewport(vp_w, vp_h);

        let scale_factor = self.window.as_ref().unwrap().scale_factor() as f32;
        // self.egui_state.as_ref().unwrap().egui_ctx().set_pixels_per_point(scale_factor);
    }

    fn on_redraw(&mut self, ctrlflow: &ActiveEventLoop) {
        let prev_time = self.data.prev_frame_time;
        let curr_time = Instant::now();
        let dt = curr_time - prev_time;

        self.data.prev_frame_time = curr_time;
        self.data.delta_time = dt;

        self.camera.set_aspect(
            self.data.viewport_rect.width() as u32,
            self.data.viewport_rect.height() as u32,
        );
        self.camera.time_step(dt);

        if self.data.viewport_dragged {
            self.camera
                .process_mouse(self.data.mouse_delta.x, self.data.mouse_delta.y);
        }

        let renderer = self.renderer.as_mut().unwrap();
        let prev_viewport_size = self.data.viewport_rect;
        let prev_render_config = self.settings.render_config;
        let prev_camera_mode = self.settings.camera_mode;
        //let mut settings = self.settings;

        let egui_state = self.egui_state.as_mut().unwrap();
        let raw_input = egui_state.take_egui_input(self.window.as_ref().unwrap());

        egui_state.egui_ctx().begin_pass(raw_input);
        self.data.ui_pixel_per_point = self.window.as_ref().unwrap().scale_factor() as f32;
        // self.data.ui_pixel_per_point = egui_state.egui_ctx().input(|i| i.pixels_per_point);

        let access = ui::UiAccess {
            vp_texture: renderer.fb_egui_id,
            camera: &self.camera,
            window_info: &mut self.data,
            settings: &mut self.settings,
        };

        self.ui_state.ui(&egui_state.egui_ctx(), access);

        self.camera.config.fov_rad = self.settings.render_config.fov.to_radians();
        self.data.mouse_delta = Vec2::ZERO;

        if prev_camera_mode != self.settings.camera_mode {
            let mode = match self.settings.camera_mode {
                CameraMode::Pan2D => {
                    let zoom = if let camera::CameraMode::Drag2D(drag_2d) = &self.camera.mode {
                        drag_2d.zoom
                    } else {
                        1.0
                    };
                    camera::CameraMode::Pan2D(camera::Pan2D::new(self.pos_2d, 1.0))
                }
                CameraMode::Orbit3D => {
                    camera::CameraMode::Orbit3D(camera::Orbit3D::new(self.pos_3d, Vec3::ZERO))
                }
                CameraMode::Drag2D => {
                    let zoom = if let camera::CameraMode::Pan2D(pan_2d) = &self.camera.mode {
                        pan_2d.zoom
                    } else {
                        1.0
                    };
                    camera::CameraMode::Drag2D(camera::Drag2D::new(self.pos_2d, zoom as f32))
                }
            };
            self.camera.switch_mode(mode);
        }

        let mut lines = vec![];

        if !self.camera.mode.is_drag_2d() {
            if let camera::CameraMode::Pan2D(c) = &mut self.camera.mode {
                let (min, max) = c.get_bounds(&self.camera.config);
                self.settings.iso_2d_config.min = min.into();
                self.settings.iso_2d_config.max = max.into();
                self.settings.rebuild_mesh = false;
                let start = Instant::now();
                lines = self.rebuild_mesh_2d();
                let end = Instant::now();
                self.data.mesh_gen_time = (end - start).as_secs_f64() * 1000.0;
            }
        } 
        // else if self.settings.rebuild_mesh {
        //     self.settings.rebuild_mesh = false;
        //     renderer.rebuild_mesh(&self.settings);
        // }

        let renderer = self.renderer.as_mut().unwrap();
        let vp_size = self.data.viewport_dim();
        // let vp_size = renderer.viewport_size;
        let (vp_w, vp_h) = (vp_size.x as f32, vp_size.y as f32);

        renderer.world_uniform.line_thickness =
            self.settings.iso_2d_config.line_thickness / (vp_w * vp_w + vp_h * vp_h).sqrt();
        if let camera::CameraMode::Orbit3D(c) = &self.camera.mode {
            renderer.world_uniform.light_pos = c.eye();
        }
        renderer.world_uniform.view_proj = self.camera.view_proj_mat();
        renderer.update_world_uniform();


        let mesh_2d = self.mesh_2d.as_mut().unwrap();
        let renderer = self.renderer.as_mut().unwrap();

        mesh_2d.upload_or_new(&renderer.wgpu, bytemuck::cast_slice(&lines), lines.len() as u64);
        renderer.render_model_inst(mesh_2d);

        self.window.as_ref().unwrap().pre_present_notify();

        let egui_state = self.egui_state.as_mut().unwrap();
        let egui_winit::egui::FullOutput {
            textures_delta,
            shapes,
            pixels_per_point,
            platform_output,
            ..
        } = egui_state.egui_ctx().end_pass();

        egui_state
            .handle_platform_output(&self.window.as_ref().unwrap(), platform_output);

        let paint_jobs = egui_state
            .egui_ctx()
            .tessellate(shapes, pixels_per_point);

        let size = self.window.as_ref().unwrap().inner_size();
        let screen_descriptor = {
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: [size.width, size.height],
                pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32,
            }
        };

        renderer.render_frame(self.window.as_ref().unwrap(), screen_descriptor, paint_jobs, textures_delta);

        if self.settings.render_config != prev_render_config {
            // renderer.rebuild_from_settings(&self.settings);
        } else if prev_viewport_size != self.data.viewport_rect {
                renderer.resize_viewport(self.data.viewport_rect.width() as u32, self.data.viewport_rect.height()as u32);
        }

        // match renderer.present(self.egui_state.as_mut().unwrap(), &self.window.get_handle()) {
        //     Ok(_) => (),

        //     Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        //         // renderer.resize_window(renderer.window_size)
        //     }
        //     Err(err @ wgpu::SurfaceError::Timeout) => {
        //         log::warn!("{err}")
        //     }
        //     Err(err) => {
        //         log::error!("{err}");
        //         ctrlflow.exit()
        //     }
        // }
    }

    fn on_scroll(&mut self, delta: &MouseScrollDelta) {
        // self.camera.process_scroll(&delta);
        self.camera.process_scroll(delta);
    }

    fn on_window_event(&mut self, event: &WindowEvent) -> bool {
        use WindowEvent as WE;

        self.egui_state.as_mut().unwrap().on_window_event(&self.window.as_ref().unwrap(), event);
        // self.egui_state
        //     .as_mut()
        //     .unwrap()
        //     .handle_input(&self.window.get_handle(), event);
        // self.renderer.as_mut().unwrap().input(event);

        match event {
            WE::CursorMoved { position, .. } => {
                let mut pos: Vec2 = (position.x as f32, position.y as f32).into();
                let prev_pos = self.data.mouse_pixel_pos;

                let vp_dim = self.data.viewport_dim();
                let vp_pixel_dim = vp_dim * self.data.ui_pixel_per_point;
                let vp_pos = self.pixel_to_vp_space(pos);
                let mut cursor_wrapped = false;

                if vp_pos.x < 0.0 {
                    pos.x += vp_pixel_dim.x;
                    cursor_wrapped = true;
                }
                if vp_pos.x >= vp_dim.x {
                    pos.x -= vp_pixel_dim.x;
                    cursor_wrapped = true;
                }
                if vp_pos.y < 0.0 {
                    pos.y += vp_pixel_dim.y;
                    cursor_wrapped = true;
                }
                if vp_pos.y >= vp_dim.y {
                    pos.y -= vp_pixel_dim.y;
                    cursor_wrapped = true;
                }

                #[cfg(target_arch = "wasm32")]
                {
                    cursor_wrapped = false;
                }

                self.data.mouse_pixel_pos = pos;

                if cursor_wrapped {
                    if self.data.viewport_dragged {
                        self.window.as_ref().unwrap().set_cursor_position(PhysicalPosition::new(pos.x, pos.y));
                    }
                } else {
                    // only compute dpos if no jump occured
                    self.data.mouse_delta = pos - prev_pos;
                }
                false
            }
            WE::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera.process_scroll(delta);
                true
            }
            _ => false,
        }
    }
    fn pixel_to_vp_space(&self, p: Vec2) -> Vec2 {
        p / self.data.ui_pixel_per_point - self.data.vp_rect_min()
    }

    fn vp_to_pixel_space(&self, p: Vec2) -> Vec2 {
        (p + self.data.vp_rect_min()) * self.data.ui_pixel_per_point
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resumed_native(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(winit::window::Window::default_attributes().with_title("Atlas"))
            .unwrap();

        let window_handle = Arc::new(window);
        self.window = Some(window_handle.clone());

        let size = window_handle.inner_size();
        let scale_factor = window_handle.scale_factor() as f32;

        let ui_context = egui::Context::default();
        // ui_context.set_pixels_per_point(scale_factor);
        let vp_id = ui_context.viewport_id();

        let ui_state = egui_winit::State::new(
            ui_context,
            vp_id,
            &window_handle,
            Some(scale_factor),
            Some(winit::window::Theme::Dark),
            None,
        );

        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            // .filter_module("atlas", log::LevelFilter::Info)
            // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Error)
            // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Warn)
            .format_timestamp(None)
            .init();
        let renderer = pollster::block_on(async move {
            AtlasRenderer::new_async(window_handle.clone(), size.width, size.height).await
        });

        let unit_rect = [
            Vertex {
                pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
            Vertex {
                pos: Vec4::new(0.0, 1.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
            Vertex {
                pos: Vec4::new(1.0, 0.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
            Vertex {
                pos: Vec4::new(1.0, 1.0, 0.0, 1.0),
                col: Vec4::ONE,
            },
        ];

        let wgpu = &renderer.wgpu;
        let vertex = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("unit_rect_vertex_buffer"),
            contents: bytemuck::cast_slice(&unit_rect),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let n_max_instances = 1024;

        let instance = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: n_max_instances * std::mem::size_of::<LineSegmentInst>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });


        self.mesh_2d = Some(ModelInstance {
            pipeline: load_line_shader(&wgpu).into(),
            vertex,
            n_vertices: 4,
            instance,
            n_instances: 0,
            instance_size: std::mem::size_of::<LineSegmentInst>() as u64,
            n_max_instances,
        });

        self.egui_state = Some(ui_state);
        self.renderer = Some(renderer);
        self.initialized = true;
        self.last_size = (size.width, size.height).into();
        // self.data.ui_pixel_per_point = scale_factor;
    }

    #[cfg(target_arch = "wasm32")]
    fn resumed_wasm(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut attributes = winit::window::Window::default_attributes().with_title("Atlas");

        use winit::platform::web::WindowAttributesExtWebSys;
        let canvas = wgpu::web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<wgpu::web_sys::HtmlCanvasElement>()
            .unwrap();
        let canvas_width = canvas.width().max(1);
        let canvas_height = canvas.height().max(1);
        self.last_size = (canvas_width, canvas_height).into();
        attributes = attributes.with_canvas(Some(canvas));

        if let Ok(window) = event_loop.create_window(attributes) {
            let first_window_handle = self.window.is_none();
            let window_handle = Arc::new(window);
            self.window = Some(window_handle.clone());

            if first_window_handle {
                let ui_context = egui::Context::default();

                // self.data.ui_pixel_per_point = window_handle.scale_factor() as f32;
                // #[cfg(target_arch = "wasm32")] 
                // {
                //     ui_context.set_pixels_per_point(window_handle.scale_factor() as f32);
                // }

                let viewport_id = ui_context.viewport_id();
                let ui_state = egui_winit::State::new(
                    ui_context,
                    viewport_id,
                    &window_handle,
                    Some(window_handle.scale_factor() as f32),
                    Some(winit::window::Theme::Dark),
                    None,
                );


                let (sender, receiver) = futures::channel::oneshot::channel();
                self.renderer_receiver = Some(receiver);
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));

                console_log::init().expect("Failed to initialize logger!");
                log::info!("Canvas dimensions: ({canvas_width} x {canvas_height})");

                wasm_bindgen_futures::spawn_local(async move {
                    let renderer =
                        AtlasRenderer::new_async(window_handle.clone(), canvas_width, canvas_height)
                        .await;
                    if sender.send(renderer).is_err() {
                        log::error!("Failed to create and send renderer!");
                    }
                });
                self.last_render_time = Some(Instant::now());
                self.egui_state = Some(ui_state);

            }
        }
    }
}

fn is_pressed(event: &KeyEvent, key_code: KeyCode) -> bool {
    match event {
        KeyEvent {
            state: ElementState::Pressed,
            physical_key: PhysicalKey::Code(kc),
            ..
        } => *kc == key_code,
        _ => false,
    }
}


impl ApplicationHandler for AtlasApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        self.resumed_native(event_loop);
        #[cfg(target_arch = "wasm32")]
        self.resumed_wasm(event_loop);
    }
    // fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    //     log::debug!("creating window...");
    //     let window = event_loop
    //         .create_window(winit::window::Window::default_attributes().with_title("Atlas"))
    //         .unwrap();

    //     // let gpu_ctx = gpu::WgpuContext::new(self.window.get_handle().clone());

    //     let (width, height) = (window.inner_size().width, window.inner_size().height);

    //     self.window = Some(window.into());

    //     let ui_context = egui::Context::default();
    //     let viewport_id = ui_context.viewport_id();
    //     let egui_state = egui_winit::State::new(
    //         ui_context,
    //         viewport_id,
    //         &self.window.as_ref().unwrap(),
    //         Some(self.window.as_ref().unwrap().scale_factor() as f32),
    //         Some(winit::window::Theme::Dark),
    //         None,
    //     );

    //     egui_state.egui_ctx().style_mut_of(egui::Theme::Dark, |style| {
    //         for (_text_style, font_id) in style.text_styles.iter_mut() {
    //             font_id.size = 16.0;
    //         }
    //     });
    //     egui_state.egui_ctx().style_mut_of(egui::Theme::Light, |style| {
    //         for (_text_style, font_id) in style.text_styles.iter_mut() {
    //             font_id.size = 16.0;
    //         }
    //     });
    //     // let mut egui_state = egui_state::EguiState::new(
    //     //     &wgpu.device,
    //     //     wgpu.surface_format,
    //     //     None,
    //     //     1,
    //     //     &self.window.get_handle(),
    //     // );
    //     self.renderer = AtlasRenderer::new(self.window.as_ref().unwrap().clone(), width, height).into();
    //     self.egui_state = Some(egui_state);

    //     let wgpu = &self.renderer.as_ref().unwrap().wgpu;

    //     let unit_rect = [
    //         Vertex {
    //             pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
    //             col: Vec4::ONE,
    //         },
    //         Vertex {
    //             pos: Vec4::new(0.0, 1.0, 0.0, 1.0),
    //             col: Vec4::ONE,
    //         },
    //         Vertex {
    //             pos: Vec4::new(1.0, 0.0, 0.0, 1.0),
    //             col: Vec4::ONE,
    //         },
    //         Vertex {
    //             pos: Vec4::new(1.0, 1.0, 0.0, 1.0),
    //             col: Vec4::ONE,
    //         },
    //     ];

    //     let vertex = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //         label: Some("unit_rect_vertex_buffer"),
    //         contents: bytemuck::cast_slice(&unit_rect),
    //         usage: wgpu::BufferUsages::VERTEX,
    //     });

    //     let n_max_instances = 1024;

    //     let instance = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
    //         label: Some("instance_buffer"),
    //         size: n_max_instances * std::mem::size_of::<LineSegmentInst>() as u64,
    //         usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    //         mapped_at_creation: false,
    //     });


    //     self.mesh_2d = Some(ModelInstance {
    //         pipeline: load_line_shader(&wgpu).into(),
    //         vertex,
    //         n_vertices: 4,
    //         instance,
    //         n_instances: 0,
    //         n_max_instances,
    //     });
    // }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if !self.try_init() {
            return
        }
        if let winit::event::DeviceEvent::MouseWheel { delta } = event {
            self.on_scroll(&delta);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if !self.try_init() {
            return
        }
        if self.window.as_ref().unwrap().id() == window_id && !self.on_window_event(&event) {
            use WindowEvent as WE;
            match event {
                WE::RedrawRequested => {
                    self.on_redraw(event_loop);
                }
                WE::Resized(PhysicalSize { width, height }) => {
                    let (width, height) = (width.max(1), height.max(1));
                    self.last_size = (width, height).into();
                    self.resize(width, height);
                    // self.renderer.as_mut().unwrap().resize(width, height);

                    // self.renderer.as_mut().unwrap().resize_window(physical_size);
                }
                WE::CloseRequested => event_loop.exit(),
                _ => (),
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.try_init() {
            return
        }
        self.window.as_ref().unwrap().request_redraw();
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if !self.try_init() {
            return
        }
        match cause {
            winit::event::StartCause::Init => (),
            _ => self.window.as_ref().unwrap().request_redraw(),
        }
    }
}

#[derive(Debug, derive_more::Display, Copy, Clone, PartialEq, EguiProbe, Default)]
pub enum CullMode {
    #[default]
    None,
    Front,
    Back,
}

impl From<CullMode> for Option<wgpu::Face> {
    fn from(value: CullMode) -> Self {
        match value {
            CullMode::None => None,
            CullMode::Front => Some(wgpu::Face::Front),
            CullMode::Back => Some(wgpu::Face::Back),
        }
    }
}

#[derive(Debug, derive_more::Display, Copy, Clone, PartialEq, EguiProbe, Default)]
pub enum PolygonMode {
    #[default]
    Fill,
    #[cfg(not(target_arch = "wasm32"))]
    Line,
}

impl From<PolygonMode> for wgpu::PolygonMode {
    fn from(value: PolygonMode) -> Self {
        match value {
            PolygonMode::Fill => wgpu::PolygonMode::Fill,
            #[cfg(not(target_arch = "wasm32"))]
            PolygonMode::Line => wgpu::PolygonMode::Line,
        }
    }
}


struct UniformBinding {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

pub struct WGPU {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl WGPU {
    pub fn aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height.max(1) as f32
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_framebuffer_resolve_texture(&self, width: u32, height: u32) -> wgpu::TextureView {
        let width = width.max(1);
        let height = height.max(1);
        let texture = self.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Framebuffer Resolve Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.surface_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(self.surface_format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
            usage: None,
        })
    }

    pub fn create_framebuffer_msaa_texture(&self, width: u32, height: u32) -> Option<wgpu::TextureView> {
        let width = width.max(1);
        let height = height.max(1);
        if !MULTISAMPLE {
            return None;
        }

        let texture = self.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Framebuffer Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: self.surface_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        Some(texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(self.surface_format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
            usage: None,
        }))
    }

    pub fn create_depth_texture(&self, width: u32, height: u32) -> wgpu::TextureView {
        let width = width.max(1);
        let height = height.max(1);
        let texture = self.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: if MULTISAMPLE { 4 } else { 1 },
                dimension: wgpu::TextureDimension::D2,
                format: AtlasRenderer::DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(AtlasRenderer::DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
            usage: None,
        })
    }

    pub async fn new_async(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(any(target_os = "linux"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_os = "macos")]
            backends: wgpu::Backends::METAL,
            #[cfg(target_os = "windows")]
            backends: wgpu::Backends::DX12 | wgpu::Backends::GL,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");
        let (device, queue) = {
            log::info!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("WGPU Device"),
                        memory_hints: wgpu::MemoryHints::default(),
                        // required_features: wgpu::Features::default(),
                        #[cfg(not(target_arch = "wasm32"))]
                        required_features: wgpu::Features::POLYGON_MODE_LINE,

                        #[cfg(target_arch = "wasm32")]
                        required_features: wgpu::Features::default(),

                        #[cfg(not(target_arch = "wasm32"))]
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                        #[cfg(all(target_arch = "wasm32", feature = "webgpu"))]
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                        #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                    },
                    None,
                )
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb()) // egui wants a non-srgb surface texture
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            surface_config,
            surface_format,
        }
    }
}

struct AtlasRenderer {
    framebuffer_msaa: Option<wgpu::TextureView>,
    framebuffer_resolve: wgpu::TextureView,
    fb_egui_id: egui::TextureId,
    depth_texture_view: wgpu::TextureView,
    // depthbuffer: gpu::Texture,

    show_vertices: bool,
    show_lines: bool,

    line_pipeline: wgpu::RenderPipeline,
    line_segments: wgpu::Buffer,
    n_line_segments: u32,

    world_uniform_binding: UniformBinding,
    world_uniform: WorldUniform,

    // surface: wgpu::Surface<'static>,
    // device: wgpu::Device,
    // queue: wgpu::Queue,
    // config: wgpu::SurfaceConfiguration,
    wgpu: WGPU,


    ui_renderer: egui_wgpu::Renderer,
    // drop last
}

fn build_mesh_2d(settings: &AtlasSettings) -> (Vec<Vertex>, Vec<LineSegmentInst>) {
    let start = Instant::now();
    let (vertices, segments) = iso4::build_2d(settings.iso_2d_config);

    log::info!(
        "extracted isosurface in: {} s / {} ms",
        (Instant::now() - start).as_secs_f64(),
        (Instant::now() - start).as_secs_f64() * 1000.0,
    );

    if !vertices.is_empty() {
        log::info!("#of vertices: {}", vertices.len());
    }
    if !segments.is_empty() {
        log::info!("#of segments: {}", segments.len());
    }
    (vertices, segments)
}

impl AtlasRenderer {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    async fn new_async(window: impl Into<wgpu::SurfaceTarget<'static>>, width: u32, height: u32) -> Self {

        let width = width.max(1);
        let height = height.max(1);

        let wgpu = WGPU::new_async(window, width, height).await;

        let viewport_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let world_uniform = WorldUniform::new(Mat4::IDENTITY, Vec3::ZERO);

        // let mut egui_state = egui_state::EguiState::new(&device, config.format, None, 1, &window);

        let mut ui_renderer = egui_wgpu::Renderer::new(
            &wgpu.device,
            wgpu.surface_format,
            None,
            1,
            false,
        );

        // let framebuffer_msaa = gpu::TextureConfig::d2(viewport_size, wgpu.surface_format)
        //     .msaa_samples(4)
        //     .as_render_attachment()
        //     .as_texture_binding()
        //     .build(&wgpu.device);

        // let framebuffer = gpu::TextureConfig::d2(viewport_size, wgpu.surface_format)
        //     .as_render_attachment()
        //     .as_texture_binding()
        //     // .use_with_egui(egui_state)
        //     .build(&wgpu.device);

        let framebuffer_msaa = wgpu.create_framebuffer_msaa_texture(width, height);
        let framebuffer_resolve = wgpu.create_framebuffer_resolve_texture(width, height);

        let fb_egui_id = ui_renderer.register_native_texture(
            &wgpu.device,
            &framebuffer_resolve,
            wgpu::FilterMode::Linear,
        );

        // let depthbuffer = gpu::TextureConfig::depthf32(viewport_size)
        //     .msaa_samples(4)
        //     .as_render_attachment()
        //     .as_texture_binding()
        //     .build(&wgpu.device);


        let world_buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "world_buffer".into(),
            contents: bytemuck::cast_slice(&[world_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let world_bind_group_layout =
            wgpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: "world_bind_group_layout".into(),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let world_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: "world_bind_group".into(),
            layout: &world_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: world_buffer.as_entire_binding(),
            }],
        });

        log::debug!("setup framebuffers");

        //let module = gpu::ShaderConfig::from_wgsl(include_str!("shader.wgsl"))
        let mesh_shader = gpu::ShaderConfig::from_wgsl(include_str!("shader.wgsl"))
            .with_struct::<Vertex>("VertexInput")
            .with_struct::<WorldUniform>("WorldUniform")
            .build(&wgpu.device);

        log::debug!("finish initializing wgpu context");

        // let line_shader = gpu::ShaderConfig::from_wgsl(include_str!("line.wgsl"))
        //     .with_struct::<Vertex>("VertexInput")
        //     .with_struct::<WorldUniform>("WorldUniform")
        //     .build(&wgpu.device);

        let line_pipeline = load_line_shader(&wgpu);

        // let line_pipeline = gpu::PipelineConfig::new(&line_shader)
        //     .color::<Vertex>(wgpu.surface_format)
        //     .depth_format(Self::DEPTH_FORMAT)
        //     .with_instances::<LineSegmentInst>()
        //     .msaa_samples(4)
        //     .set_cull_mode(CullMode::None.into())
        //     // .polygon_mode(settings.render_config.polygon_mode.into())
        //     .primitive_topology(wgpu::PrimitiveTopology::TriangleStrip)
        //     .bind_group_layouts(&[&world_bind_group_layout])
        //     .label("line pipeline")
        //     .build(&wgpu.device);

        let line_segments = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line segments"),
            size: 128 * std::mem::size_of::<[Vec3; 2]>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let n_line_segments = 0;


        let show_vertices = false;
        let show_lines = true;

        let world_uniform_binding = UniformBinding {
            buffer: world_buffer,
            bind_group: world_bind_group,
            bind_group_layout: world_bind_group_layout,
        };

        let depth_texture_view = wgpu.create_depth_texture(width, height);

        Self {
            wgpu,
            framebuffer_msaa,
            framebuffer_resolve,
            depth_texture_view,
            // depthbuffer,
            show_vertices,
            show_lines,
            fb_egui_id,
            // mesh_pipeline,
            // mesh_verts,
            // mesh_indxs,
            line_pipeline,
            line_segments,
            n_line_segments,
            // n_indices,
            world_uniform_binding,
            world_uniform,
            // egui_state,
            ui_renderer,
        }
    }

    fn rebuild_mesh(&mut self, settings: &AtlasSettings) {
        self.show_vertices = settings.show_tree;
        self.show_lines = settings.show_mesh;

        self.show_vertices = true;
        let (vertices, segments) = build_mesh_2d(settings);

        self.n_line_segments = segments.len() as u32;

        if !segments.is_empty() {
            self.line_segments =
                self.wgpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("line segments"),
                        contents: bytemuck::cast_slice(&segments),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
        } else {
            self.show_lines = false;
        }
    }

    fn resize_viewport(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.framebuffer_msaa = self.wgpu.create_framebuffer_msaa_texture(width, height);
        self.framebuffer_resolve = self.wgpu.create_framebuffer_resolve_texture(width, height);
        self.depth_texture_view = self.wgpu.create_depth_texture(width, height);

        self.fb_egui_id = self.ui_renderer.register_native_texture(
            &self.wgpu.device,
            &self.framebuffer_resolve,
            wgpu::FilterMode::Linear,
        );
    }

    // fn resize_window(&mut self, new_size: PhysicalSize<u32>) {
    //     if new_size.width == 0 || new_size.height == 0 {
    //         return;
    //     }

    //     self.config.width = new_size.width;
    //     self.config.height = new_size.height;
    //     self.surface.configure(&self.device, &self.config);
    // }

    fn update_world_uniform(&mut self) {
        self.wgpu.queue.write_buffer(
            &self.world_uniform_binding.buffer,
            0,
            bytemuck::cast_slice(&[self.world_uniform]),
        );
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.wgpu.resize(width, height);
        // self.depth_texture_view = self.wgpu.create_depth_texture(width, height);
        // self.framebuffer_msaa = self.wgpu.create_framebuffer_msaa_texture(width, height);
        // self.framebuffer_resolve = self.wgpu.create_framebuffer_resolve_texture(width, height);

        // self.fb_egui_id = self.ui_renderer.register_native_texture(
        //     &self.wgpu.device,
        //     &self.framebuffer_resolve,
        //     wgpu::FilterMode::Linear,
        // );
    }

    fn render_model_inst(&self, model: &ModelInstance) {
        if self.wgpu.surface_config.width == 1 || self.wgpu.surface_config.height == 1 {
            return
        }

        let mut encoder =
            self.wgpu.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.framebuffer_msaa.as_ref().unwrap_or(&self.framebuffer_resolve),
                    resolve_target: if MULTISAMPLE { Some(&self.framebuffer_resolve) } else { None },
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(hex_to_col("#1b1b1b")),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                label: None,
                occlusion_query_set: None,
            });

            if model.n_instances != 0 {
                println!("{:#?}", model);
                render_pass.set_vertex_buffer(0, model.vertex.slice(0..model.n_vertices * std::mem::size_of::<Vertex>() as u64));
                render_pass.set_vertex_buffer(1, model.instance.slice(0..model.n_instances * std::mem::size_of::<LineSegmentInst>() as u64));
                render_pass.set_pipeline(&model.pipeline);
                render_pass.set_bind_group(0, &self.world_uniform_binding.bind_group, &[]);
                render_pass.draw(0..model.n_vertices as u32, 0..model.n_instances as u32);
            }
        }


        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
    }
    
    //fn render_mesh(&mut self) {
    //    let segment_verts = vec![
    //        Vertex {
    //            pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //        Vertex {
    //            pos: Vec4::new(0.0, 1.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //        Vertex {
    //            pos: Vec4::new(1.0, 0.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //        Vertex {
    //            pos: Vec4::new(1.0, 1.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //    ];

    //    let segment_buffer = self
    //        .wgpu.device
    //        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //            label: Some("line segment"),
    //            contents: bytemuck::cast_slice(&segment_verts),
    //            usage: wgpu::BufferUsages::VERTEX,
    //        });

    //    let mut encoder =
    //        self.wgpu.device
    //            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //                label: Some("Render Encoder"),
    //            });

    //    //self.viewport_sc.render(&mut self.active_encoder);
    //    gpu::RenderPass::target_color(&self.framebuffer_msaa)
    //        .set_if(true, |rp| {
    //            rp.depth_target(&self.depth_texture_view)
    //        })
    //        .resolve_target(&self.framebuffer_resolve)
    //        .clear_hex("#24273a")
    //        .draw(&mut encoder, |mut rpass| {
    //            rpass.set_bind_group(0, &self.world_uniform_binding.bind_group, &[]);

    //                rpass.set_vertex_buffer(0, segment_buffer.slice(..));
    //                rpass.set_vertex_buffer(1, self.line_segments.slice(..));
    //                rpass.set_pipeline(&self.line_pipeline);
    //                rpass.draw(0..4_u32, 0..self.n_line_segments);
    //        });

    //    self.wgpu.queue.submit(std::iter::once(encoder.finish()));
    //}

    //fn render_mesh(&mut self) {
    //    let segment_verts = vec![
    //        Vertex {
    //            pos: Vec4::new(0.0, 0.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //        Vertex {
    //            pos: Vec4::new(0.0, 1.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //        Vertex {
    //            pos: Vec4::new(1.0, 0.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //        Vertex {
    //            pos: Vec4::new(1.0, 1.0, 0.0, 1.0),
    //            col: Vec4::ONE,
    //        },
    //    ];

    //    let segment_buffer = self
    //        .wgpu.device
    //        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //            label: Some("line segment"),
    //            contents: bytemuck::cast_slice(&segment_verts),
    //            usage: wgpu::BufferUsages::VERTEX,
    //        });

    //    //self.viewport_sc.render(&mut self.active_encoder);
    //    gpu::RenderPass::target_color(&self.framebuffer_msaa)
    //        .set_if(true, |rp| {
    //            rp.depth_target(&self.depthbuffer)
    //        })
    //        .resolve_target(&self.framebuffer_resolve)
    //        .clear_hex("#24273a")
    //        .draw(&mut self.active_encoder, |mut rpass| {
    //            rpass.set_bind_group(0, &self.world_uniform_binding.bind_group, &[]);

    //            if self.show_lines {
    //                rpass.set_vertex_buffer(0, segment_buffer.slice(..));
    //                rpass.set_vertex_buffer(1, self.line_segments.slice(..));
    //                rpass.set_pipeline(&self.line_pipeline);
    //                rpass.draw(0..4_u32, 0..self.n_line_segments);
    //            }
    //        });
    //}

    // fn new_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    //     device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //         label: "main encoder".into(),
    //     })
    // }
    pub fn render_frame(&mut self, window: &winit::window::Window, screen_descriptor: egui_wgpu::ScreenDescriptor, paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta) {
        println!("render frame");


        for (id, image_delta) in &textures_delta.set {
            self.ui_renderer
                .update_texture(&self.wgpu.device, &self.wgpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.ui_renderer.free_texture(id);
        }

        let mut encoder =
            self.wgpu.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        self.ui_renderer.update_buffers(
            &self.wgpu.device,
            &self.wgpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture = self
            .wgpu.surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.wgpu.surface_format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: None,
                });

        encoder.insert_debug_marker("Render scene");

        if self.wgpu.surface_config.width > 1 && self.wgpu.surface_config.height > 1 
        {
            let mut render_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    label: Some("egui main render pass"),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();

            self.ui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}
