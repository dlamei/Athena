mod camera;
mod egui_state;
mod wgpu_utils;

use camera::{Camera, OribtCamera};
use egui::Rect;
use wgpu_utils::gpu;

use egui_state::GizmoExt;
use egui_tiles as tiles;
use glam::{Mat4, Vec2, Vec3};
use std::{fmt, sync::Arc, time};
use wgpu::util::DeviceExt;
use transform_gizmo as gizmo;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    error::EventLoopError,
    event::{ElementState, Event, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub type Instant = quanta::Instant;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run_wasm() {
    Atlas::init().run().unwrap()
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pos: Vec3,
    norm: Vec3,
}

pub trait VertexFormat {
    const VERT_FORMAT: wgpu::VertexFormat;
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

impl VertexFormat for Vec3 {
    const VERT_FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Float32x3;
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        new_vert_attrib_array([Vec3::VERT_FORMAT, Vec3::VERT_FORMAT]);
    //wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

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

const INDICES: &[u16] = &[
    // Front face
    2, 1, 0, 0, 3, 2, // Back face
    4, 5, 6, 6, 7, 4, // Left face
    10, 11, 8, 8, 9, 10, // Right face
    12, 15, 14, 14, 13, 12, // Top face
    18, 19, 16, 16, 17, 18, // Bottom face
    20, 23, 22, 22, 21, 20,
];

pub struct Atlas {
    //window: AtlasApp,
    event_loop: EventLoop<()>,
    //window: Window,
}

impl Atlas {
    pub fn init() -> Self {
        log::debug!("Atlas::init");

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
            } else {
                env_logger::builder()
                    .filter_level(log::LevelFilter::Warn)
                    .filter_module("atlas", log::LevelFilter::Info)
                    .init();
            }
        }

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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
enum UiTab {
    Viewport,
    Inspector,
    Settings,
    Placeholder,
}

impl fmt::Display for UiTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiTab::Viewport => write!(f, "viewport"),
            UiTab::Inspector => write!(f, "inspector"),
            UiTab::Placeholder => write!(f, "placeholder"),
            UiTab::Settings => write!(f, "settings"),
        }
    }
}

struct UiAccess<'a> {
    vp_texture: &'a gpu::Texture,
    camera: &'a dyn Camera,
    gizmo: &'a mut gizmo::Gizmo,
    window_info: &'a mut WindowData,
    //vp_dragged: &'a mut bool,
    //vp_rect: &'a mut egui::Rect,
    render_config: &'a mut AtlasSettings,
}

//type UiDemo = egui_demo_lib::WidgetGallery;

struct UiViewer<'a> {
    access: UiAccess<'a>,
    //ui_demo: &'a mut UiDemo,
    egui_ctx: &'a egui::Context,
}

mod ui_lib {
    pub fn selection<T>(label: &str, curr_mut: &mut T, options: &[(T, &str)], ui: &mut egui::Ui)
    where
        T: PartialEq + Copy,
    {
        let curr_name = options
            .iter()
            .find(|(val, _)| *val == *curr_mut)
            .expect("could not find current selection in options")
            .1;

        egui::ComboBox::from_label(label)
            .selected_text(curr_name.to_string())
            .show_ui(ui, |ui| {
                for (opt, opt_name) in options {
                    ui.selectable_value(curr_mut, *opt, opt_name.to_string());
                }
            });
    }
}

