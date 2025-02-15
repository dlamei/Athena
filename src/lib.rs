mod camera;
mod egui_state;
mod gpu;
mod gui;

pub mod iso;
// mod iso2;
mod ui;

pub mod vm;

pub extern crate self as atlas;

use atl_macro::ShaderStruct;
use camera::OrbitCamera;

use egui::Rect;

use egui_probe::EguiProbe;
use glam::{DVec3, Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use std::{fmt, sync::Arc, time};
use transform_gizmo as gizmo;
use vm::op;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    error::EventLoopError,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub type Instant = quanta::Instant;

#[derive(Debug, Clone)]
pub enum WindowHandle {
    UnInit,
    Init(Arc<Window>),
}

impl WindowHandle {
    fn get_handle(&self) -> &Arc<Window> {
        match self {
            WindowHandle::UnInit => panic!("window was not initialized"),
            WindowHandle::Init(window) => &window,
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

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, ShaderStruct)]
#[repr(C)]
pub struct WorldUniform {
    pub light_pos: Vec3,
    pub pad1: u32,
    pub view_proj: Mat4,
}

impl WorldUniform {
    pub fn new(view_proj: Mat4, light_pos: Vec3) -> Self {
        Self {
            light_pos,
            pad1: 0,
            view_proj,
        }
    }
}

/*
macro_rules! vert {
    ($pos:expr, $norm:expr) => {
        Vertex {
            pos: Vec3::from_array($pos),
            norm: Vec3::from_array($norm),
        }
    };
}

const VERTICES: &[Vertex] = &[
    vert! { [-0.2, -0.2,  0.5], [ 0.0,  0.0,  1.0] }, // 0
    vert! { [ 0.2, -0.2,  0.5], [ 0.0,  0.0,  1.0] }, // 1
    vert! { [ 0.2,  0.2,  0.5], [ 0.0,  0.0,  1.0] }, // 2
    vert! { [-0.2,  0.2,  0.5], [ 0.0,  0.0,  1.0] }, // 3
    vert! { [-0.5, -0.5, -0.5], [ 0.0,  0.0, -1.0] }, // 4
    vert! { [ 0.5, -0.5, -0.5], [ 0.0,  0.0, -1.0] }, // 5
    vert! { [ 0.5,  0.5, -0.5], [ 0.0,  0.0, -1.0] }, // 6
    vert! { [-0.5,  0.5, -0.5], [ 0.0,  0.0, -1.0] }, // 7
    vert! { [-0.5, -0.5, -0.5], [-1.0,  0.0,  0.0] }, // 8
    vert! { [-0.5,  0.5, -0.5], [-1.0,  0.0,  0.0] }, // 9
    vert! { [-0.2,  0.2,  0.5], [-1.0,  0.0,  0.0] }, // 10
    vert! { [-0.2, -0.2,  0.5], [-1.0,  0.0,  0.0] }, // 11
    vert! { [ 0.5, -0.5, -0.5], [ 1.0,  0.0,  0.0] }, // 12
    vert! { [ 0.5,  0.5, -0.5], [ 1.0,  0.0,  0.0] }, // 13
    vert! { [ 0.2,  0.2,  0.5], [ 1.0,  0.0,  0.0] }, // 14
    vert! { [ 0.2, -0.2,  0.5], [ 1.0,  0.0,  0.0] }, // 15
    vert! { [-0.5,  0.5, -0.5], [ 0.0,  1.0,  0.0] }, // 16
    vert! { [ 0.5,  0.5, -0.5], [ 0.0,  1.0,  0.0] }, // 17
    vert! { [ 0.2,  0.2,  0.5], [ 0.0,  1.0,  0.0] }, // 18
    vert! { [-0.2,  0.2,  0.5], [ 0.0,  1.0,  0.0] }, // 19
    vert! { [-0.5, -0.5, -0.5], [ 0.0, -1.0,  0.0] }, // 20
    vert! { [ 0.5, -0.5, -0.5], [ 0.0, -1.0,  0.0] }, // 21
    vert! { [ 0.2, -0.2,  0.5], [ 0.0, -1.0,  0.0] }, // 22
    vert! { [-0.2, -0.2,  0.5], [ 0.0, -1.0,  0.0] }, // 23
];

const INDICES: &[u32] = &[
    // Front face
    2, 1, 0, 0, 3, 2, // Back face
    4, 5, 6, 6, 7, 4, // Left face
    10, 11, 8, 8, 9, 10, // Right face
    12, 15, 14, 14, 13, 12, // Top face
    18, 19, 16, 16, 17, 18, // Bottom face
    20, 23, 22, 22, 21, 20,
];
*/

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
    delta_time: time::Duration,
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
    Iso3D,
}

#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
struct AtlasSettings {
    tol: f64,
    min_depth: u32,
    #[egui_probe(with ui::vec3_probe)]
    dim_min: Vec3,
    #[egui_probe(with ui::vec3_probe)]
    dim_max: Vec3,
    show_tree: bool,
    show_mesh: bool,
    shade_smooth: bool,

    #[egui_probe(with ui::button_probe("rebuild"))]
    rebuild_mesh: bool,
    mesh_gen: MeshGenerator,
    render_config: RenderConfig,
}

impl Default for AtlasSettings {
    fn default() -> Self {
        Self {
            tol: 1e-3,
            min_depth: 4,
            dim_min: [-1.0, -1.0, -1.0].into(),
            dim_max: [1.0, 1.0, 1.0].into(),
            rebuild_mesh: false,
            show_tree: false,
            show_mesh: true,
            mesh_gen: MeshGenerator::Iso3D,
            shade_smooth: false,
            render_config: RenderConfig {
                cull_mode: CullMode::None,
                polygon_mode: PolygonMode::Line,
                fov: 90.0,
                depthbuffer: true,
            },
        }
    }
}

struct AtlasApp {
    renderer: Option<AtlasRenderer>,
    camera: OrbitCamera,
    gizmo: gizmo::Gizmo,

    ui_state: ui::UiState,
    ui_context: gui::UiContext,

    data: WindowData,
    settings: AtlasSettings,

    window: WindowHandle,
}

impl AtlasApp {
    fn new() -> Self {
        log::debug!("init atlas app");

        let settings = AtlasSettings::default();

        let camera = OrbitCamera::look_at(
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::ZERO,
            settings.render_config.fov.to_radians(),
        );

        let data = WindowData {
            mouse_pixel_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            viewport_dragged: false,
            viewport_rect: Rect::ZERO,
            ui_pixel_per_point: 0.0,
            delta_time: time::Duration::ZERO,
            prev_frame_time: Instant::now(),
        };

        let gizmo = gizmo::Gizmo::new(gizmo::GizmoConfig {
            modes: gizmo::GizmoMode::all_translate() | gizmo::GizmoMode::all_scale(),
            ..Default::default()
        });

        Self {
            renderer: None,
            gizmo,
            ui_context: gui::UiContext::default(),
            ui_state: ui::UiState::new(),
            data,
            settings,
            camera,
            window: WindowHandle::UnInit,
        }
    }

    fn frame_update(&mut self) {
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
            self.camera.process_mouse(
                self.data.mouse_delta.x.into(),
                self.data.mouse_delta.y.into(),
            );
        }
    }

    fn on_update(&mut self) {
        let renderer = self.renderer.as_mut().unwrap();
        let prev_viewport_size = renderer.viewport_size;
        let prev_render_config = self.settings.render_config;
        //let mut settings = self.settings;

        renderer
            .egui_state
            .update(&self.window.get_handle(), |ctx| {
                self.data.ui_pixel_per_point = ctx.input(|i| i.pixels_per_point);

                let access = ui::UiAccess {
                    vp_texture: &renderer.framebuffer,
                    camera: &self.camera,
                    gizmo: &mut self.gizmo,
                    window_info: &mut self.data,
                    settings: &mut self.settings,
                };

                self.ui_state.ui(ctx, access);

                renderer.viewport_size = wgpu::Extent3d {
                    width: self.data.viewport_rect.width() as u32,
                    height: self.data.viewport_rect.height() as u32,
                    depth_or_array_layers: 1,
                }
            });

        self.camera.fov_rad = self.settings.render_config.fov.to_radians();
        self.data.mouse_delta = Vec2::ZERO;

        if self.settings.render_config != prev_render_config {
            renderer.rebuild_from_settings(&self.settings);
        } else if prev_viewport_size != renderer.viewport_size {
            renderer.resize_viewport();
        }

        if self.settings.rebuild_mesh {
            self.settings.rebuild_mesh = false;
            renderer.rebuild_mesh(&self.settings);
        }
    }

    fn on_redraw(&mut self, ctrlflow: &ActiveEventLoop) {
        self.on_update();

        let renderer = self.renderer.as_mut().unwrap();

        renderer.update_camera(&self.camera);
        renderer.render_mesh();
        self.window.get_handle().pre_present_notify();

        match renderer.present() {
            Ok(_) => (),

            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                renderer.resize_window(renderer.window_size)
            }
            Err(err @ wgpu::SurfaceError::Timeout) => {
                log::warn!("{err}")
            }
            Err(err) => {
                log::error!("{err}");
                ctrlflow.exit()
            }
        }
    }

    fn on_scroll(&mut self, delta: &MouseScrollDelta) {
        self.camera.process_scroll(&delta);
    }

    fn on_window_event(&mut self, event: &WindowEvent) -> bool {
        use WindowEvent as WE;

        self.renderer.as_mut().unwrap().input(&event);

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

                self.data.mouse_pixel_pos = pos;

                if cursor_wrapped {
                    if self.data.viewport_dragged {
                        self.window.set_mouse_pos(pos);
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
                self.camera.process_scroll(&delta);
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
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("creating window...");
        let window = event_loop
            .create_window(winit::window::Window::default_attributes().with_title("Atlas"))
            .unwrap();

        self.window = window.into();
        let gpu_ctx = gpu::WgpuContext::new(self.window.get_handle().clone());
        self.renderer = AtlasRenderer::new(gpu_ctx, &self.settings).into();
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::MouseWheel { delta } => {
                self.on_scroll(&delta);
            }
            _ => (),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.frame_update();

        if self.window.id() == window_id && !self.on_window_event(&event) {
            use WindowEvent as WE;
            match event {
                WE::RedrawRequested => {
                    self.on_redraw(&event_loop);
                }
                WE::Resized(physical_size) => {
                    self.renderer.as_mut().unwrap().resize_window(physical_size);
                }
                WE::CloseRequested => event_loop.exit(),
                _ => (),
            }
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        match cause {
            winit::event::StartCause::Init => return,
            _ => self.window.request_redraw(),
        }
    }

    //fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    //    self.window.request_redraw();
    //}
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
    Line,
}

impl From<PolygonMode> for wgpu::PolygonMode {
    fn from(value: PolygonMode) -> Self {
        match value {
            PolygonMode::Fill => wgpu::PolygonMode::Fill,
            PolygonMode::Line => wgpu::PolygonMode::Line,
        }
    }
}

//     pub fn drag_angle(&mut self, radians: &mut f32) -> Response {
//         let mut degrees = radians.to_degrees();
//         let mut response = self.add(DragValue::new(&mut degrees).speed(1.0).suffix("°"));

//         // only touch `*radians` if we actually changed the degree value
//         if degrees != radians.to_degrees() {
//             *radians = degrees.to_radians();
//             response.changed = true;
//         }

//         response
//     }

struct AtlasRenderer {
    //render_pipeline: wgpu::RenderPipeline,
    //vertex_buffer: wgpu::Buffer,
    //index_buffer: wgpu::Buffer,
    //n_indices: usize,
    framebuffer_msaa: gpu::Texture,
    framebuffer: gpu::Texture,
    depthbuffer: gpu::Texture,
    use_depthbuffer: bool,

    mesh_pipeline: wgpu::RenderPipeline,
    mesh_verts: wgpu::Buffer,
    mesh_indxs: wgpu::Buffer,
    n_indices: usize,

    // gui_pipeline: wgpu::RenderPipeline,
    world_buffer: wgpu::Buffer,
    world_bind_group: wgpu::BindGroup,
    world_bind_group_layout: wgpu::BindGroupLayout,

    // gui_bind_group: wgpu::BindGroup,
    // gui_bind_group_layout: wgpu::BindGroupLayout,
    // ui_rectangle: wgpu::Buffer,

    // viewport_sc: SceneGraph,
    egui_state: egui_state::EguiState,

    viewport_size: wgpu::Extent3d,

    //camera_buffer: wgpu::Buffer,
    //camera_bind_group: wgpu::BindGroup,
    //camera_bind_group_layout: wgpu::BindGroupLayout,
    window_size: PhysicalSize<u32>,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    active_encoder: wgpu::CommandEncoder,
    // drop last
    window: Arc<Window>,
}

//fn quad(p1: Vec3, p2: Vec3, p3: Vec3, p4: Vec3) -> [Vertex; 6] {
//    let a = p1 - p2;
//    let b = p2 - p3;
//
//    let norm = (a.cross(b)).normalize();
//
//    [
//        Vertex { pos: p1, norm, depth: 0 },
//        Vertex { pos: p2, norm, depth: 0 },
//        Vertex { pos: p3, norm, depth: 0 },
//        Vertex { pos: p1, norm, depth: 0 },
//        Vertex { pos: p3, norm, depth: 0 },
//        Vertex { pos: p4, norm, depth: 0 },
//    ]
//}
//
//fn triangle(p1: Vec3, p2: Vec3, p3: Vec3) -> [Vertex; 3] {
//    let a = p1 - p2;
//    let b = p2 - p3;
//
//    let norm = (a.cross(b)).normalize();
//
//    [
//        Vertex { pos: p1, norm },
//        Vertex { pos: p2, norm },
//        Vertex { pos: p3, norm },
//    ]
//}

fn iso_triangle3(p1: Vec3, p2: Vec3, p3: Vec3) -> [Vertex; 3] {
    [
        Vertex::new(p1, Vec4::splat(1.0)),
        Vertex::new(p2, Vec4::splat(1.0)),
        Vertex::new(p3, Vec4::splat(1.0)),
    ]
}

fn octant_as_mesh_2(oct: u64, min: Vec3, max: Vec3) -> Vec<Vertex> {
    let mut vertices = vec![];

    // let vts = iso::v3::corner_position(corner, min.as_dvec3(), max.as_dvec3());
    let mut vts = [Vec3::ZERO; 8];

    let cors = iso::v3::corner_locations(oct);
    for i in 0..8 {
        vts[i] = iso::v3::corner_position(cors[i], min.as_dvec3(), max.as_dvec3()).as_vec3();
    }

    let dl = vts[0];
    let dr = vts[1];
    let dfl = vts[2];
    let dfr = vts[3];
    let upl = vts[4];
    let upr = vts[5];
    let upfl = vts[6];
    let upfr = vts[7];

    // bottom
    vertices.extend(iso_triangle3(dl, dr, dfl));
    vertices.extend(iso_triangle3(dr, dfr, dfl));
    // front
    vertices.extend(iso_triangle3(dl, upl, dr));
    vertices.extend(iso_triangle3(dr, upl, upr));
    // left
    vertices.extend(iso_triangle3(dl, upfl, upl));
    vertices.extend(iso_triangle3(dl, dfl, upfl));
    // right
    vertices.extend(iso_triangle3(dr, upr, upfr));
    vertices.extend(iso_triangle3(dr, upfr, dfr));
    // back
    vertices.extend(iso_triangle3(dfl, dfr, upfl));
    vertices.extend(iso_triangle3(dfr, upfr, upfl));
    // top
    vertices.extend(iso_triangle3(upl, upfr, upr));
    vertices.extend(iso_triangle3(upl, upfl, upfr));

    vertices
}

fn octant_as_mesh(vts: &[Vec3]) -> Vec<Vertex> {
    let mut vertices = vec![];

    let dl = vts[0];
    let dr = vts[1];
    let dfl = vts[2];
    let dfr = vts[3];
    let upl = vts[4];
    let upr = vts[5];
    let upfl = vts[6];
    let upfr = vts[7];

    // bottom
    vertices.extend(iso_triangle3(dl, dr, dfl));
    vertices.extend(iso_triangle3(dr, dfr, dfl));
    // front
    vertices.extend(iso_triangle3(dl, upl, dr));
    vertices.extend(iso_triangle3(dr, upl, upr));
    // left
    vertices.extend(iso_triangle3(dl, upfl, upl));
    vertices.extend(iso_triangle3(dl, dfl, upfl));
    // right
    vertices.extend(iso_triangle3(dr, upr, upfr));
    vertices.extend(iso_triangle3(dr, upfr, dfr));
    // back
    vertices.extend(iso_triangle3(dfl, dfr, upfl));
    vertices.extend(iso_triangle3(dfr, upfr, upfl));
    // top
    vertices.extend(iso_triangle3(upl, upfr, upr));
    vertices.extend(iso_triangle3(upl, upfl, upfr));

    vertices
}

fn build_unit_square() -> Vec<Vertex> {
    let tr = Vec3::new(1.0, 1.0, 0.0);
    let tl = Vec3::new(0.0, 1.0, 0.0);
    let bl = Vec3::new(0.0, 0.0, 0.0);
    let br = Vec3::new(1.0, 0.0, 0.0);

    let col = Vec3::Z.extend(1.0);

    vec![
        Vertex::new(bl, col),
        Vertex::new(br, col),
        Vertex::new(tl, col),
        Vertex::new(br, col),
        Vertex::new(tr, col),
        Vertex::new(tl, col),
    ]
}

/*
fn build_iso2(settings: &AtlasSettings) -> Vec<Vertex> {
    let min: Vec3 = settings.mesh_min.into();
    let max: Vec3 = settings.mesh_max.into();

    let tree = iso2::build(min.xy(), max.xy(), settings.min_depth, settings.max_cells);

    let mut vertices = vec![];
    if settings.show_tree {
        for cell in tree.cells {
            let verts = cell.verts;
            let p0 = Vec2::from(verts[0]).extend(0.0);
            let p1 = Vec2::from(verts[1]).extend(0.0);
            let p2 = Vec2::from(verts[2]).extend(0.0);
            let p3 = Vec2::from(verts[3]).extend(0.0);
            let norm = Vec3::ZERO;

            vertices.extend([
                Vertex { pos: p0, norm },
                Vertex { pos: p1, norm },
                Vertex { pos: p3, norm },
                Vertex { pos: p3, norm },
                Vertex { pos: p2, norm },
                Vertex { pos: p0, norm },
            ]);
        }
    }
    vertices
}
*/
fn fmt_num(n: impl Into<u64>) -> String {
    let n = n.into();
    let s = n.to_string();
    let mut res = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            res.push('_');
        }
        res.push(ch);
    }
    res.chars().rev().collect()
}

