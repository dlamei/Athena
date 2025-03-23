use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};
use std::f32::consts::FRAC_PI_2;
use std::time::{Duration, Instant};
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub fov_rad: f32,
    pub aspect: f32,
    pub vp_height: f32,
    pub vp_width: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub anim_len: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            fov_rad: 90f32.to_radians(),
            aspect: 1f32,
            vp_height: 1f32,
            vp_width: 1f32,
            z_near: -1f32,
            z_far: 1f32,
            anim_len: 0.5f32,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CameraMode {
    Orbit3D(Orbit3D), Pan2D(Pan2D),
}

#[derive(Debug, Clone)]
pub struct Pan2D {
    pos: Vec2,
    zoom: f32,
    d_pos: Vec2,
    d_zoom: f32,
}

impl Pan2D {
    pub fn new(pos: Vec2, scale: f32) -> Self {
        Self {
            pos, zoom: scale, d_pos: Vec2::ZERO, d_zoom: 0.0
        }
    }

    pub fn view_mat(&self) -> Mat4 {
        Mat4::from_translation(Vec3::new(-self.pos.x, -self.pos.y, 0.0))
    }

    pub fn get_bounds(&self) -> (Vec2, Vec2) {
        todo!()
    }

    pub fn proj_mat(&self, config: &CameraConfig) -> Mat4 {
        let half_w = self.zoom * config.aspect;
        let half_h = self.zoom;

        Mat4::orthographic_lh(
            -half_w,
            half_w,
            -half_h,
            half_h,
            -1.0,
            1.0,
        )
    }

    pub fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.d_pos += Vec2::new(-mouse_dx, mouse_dy)
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.d_zoom += match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        }
    }

    pub fn time_step(&mut self, dt: Duration, config: &CameraConfig) {
        let mut d_world_pos_x = self.d_pos.x * (2.0 * self.zoom) / config.vp_height;
        let mut d_world_pos_y = self.d_pos.y * (2.0 * self.zoom) / config.vp_height;

        if !d_world_pos_x.is_normal() {
            d_world_pos_x = 0.0;
        }
        if !d_world_pos_y.is_normal() {
            d_world_pos_y = 0.0;
        }


        self.pos.x += d_world_pos_x;
        self.pos.y += d_world_pos_y;

        self.zoom *= 1.0 + self.d_zoom * 0.1;
        self.zoom = self.zoom.max(0.0001);

        // let dt = dt.as_secs_f32();
        // let mut pan_sensitivity = (2.0 * self.scale) / config.vp_height * 100.0;
        // if !pan_sensitivity.is_normal() {
        //     pan_sensitivity = 1.0;
        // }
        // self.pos += self.d_pos * self.scale * dt * pan_sensitivity;
        // println!("{pan_sensitivity}: {}", self.pos);
        // self.scale *= 1.0 + self.d_zoom * dt;
        // self.scale = self.scale.max(0.001);
        self.d_pos = Vec2::ZERO;
        self.d_zoom = 0.0;
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct Orbit3D {
    target: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
    local_basis: Mat3,
    d_pitch: f32,
    d_yaw: f32,
    d_zoom: f32,
}

impl Orbit3D {
    pub fn new(eye: Vec3, target: Vec3) -> Self {
        let dir = eye - target;
        let radius = dir.length();
        let yaw = dir.x.atan2(dir.z);
        let pitch = (dir.y / radius).asin();

        Self {
            target,
            radius,
            yaw,
            pitch,
            local_basis: compute_local_basis(pitch, yaw),
            d_pitch: 0.0,
            d_yaw: 0.0,
            d_zoom: 0.0,
        }
    }

    pub const fn target(&self) -> Vec3 {
        self.target
    }
    #[inline]
    pub fn eye(&self) -> Vec3 {
        self.radius * vec3_from_pitch_and_yaw(self.pitch, self.yaw)
    }

    #[inline]
    pub fn look_dir(&self) -> Vec3 {
        self.target() - self.eye()
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        self.local_basis.y_axis
    }
    #[inline]
    pub fn right(&self) -> Vec3 {
        self.local_basis.x_axis
    }
    #[inline]
    pub fn up(&self) -> Vec3 {
        self.local_basis.z_axis
    }

    #[inline]
    pub fn view_mat(&self) -> Mat4 {
        Mat4::look_at_lh(self.eye(), self.target(), self.up())
    }

    #[inline]
    pub fn proj_mat(&self, config: &CameraConfig) -> Mat4 {
        Mat4::perspective_lh(config.fov_rad, config.aspect, config.z_near, config.z_far)
    }


    #[inline]
    pub fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.d_yaw = mouse_dx;
        self.d_pitch = mouse_dy;
    }

    #[inline]
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.d_zoom = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        };
    }

    fn time_step(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();

        let upside = if self.up().dot(Vec3::Z) > 0.0 {
            1.0
        } else {
            -1.0
        };

        self.yaw += upside * self.d_yaw * dt;
        self.pitch += self.d_pitch * dt;
        self.radius += self.d_zoom * self.radius / 10.0;

        self.radius = self.radius.max(0.0);

        self.local_basis = compute_local_basis(self.pitch, self.yaw);

        self.d_pitch = 0.0;
        self.d_yaw = 0.0;
        self.d_zoom = 0.0;
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub config: CameraConfig,
    pub mode: CameraMode,

    transition_start: Option<(Instant, Mat4)>,
}