impl UiViewer<'_> {
    fn viewport(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let min = ui.cursor().min;

        let uv = Rect::from_min_max([0., 0.].into(), [1., 1.].into());
        ui.painter().image(
            self.access.vp_texture.egui_id(),
            ui.max_rect(),
            uv,
            egui::Color32::WHITE,
        );

        //ui.allocate_space(ui.available_size());
        let resp = ui.allocate_rect(ui.max_rect(), egui::Sense::drag());


        self.access.window_info.viewport_rect = resp.rect;
        self.access.window_info.viewport_dragged = resp.dragged();

        let gizmo = &mut self.access.gizmo;

        let mut config = gizmo.config().clone();
        let view = self.access.camera.view_mat().as_dmat4();
        let proj = self.access.camera.proj_mat().as_dmat4();
        let vp_rect = resp.rect;

        config.view_matrix = mint::RowMatrix4::from(view);
        config.projection_matrix = mint::RowMatrix4::from(proj);
        config.viewport = gizmo::Rect::from_min_max((vp_rect.min.x, vp_rect.min.y).into(), (vp_rect.max.x, vp_rect.max.y).into());
        config.pixels_per_point = self.access.window_info.ui_pixel_per_point;

        gizmo.update_config(config);

        let hover_pos = resp.hover_pos().unwrap_or_default();
        let hovered = resp.hovered();

        let gizmo_result = gizmo.update(
            gizmo::GizmoInteraction {
                cursor_pos: (hover_pos.x, hover_pos.y),
                hovered,
                drag_started: resp.drag_started(), //ui .input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)),
                dragging: resp.dragged(), //ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary)),
            },
            &[],
        );

        if gizmo_result.is_some() {
            self.access.window_info.viewport_dragged = false;
        }

        let draw_data = gizmo.draw();

        //egui::Painter::new(ui.ctx().clone(), ui.layer_id(), vp_rect)
        ui.painter()
            .add(egui::Mesh {
            indices: draw_data.indices,
            vertices: draw_data
                .vertices
                .into_iter()
                .zip(draw_data.colors)
                .map(|(pos, [r, g, b, a])| egui::epaint::Vertex {
                    pos: pos.into(),
                    uv: egui::Pos2::default(),
                    color: egui::Rgba::from_rgba_premultiplied(r, g, b, a).into(),
                })
                .collect(),
            ..Default::default()
        });


        //self.access.gizmo.interact(ui, &[]);


        tiles::UiResponse::None
    }

    fn inspector(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        egui::widgets::global_theme_preference_switch(ui);
        //egui_demo_lib::View::ui(self.ui_demo, ui);
        tiles::UiResponse::None
    }

    fn placeholder(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let color = egui::epaint::Rgba::from_rgb(0.2, 0.0, 0.2);
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);

        //self.test_ui.ui(ui);

        let dragged = ui
            .allocate_rect(ui.max_rect(), egui::Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::Grab)
            .dragged();
        if dragged {
            tiles::UiResponse::DragStarted
        } else {
            tiles::UiResponse::None
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let render_config = &mut self.access.render_config;

        ui.add_space(20.0);

        ui.add(egui::Slider::new(&mut render_config.fov, 1.0..=180.0).text("FOV"));

        ui_lib::selection(
            "cull mode",
            &mut render_config.cull_mode,
            &[
                (None, "none"),
                (Some(wgpu::Face::Back), "back"),
                (Some(wgpu::Face::Front), "front"),
            ],
            ui,
        );

        ui_lib::selection(
            "msaa samples",
            &mut render_config.msaa_samples,
            &[(1, "off"), (4, "4x")],
            ui,
        );

        ui_lib::selection(
            "polygon mode",
            &mut render_config.polygon_mode,
            &[
                (wgpu::PolygonMode::Fill, "fill"),
                (wgpu::PolygonMode::Line, "line"),
            ],
            ui,
        );

        ui.add_space(12.0);

        let info = &self.access.window_info;

        egui::Grid::new("Debug Values").show(ui, |ui| {
            ui.separator();
            ui.label("mouse");

            ui.end_row();
            ui.label("pixel pos");
            let pos = info.mouse_pixel_pos;
            ui.label(format!("({:4.0}, {:4.0})", pos.x, pos.y));
            ui.end_row();
            ui.label("dpos");
            let dpos = info.mouse_delta;
            ui.label(format!("({:4.0}, {:4.0})", dpos.x, dpos.y));
            ui.end_row();

            ui.add(egui::Separator::default().horizontal());
            ui.label("viewport");

            ui.end_row();
            ui.label("rect");
            let min = info.viewport_rect.min;
            let max = info.viewport_rect.max;
            ui.label(format!(
                "({:4.0}, {:4.0}) ({:4.0}, {:4.0})",
                min.x, min.y, max.x, max.y
            ));
            ui.end_row();
            ui.label("dragged");
            ui.label(info.viewport_dragged.to_string());
            ui.end_row();
            let dt = info.delta_time.as_secs_f32();
            let fps = (1.0 / dt) as u32;
            let fps = if fps > 420 {
                "420".into()
            } else {
                format!("{fps:3}")
            };
            ui.label(format!("{:2.2} ms / {fps} fps", dt * 1000.0));
        });

        tiles::UiResponse::None
    }
}

