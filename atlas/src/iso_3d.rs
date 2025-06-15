use glam::{DVec2, DVec3, Vec3};

use crate::graph_3d_shader::Vertex;

fn implicit_fn(x: f64, y: f64) -> f64 {
    x.sin() * y.sin()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Iso3DConfig {
    pub min: DVec3,
    pub max: DVec3,
}

pub fn build(config: Iso3DConfig) -> Vec<Vertex> {
    Vertex::cube(config.min.as_vec3(), config.max.as_vec3())
}