fn vec3_from_pitch_and_yaw(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let dir = Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);
    dir
}

fn compute_local_basis(pitch: f32, yaw: f32) -> Mat3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();

    let right = Vec3::new(sin_yaw, -cos_yaw, 0.0).normalize();
    let forward = Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);
    let up = forward.cross(right).normalize();

    -Mat3::from_cols(right, forward, up)
}

impl Camera {

    pub fn pan_2d(pos: Vec2, scale: f32) -> Self {
        Self {
            config: CameraConfig {
                z_near: 0.001,
                z_far: 1000.0,
                ..CameraConfig::default()
            },
            mode: CameraMode::Pan2D(Pan2D::new(pos, scale)),
            transition_start: None,
        }
    }

    pub fn orbit_3d(eye: Vec3, target: Vec3, fov_rad: f32) -> Self {
        Self {
            config: CameraConfig {
                z_near: 0.001,
                z_far: 1000.0,
                fov_rad,
                ..CameraConfig::default()
            },
            mode: CameraMode::Orbit3D(Orbit3D::new(eye, target)),
            transition_start: None,
        }
    }

    pub fn switch_mode(&mut self, mode: CameraMode) {
        self.transition_start = Some((Instant::now(), self.view_proj_mat()));
        self.mode = mode;
    }

    #[inline]
    pub fn view_mat(&self) -> Mat4 {
        match &self.mode {
            CameraMode::Orbit3D(c) => c.view_mat(),
            CameraMode::Pan2D(c) => c.view_mat(),
        }
    }

    #[inline]
    pub fn proj_mat(&self) -> Mat4 {
        match &self.mode {
            CameraMode::Orbit3D(c) => c.proj_mat(&self.config),
            CameraMode::Pan2D(c) => c.proj_mat(&self.config),
        }
    }

    #[inline]
    pub fn set_aspect(&mut self, width: u32, height: u32) {
        self.config.vp_height = height as f32;
        self.config.vp_width = width as f32;
        self.config.aspect = width as f32 / height as f32;
    }

    #[inline]
    pub fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {
        match &mut self.mode {
            CameraMode::Orbit3D(c) => c.process_mouse(mouse_dx, mouse_dy),
            CameraMode::Pan2D(c) => c.process_mouse(mouse_dx, mouse_dy),
        }
    }

    #[inline]
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        match &mut self.mode {
            CameraMode::Orbit3D(c) => c.process_scroll(delta),
            CameraMode::Pan2D(c) => c.process_scroll(delta),
        }
    }

    pub fn time_step(&mut self, dt: Duration) {
        match &mut self.mode {
            CameraMode::Orbit3D(c) => c.time_step(dt),
            CameraMode::Pan2D(c) => c.time_step(dt, &self.config),
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        match &mut self.mode {
            CameraMode::Orbit3D(c) => c.process_keyboard(key, state),
            CameraMode::Pan2D(c) => c.process_keyboard(key, state),
        }
    }

    pub fn view_proj_mat(&mut self) -> Mat4 {
        let a = self.proj_mat() * self.view_mat();

        if let Some((start_time, b)) = self.transition_start {
            let curr_time = Instant::now();
            let elapsed = curr_time.duration_since(start_time).as_secs_f32();
            if elapsed >= self.config.anim_len {
                self.transition_start = None;
                a
            } else {
                b + (a-b) * elapsed / self.config.anim_len
            }
        } else {
            a
        }

    }
}