fn build_mesh(settings: &AtlasSettings) -> Vec<Vertex> {
    let start = time::Instant::now();

    // let mut mesh = match settings.mesh_gen {
    //     MeshGenerator::Iso2D => build_mesh_3d(settings),
    //     MeshGenerator::Iso3D => build_mesh_v3(settings),
    // };

    let mut mesh = build_mesh_3d(settings);

    let size = (settings.dim_max - settings.dim_min).extend(1.0);
    let center = ((settings.dim_max + settings.dim_min) / 2.0).extend(0.0);
    // TODO: keep ratio while normalizing
    for v in &mut mesh {
        v.pos -= center;
        v.pos /= size;
    }

    log::info!(
        "extracted isosurface in: {} s / {} ms",
        (time::Instant::now() - start).as_secs_f64(),
        (time::Instant::now() - start).as_secs_f64() * 1000.0,
    );

    println!("#of vertices: {}", fmt_num(mesh.len() as u64));

    mesh
}

// fn build_mesh_2d(settings: &AtlasSettings) -> Vec<Vertex> {
//     // let f = |n: Vec2| -> f32 {
//     //     let (x, y) = (n.x, n.y);
//     //     1.0 / 3f32.powf(x).sin() + y.sin() - y
//     // };

//     // x * cos(x*y) + y - 4 = 0
//     let f1 = [
//         op::MUL_LHS_RHS(1, 2, 3),
//         op::COS(3, 3),
//         op::MUL_LHS_RHS(1, 3, 1),
//         op::ADD_LHS_RHS(1, 2, 1),
//         op::SUB_LHS_IMM(1, 4.0, 1),
//         op::EXT(0),
//     ];

