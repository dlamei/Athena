use glam::{Mat3, Mat4, Vec3, Vec4};
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

pub trait Camera {
    fn view_mat(&self) -> Mat4;
    fn proj_mat(&self) -> Mat4;

    fn view_proj_mat(&self) -> Mat4 {
        self.proj_mat() * self.view_mat()
    }

    fn set_aspect(&mut self, width: u32, height: u32) {}
    fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        false
    }
    fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {}
    fn process_scroll(&mut self, delta: &MouseScrollDelta) {}
    fn time_step(&mut self, dt: Duration) {}
}

#[derive(Debug, Clone)]
pub struct OribtCamera {
    pub target: Vec3,
    pub fov_rad: f32,
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,

    radius: f32,
    yaw: f32,
    pitch: f32,
    local_basis: Mat3,

    d_pitch: f32,
    d_yaw: f32,
    d_zoom: f32,
}

#[inline]
fn vec3_from_pitch_and_yaw(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let dir = Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);
    dir
}

#[inline]
fn compute_local_basis(pitch: f32, yaw: f32) -> Mat3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();

    let right = Vec3::new(sin_yaw, -cos_yaw, 0.0).normalize();
    let forward = Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);
    let up = forward.cross(right).normalize();

    -Mat3::from_cols(right, forward, up)
}

impl OribtCamera {
    pub fn look_at(eye: Vec3, target: Vec3, fov_rad: f32) -> Self {
        let dir = eye - target;
        let radius = dir.length();
        let yaw = dir.x.atan2(dir.z);
        let pitch = (dir.y / radius).asin();

        Self {
            target,
            fov_rad,
            aspect: 16.0 / 9.0,
            z_near: 0.001,
            z_far: 100.0,

            radius,
            yaw,
            pitch,
            local_basis: compute_local_basis(pitch, yaw),

            d_pitch: 0.0,
            d_yaw: 0.0,
            d_zoom: 0.0,
        }
    }

    #[inline]
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
}

impl Camera for OribtCamera {
    fn view_mat(&self) -> Mat4 {
        Mat4::look_at_lh(self.eye(), self.target(), self.up())
    }

    fn proj_mat(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov_rad, self.aspect, self.z_near, self.z_far)
    }

    fn set_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.d_yaw = mouse_dx;
        self.d_pitch = mouse_dy;
    }

    fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.d_zoom = match delta {
                MouseScrollDelta::LineDelta(_, scroll) => -scroll,
                MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
            };
    }

    fn time_step(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();

        self.yaw += self.d_yaw * dt;
        self.pitch += self.d_pitch * dt;
        self.radius += self.d_zoom;

        self.radius = self.radius.max(0.0);

        self.local_basis = compute_local_basis(self.pitch, self.yaw);

        self.d_pitch = 0.0;
        self.d_yaw = 0.0;
        self.d_zoom = 0.0;
    }
}
