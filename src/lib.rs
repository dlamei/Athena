mod camera;
mod egui_state;
mod gpu;

mod iso;
mod iso2;
mod ui;

pub mod vm;

pub extern crate self as atlas;

use atl_macro::ShaderStruct;
use camera::{Camera, OribtCamera};

use egui::Rect;

use egui_probe::EguiProbe;
use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};
use std::{fmt, sync::Arc, time};
use transform_gizmo as gizmo;
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
    pub pos: Vec3,
    #[wgsl(@location(1))]
    pub norm: Vec3,
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

impl gpu::VertexFormat for Vec3 {
    const VERT_FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Float32x3;
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
    //gpu::new_vert_attrib_array([Vec3::VERT_FORMAT, Vec3::VERT_FORMAT, u32::VERT_FORMAT]);

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
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
        event_loop.set_control_flow(ControlFlow::Wait);

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
        mint::Vector2::from(self.viewport_rect.min.to_vec2()).into()
    }
    fn vp_rect_max(&self) -> Vec2 {
        mint::Vector2::from(self.viewport_rect.max.to_vec2()).into()
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
    depth_buffer: bool,
}


#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
pub enum MeshGenerator {
    Iso2DNew,
    Iso2D,
    Iso3D,
}

#[derive(Debug, Copy, Clone, PartialEq, EguiProbe)]
struct AtlasSettings {
    max_cells: u32,
    min_depth: u32,
    #[egui_probe(with ui::vec3_probe)]
    mesh_min: Vec3,
    #[egui_probe(with ui::vec3_probe)]
    mesh_max: Vec3,
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
            max_cells: 10000,
            min_depth: 4,
            mesh_min: [-10.0, -10.0, -10.0].into(),
            mesh_max: [10.0, 10.0, 10.0].into(),
            rebuild_mesh: false,
            show_tree: true,
            show_mesh: true,
            mesh_gen: MeshGenerator::Iso2DNew,
            shade_smooth: false,
            render_config: RenderConfig {
                cull_mode: CullMode::None,
                polygon_mode: PolygonMode::Line,
                fov: 90.0,
                depth_buffer: true,
            },
        }
    }
}

struct AtlasApp {
    renderer: Option<AtlasRenderer>,
    camera: OribtCamera,
    gizmo: gizmo::Gizmo,

    ui_state: ui::UiState,

    data: WindowData,
    settings: AtlasSettings,

    window: WindowHandle,
}

