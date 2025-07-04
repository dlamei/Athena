mod camera;
pub mod graph_3d_shader;
pub mod iso;
pub mod iso_3d;
// pub mod pdb;
mod ui;

pub mod vm;
pub mod vm2;

pub extern crate self as atlas;

use camera::Camera;
// use macros::ShaderStruct;

use egui::Rect;

use egui_probe::EguiProbe;
use glam::{DVec3, DVec4, Mat4, UVec2, Vec2, Vec3, Vec4, Vec4Swizzles};
use std::rc::Rc;
use std::sync::Arc;
use web_time::{Duration, Instant};
use wgpu::util::DeviceExt;
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

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec4,
    pub col: Vec4,
}

#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct LineSegmentInst {
    pub a: Vec3,
    pub b: Vec3,
}

impl Vertex {
    pub fn new(pos: Vec3, col: Vec4) -> Self {
        Self {
            pos: pos.extend(0.0),
            col,
        }
    }
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
        [r, g, b] => (*r, *g, *b, 255),
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

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct WorldUniform {
    pub light_pos: Vec3,
    pub _pad0: f32,
    pub camera_pos: Vec3,
    pub _pad1: f32,

    pub line_thickness_and_pad: Vec4,
    pub view: Mat4,
    pub proj: Mat4,
    // pub view_proj: Mat4,
}

impl WorldUniform {
    pub fn new(view: Mat4, proj: Mat4, light_pos: Vec3) -> Self {
        Self {
            light_pos,
            camera_pos: Vec3::ZERO,
            line_thickness_and_pad: Vec4::new(0.1, 0., 0., 0.),
            view,
            proj,

            // view_proj,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }

    pub fn layout(wgpu: &WGPU) -> wgpu::BindGroupLayout {
        wgpu.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            })
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

#[derive(Debug, Clone, PartialEq, EguiProbe)]
struct AtlasSettings {
    iso_2d_config: iso::Iso2DConfig,
    iso_3d_config: iso_3d::Iso3DConfig,
    // iso_3d_config: iso::Iso3DConfig,
    #[egui_probe(skip)]
    show_tree: bool,
    #[egui_probe(skip)]
    show_mesh: bool,

    camera_mode: camera::CameraKind,
    lock_zoom: bool,

    // #[egui_probe(with ui::button_probe("rebuild"))]
    rebuild_mesh: bool,
    #[egui_probe(skip)]
    mesh_gen: MeshGenerator,
    #[egui_probe(skip)]
    render_config: RenderConfig,
}

impl Default for AtlasSettings {
    fn default() -> Self {
        Self {
            iso_2d_config: iso::Iso2DConfig {
                min: [-10.0, -10.0].into(),
                max: [10.0, 10.0].into(),
                intrvl_depth: 4,
                subdiv_depth: 4,
                line_thickness: 1.5,
                ..Default::default()
            },
            iso_3d_config: Default::default(),
            camera_mode: camera::CameraKind::Orbit,
            lock_zoom: true,

            rebuild_mesh: true,
            show_tree: false,
            show_mesh: true,
            mesh_gen: MeshGenerator::Iso2D,
            render_config: RenderConfig {
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
    fn new_rect_inst(
        wgpu: &WGPU,
        pipeline: impl Into<Rc<wgpu::RenderPipeline>>,
        data: &[u8],
        instance_size: u64,
        n_max_instances: u64,
    ) -> Self {
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

        let vertex = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
            let instance = wgpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

pub enum AtlasApp {
    UnInit {
        window: Option<Arc<Window>>,
        #[cfg(target_arch = "wasm32")]
        renderer_receiver: Option<futures::channel::oneshot::Receiver<AtlasRenderer>>,
    },
    Init(AppData),
}

impl Default for AtlasApp {
    fn default() -> Self {
        Self::UnInit {
            window: None,
            #[cfg(target_arch = "wasm32")]
            renderer_receiver: None,
        }
    }
}

impl AtlasApp {
    fn is_init(&self) -> bool {
        matches!(self, Self::Init(_))
    }

    fn init_app(window: Arc<Window>, renderer: AtlasRenderer) -> AppData {
        let ui_context = egui::Context::default();
        let vp_id = ui_context.viewport_id();

        let scale_factor = window.scale_factor() as f32;

        let ui_state = egui_winit::State::new(
            ui_context,
            vp_id,
            &window,
            Some(scale_factor),
            Some(winit::window::Theme::Dark),
            None,
        );

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
        let vertex = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

        let mesh_2d = ModelInstance {
            pipeline: load_line_shader(&wgpu).into(),
            vertex,
            n_vertices: 4,
            instance,
            n_instances: 0,
            instance_size: std::mem::size_of::<LineSegmentInst>() as u64,
            n_max_instances,
        };

        let pipeline_3d = graph_3d_shader::Pipeline::init(&wgpu);

        let data = WindowData {
            mouse_pixel_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            viewport_dragged: false,
            viewport_rect: Rect {
                min: egui::Pos2::ZERO,
                max: egui::Pos2::new(1.0, 1.0),
            },
            ui_pixel_per_point: 0.0,
            delta_time: Duration::ZERO,
            mesh_gen_time: 0.0,
            prev_frame_time: Instant::now(),
        };

        let camera_controll = camera::CameraController::orbit(-Vec3::splat(2.0), Vec3::ZERO, 90.0);

        // let camera_controll = camera::CameraControll {
        //     kind: camera::CameraKind::Orbit,
        //     fov_rad: 90.0f32.to_radians(),
        //     aspect: 1.0,
        //     vp_height: 1.0,
        //     vp_width: 1.0,
        //     z_near: 0.0001,
        //     z_far: 1000.0,
        //     anim_len: 0.5,
        //     center: DVec3::ZERO,
        //     zoom: 1.0,
        //     yaw: 0.0,
        //     pitch: -1.0,

        //     d_pitch: 0.0,
        //     d_yaw: 0.0,
        //     d_zoom: 0.0,
        //     d_pos: DVec3::ZERO,
        // };

        AppData {
            window: window,
            renderer: renderer,
            camera_controll,
            // camera: Camera::pan_2d(Vec2::ZERO, 1.0),
            pos_3d: Vec3::splat(2.0),
            pos_2d: Vec2::ZERO,
            ui_state: ui::UiState::new(),
            data,
            settings: AtlasSettings::default(),
            egui_state: ui_state,
            mesh_2d,
            pipeline_3d,
            last_size: UVec2::ZERO,
            last_render_time: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resumed_native(&mut self, event_loop: &ActiveEventLoop) {
        if self.is_init() {
            return;
        }

        let window = event_loop
            .create_window(winit::window::Window::default_attributes().with_title("Atlas"))
            .unwrap();

        let window_handle = Arc::new(window);
        // self.window = Some(window_handle.clone());

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
            .filter_module("cranelift_jit::backend", log::LevelFilter::Warn)
            // .filter_module("atlas", log::LevelFilter::Info)
            // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Error)
            // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Warn)
            .format_timestamp(None)
            .init();

        let window_handle_2 = window_handle.clone();
        let renderer = pollster::block_on(async move {
            AtlasRenderer::new_async(window_handle_2, size.width, size.height).await
        });

        *self = Self::Init(Self::init_app(window_handle, renderer));
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
        // self.last_size = (canvas_width, canvas_height).into();
        attributes = attributes.with_canvas(Some(canvas));

        if let Ok(new_window) = event_loop.create_window(attributes) {
            if let Self::UnInit {
                window,
                renderer_receiver,
            } = self
            {
                let first_window_handle = window.is_none();
                let window_handle = Arc::new(new_window);

                if first_window_handle {
                    let (sender, receiver) = futures::channel::oneshot::channel();
                    // self.renderer_receiver = Some(receiver);
                    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

                    console_log::init().expect("Failed to initialize logger!");
                    log::info!("Canvas dimensions: ({canvas_width} x {canvas_height})");

                    let window_handle_2 = window_handle.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let renderer =
                            AtlasRenderer::new_async(window_handle_2, canvas_width, canvas_height)
                                .await;
                        if sender.send(renderer).is_err() {
                            log::error!("Failed to create and send renderer!");
                        }
                    });

                    *window = Some(window_handle);
                    *renderer_receiver = Some(receiver);
                }
            }
        }
    }

    fn try_init(&mut self) -> Option<&mut AppData> {
        if let Self::Init(app) = self {
            return Some(app);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let Self::UnInit {
                window,
                renderer_receiver,
            } = self
            else {
                panic!();
            };
            // let mut renderer_received = false;
            if let Some(receiver) = renderer_receiver.as_mut() {
                if let Ok(Some(renderer)) = receiver.try_recv() {
                    *self = Self::Init(Self::init_app(window.as_ref().unwrap().clone(), renderer));
                    if let Self::Init(app) = self {
                        return Some(app);
                    }
                }
            }
        }

        None
    }
}

impl ApplicationHandler for AtlasApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        self.resumed_native(event_loop);
        #[cfg(target_arch = "wasm32")]
        self.resumed_wasm(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(app) = self.try_init() {
            app.window_event(event_loop, window_id, event);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(app) = self.try_init() {
            app.device_event(event_loop, device_id, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.try_init() {
            app.about_to_wait(event_loop);
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if let Some(app) = self.try_init() {
            app.new_events(event_loop, cause);
        }
    }
}

struct AppData {
    renderer: AtlasRenderer,
    camera_controll: camera::CameraController,

    pos_3d: Vec3,
    pos_2d: Vec2,

    ui_state: ui::UiState,
    egui_state: egui_winit::State,

    data: WindowData,
    settings: AtlasSettings,

    mesh_2d: ModelInstance,

    pipeline_3d: graph_3d_shader::Pipeline,

    last_size: UVec2,
    last_render_time: Option<Instant>,
    window: Arc<Window>,
}

// fn load_atom_shader(wgpu: &WGPU) -> wgpu::RenderPipeline {
//     let world_bind_group_layout =
//         wgpu.device
//             .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//                 label: "world_bind_group_layout".into(),
//                 entries: &[wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Uniform,
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 },
//                 ],
//             });

//     let molecule_bind_group_layout =
//         wgpu.device
//         .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             label: "symmetries_bind_group_layout".into(),
//             entries: &[
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::VERTEX,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Storage { read_only: true },
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: wgpu::ShaderStages::VERTEX,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Storage { read_only: true },
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 },
//                 ],
//         });

//     let atom_shader_module = wgpu
//         .device
//         .create_shader_module(wgpu::include_wgsl!("atom_shader.wgsl"));

//     let atom_pipeline_layout =
//         wgpu.device
//         .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//             label: Some("atom_pipeline_layout"),
//             bind_group_layouts: &[&world_bind_group_layout, &molecule_bind_group_layout],
//             push_constant_ranges: &[],
//         });

//     let atom_pipeline = wgpu
//         .device
//         .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//             label: Some("atom_pipeline"),
//             layout: Some(&atom_pipeline_layout),
//             vertex: wgpu::VertexState {
//                 module: &atom_shader_module,
//                 entry_point: Some("vs_main"),
//                 compilation_options: Default::default(),
//                 buffers: &[
//                     wgpu::VertexBufferLayout {
//                         attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
//                     },
//                 ],
//             },
//             fragment: Some(wgpu::FragmentState {
//                 module: &atom_shader_module,
//                 entry_point: Some("fs_main"),
//                 compilation_options: Default::default(),
//                 targets: &[Some(wgpu::ColorTargetState {
//                     format: wgpu.surface_format,
//                     blend: Some(wgpu::BlendState::ALPHA_BLENDING),
//                     write_mask: wgpu::ColorWrites::ALL,
//                 })],
//             }),
//             primitive: wgpu::PrimitiveState {
//                 cull_mode: None,
//                 unclipped_depth: false,
//                 front_face: wgpu::FrontFace::Ccw,
//                 polygon_mode: wgpu::PolygonMode::Fill,
//                 strip_index_format: None,
//                 topology: wgpu::PrimitiveTopology::TriangleStrip,
//                 conservative: false,
//             },
//             depth_stencil: Some(wgpu::DepthStencilState {
//                 format: AtlasRenderer::DEPTH_FORMAT,
//                 depth_write_enabled: true,
//                 depth_compare: wgpu::CompareFunction::Less,
//                 stencil: wgpu::StencilState::default(),
//                 bias: wgpu::DepthBiasState::default(),
//             }),
//             multisample: multisample_state(),
//             multiview: None,
//             cache: None,
//         });

//     atom_pipeline
// }

fn load_line_shader(wgpu: &WGPU) -> wgpu::RenderPipeline {
    let world_bind_group_layout =
        wgpu.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let line_shader_module = wgpu
        .device
        .create_shader_module(wgpu::include_wgsl!("line.wgsl"));

    let line_pipeline_layout =
        wgpu.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("line_pipeline_layout"),
                bind_group_layouts: &[&world_bind_group_layout],
                push_constant_ranges: &[],
            });

    let line_pipeline = wgpu
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line_pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineSegmentInst>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x3, 3 => Float32x3],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader_module,
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

    line_pipeline
}

impl AppData {
    fn rebuild_mesh_2d(&mut self) -> (Vec<Vertex>, Vec<LineSegmentInst>) {
        let (verts, mut lines) = build_mesh_2d(&self.settings);
        for l in &mut lines {
            l.a = l.a * 2.0;
            l.b = l.b * 2.0;
        }
        // println!(
        //     "{}",
        //     (std::mem::size_of::<LineSegmentInst>() * lines.len()) as f64 / 1e6 as f64
        // );
        (verts, lines)
    }

    fn resize(&mut self, w: u32, h: u32) {
        let w = w.max(1);
        let h = h.max(1);
        self.renderer.resize(w, h);

        let vp_rect = self.data.viewport_rect;
        let vp_w = (vp_rect.width() as u32).max(1);
        let vp_h = (vp_rect.height() as u32).max(1);
        self.renderer.resize_viewport(vp_w, vp_h);

        let scale_factor = self.window.scale_factor() as f32;
    }

    fn on_redraw(&mut self, ctrlflow: &ActiveEventLoop) {
        let prev_time = self.data.prev_frame_time;
        let curr_time = Instant::now();
        let dt = curr_time - prev_time;

        self.data.prev_frame_time = curr_time;
        self.data.delta_time = dt;

        self.camera_controll.set_aspect(
            self.data.viewport_rect.width() as u32,
            self.data.viewport_rect.height() as u32,
        );
        self.camera_controll.time_step(dt);

        if self.data.viewport_dragged {
            self.camera_controll
                .process_mouse(self.data.mouse_delta.x, self.data.mouse_delta.y);
        }

        let prev_viewport_size = self.data.viewport_rect;
        let prev_render_config = self.settings.render_config;
        // let prev_camera_mode = self.settings.camera_mode;
        //let mut settings = self.settings;

        let raw_input = self.egui_state.take_egui_input(&self.window);

        self.egui_state.egui_ctx().begin_pass(raw_input);
        self.data.ui_pixel_per_point = self.window.scale_factor() as f32;
        // self.data.ui_pixel_per_point = egui_state.egui_ctx().input(|i| i.pixels_per_point);

        let access = ui::UiAccess {
            vp_texture: self.renderer.fb_egui_id,
            // camera: &self.camera,
            window_info: &mut self.data,
            settings: &mut self.settings,
        };

        self.ui_state.ui(&self.egui_state.egui_ctx(), access);

        self.camera_controll.fov_rad = self.settings.render_config.fov.to_radians();
        // self.camera.config.fov_rad = self.settings.render_config.fov.to_radians();
        self.data.mouse_delta = Vec2::ZERO;
        self.camera_controll
            .set_camera_kind(self.settings.camera_mode);

        let (min, max) = self.camera_controll.pan_get_bounds();
        self.settings.iso_2d_config.min = min.into();
        self.settings.iso_2d_config.max = max.into();
        self.settings.iso_3d_config.max = max.extend(max.x).into();
        self.settings.iso_3d_config.min = min.extend(min.x).into();

        let mut verts = vec![];
        if self.settings.rebuild_mesh {
            let start = Instant::now();
            let (vertss, lines) = self.rebuild_mesh_2d();
            verts = vertss;
            let end = Instant::now();
            self.data.mesh_gen_time = (end - start).as_secs_f64() * 1000.0;

            self.mesh_2d.upload_or_new(
                &self.renderer.wgpu,
                bytemuck::cast_slice(&lines),
                lines.len() as u64,
            );
        }

        let vp_size = self.data.viewport_dim();
        // let vp_size = renderer.viewport_size;
        let (vp_w, vp_h) = (vp_size.x as f32, vp_size.y as f32);

        self.renderer.world_uniform.line_thickness_and_pad.x =
            self.settings.iso_2d_config.line_thickness / (vp_w * vp_w + vp_h * vp_h).sqrt();

        if self.settings.lock_zoom {
            // self.renderer.world_uniform.view_proj = self.camera_controll.view_proj_mat_zoomed();
            self.renderer.world_uniform.view = self.camera_controll.view_mat_zoomed();
            self.renderer.world_uniform.proj = self.camera_controll.proj_mat();
        } else {
            // self.renderer.world_uniform.view_proj = self.camera_controll.view_proj_mat();
            self.renderer.world_uniform.view = self.camera_controll.view_mat();
            self.renderer.world_uniform.proj = self.camera_controll.proj_mat();
        }
        self.renderer.world_uniform.camera_pos = self.camera_controll.orbit_eye();
        //self.renderer.world_uniform.light_pos = self.camera_controll.orbit_eye();
        self.renderer.world_uniform.light_pos = Vec3::new(1000., 1000., 0.).normalize();
        self.renderer.update_world_uniform();

        self.renderer.render_model_inst(&self.mesh_2d);

        self.pipeline_3d
            .update(&self.renderer.wgpu, &self.settings.iso_3d_config);
        self.renderer.render_3d_graph(&self.pipeline_3d);
        // self.pipeline_3d.render_2d_vertex(&self.renderer.wgpu);
        if self.settings.iso_2d_config.debug {
            // self.pipeline_3d.upload_verts(&self.renderer.wgpu, &verts);
        }

        // self.window.as_ref().unwrap().pre_present_notify();
        self.window.pre_present_notify();

        let egui_winit::egui::FullOutput {
            textures_delta,
            shapes,
            pixels_per_point,
            platform_output,
            ..
        } = self.egui_state.egui_ctx().end_pass();

        self.egui_state
            .handle_platform_output(&self.window, platform_output);

        let paint_jobs = self
            .egui_state
            .egui_ctx()
            .tessellate(shapes, pixels_per_point);

        let size = self.window.inner_size();
        let screen_descriptor = {
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: [size.width, size.height],
                pixels_per_point: self.window.scale_factor() as f32,
            }
        };

        self.renderer
            .render_frame(&self.window, screen_descriptor, paint_jobs, textures_delta);

        if self.settings.render_config != prev_render_config {
            // renderer.rebuild_from_settings(&self.settings);
        } else if prev_viewport_size != self.data.viewport_rect {
            self.renderer.resize_viewport(
                self.data.viewport_rect.width() as u32,
                self.data.viewport_rect.height() as u32,
            );
        }
    }

    fn on_scroll(&mut self, delta: &MouseScrollDelta) {
        // self.camera.process_scroll(&delta);
        self.camera_controll.process_scroll(delta);
    }

    fn on_window_event(&mut self, event: &WindowEvent) -> bool {
        use WindowEvent as WE;

        self.egui_state.on_window_event(&self.window, event);

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
                        self.window
                            .set_cursor_position(PhysicalPosition::new(pos.x, pos.y));
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
            } => false, //self.camera_controll.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controll.process_scroll(delta);
                true
            }
            _ => false,
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.window.id() == window_id && !self.on_window_event(&event) {
            use WindowEvent as WE;
            match event {
                WE::RedrawRequested => {
                    self.on_redraw(event_loop);
                }
                WE::Resized(PhysicalSize { width, height }) => {
                    let (width, height) = (width.max(1), height.max(1));
                    self.last_size = (width, height).into();
                    self.resize(width, height);
                }
                WE::CloseRequested => event_loop.exit(),
                _ => (),
            }
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseWheel { delta } = event {
            self.on_scroll(&delta);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.window.request_redraw();
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        match cause {
            winit::event::StartCause::Init => (),
            _ => self.window.request_redraw(),
        }
    }

    fn pixel_to_vp_space(&self, p: Vec2) -> Vec2 {
        p / self.data.ui_pixel_per_point - self.data.vp_rect_min()
    }

    fn vp_to_pixel_space(&self, p: Vec2) -> Vec2 {
        (p + self.data.vp_rect_min()) * self.data.ui_pixel_per_point
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

    pub fn create_framebuffer_msaa_texture(
        &self,
        width: u32,
        height: u32,
    ) -> Option<wgpu::TextureView> {
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

    let (vertices, segments) = iso::build_2d(&settings.iso_2d_config);

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

    async fn new_async(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let wgpu = WGPU::new_async(window, width, height).await;

        let viewport_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let world_uniform = WorldUniform::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO);

        let mut ui_renderer =
            egui_wgpu::Renderer::new(&wgpu.device, wgpu.surface_format, None, 1, false);

        let framebuffer_msaa = wgpu.create_framebuffer_msaa_texture(width, height);
        let framebuffer_resolve = wgpu.create_framebuffer_resolve_texture(width, height);

        let fb_egui_id = ui_renderer.register_native_texture(
            &wgpu.device,
            &framebuffer_resolve,
            wgpu::FilterMode::Linear,
        );

        let world_buffer = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: "world_buffer".into(),
                contents: bytemuck::cast_slice(&[world_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let world_bind_group_layout =
            wgpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        log::debug!("finish initializing wgpu context");

        let line_pipeline = load_line_shader(&wgpu);

        let line_segments = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line segments"),
            size: 128 * std::mem::size_of::<[Vec3; 2]>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let depth_texture_view = wgpu.create_depth_texture(width, height);

        let n_line_segments = 0;

        let world_uniform_binding = UniformBinding {
            buffer: world_buffer,
            bind_group: world_bind_group,
            bind_group_layout: world_bind_group_layout,
        };

        Self {
            wgpu,
            framebuffer_msaa,
            framebuffer_resolve,
            depth_texture_view,
            // depthbuffer,
            fb_egui_id,
            // mesh_pipeline,
            // mesh_verts,
            // mesh_indxs,
            line_pipeline,
            line_segments,
            n_line_segments,

            world_uniform_binding,
            world_uniform,
            // egui_state,
            ui_renderer,
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

    fn render_3d_graph(&self, pipeline: &graph_3d_shader::Pipeline) {
        let resolve = if MULTISAMPLE {
            Some(&self.framebuffer_resolve)
        } else {
            None
        };
        let target = self
            .framebuffer_msaa
            .as_ref()
            .unwrap_or(&self.framebuffer_resolve);

        pipeline.render(
            &self.wgpu,
            target,
            &self.depth_texture_view,
            &self.world_uniform_binding.bind_group,
            resolve,
        );
    }

    fn render_model_inst(&self, model: &ModelInstance) {
        if self.wgpu.surface_config.width == 1 || self.wgpu.surface_config.height == 1 {
            return;
        }

        let mut encoder =
            self.wgpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self
                        .framebuffer_msaa
                        .as_ref()
                        .unwrap_or(&self.framebuffer_resolve),
                    resolve_target: if MULTISAMPLE {
                        Some(&self.framebuffer_resolve)
                    } else {
                        None
                    },
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
                render_pass.set_vertex_buffer(
                    0,
                    model
                        .vertex
                        .slice(0..model.n_vertices * std::mem::size_of::<Vertex>() as u64),
                );
                render_pass.set_vertex_buffer(
                    1,
                    model.instance.slice(
                        0..model.n_instances * std::mem::size_of::<LineSegmentInst>() as u64,
                    ),
                );
                render_pass.set_pipeline(&model.pipeline);
                render_pass.set_bind_group(0, &self.world_uniform_binding.bind_group, &[]);
                render_pass.draw(0..model.n_vertices as u32, 0..model.n_instances as u32);
            }
        }

        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn render_frame(
        &mut self,
        window: &winit::window::Window,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.ui_renderer
                .update_texture(&self.wgpu.device, &self.wgpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.ui_renderer.free_texture(id);
        }

        let mut encoder =
            self.wgpu
                .device
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
            .wgpu
            .surface
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

        if self.wgpu.surface_config.width > 1 && self.wgpu.surface_config.height > 1 {
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