//     // let f2 = atl_macro::implicit_fn!(sin(1/x) - y);
//     let f2 = atl_macro::implicit_fn!(sin(1 / x) - y);

//     vm::dbg_bytecode(&f2);

//     // let program = [
//     //     op::ADD_LHS_RHS(1, 2, 3),
//     //     op::POW_IMM_RHS(3.0, 3, 3),
//     //     op::SIN(3, 3),
//     //     op::SIN(1, 1),
//     //     op::SIN(2, 2),
//     //     op::ADD_LHS_RHS(1, 2, 1),
//     //     op::POW_IMM_RHS(3.0, 1, 1),
//     //     op::SUB_LHS_RHS(1, 3, 1),

//     //     // op::ADD_LHS_RHS(1, 2, 3),
//     //     // op::SIN(3, 3),
//     //     // op::SIN(1, 1),
//     //     // op::COS(2, 2),
//     //     // op::ADD_LHS_RHS(1, 2, 1),
//     //     // op::SUB_LHS_RHS(1, 3, 1),
//     //     op::EXT(0),
//     // ];

//     let min: Vec3 = settings.dim_min.into();
//     let max: Vec3 = settings.dim_max.into();

//     let (lines, tree) =
//         iso::line::build(min.xy(), max.xy(), settings.min_depth, 1, &f2, settings.tol);