impl AtlasApp {
    fn new() -> Self {
        log::debug!("init atlas app");

        let settings = AtlasSettings::default();

        let camera = OribtCamera::look_at(
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

    fn on_redraw(&mut self, ctrlflow: &ActiveEventLoop) {
        let renderer = self.renderer.as_mut().unwrap();

        let prev_viewport_size = renderer.viewport_size;
        let prev_render_config = self.settings.render_config;
        //let mut settings = self.settings;

        renderer
            .egui_state
            .update(&self.window.get_handle(), |ctx| {
                self.data.ui_pixel_per_point = ctx.input(|i| i.pixels_per_point);

                let access = ui::UiAccess {
                    vp_texture: &renderer.framebuffer_resolve,
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

        match renderer.render(&self.camera) {
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.request_redraw();
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
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    n_indices: usize,

    egui_state: egui_state::EguiState,

    viewport_size: wgpu::Extent3d,
    framebuffer: gpu::Texture,
    framebuffer_resolve: gpu::Texture,
    depthbuffer: Option<gpu::Texture>,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    window_size: PhysicalSize<u32>,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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

fn iso_triangle(
    p1: iso::EvalPoint<3>,
    p2: iso::EvalPoint<3>,
    p3: iso::EvalPoint<3>,
) -> [Vertex; 3] {
    [
        Vertex {
            pos: p1.pos.into(),
            norm: Vec3::splat(0.0),
        },
        Vertex {
            pos: p2.pos.into(),
            norm: Vec3::splat(0.0),
        },
        Vertex {
            pos: p3.pos.into(),
            norm: Vec3::splat(0.0),
        },
    ]
}

fn cell_verts_to_vertex(vts: &[iso::EvalPoint<3>]) -> Vec<Vertex> {
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
    vertices.extend(iso_triangle(dl, dr, dfl));
    vertices.extend(iso_triangle(dr, dfr, dfl));
    // front
    vertices.extend(iso_triangle(dl, upl, dr));
    vertices.extend(iso_triangle(dr, upl, upr));
    // left
    vertices.extend(iso_triangle(dl, upfl, upl));
    vertices.extend(iso_triangle(dl, dfl, upfl));
    // right
    vertices.extend(iso_triangle(dr, upr, upfr));
    vertices.extend(iso_triangle(dr, upfr, dfr));
    // back
    vertices.extend(iso_triangle(dfl, dfr, upfl));
    vertices.extend(iso_triangle(dfr, upfr, upfl));
    // top
    vertices.extend(iso_triangle(upl, upfr, upr));
    vertices.extend(iso_triangle(upl, upfl, upfr));

    vertices
}

fn build_unit_square() -> Vec<Vertex> {
    let tr = Vec3::new(1.0, 1.0, 0.0);
    let tl = Vec3::new(0.0, 1.0, 0.0);
    let bl = Vec3::new(0.0, 0.0, 0.0);
    let br = Vec3::new(1.0, 0.0, 0.0);

    let norm = Vec3::Z;

    vec![
        Vertex { pos: bl, norm },
        Vertex { pos: br, norm },
        Vertex { pos: tl, norm },
        Vertex { pos: br, norm },
        Vertex { pos: tr, norm },
        Vertex { pos: tl, norm },
    ]
}

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

fn build_mesh(settings: &AtlasSettings) -> Vec<Vertex> {
    match settings.mesh_gen {
        MeshGenerator::Iso2DNew => build_iso2(settings),
        MeshGenerator::Iso2D => build_mesh_2d(settings),
        MeshGenerator::Iso3D => build_mesh_3d(settings),
    }
}

fn build_mesh_2d(settings: &AtlasSettings) -> Vec<Vertex> {
    let f = |n: Vec2| -> f32 {
        let (x, y) = (n.x, n.y);
        1.0 / 3f32.powf(x).sin() + y.sin() - y
    };

    let min: Vec3 = settings.mesh_min.into();
    let max: Vec3 = settings.mesh_max.into();

    let (lines, tree) = iso::line::build(
        min.xy(),
        max.xy(),
        settings.min_depth,
        settings.max_cells,
        f,
    );

    let mut vertices = vec![];
    for line in lines {
        for pts in line.as_slice().windows(3) {
            let p0 = pts[0].extend(0.0);
            let p1 = pts[1].extend(0.0);
            let p2 = pts[2].extend(0.0);

            let norm = (0.0, 0.0, 0.0).into();

            vertices.extend([
                Vertex { pos: p0, norm },
                Vertex { pos: p1, norm },
                Vertex { pos: p2, norm },
            ]);
        }
    }

    if settings.show_tree {
        for cell in tree.cells {
            let verts = cell.verts.as_ref();

            let p0 = Vec2::from(verts[0].pos).extend(0.0);
            let p1 = Vec2::from(verts[1].pos).extend(0.0);
            let p2 = Vec2::from(verts[2].pos).extend(0.0);
            let p3 = Vec2::from(verts[3].pos).extend(0.0);
            let norm = Vec3::ZERO;

            vertices.extend([
                Vertex { pos: p0, norm },
                Vertex { pos: p1, norm },
                Vertex { pos: p3, norm },
                Vertex { pos: p0, norm },
                Vertex { pos: p3, norm },
                Vertex { pos: p2, norm },
            ]);
        }
    }

    vertices
}

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
    let min = settings.mesh_min.into();
    let max = settings.mesh_max.into();

    let start = time::Instant::now();
    let (tris, tree) = iso::surface::build(min, max, settings.min_depth, settings.max_cells, f);

    log::info!(
        "extracted isosurface in: {} s",
        (time::Instant::now() - start).as_secs_f64()
    );

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
                Vertex { pos: p1, norm: n1 },
                Vertex { pos: p2, norm: n2 },
                Vertex { pos: p3, norm: n3 },
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
        for cell in tree.cells {
            vertices.extend(cell_verts_to_vertex(cell.verts.as_ref()));
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

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Camera buffer".into(),
            contents: bytemuck::cast_slice(&[world_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: "camera_bind_group_layout".into(),
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: "camera_bind_group".into(),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let mut egui_state = egui_state::EguiState::new(&device, config.format, None, 1, &window);

        let framebuffer = gpu::TextureConfig::d2(viewport_size, config.format)
            .msaa_samples(4)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        let framebuffer_resolve = gpu::TextureConfig::d2(viewport_size, config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut egui_state)
            .build(&device);

        let depthbuffer = if settings.render_config.depth_buffer {
            Some(
                gpu::TextureConfig::depthf32(viewport_size)
                    .msaa_samples(4)
                    .as_render_attachment()
                    .as_texture_binding()
                    .build(&device),
            )
        } else {
            None
        };

        log::debug!("setup framebuffers");

        //let module = gpu::ShaderConfig::from_wgsl(include_str!("shader.wgsl"))
        let module = gpu::ShaderConfig::from_wgsl(include_str!("mpr.wgsl"))
            .with_struct::<Vertex>("VertexInput")
            .with_struct::<WorldUniform>("WorldUniform")
            .build(&device);

        let render_pipeline = gpu::PipelineConfig::new(&module)
            //.color_depth(framebuffer.format(), depthbuffer.format())
            .color(framebuffer.format())
            .set_if(depthbuffer.is_some(), |p| {
                p.depth_format(depthbuffer.as_ref().unwrap().format())
            })
            .msaa_samples(framebuffer.msaa_samples())
            .polygon_mode(settings.render_config.polygon_mode.into())
            .set_cull_mode(settings.render_config.cull_mode.into())
            .bind_group_layouts(&[&camera_bind_group_layout])
            .label("render pipeline")
            .build(&device);

        let vertices = build_mesh(settings);

        let indices: Vec<_> = (0..vertices.len() as u32).collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let n_indices = indices.len();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        log::debug!("finish initializing wgpu context");

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            n_indices,
            egui_state,
            viewport_size,
            framebuffer,
            framebuffer_resolve,
            depthbuffer,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            window_size,
            window,
        }
    }

    fn rebuild_mesh(&mut self, settings: &AtlasSettings) {
        let vertices = build_mesh(settings);

        let indices: Vec<_> = (0..vertices.len() as u32).collect();

        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.n_indices = indices.len();
        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
    }

    fn rebuild_from_settings(&mut self, settings: &AtlasSettings) {
        let msaa_samples = 4;

        self.framebuffer = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .msaa_samples(msaa_samples)
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.framebuffer_resolve = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut self.egui_state)
            .build(&self.device);

        self.depthbuffer = if settings.render_config.depth_buffer {
            Some(
                gpu::TextureConfig::depthf32(self.viewport_size)
                    .msaa_samples(msaa_samples)
                    .as_render_attachment()
                    .as_texture_binding()
                    .build(&self.device),
            )
        } else {
            None
        };

        let module = gpu::ShaderConfig::from_wgsl(include_str!("mpr.wgsl"))
            .with_struct::<Vertex>("VertexInput")
            .with_struct::<WorldUniform>("WorldUniform")
            .build(&self.device);

        self.render_pipeline = gpu::PipelineConfig::new(&module)
            //.color_depth(framebuffer.format(), depthbuffer.format())
            .color(self.framebuffer.format())
            .set_if(self.depthbuffer.is_some(), |p| {
                p.depth_format(self.depthbuffer.as_ref().unwrap().format())
            })
            .msaa_samples(self.framebuffer.msaa_samples())
            .polygon_mode(settings.render_config.polygon_mode.into())
            .set_cull_mode(settings.render_config.cull_mode.into())
            .bind_group_layouts(&[&self.camera_bind_group_layout])
            .label("render pipeline")
            .build(&self.device);
    }

    fn rebuild_framebuffer(&mut self) {
        self.framebuffer = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .msaa_samples(self.framebuffer.msaa_samples())
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.framebuffer_resolve = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut self.egui_state)
            .build(&self.device);

        if self.depthbuffer.is_some() {
            self.depthbuffer = Some(
                gpu::TextureConfig::depthf32(self.viewport_size)
                    .msaa_samples(self.depthbuffer.as_ref().unwrap().msaa_samples())
                    .as_render_attachment()
                    .as_texture_binding()
                    .build(&self.device),
            );
        }
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

    fn render(&mut self, camera: &OribtCamera) -> Result<(), wgpu::SurfaceError> {
        self.render_scene(camera)?;
        self.render_ui()
    }

    fn render_scene(&mut self, camera: &OribtCamera) -> Result<(), wgpu::SurfaceError> {
        if self.n_indices == 0 {
            return Ok(());
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: "Viewport Encoder".into(),
            });

        let world_uniform = WorldUniform::new(camera.view_proj_mat(), camera.eye());

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[world_uniform]),
        );

        //let rpass = gpu::RenderPass::target_color_depth(&self.framebuffer, &self.depthbuffer)
        //gpu::RenderPass::target_color_depth(&self.framebuffer, &self.depthbuffer)
        gpu::RenderPass::target_color(&self.framebuffer)
            .set_if(self.depthbuffer.is_some(), |rpass| {
                rpass.depth_target(self.depthbuffer.as_ref().unwrap())
            })
            .resolve_target(&self.framebuffer_resolve)
            .label("main renderpass")
            .clear_hex("#24273a")
            .draw(&mut encoder, |mut rpass| {
                rpass.set_bind_group(0, &self.camera_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                rpass.set_pipeline(&self.render_pipeline);
                rpass.draw(0..self.n_indices as u32, 0..1);
            });

        self.queue.submit([encoder.finish()]);

        Ok(())
    }

    fn render_ui(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: "Viewport Encoder".into(),
            });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window().scale_factor() as f32,
        };

        self.egui_state.render(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &output_view,
            screen_descriptor,
        );

        self.queue.submit([encoder.finish()]);
        output.present();

        Ok(())
    }

    fn window(&self) -> &winit::window::Window {
        &self.window
    }
}
