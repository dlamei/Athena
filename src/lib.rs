mod camera;
mod egui_state;
mod wgpu_utils;

use camera::{Camera, OribtCamera};
use wgpu_utils::gpu;

use egui_tiles as tiles;
use glam::{Mat4, Vec2, Vec3};
use std::{fmt, sync::Arc, time};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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
    window: AtlasApp,
    event_loop: EventLoop<()>,
}

impl Atlas {
    pub fn init() -> Self {
        env_logger::builder()
            .filter_level(log::LevelFilter::Warn)
            .filter_module("atlas", log::LevelFilter::Info)
            .init();

        let event_loop = EventLoop::new().unwrap();
        // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
        // dispatched any events. This is ideal for games and similar applications.
        //event_loop.set_control_flow(ControlFlow::Poll);

        // ControlFlow::Wait pauses the event loop if no events are available to process.
        // This is ideal for non-game applications that only update in response to user
        // input, and uses significantly less power/CPU time than ControlFlow::Poll.
        event_loop.set_control_flow(ControlFlow::Wait);

        Self {
            event_loop,
            window: AtlasApp::new(),
        }
    }

    pub fn run(mut self) -> Result<(), EventLoopError> {
        self.event_loop.run_app(&mut self.window)
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
    window_info: &'a mut WindowData,
    //vp_dragged: &'a mut bool,
    //vp_rect: &'a mut egui::Rect,

    render_config: &'a mut AtlasSettings,
}

type UiDemo = egui_demo_lib::WidgetGallery;

struct UiViewer<'a> {
    access: UiAccess<'a>,
    ui_demo: &'a mut UiDemo,
    egui_ctx: &'a egui::Context,
}

mod ui_lib {
    pub fn selection<T>(
        label: &str,
        curr_mut: &mut T,
        options: &[(T, &str)],
        ui: &mut egui::Ui,
    )
    where
        T: PartialEq + Copy,
    {
        let curr_name = options
            .iter()
            .find(|(val, _)| *val == *curr_mut)
            .expect("could not find current selection in options").1;

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
        let uv = egui::Rect::from_min_max([0., 0.].into(), [1., 1.].into());
        ui.painter().image(
            self.access.vp_texture.egui_id(),
            ui.max_rect(),
            uv,
            egui::Color32::WHITE,
        );

        //ui.allocate_space(ui.available_size());
        let response = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());

        self.access.window_info.viewport_dragged = response.dragged();
        self.access.window_info.viewport_rect = ui.min_rect();