//     let mut vertices = vec![];
//     for line in lines {
//         for pts in line.as_slice().windows(3) {
//             let p0 = pts[0].extend(0.0);
//             let p1 = pts[1].extend(0.0);
//             let p2 = pts[2].extend(0.0);

//             let norm = (0.0, 0.0, 0.0, 0.0).into();

//             vertices.extend([
//                 Vertex::new(p0, norm),
//                 Vertex::new(p1, norm),
//                 Vertex::new(p2, norm),
//             ]);
//         }
//     }

//     if settings.show_tree {
//         for cell in tree.cells {
//             let verts = cell.verts.as_ref();

//             let p0 = Vec2::from(verts[0].pos).extend(0.0);
//             let p1 = Vec2::from(verts[1].pos).extend(0.0);
//             let p2 = Vec2::from(verts[2].pos).extend(0.0);
//             let p3 = Vec2::from(verts[3].pos).extend(0.0);
//             let norm = Vec4::ZERO;

//             vertices.extend([
//                 Vertex::new(p0, norm),
//                 Vertex::new(p1, norm),
//                 Vertex::new(p3, norm),
//                 Vertex::new(p0, norm),
//                 Vertex::new(p3, norm),
//                 Vertex::new(p2, norm),
//             ]);
//         }
//     }

