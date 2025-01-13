mod camera;
mod egui_state;
mod wgpu_utils;

use camera::{Camera, CameraController};
use wgpu_utils::gpu;

use egui_tiles as tiles;
use glam::{Vec2, Vec3};
use std::{fmt, sync::Arc};
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
    vert! { [-0.5, -0.5,  0.5], [ 0.0,  0.0,  1.0] },
    vert! { [ 0.5, -0.5,  0.5], [ 0.0,  0.0,  1.0] },
    vert! { [ 0.5,  0.5,  0.5], [ 0.0,  0.0,  1.0] },
    vert! { [-0.5,  0.5,  0.5], [ 0.0,  0.0,  1.0] },
    vert! { [-0.5, -0.5, -0.5], [ 0.0,  0.0, -1.0] },
    vert! { [ 0.5, -0.5, -0.5], [ 0.0,  0.0, -1.0] },
    vert! { [ 0.5,  0.5, -0.5], [ 0.0,  0.0, -1.0] },
    vert! { [-0.5,  0.5, -0.5], [ 0.0,  0.0, -1.0] },
    vert! { [-0.5, -0.5, -0.5], [-1.0,  0.0,  0.0] },
    vert! { [-0.5,  0.5, -0.5], [-1.0,  0.0,  0.0] },
    vert! { [-0.5,  0.5,  0.5], [-1.0,  0.0,  0.0] },
    vert! { [-0.5, -0.5,  0.5], [-1.0,  0.0,  0.0] },
    vert! { [ 0.5, -0.5, -0.5], [ 1.0,  0.0,  0.0] },
    vert! { [ 0.5,  0.5, -0.5], [ 1.0,  0.0,  0.0] },
    vert! { [ 0.5,  0.5,  0.5], [ 1.0,  0.0,  0.0] },
    vert! { [ 0.5, -0.5,  0.5], [ 1.0,  0.0,  0.0] },
    vert! { [-0.5,  0.5, -0.5], [ 0.0,  1.0,  0.0] },
    vert! { [ 0.5,  0.5, -0.5], [ 0.0,  1.0,  0.0] },
    vert! { [ 0.5,  0.5,  0.5], [ 0.0,  1.0,  0.0] },
    vert! { [-0.5,  0.5,  0.5], [ 0.0,  1.0,  0.0] },
    vert! { [-0.5, -0.5, -0.5], [ 0.0, -1.0,  0.0] },
    vert! { [ 0.5, -0.5, -0.5], [ 0.0, -1.0,  0.0] },
    vert! { [ 0.5, -0.5,  0.5], [ 0.0, -1.0,  0.0] },
    vert! { [-0.5, -0.5,  0.5], [ 0.0, -1.0,  0.0] },
];

const INDICES: &[u16] = &[
    // Front face
    0, 1, 2, 2, 3, 0, // Back face
    4, 5, 6, 6, 7, 4, // Left face
    8, 11, 10, 10, 9, 8, // Right face
    12, 15, 14, 14, 13, 12, // Top face
    16, 19, 18, 18, 17, 16, // Bottom face
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
    Placeholder,
}

impl fmt::Display for UiTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiTab::Viewport => write!(f, "viewport"),
            UiTab::Inspector => write!(f, "inspector"),
            UiTab::Placeholder => write!(f, "placeholder"),
        }
    }
}

struct UiAccess<'a> {
    vp_texture: &'a gpu::Texture,
    camera: &'a Camera,
    vp_dragged: &'a mut bool,
    vp_rect: &'a mut egui::Rect,
}

type UiDemo = egui_demo_lib::WidgetGallery;