        tiles::UiResponse::None
    }

    fn inspector(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        egui::widgets::global_theme_preference_switch(ui);
        egui_demo_lib::View::ui(self.ui_demo, ui);
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

    fn render_settings(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let render_config = &mut self.access.render_config;

        ui.add_space(20.0);
        ui.spacing_mut().item_spacing.y = 10.0;

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
            &[
                (1, "off"),
                (4, "4x"),
            ],
            ui,
        );

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
            UiTab::Settings => self.render_settings(ui, tile_id),
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
    ui_demo: UiDemo,
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

        let ui_demo = UiDemo::default();
        Self {
            tile_state,
            ui_demo,
        }
    }

    fn ui(&mut self, ctx: &egui::Context, access: UiAccess) {
        let ui_demo = &mut self.ui_demo;
        let mut viewer = UiViewer {
            access,
            ui_demo,
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
    mouse_position: Option<Vec2>,
    mouse_delta: Vec2,
    viewport_drag_offset: Vec2,
    viewport_dragged: bool,
    viewport_rect: egui::Rect,
    prev_frame_time: time::Instant,
}

struct AtlasApp {
    wgpu_ctx: Option<WgpuContext>,
    camera: OribtCamera,
    ui_state: UiState,

    data: WindowData,
    settings: AtlasSettings,
}

impl AtlasApp {
    fn new() -> Self {
        let settings = AtlasSettings {
            msaa_samples: 4,
            cull_mode: None,
            fov: 90.0,
        };

        let camera = OribtCamera::look_at(Vec3::new(2.0, 2.0, 2.0), Vec3::ZERO, settings.fov.to_radians());

        let data = WindowData {
            mouse_position: None,
            mouse_delta: Vec2::ZERO,
            viewport_drag_offset: Vec2::ZERO,
            viewport_dragged: false,
            viewport_rect: egui::Rect::ZERO,
            prev_frame_time: time::Instant::now(),
        };

        Self {
            wgpu_ctx: None,
            ui_state: UiState::new(),
            data,
            settings,
            camera,
        }
    }

    fn wgpu_ctx(&self) -> &WgpuContext {
        self.wgpu_ctx.as_ref().unwrap()
    }

    fn wgpu_ctx_mut(&mut self) -> &mut WgpuContext {
        self.wgpu_ctx.as_mut().unwrap()
    }

    fn update(&mut self) {
        let prev_time = self.data.prev_frame_time;
        let curr_time = time::Instant::now();
        let dt = curr_time - prev_time;
        self.data.prev_frame_time = curr_time;

        self.camera.set_aspect(
            self.data.viewport_rect.width() as u32,
            self.data.viewport_rect.height() as u32,
        );
        self.camera.time_step(dt);

        if self.data.viewport_dragged {
            self.camera
                .process_mouse(self.data.mouse_delta.x.into(), self.data.mouse_delta.y.into());
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
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

    fn window(&self) -> &winit::window::Window {
        self.wgpu_ctx.as_ref().unwrap().window()
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
        let window = event_loop
            .create_window(winit::window::Window::default_attributes().with_title("Atlas"))
            .unwrap();

        self.wgpu_ctx = WgpuContext::new(window, &self.settings).into();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.update();

        if self.window().id() == window_id && !self.input(&event) {
            let wgpu_ctx = self.wgpu_ctx.as_mut().unwrap();
            wgpu_ctx.input(&event);

            self.data.mouse_delta = Vec2::ZERO;

            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    wgpu_ctx.resize_window(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    wgpu_ctx.window().request_redraw();

                    let prev_viewport_size = wgpu_ctx.viewport_size;
                    let mut settings = self.settings;

                    wgpu_ctx.egui_state.update(&wgpu_ctx.window, |ctx| {
                        let access = UiAccess {
                            vp_texture: &wgpu_ctx.framebuffer_resolve,
                            camera: &self.camera,
                            window_info: &mut self.data,
                            render_config: &mut settings,
                        };
                        self.ui_state.ui(ctx, access);

                        wgpu_ctx.viewport_size = wgpu::Extent3d {
                            width: self.data.viewport_rect.width() as u32,
                            height: self.data.viewport_rect.height() as u32,
                            depth_or_array_layers: 1,
                        }
                    });

                    self.camera.fov_rad = settings.fov.to_radians();
                    match wgpu_ctx.render(&self.camera) {
                        Ok(_) => (),

                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            wgpu_ctx.resize_window(wgpu_ctx.window_size)
                        }
                        Err(err @ wgpu::SurfaceError::Timeout) => {
                            log::warn!("{err}")
                        }
                        Err(err) => {
                            log::error!("{err}");
                            event_loop.exit()
                        }
                    }

                    if self.settings != settings {
                        self.settings = settings;
                        wgpu_ctx.rebuild_from_settings(&self.settings);
                    } else if prev_viewport_size != wgpu_ctx.viewport_size {
                        wgpu_ctx.resize_viewport();
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let data = &mut self.data;
                    let pos = Vec2::new(position.x as f32, position.y as f32);

                    let prev_pos = if let Some(prev_pos) = data.mouse_position {
                        prev_pos
                    } else {
                        data.mouse_position = Some(pos);
                        pos
                    };

                    let window = wgpu_ctx.window();

                    let window_size = window.inner_size();

                    let width = window_size.width as f32;
                    let height = window_size.height as f32;

                    let mut set_cursor_pos = |new_x: f32, new_y: f32| -> Option<()> {
                        window
                            .set_cursor_position(winit::dpi::PhysicalPosition::new(new_x, new_y))
                            .ok()?;
                        data.viewport_drag_offset = (pos.x - new_x, pos.y - new_y).into();
                        data.mouse_position = Some((new_x, new_y).into());
                        None
                    };

                    if pos.x < 0.0 {
                        set_cursor_pos(width - 1.0, pos.y);
                    } else if pos.x >= width {
                        set_cursor_pos(0.0, pos.y);
                    } else if pos.y < 0.0 {
                        set_cursor_pos(pos.x, height - 1.0);
                    } else if pos.y >= height {
                        set_cursor_pos(pos.x, 0.0);
                    } else {
                        data.viewport_drag_offset = Vec2::ZERO;
                        data.mouse_delta = pos - prev_pos;
                        data.mouse_position = Some(pos);
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let window = self.wgpu_ctx.as_ref().unwrap().window();
        window.request_redraw();
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct AtlasSettings {
    msaa_samples: u32,
    cull_mode: Option<wgpu::Face>,
    fov: f32,
}

struct WgpuContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    egui_state: egui_state::EguiState,

    viewport_size: wgpu::Extent3d,
    framebuffer: gpu::Texture,
    framebuffer_resolve: gpu::Texture,
    depthbuffer: gpu::Texture,

    //camera: FPCamera,
    //camera_controller: FPCameraController,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    //prev_frame_time: time::Instant,
    window_size: winit::dpi::PhysicalSize<u32>,
    // drop last
    window: Arc<winit::window::Window>,
}

impl WgpuContext {
    fn new(winit_window: winit::window::Window, render_config: &AtlasSettings) -> Self {
        let window = Arc::new(winit_window);
        let window_size = window.inner_size();
        let instance = gpu::init_instance();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = gpu::init_adapter(instance, &surface);
        let (device, queue) = gpu::init_device(&adapter);
        let surface_caps = surface.get_capabilities(&adapter);
        let config = gpu::default_surface_config(window_size, surface_caps);
        surface.configure(&device, &config);

        let adapter_info = adapter.get_info();
        log::info!("{adapter_info:#?}");

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

        let render_pipeline =
            gpu::PipelineConfig::color_depth(framebuffer.format(), depthbuffer.format())
                .msaa_samples(framebuffer.msaa_samples())
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

        //let prev_frame_time = time::Instant::now();

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
            //prev_frame_time,
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
               .set_cull_mode(cull_mode)
               .bind_group_layouts(&[&self.camera_bind_group_layout])
               .build(include_str!("shader.wgsl"), &self.device);

       //self.rebuild_render_pipeline();
    }

    //fn rebuild_render_pipeline(&mut self) {
    //    self.render_pipeline =
    //        gpu::PipelineConfig::color_depth(self.framebuffer.format(), self.depthbuffer.format())
    //            .msaa_samples(self.framebuffer.msaa_samples())
    //            .set_cull_mode(self.render_config.cull_mode)
    //            .bind_group_layouts(&[&self.camera_bind_group_layout])
    //            .build(include_str!("shader.wgsl"), &self.device);
    //}

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
        self.rebuild_framebuffer();
        //self.camera
        //    .set_aspect(self.viewport_size.width, self.viewport_size.height);
    }

    fn resize_window(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
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
        //let prev_time = self.prev_frame_time;
        //self.prev_frame_time = time::Instant::now();

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
            .clear_rgb(0.1, 0.2, 0.3)
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