//     vertices
// }

fn build_mesh_3d(settings: &AtlasSettings) -> Vec<Vertex> {
    let program = [
        op::SIN(1, 4),            // sin(x)
        op::SIN(2, 5),            // sin(y)
        op::SIN(3, 6),            // sin(z)
        op::COS(1, 1),            // cos(x)
        op::COS(2, 2),            // cos(y)
        op::COS(3, 3),            // cos(z)
        op::MUL_LHS_RHS(6, 1, 1), // sin(z)*cos(x)
        op::MUL_LHS_RHS(5, 3, 3), // sin(y)*cos(z)
        op::MUL_LHS_RHS(4, 2, 2), // sin(x)*cos(y)
        op::ADD_LHS_RHS(2, 1, 1),
        op::ADD_LHS_RHS(3, 1, 1),
        op::EXT(0),
    ];

//     let program = [
//         op::POW_LHS_IMM(1, 2.0, 1),
//         op::POW_LHS_IMM(2, 2.0, 2),
//         op::POW_LHS_IMM(3, 2.0, 3),
//         op::ADD_LHS_RHS(1, 2, 1),
//         op::ADD_LHS_RHS(1, 3, 1),
//         op::SUB_LHS_IMM(1, 0.5, 1),
//         op::EXT(0),
//     ];

    // let program = [
    //     op::SIN(1, 1),
    //     op::SIN(2, 2),
    //     op::SUB_LHS_RHS(1, 2, 1),
    //     op::SUB_LHS_RHS(1, 3, 1),
    //     op::EXT(0),
    // ];

    let min = settings.dim_min;
    let max = settings.dim_max;

    let (tris, tree) = iso::v3::build(min, max, settings.min_depth, &program, settings.tol);

    let mut max_depth = 0;

    for oct in &tree.cells {
        max_depth = iso::v3::octant_depth(*oct).max(max_depth);
    }

    let mut vertices = vec![];

    if settings.show_tree {
        for oct in &tree.cells {
            let cell_bounds = iso::v3::octant_corners(min, max, *oct);
            let mut verts = octant_as_mesh(&cell_bounds);
            // let mut verts = octant_as_mesh_2(*oct, min, max);
            for v in &mut verts {
                v.col.w = iso::v3::octant_depth(*oct) as f32 / (max_depth + 1) as f32;
            }
            vertices.extend(verts);

        }
    }

    // let mut deriv_vm = vm::VM::with_instr_table(vm::FDerivInstrTable);

    // let mut df = |p: Vec3| {
    //     let p = p.as_dvec3();
    //     let vx = vm::FDeriv::var(p.x);
    //     let vy = vm::FDeriv::var(p.y);
    //     let vz = vm::FDeriv::var(p.z);
    //     let cx = vm::FDeriv::cnst(p.x);
    //     let cy = vm::FDeriv::cnst(p.y);
    //     let cz = vm::FDeriv::cnst(p.z);

    //     deriv_vm.reg[1] = vx;
    //     deriv_vm.reg[2] = cy;
    //     deriv_vm.reg[3] = cz;
    //     deriv_vm.eval(&program);
    //     let dx = deriv_vm.reg[1].grad;

    //     deriv_vm.reg[1] = cx;
    //     deriv_vm.reg[2] = vy;
    //     deriv_vm.reg[3] = cz;
    //     deriv_vm.eval(&program);
    //     let dy = deriv_vm.reg[1].grad;

    //     deriv_vm.reg[1] = cx;
    //     deriv_vm.reg[2] = cy;
    //     deriv_vm.reg[3] = vz;
    //     deriv_vm.eval(&program);
    //     let dz = deriv_vm.reg[1].grad;

    //     DVec3::new(dx, dy, dz).as_vec3()
    // };

    let df = |n: Vec3| -> Vec3 {
        (
            n.x.cos() * n.y.cos() - n.z.sin() * n.x.sin(),
            -n.x.sin() * n.y.sin() + n.y.cos() * n.z.cos(),
            -n.y.sin() * n.z.sin() + n.z.cos() * n.x.cos(),
        )
            .into()
    };
    if settings.show_mesh {
        for t in tris {
            let p1 = t[0];
            let p2 = t[1];
            let p3 = t[2];

            let (n1, n2, n3) = if settings.shade_smooth {
                (df(p1).normalize(), df(p2).normalize(), df(p3).normalize())
            } else {
                let n = df((p1 + p2 + p3) / 3.0).normalize();
                (n, n, n)
            };

            vertices.extend([
                Vertex::new(p1, n1.extend(1.0)),
                Vertex::new(p2, n2.extend(1.0)),
                Vertex::new(p3, n3.extend(1.0)),
            ]);

            //let v1 = a - b;
            //let v2 = c - a;
            //let norm = v1.cross(v2).normalize();
            //vertices.extend_from_slice(&[
            //    Vertex { pos: a, norm },
            //    Vertex { pos: b, norm },
            //    Vertex { pos: c, norm },
            //]);
        }
    }

    vertices
}