struct UiViewer<'a> {
    access: UiAccess<'a>,
    ui_demo: &'a mut UiDemo,
    egui_ctx: &'a egui::Context,
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
        let drag_delta = response.drag_delta();

        *self.access.vp_dragged = response.dragged();
        *self.access.vp_rect = ui.min_rect();

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
        return if dragged {
            tiles::UiResponse::DragStarted
        } else {
            tiles::UiResponse::None
        };
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
        let mut tabs = vec![];

        tabs.push(tiles.insert_pane(UiTab::Viewport));
        tabs.push(tiles.insert_pane(UiTab::Inspector));
        tabs.push(tiles.insert_pane(UiTab::Placeholder));

        let root = tiles.insert_tab_tile(tabs);
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
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.))
            .show(ctx, |ui| {
                self.tile_state.ui(&mut viewer, ui);
            });
    }
}

struct AtlasApp {
    wgpu_ctx: Option<WgpuContext>,
    ui_state: UiState,
    mouse_position: Option<Vec2>,
    mouse_delta: Vec2,
    viewport_drag_offset: Vec2,
    viewport_dragged: bool,
    viewport_rect: egui::Rect,
}

impl AtlasApp {
    fn new() -> Self {
        Self {
            wgpu_ctx: None,
            ui_state: UiState::new(),
            mouse_position: None,
            mouse_delta: Vec2::ZERO,
            viewport_drag_offset: Vec2::ZERO,
            viewport_dragged: false,
            viewport_rect: egui::Rect::ZERO,
        }
    }

    fn wgpu_ctx(&self) -> &WgpuContext {
        self.wgpu_ctx.as_ref().unwrap()
    }

    fn wgpu_ctx_mut(&mut self) -> &mut WgpuContext {
        self.wgpu_ctx.as_mut().unwrap()
    }

