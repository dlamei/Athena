use egui_probe::EguiProbe;
use glam::{DMat3, DVec2, DVec3, Vec3};

use crate::{
    graph_3d_shader::Vertex,
    iso::{self, JitFunction},
    vm,
};

fn implicit_fn(x: f64, y: f64) -> f64 {
    x.sin() * y.sin()
}

pub struct Bounds {
    pub min: DVec3,
    pub max: DVec3,
}

pub fn subdivide_octree(f: &JitFunction, cfg: &Iso3DConfig) -> Vec<Bounds> {
    let mut stack = Vec::new();
    let mut leaves = Vec::new();
    stack.push((cfg.min, cfg.max, 0));

    while let Some((bmin, bmax, depth)) = stack.pop() {
        let (fmin, fmax) = f.intrvl_3d(bmin, bmax);
        if fmin > 0.0 || fmax < 0.0 {
            continue;
        }

        let size = (bmax - bmin).abs();
        let max_extent = size.x.max(size.y).max(size.z);
        if depth >= cfg.max_depth || max_extent <= cfg.min_size {
            leaves.push(Bounds {
                min: bmin,
                max: bmax,
            });
        } else {
            let mid = (bmin + bmax) * 0.5;
            for &ix in &[bmin.x, mid.x] {
                for &iy in &[bmin.y, mid.y] {
                    for &iz in &[bmin.z, mid.z] {
                        let child_min = DVec3::new(ix, iy, iz);
                        let child_max = DVec3::new(
                            if ix == bmin.x { mid.x } else { bmax.x },
                            if iy == bmin.y { mid.y } else { bmax.y },
                            if iz == bmin.z { mid.z } else { bmax.z },
                        );
                        stack.push((child_min, child_max, depth + 1));
                    }
                }
            }
        }
    }

    leaves
}

#[derive(Debug, Clone, Copy, PartialEq, egui_probe::EguiProbe)]
pub enum Program3D {
    Sphere,
    Plane,
    Waves,
}

impl Program3D {
    pub fn opcode(&self) -> Vec<vm::Opcode> {
        use vm::op;
        match self {
            Self::Sphere => [
                op::MUL_REG_REG(1, 1, 1),
                op::MUL_REG_REG(2, 2, 2),
                op::MUL_REG_REG(3, 3, 3),
                op::ADD_REG_REG(1, 2, 1),
                op::ADD_REG_REG(1, 3, 1),
                op::SUB_REG_IMM(1, 1.0, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program3D::Plane => [
                op::SUB_REG_REG(1, 2, 1),
                op::SUB_REG_REG(1, 3, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program3D::Waves => [
                op::SIN(1, 1),
                op::SIN(2, 2),
                op::MUL_REG_REG(1, 2, 1),
                op::SUB_REG_REG(1, 3, 1),
                op::EXT(0),
            ]
            .to_vec(),
        }
    }

    pub fn bytecode(&self) -> Vec<compiler::jit::Instr> {
        match self {
            Self::Sphere => compiler::bytecode![
                MUL[0, 0] -> 0,
                MUL[1, 1] -> 1,
                MUL[2, 2] -> 2,
                ADD[1, 0] -> 0,
                ADD[2, 0] -> 0,
                SUB[0, imm(1.0)] -> 0,
            ]
            .to_vec(),
            Program3D::Sphere => vec![],
            Program3D::Plane => vec![],
            Program3D::Waves => vec![],
        }
    }
}

pub fn debug_cubes(bounds: &[Bounds]) -> Vec<Vertex> {
    let mut verts = Vec::new();
    for b in bounds {
        let min = b.min;
        let max = b.max;
        let c = [
            Vec3::new(min.x as f32, min.y as f32, min.z as f32), // 0
            Vec3::new(max.x as f32, min.y as f32, min.z as f32), // 1
            Vec3::new(max.x as f32, max.y as f32, min.z as f32), // 2
            Vec3::new(min.x as f32, max.y as f32, min.z as f32), // 3
            Vec3::new(min.x as f32, min.y as f32, max.z as f32), // 4
            Vec3::new(max.x as f32, min.y as f32, max.z as f32), // 5
            Vec3::new(max.x as f32, max.y as f32, max.z as f32), // 6
            Vec3::new(min.x as f32, max.y as f32, max.z as f32), // 7
        ];
        let idx = [
            0, 1, 2, 2, 3, 0, // bottom
            4, 7, 6, 6, 5, 4, // top
            0, 4, 5, 5, 1, 0, // front
            3, 2, 6, 6, 7, 3, // back
            0, 3, 7, 7, 4, 0, // left
            1, 5, 6, 6, 2, 1, // right
        ];
        for &i in &idx {
            verts.push(Vertex { pos: c[i] });
        }
    }
    verts
}

pub fn normalize_vertices(verts: &mut [Vertex]) {
    if verts.is_empty() {
        return;
    }

    // compute axis-aligned bounds
    let mut vmin = verts[0].pos;
    let mut vmax = verts[0].pos;
    for v in verts.iter().skip(1) {
        vmin = vmin.min(v.pos);
        vmax = vmax.max(v.pos);
    }

    let center = (vmin + vmax) * 0.5;
    let half_extents = (vmax - vmin) * 0.5;
    let max_half = half_extents.max_element().max(1e-6);

    let scale = 1.0 / max_half;

    for v in verts {
        v.pos = (v.pos - center) * scale;
    }
}

#[derive(Debug, Clone, PartialEq, EguiProbe)]
pub struct Iso3DConfig {
    #[egui_probe(skip)]
    pub min: DVec3,
    #[egui_probe(skip)]
    pub max: DVec3,
    pub max_depth: u32,
    #[egui_probe(skip)]
    pub min_size: f64,

    #[egui_probe(with crate::ui::f64_drag(0.1))]
    pub flat_tol: f64,
    // pub grad_thresh: f64,
    pub program: Program3D,
    pub rotate_every: u32,
}

impl Default for Iso3DConfig {
    fn default() -> Self {
        Self {
            min: DVec3::new(-1.0, -1.0, -1.0),
            max: DVec3::new(1.0, 1.0, 1.0),
            min_size: 0.0001,
            flat_tol: 0.1,
            // grad_thresh: 0.1,
            rotate_every: 1,
            max_depth: 4,
            program: crate::iso_3d::Program3D::Sphere,
        }
    }
}

pub fn build(config: &Iso3DConfig) -> Vec<Vertex> {
    let f = JitFunction::new_3d(config.program);
    // let bounds: Vec<_> = subdivide_oriented_octree2(&f, &config).into_iter().filter(|b| b.depth >= config.render_depth).collect();
    // let bounds: Vec<_> = subdivide_adaptive_cuboids(&f, &config);
    let bounds: Vec<_> = subdivide_octree(&f, &config);

    let mut verts = debug_cubes(&bounds);

    normalize_vertices(&mut verts);

    // println!("n_verts: {}", verts.len());
    // println!("verts: {:?}", verts);
    verts
    // Vertex::cube(config.min.as_vec3(), config.max.as_vec3())
}
