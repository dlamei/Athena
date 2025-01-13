use glam::{Mat4, Vec3, Vec4};
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub yaw_rad: f32,
    pub pitch_rad: f32,
    pub aspect: f32,
    pub fovy_rad: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn view_mat(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch_rad.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw_rad.sin_cos();

        Mat4::look_to_rh(
            self.position,
            Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vec3::Z,
        )
    }

    pub fn proj_mat(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy_rad, self.aspect, self.znear, self.zfar)
    }

    pub fn view_proj_mat(&self) -> Mat4 {
        let mat = self.proj_mat() * self.view_mat();
        let res = mat * Vec4::new(1., 1., 1., 1.);
        mat
    }

    pub fn set_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 0.5,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        // Convert duration to seconds
        let dt = dt.as_secs_f32();

        // Calculate yaw and pitch rotation
        let yaw_change = self.rotate_horizontal * self.sensitivity * dt;
        let pitch_change = self.rotate_vertical * self.sensitivity * dt;

        camera.yaw_rad += yaw_change;
        camera.pitch_rad = (camera.pitch_rad + pitch_change)
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2); // Clamp pitch to avoid flipping

        // Reset rotation values to prevent accumulation
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Calculate forward and right vectors
        let (sin_pitch, cos_pitch) = camera.pitch_rad.sin_cos();
        let (sin_yaw, cos_yaw) = camera.yaw_rad.sin_cos();

        // Forward vector for movement (ignore pitch for horizontal movement)
        let forward_vec = Vec3::new(cos_yaw, sin_pitch, -sin_yaw);

        // Forward vector for movement in horizontal plane only
        let forward_vec_flat = Vec3::new(cos_yaw, 0.0, -sin_yaw).normalize();

        // Right vector (perpendicular to forward and up)
        let right_vec = Vec3::new(sin_yaw, 0.0, cos_yaw).normalize();

        // Up vector is simply Z
        let up_vec = Vec3::Z;

        // Compute movement based on input
        let forward = (self.amount_forward - self.amount_backward) * self.speed * dt;
        let right = (self.amount_right - self.amount_left) * self.speed * dt;
        let up = (self.amount_up - self.amount_down) * self.speed * dt;

        // Update camera position
        camera.position += forward * forward_vec_flat; // Use flat forward vector for horizontal movement
        camera.position += right * right_vec;
        camera.position += up * up_vec;

        // Optionally handle zooming using the scroll input (adjust the field of view)
        camera.fovy_rad = (camera.fovy_rad + self.scroll * self.sensitivity * dt)
            .clamp(0.1, std::f32::consts::FRAC_PI_2); // Clamp FOV between ~5.7 and 90 degrees
        self.scroll = 0.0; // Reset scroll input
    }
}