    fn update(&mut self) {
        let ctx = self.wgpu_ctx.as_mut().unwrap();

        if self.viewport_dragged {
            //if self.viewport_drag_offset != Vec2::ZERO {
            //    log::info!("{} vs {}", self.mouse_delta, self.mouse_delta - self.viewport_drag_offset);
            //} else {
            //    log::info!("{}", self.mouse_delta);
            //}
            ctx.camera_controller
                .process_mouse(self.mouse_delta.x.into(), self.mouse_delta.y.into());
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        let ctx = self.wgpu_ctx.as_mut().unwrap();

        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => ctx.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                ctx.camera_controller.process_scroll(delta);
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

        self.wgpu_ctx = WgpuContext::new(window).into();
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

            self.mouse_delta = Vec2::ZERO;

            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    wgpu_ctx.resize_window(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    wgpu_ctx.window().request_redraw();
                    wgpu_ctx.update();

                    let prev_viewport_size = wgpu_ctx.viewport_size;

                    wgpu_ctx.egui_state.update(&wgpu_ctx.window, |ctx| {
                        let access = UiAccess {
                            vp_texture: &wgpu_ctx.framebuffer_resolve,
                            camera: &wgpu_ctx.camera,
                            vp_rect: &mut self.viewport_rect,
                            vp_dragged: &mut self.viewport_dragged,
                        };
                        self.ui_state.ui(ctx, access);

                        wgpu_ctx.viewport_size = wgpu::Extent3d {
                            width: self.viewport_rect.width() as u32,
                            height: self.viewport_rect.height() as u32,
                            depth_or_array_layers: 1,
                        }
                    });

                    match wgpu_ctx.render() {
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

                    if prev_viewport_size != wgpu_ctx.viewport_size {
                        wgpu_ctx.resize_viewport();
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let pos = Vec2::new(position.x as f32, position.y as f32);

                    let prev_pos = if let Some(prev_pos) = self.mouse_position {
                        prev_pos
                    } else {
                        self.mouse_position = Some(pos);
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
                        self.viewport_drag_offset = (pos.x - new_x, pos.y - new_y).into();
                        self.mouse_position = Some((new_x, new_y).into());
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
                        self.viewport_drag_offset = Vec2::ZERO;
                        self.mouse_delta = pos - prev_pos;
                        self.mouse_position = Some(pos);
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

struct RenderConfig {
    msaa: u32,
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

    camera: Camera,
    camera_controller: CameraController,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    render_config: RenderConfig,

    prev_frame_time: std::time::Instant,
    window_size: winit::dpi::PhysicalSize<u32>,
    // drop last
    window: Arc<winit::window::Window>,
}

impl WgpuContext {
    fn new(winit_window: winit::window::Window) -> Self {
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

        let camera = Camera {
            position: (0., 0., 3.).into(),
            fovy_rad: 90f32.to_radians(),
            yaw_rad: -90f32.to_radians(),
            pitch_rad: -20f32.to_radians(),
            aspect: config.width as f32 / config.height as f32,
            znear: 0.001,
            zfar: 10000.0,
        };

        let camera_controller = CameraController::new(4., 0.4);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Camera buffer".into(),
            contents: bytemuck::cast_slice(&[camera.view_proj_mat()]),
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

        let render_config = RenderConfig { msaa: 4 };

        let mut egui_state = egui_state::EguiState::new(&device, config.format, None, 1, &window);

        //let framebuffer = build_framebuffer(viewport_size, config.format, &device, &mut egui_state);
        //let framebuffer_resolve =
        //    build_framebuffer_resolve(viewport_size, config.format, &device, &mut egui_state);
        //let depthbuffer = build_depthbuffer(viewport_size, config.format, &device);

        let framebuffer = gpu::TextureConfig::d2(viewport_size, config.format)
            .msaa_samples(render_config.msaa)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        let framebuffer_resolve = gpu::TextureConfig::d2(viewport_size, config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut egui_state)
            .build(&device);

        let depthbuffer = gpu::TextureConfig::depthf32(viewport_size)
            .msaa_samples(render_config.msaa)
            .as_render_attachment()
            .as_texture_binding()
            .build(&device);

        let render_pipeline =
            gpu::PipelineConfig::color_depth(framebuffer.format(), depthbuffer.format())
                .msaa_samples(framebuffer.msaa_samples())
                .cull_mode(wgpu::Face::Back)
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

        let prev_frame_time = std::time::Instant::now();

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
            camera,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            render_config,
            prev_frame_time,
            window_size,
            window,
        }
    }

    fn resize_viewport(&mut self) {
        self.framebuffer = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .msaa_samples(self.render_config.msaa)
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.framebuffer_resolve = gpu::TextureConfig::d2(self.viewport_size, self.config.format)
            .as_render_attachment()
            .as_texture_binding()
            .use_with_egui(&mut self.egui_state)
            .build(&self.device);

        self.depthbuffer = gpu::TextureConfig::depthf32(self.viewport_size)
            .msaa_samples(self.render_config.msaa)
            .as_render_attachment()
            .as_texture_binding()
            .build(&self.device);

        self.camera
            .set_aspect(self.viewport_size.width, self.viewport_size.height);
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

    fn update(&mut self) {
        let prev_time = self.prev_frame_time;
        self.prev_frame_time = std::time::Instant::now();

        let delta = self.prev_frame_time - prev_time;
        self.camera_controller
            .update_camera(&mut self.camera, delta);

        self.update_camera(&self.camera);
    }

    fn update_camera(&self, camera: &Camera) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.view_proj_mat()]),
        );
    }

    fn input(&mut self, event: &WindowEvent) {
        self.egui_state.handle_input(&self.window, event);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.render_scene()?;
        self.render_ui()
    }

    fn render_scene(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: "Viewport Encoder".into(),
            });

        gpu::RenderPass::with_color_depth(&self.framebuffer, &self.depthbuffer)
            .label("main renderpass")
            .clear_rgb(0.1, 0.2, 0.3)
            .resolve_target(&self.framebuffer_resolve)
            .render_pipeline(&self.render_pipeline)
            .bind_group(&self.camera_bind_group)
            .vertex_buffer(self.vertex_buffer.slice(..))
            .index_buffer(
                self.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
                0..INDICES.len() as u32,
            )
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