/*
fn build_mesh_v2(settings: &AtlasSettings) -> Vec<Vertex> {
    let program = [
        op::SIN(1, 4),            // sin(x)
        op::SIN(2, 5),            // sin(y)
        op::SIN(3, 6),            // sin(z)
        op::COS(1, 1),            // cos(x)
        op::COS(2, 2),            // cos(y)
        op::COS(3, 3),            // cos(z)
        op::MUL_LHS_RHS(6, 1, 1), // sin(z)*cos(x)
        op::MUL_LHS_RHS(5, 3, 3), // sin(y)*cos(z)
        op::MUL_LHS_RHS(4, 2, 2), // sin(x)*cos(y)
        op::ADD_LHS_RHS(2, 1, 1),
        op::ADD_LHS_RHS(3, 1, 1),
        op::EXT(0),
    ];

    // let program = [
    //     op::POW_LHS_IMM(1, 2.0, 1),
    //     op::POW_LHS_IMM(2, 2.0, 2),
    //     op::POW_LHS_IMM(3, 2.0, 3),
    //     op::ADD_LHS_RHS(1, 2, 1),
    //     op::ADD_LHS_RHS(1, 3, 1),
    //     op::SUB_LHS_IMM(1, 1.0, 1),
    //     op::EXT(0),
    // ];

    let min = settings.dim_min.into();
    let max = settings.dim_max.into();

    let tree = iso::v2::build(min, max, settings.min_depth, 1, &program, settings.tol);

    let mut max_depth = 0;

    for cell in &tree.cells {
        max_depth = max_depth.max(cell.depth);
    }

    let mut vertices = vec![];
    for cell in tree.cells {
        let cell_bounds = cell.get_corners();
        let mut verts = cell_as_mesh(&cell_bounds);
        for v in &mut verts {
            v.col.w = cell.depth as f32 / (max_depth + 1) as f32;
        }
        vertices.extend(verts);
    }

    vertices
}
*/

/*
fn build_mesh_3d(settings: &AtlasSettings) -> Vec<Vertex> {
    let f = |n: Vec3| -> f32 {
        if n.x < 0.0 {
            1.0
        } else {
            // n.x*n.x + n.y*n.y + n.z*n.z - 1.0
            n.x.sin() * n.y.cos() + n.y.sin() * n.z.cos() + n.z.sin() * n.x.cos()
        }
        //(n.x.sin() + n.y.sin() - n.z.sin()) * n.x.sin()*n.y.sin()*n.z.sin() - 1.0
        //0.5 * (n.x.powi(4) + n.y.powi(4) + n.z.powi(4))
        //    - 8.0 * (n.x.powi(2) + n.y.powi(2) + n.z.powi(2))
        //    + 60.0
    };

    let df = |n: Vec3| -> Vec3 {
        (
            n.x.cos() * n.y.cos() - n.z.sin() * n.x.sin(),
            -n.x.sin() * n.y.sin() + n.y.cos() * n.z.cos(),
            -n.y.sin() * n.z.sin() + n.z.cos() * n.x.cos(),
        )
            .into()
        //(
        //    2.0 * n.x.powi(3) - 16.0 * n.x,
        //    2.0 * n.y.powi(3) - 16.0 * n.y,
        //    2.0 * n.z.powi(3) - 16.0 * n.z,
        //)
        //    .into()
    };

    let program = [
        op::SIN(1, 4),            // sin(x)
        op::SIN(2, 5),            // sin(y)
        op::SIN(3, 6),            // sin(z)
        op::COS(1, 1),            // cos(x)
        op::COS(2, 2),            // cos(y)
        op::COS(3, 3),            // cos(z)
        op::MUL_LHS_RHS(6, 1, 1), // sin(z)*cos(x)
        op::MUL_LHS_RHS(5, 3, 3), // sin(y)*cos(z)
        op::MUL_LHS_RHS(4, 2, 2), // sin(x)*cos(y)
        op::ADD_LHS_RHS(2, 1, 1),
        op::ADD_LHS_RHS(3, 1, 1),
        op::EXT(0),
    ];


    //let finite_diff = |p: Vec3| -> Vec3 {
    //    let h = 0.5;
    //    let (x, y, z) = (p.x, p.y, p.z);
    //    (
    //        f((x + h, y, z).into()) - f(Vec3::new(x - h, y, z) / (2.0 * h)),
    //        f((x, y + h, z).into()) - f(Vec3::new(x, y - h, z) / (2.0 * h)),
    //        f((x, y, z + h).into()) - f(Vec3::new(x, y, z - h) / (2.0 * h)),
    //    ).into()
    //};

    //let min = Vec3::splat(-40.0);
    //let max = Vec3::splat(40.0);
    let min = settings.dim_min.into();
    let max = settings.dim_max.into();

    let (tris, tree) = iso::surface::build(min, max, settings.min_depth, 1, &program, settings.tol);

    let mut vertices = vec![];

    if settings.show_mesh {
        for t in tris {
            let p1 = t[0];
            let p2 = t[1];
            let p3 = t[2];

            let (n1, n2, n3) = if settings.shade_smooth {
                (df(p1).normalize(), df(p2).normalize(), df(p3).normalize())
            } else {
                let n = df((p1 + p2 + p3) / 3.0).normalize();
                (n, n, n)
            };

            vertices.extend([
                Vertex::new(p1, n1.extend(1.0)),
                Vertex::new(p2, n2.extend(1.0)),
                Vertex::new(p3, n3.extend(1.0)),
            ]);

            //let v1 = a - b;
            //let v2 = c - a;
            //let norm = v1.cross(v2).normalize();
            //vertices.extend_from_slice(&[
            //    Vertex { pos: a, norm },
            //    Vertex { pos: b, norm },
            //    Vertex { pos: c, norm },
            //]);
        }
    }

    if settings.show_tree {
        let mut max_depth = 0;

        for cell in &tree.cells {
            max_depth = max_depth.max(cell.depth);
        }

        for cell in tree.cells {
            let mut verts = cell_verts_to_vertex(cell.verts.as_ref());
            for v in &mut verts {
                v.col = Vec4::splat(cell.depth as f32 / (max_depth + 1) as f32);
            }
            vertices.extend(verts);
        }
    }

    //if show_tree {
    //    vertices.extend(
    //        quad((-10.0, -10.0, 0.0).into(), (-10.0, 10.0, 0.0).into(), (10.0, 10.0, 0.0).into(), (10.0, -10.0, 0.0).into()),
    //    );
    //    vertices.extend(
    //        quad((-10.0, 0.0, -10.0).into(), (-10.0, 0.0, 10.0).into(), (10.0, 0.0, 10.0).into(), (10.0, 0.0, -10.0).into()),
    //    );
    //}

    vertices
}
*/