impl tiles::Behavior<UiTab> for UiViewer<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        tile_id: tiles::TileId,
        pane: &mut UiTab,
    ) -> tiles::UiResponse {
        match pane {
            UiTab::Viewport => self.viewport(ui, tile_id),
            UiTab::Inspector => self.inspector(ui, tile_id),
            UiTab::Placeholder => self.placeholder(ui, tile_id),
            UiTab::Settings => self.settings(ui, tile_id),
        }
    }

    fn tab_title_for_pane(&mut self, pane: &UiTab) -> egui::WidgetText {
        format!("{pane}").into()
    }

    fn simplification_options(&self) -> tiles::SimplificationOptions {
        tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

struct UiState {
    tile_state: tiles::Tree<UiTab>,
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

impl UiState {
    fn new() -> Self {
        let mut tiles = tiles::Tiles::default();

        let vp = tiles.insert_pane(UiTab::Viewport);
        let insp = tiles.insert_pane(UiTab::Inspector);
        let root = tiles.insert_tab_tile(vec![vp, insp]);

        let tabs = vec![root, tiles.insert_pane(UiTab::Settings)];
        let root = tiles.insert_horizontal_tile(tabs);

        let tile_state = tiles::Tree::new("tiles", root, tiles);

        Self { tile_state }
    }

    fn ui(&mut self, ctx: &egui::Context, access: UiAccess) {
        // let ui_demo = &mut self.ui_demo;
        let mut viewer = UiViewer {
            access,
            egui_ctx: ctx,
        };

        egui::CentralPanel::default()
            //.frame(egui::Frame::central_panel(&ctx.style()))
            .show(ctx, |ui| {
                self.tile_state.ui(&mut viewer, ui);
            });
    }
}

struct WindowData {
    mouse_pixel_pos: Vec2,
    mouse_delta: Vec2,
    viewport_dragged: bool,
    viewport_rect: Rect,

    ui_pixel_per_point: f32,

    delta_time: time::Duration,
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

struct AtlasApp {
    renderer: Option<AtlasRenderer>,
    camera: OribtCamera,
    gizmo: gizmo::Gizmo,

    ui_state: UiState,

    data: WindowData,
    settings: AtlasSettings,

    window: WindowHandle,
}

impl AtlasApp {
    fn new() -> Self {
        log::debug!("init atlas app");

        let settings = AtlasSettings {
            msaa_samples: 4,
            cull_mode: None,
            fov: 90.0,
            polygon_mode: wgpu::PolygonMode::Fill,
        };

        let camera = OribtCamera::look_at(
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::ZERO,
            settings.fov.to_radians(),
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

        let gizmo = gizmo::Gizmo::new(
            gizmo::GizmoConfig {
                modes: gizmo::GizmoMode::all_translate() | gizmo::GizmoMode::all_scale(),
                ..Default::default()
            });

        Self {
            renderer: None,
            gizmo,
            ui_state: UiState::new(),
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
        let mut settings = self.settings;

        renderer
            .egui_state
            .update(&self.window.get_handle(), |ctx| {
                self.data.ui_pixel_per_point = ctx.input(|i| i.pixels_per_point);

                let access = UiAccess {
                    vp_texture: &renderer.framebuffer_resolve,
                    camera: &self.camera,
                    gizmo: &mut self.gizmo,
                    window_info: &mut self.data,
                    render_config: &mut settings,
                };

                self.ui_state.ui(ctx, access);

                renderer.viewport_size = wgpu::Extent3d {
                    width: self.data.viewport_rect.width() as u32,
                    height: self.data.viewport_rect.height() as u32,
                    depth_or_array_layers: 1,
                }
            });

        self.camera.fov_rad = settings.fov.to_radians();
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

        if self.settings != settings {
            self.settings = settings;
            renderer.rebuild_from_settings(&self.settings);
        } else if prev_viewport_size != renderer.viewport_size {
            renderer.resize_viewport();
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

#[derive(Debug, Copy, Clone, PartialEq)]
struct AtlasSettings {
    msaa_samples: u32,
    cull_mode: Option<wgpu::Face>,
    polygon_mode: wgpu::PolygonMode,
    fov: f32,
}

struct AtlasRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    egui_state: egui_state::EguiState,

    viewport_size: wgpu::Extent3d,
    framebuffer: gpu::Texture,
    framebuffer_resolve: gpu::Texture,
    depthbuffer: gpu::Texture,

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

impl AtlasRenderer {
    fn new(ctx: gpu::WgpuContext, render_config: &AtlasSettings) -> Self {
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

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Camera buffer".into(),
            contents: bytemuck::cast_slice(&[Mat4::IDENTITY]),
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
            .msaa_samples(render_config.msaa_samples)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        let framebuffer_resolve = gpu::TextureConfig::d2(viewport_size, config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut egui_state)
            .build(&device);

        let depthbuffer = gpu::TextureConfig::depthf32(viewport_size)
            .msaa_samples(render_config.msaa_samples)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        log::debug!("setup framebuffers");

        let render_pipeline =
            gpu::PipelineConfig::color_depth(framebuffer.format(), depthbuffer.format())
                .msaa_samples(framebuffer.msaa_samples())
                .polygon_mode(render_config.polygon_mode)
                .set_cull_mode(render_config.cull_mode)
                .bind_group_layouts(&[&camera_bind_group_layout])
                .build(include_str!("shader.wgsl"), &device);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
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

    fn rebuild_from_settings(&mut self, render_config: &AtlasSettings) {
        let msaa_samples = render_config.msaa_samples;
        let cull_mode = render_config.cull_mode;

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

        self.depthbuffer = gpu::TextureConfig::depthf32(self.viewport_size)
            .msaa_samples(msaa_samples)
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.render_pipeline =
            gpu::PipelineConfig::color_depth(self.framebuffer.format(), self.depthbuffer.format())
                .msaa_samples(self.framebuffer.msaa_samples())
                .polygon_mode(render_config.polygon_mode)
                .set_cull_mode(cull_mode)
                .bind_group_layouts(&[&self.camera_bind_group_layout])
                .build(include_str!("shader.wgsl"), &self.device);
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

        self.depthbuffer = gpu::TextureConfig::depthf32(self.viewport_size)
            .msaa_samples(self.depthbuffer.msaa_samples())
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);
    }

    fn resize_viewport(&mut self) {
        if self.viewport_size.width == 0 || self.viewport_size.height == 0 {
            return
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

    fn render(&mut self, camera: &impl Camera) -> Result<(), wgpu::SurfaceError> {
        self.render_scene(camera)?;
        self.render_ui()
    }

    fn render_scene(&mut self, camera: &impl Camera) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: "Viewport Encoder".into(),
            });

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.view_proj_mat()]),
        );

        let rpass = gpu::RenderPass::with_color_depth(&self.framebuffer, &self.depthbuffer)
            .label("main renderpass")
            //.clear_rgb(0.1, 0.2, 0.3)
            .clear_hex("#24273a")
            .render_pipeline(&self.render_pipeline)
            .bind_group(&self.camera_bind_group)
            .vertex_buffer(self.vertex_buffer.slice(..))
            .index_buffer(
                self.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
                0..INDICES.len() as u32,
            );

        if self.framebuffer.msaa_samples() != 1 {
            rpass.resolve_target(&self.framebuffer_resolve)
        } else {
            rpass.color_target(&self.framebuffer_resolve)
        }
        .finish(&mut encoder);

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