impl AtlasRenderer {
    fn new(ctx: gpu::WgpuContext, settings: &AtlasSettings) -> Self {
        let device = ctx.device;
        let surface = ctx.surface;
        let config = ctx.config;
        let window = ctx.window;
        let queue = ctx.queue;

        let window_size = window.inner_size();

        let viewport_size = wgpu::Extent3d {
            width: window_size.width,
            height: window_size.height,
            depth_or_array_layers: 1,
        };

        let world_uniform = WorldUniform::new(Mat4::IDENTITY, Vec3::ZERO);

        let mut egui_state = egui_state::EguiState::new(&device, config.format, None, 1, &window);

        let framebuffer_msaa = gpu::TextureConfig::d2(viewport_size, config.format)
            .msaa_samples(4)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        let framebuffer = gpu::TextureConfig::d2(viewport_size, config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut egui_state)
            .build(&device);

        let depthbuffer = gpu::TextureConfig::depthf32(viewport_size)
            .msaa_samples(4)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        let use_depthbuffer = settings.render_config.depthbuffer;

        let world_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "world_buffer".into(),
            contents: bytemuck::cast_slice(&[world_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let world_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let world_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: "world_bind_group".into(),
            layout: &world_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: world_buffer.as_entire_binding(),
            }],
        });

        // let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: "globals_buffer".into(),
        //     contents: bytemuck::cast_slice(&[ui_globals]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });

        // let gui_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         label: "gui_bind_group_layout".into(),
        //         entries: &[wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::VERTEX,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Uniform,
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         },
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 1,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Texture {
        //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                 view_dimension: wgpu::TextureViewDimension::D2,
        //                 multisampled: false,
        //             },
        //             count: None,
        //         },
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 2,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //             count: None,
        //         },
        //         ],
        //     });

        // let gui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: "gui_bind_group".into(),
        //     layout: &gui_bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: globals_buffer.as_entire_binding(),
        //     },
        //     wgpu::BindGroupEntry {
        //         binding: 1,
        //         resource: wgpu::BindingResource::TextureView(&framebuffer),
        //     },
        //     wgpu::BindGroupEntry {
        //         binding: 2,
        //         resource: wgpu::BindingResource::Sampler(framebuffer.sampler()),
        //     },
        //     ],
        // });

        log::debug!("setup framebuffers");

        //let module = gpu::ShaderConfig::from_wgsl(include_str!("shader.wgsl"))
        let mesh_shader = gpu::ShaderConfig::from_wgsl(include_str!("shader.wgsl"))
            .with_struct::<Vertex>("VertexInput")
            .with_struct::<WorldUniform>("WorldUniform")
            .build(&device);

        let mesh_pipeline = gpu::PipelineConfig::new(&mesh_shader)
            .color::<Vertex>(framebuffer_msaa.format())
            .set_if(use_depthbuffer, |p| p.depth_format(depthbuffer.format()))
            // .blend(wgpu::BlendState {
            //     color: wgpu::BlendComponent {
            //         src_factor: wgpu::BlendFactor::One,
            //         dst_factor: wgpu::BlendFactor::One,
            //         operation: wgpu::BlendOperation::Add,
            //     },
            //     alpha: wgpu::BlendComponent {
            //         src_factor: wgpu::BlendFactor::One,
            //         dst_factor: wgpu::BlendFactor::One,
            //         operation: wgpu::BlendOperation::Add,
            //     },
            // })
            .msaa_samples(framebuffer_msaa.msaa_samples())
            .polygon_mode(settings.render_config.polygon_mode.into())
            .set_cull_mode(settings.render_config.cull_mode.into())
            .bind_group_layouts(&[&world_bind_group_layout])
            .label("mesh pipeline")
            .build(&device);

        // let gui_shader = gpu::ShaderConfig::from_wgsl(include_str!("gui.wgsl")).build(&device);

        // let gui_pipeline = gpu::PipelineConfig::new(&gui_shader)
        //     .color::<gui::Vertex>(config.format)
        //     .with_instances::<gui::Instance>()
        //     .cull_mode(wgpu::Face::Back)
        //     .primitive_topology(wgpu::PrimitiveTopology::TriangleStrip)
        //     .bind_group_layouts(&[&gui_bind_group_layout])
        //     .label("gui pipeline")
        //     .build(&device);

        let vertices = build_mesh(settings);

        let indices: Vec<_> = (0..vertices.len() as u32).collect();

        let mesh_verts = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let n_indices = indices.len();
        let mesh_indxs = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        log::debug!("finish initializing wgpu context");

        let active_encoder = Self::new_encoder(&device);

        Self {
            framebuffer_msaa,
            framebuffer,
            depthbuffer,
            use_depthbuffer,
            mesh_pipeline,
            mesh_verts,
            mesh_indxs,
            n_indices,
            world_buffer,
            world_bind_group,
            world_bind_group_layout,
            surface,
            device,
            queue,
            config,
            egui_state,
            active_encoder,
            viewport_size,
            window_size,
            window,
        }
    }

    fn rebuild_mesh(&mut self, settings: &AtlasSettings) {
        let vertices = build_mesh(settings);

        let indices: Vec<_> = (0..vertices.len() as u32).collect();

        self.mesh_verts = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.n_indices = indices.len();
        self.mesh_indxs = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
    }

    fn rebuild_from_settings(&mut self, settings: &AtlasSettings) {
        let msaa_samples = 4;

        self.framebuffer_msaa = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .msaa_samples(msaa_samples)
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.framebuffer = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut self.egui_state)
            .build(&self.device);

        // self.gui_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: "gui_bind_group".into(),
        //     layout: &self.gui_bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&self.framebuffer),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(self.framebuffer.sampler()),
        //         },
        //     ],
        // });

        self.depthbuffer = gpu::TextureConfig::depthf32(self.viewport_size)
            .msaa_samples(msaa_samples)
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.use_depthbuffer = settings.render_config.depthbuffer;

        let module = gpu::ShaderConfig::from_wgsl(include_str!("shader.wgsl"))
            .with_struct::<Vertex>("VertexInput")
            .with_struct::<WorldUniform>("WorldUniform")
            .build(&self.device);

        self.mesh_pipeline = gpu::PipelineConfig::new(&module)
            //.color_depth(framebuffer.format(), depthbuffer.format())
            .color::<Vertex>(self.framebuffer_msaa.format())
            .set_if(self.use_depthbuffer, |p| {
                p.depth_format(self.depthbuffer.format())
            })
            .msaa_samples(self.framebuffer_msaa.msaa_samples())
            .polygon_mode(settings.render_config.polygon_mode.into())
            .set_cull_mode(settings.render_config.cull_mode.into())
            .bind_group_layouts(&[&self.world_bind_group_layout])
            .label("mesh pipeline")
            .build(&self.device);
    }

    fn rebuild_framebuffer(&mut self) {
        self.framebuffer_msaa = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .msaa_samples(self.framebuffer_msaa.msaa_samples())
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.framebuffer = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut self.egui_state)
            .build(&self.device);

        self.depthbuffer = gpu::TextureConfig::depthf32(self.viewport_size)
            .msaa_samples(self.depthbuffer.msaa_samples())
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        // self.gui_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: "gui_bind_group".into(),
        //     layout: &self.gui_bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&self.framebuffer),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(self.framebuffer.sampler()),
        //         },
        //     ],
        // });
    }

    fn resize_viewport(&mut self) {
        if self.viewport_size.width == 0 || self.viewport_size.height == 0 {
            return;
        }
        self.rebuild_framebuffer();
        //self.camera
        //    .set_aspect(self.viewport_size.width, self.viewport_size.height);
    }

    fn resize_window(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.window_size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn input(&mut self, event: &WindowEvent) {
        self.egui_state.handle_input(&self.window, event);
    }

    // fn render(&mut self, camera: &OribtCamera) -> Result<(), wgpu::SurfaceError> {
    //     self.render_scene(camera);
    //     self.render_ui()
    // }

    fn update_camera(&mut self, camera: &OrbitCamera) {
        let world_uniform = WorldUniform::new(camera.view_proj_mat(), camera.eye());
        self.queue.write_buffer(
            &self.world_buffer,
            0,
            bytemuck::cast_slice(&[world_uniform]),
        );
    }

    fn render_mesh(&mut self) {
        //self.viewport_sc.render(&mut self.active_encoder);

        if self.n_indices == 0 {
            return;
        }

        gpu::RenderPass::target_color(&self.framebuffer_msaa)
            .set_if(self.use_depthbuffer, |rp| {
                rp.depth_target(&self.depthbuffer)
            })
            .resolve_target(&self.framebuffer)
            .clear_hex("#24273a")
            .draw(&mut self.active_encoder, |mut rpass| {
                rpass.set_bind_group(0, &self.world_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.mesh_verts.slice(..));
                rpass.set_pipeline(&self.mesh_pipeline);
                rpass.draw(0..self.n_indices as u32, 0..1);
            });
    }

    fn new_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: "main encoder".into(),
        })
    }

    /*
    fn present2(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let verts: Vec<gui::Instance> = vec![gui::Instance {
            max: (0.0, 0.0).into(),
            min: (0.5, 0.5).into(),
            uv_min: (0.0, 0.0).into(),
            uv_max: (1.0, 1.0).into(),
            corner_radius: 0.0,
            edge_softness: 0.0,
            col: (1.0, 0.0, 0.0, 1.0).into(),
        }];

        let instance_buff = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let mut prev_encoder = Self::new_encoder(&self.device);
        std::mem::swap(&mut prev_encoder, &mut self.active_encoder);

        gpu::RenderPass::target_color(&output_view)
            .clear_rgb(0.0, 0.0, 0.0)
            .draw(&mut prev_encoder, |mut rpass| {
                rpass.set_bind_group(0, &self.gui_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.ui_rectangle.slice(..));
                rpass.set_vertex_buffer(1, instance_buff.slice(..));
                rpass.set_pipeline(&self.gui_pipeline);
                rpass.draw(0..4, 0..1);
            });

        self.queue.submit([prev_encoder.finish()]);
        output.present();

        Ok(())
    }
    */

    fn present(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut prev_encoder = Self::new_encoder(&self.device);
        std::mem::swap(&mut prev_encoder, &mut self.active_encoder);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window().scale_factor() as f32,
        };

        self.egui_state.render(
            &self.device,
            &self.queue,
            &mut prev_encoder,
            &self.window,
            &output_view,
            screen_descriptor,
        );

        self.queue.submit([prev_encoder.finish()]);
        output.present();

        Ok(())
    }

    fn window(&self) -> &winit::window::Window {
        &self.window
    }
}
