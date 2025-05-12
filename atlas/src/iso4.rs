use std::{cell::OnceCell, fmt};

#[cfg(feature="native-codegen")]
use compiler::{bytecode, jit};

use egui_probe::EguiProbe;
use glam::{DVec2, DVec3, DVec4, I64Vec2, Vec3, Vec4};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

// use utils::BitGrid;
type BitGrid = utils::BitGrid;

use crate::{ vm, LineSegmentInst, Vertex };

#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub struct Iso2DConfig {
    #[egui_probe(with crate::ui::dvec2_probe)]
    pub min: DVec2,

    #[egui_probe(with crate::ui::dvec2_probe)]
    pub max: DVec2,

    pub intrvl_depth: u32,
    pub subdiv_depth: u32,

    #[egui_probe(with crate::ui::f32_drag(0.00001))]
    pub line_thickness: f32,

    pub program: Program,
    pub debug: bool,

    #[cfg_attr(not(target_arch = "wasm32"), egui_probe(skip))]
    pub simd: bool,
}

impl Default for Iso2DConfig {
    fn default() -> Self {
        Self {
            min: DVec2::ZERO,
            max: DVec2::ZERO,
            intrvl_depth: 0,
            subdiv_depth: 0,
            line_thickness: 1.,
            program: Program::Dense3,
            simd: false,
            debug: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub enum Program {
    #[egui_probe(name = "sin(x)-y=0")]
    Sin,
    #[egui_probe(name = "cos(x)-y=0")]
    Cos,
    #[egui_probe(name = "tan(x)-y=0")]
    Tan,
    #[egui_probe(name = "(1/x)-y=0")]
    OneDivX,
    #[egui_probe(name = "sin(1/x)-y=0")]
    Sin1DivX,
    #[egui_probe(name = "cos(1/x)-y=0")]
    Cos1DivX,
    #[egui_probe(name = "3^(sin(x)+cos(x))-sin(3^(x+y))=0")]
    Dense1,
    #[egui_probe(name = "sin(sin(1/x)+cos(1/y))-cos(sin(1/(x*y))+cos(1/x))=0")]
    Dense2,
    #[egui_probe(name = "sin(sin(1/x)+sin(1/y))-sin(sin(1/(x*y))+sin(1/x))=0")]
    Dense3,
}

impl Program {
    pub(crate) fn opcode(&self) -> Vec<vm::Opcode> {
        use vm::op;
        match self {
            Program::OneDivX => [
                op::DIV_IMM_REG(1.0, 1, 1),
                op::SUB_REG_REG(1, 2, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program::Sin => [op::SIN(1, 1), op::SUB_REG_REG(2, 1, 1), op::EXT(0)].to_vec(),
            Program::Cos => [op::COS(1, 1), op::SUB_REG_REG(2, 1, 1), op::EXT(0)].to_vec(),
            Program::Tan => [op::TAN(1, 1), op::SUB_REG_REG(2, 1, 1), op::EXT(0)].to_vec(),
            Program::Sin1DivX => [
                op::DIV_IMM_REG(1.0, 1, 1),
                op::SIN(1, 1),
                op::SUB_REG_REG(1, 2, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program::Cos1DivX => [
                op::DIV_IMM_REG(1.0, 1, 1),
                op::COS(1, 1),
                op::SUB_REG_REG(1, 2, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program::Dense1 => [
                op::ADD_REG_REG(1, 2, 3),
                op::POW_IMM_REG(3.0, 3, 3),
                op::SIN(3, 3),
                op::SIN(1, 1),
                op::SIN(2, 2),
                op::ADD_REG_REG(1, 2, 1),
                op::POW_IMM_REG(3.0, 1, 1),
                op::SUB_REG_REG(1, 3, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program::Dense2 => {
                [
                    // 1 / x -> 1
                    op::DIV_IMM_REG(1.0, 1, 1),
                    // 1 / y -> 2
                    op::DIV_IMM_REG(1.0, 2, 2),
                    // sin(1/x) -> 3
                    op::SIN(1, 3),
                    // cos(1/y) -> 4
                    op::COS(2, 4),
                    // sin(1/x) + cos(1/y) -> 3
                    op::ADD_REG_REG(3, 4, 3),
                    // sin(sin(1/x) + cos(1/y)) -> 3
                    op::SIN(3, 3),
                    // 1/x * 1/y -> 5
                    op::MUL_REG_REG(1, 2, 5),
                    // sin(1/x * 1/y) -> 5
                    op::SIN(5, 5),
                    // cos(1/x) -> 6
                    op::COS(1, 6),
                    // sin(1/x * 1/y) + cos(1/x) -> 5
                    op::ADD_REG_REG(5, 6, 5),
                    // cos(sin(1/x * 1/y) + cos(1/x)) -> 5
                    op::COS(5, 5),
                    // sin(sin(1/x) + cos(1/y)) - cos(sin(1/x * 1/y) + cos(1/x)) -> 1
                    op::SUB_REG_REG(3, 5, 1),
                    op::EXT(0),
                ]
                .into()
            }
            Program::Dense3 => {
                [
                    // 1 / x -> 1
                    op::DIV_IMM_REG(1.0, 1, 1),
                    // 1 / y -> 2
                    op::DIV_IMM_REG(1.0, 2, 2),
                    // sin(1/x) -> 3
                    op::SIN(1, 3),
                    // cos(1/y) -> 4
                    op::SIN(2, 4),
                    // sin(1/x) + cos(1/y) -> 3
                    op::ADD_REG_REG(3, 4, 3),
                    // sin(sin(1/x) + cos(1/y)) -> 3
                    op::SIN(3, 3),
                    // 1/x * 1/y -> 5
                    op::MUL_REG_REG(1, 2, 5),
                    // sin(1/x * 1/y) -> 5
                    op::SIN(5, 5),
                    // cos(1/x) -> 6
                    op::SIN(1, 6),
                    // sin(1/x * 1/y) + cos(1/x) -> 5
                    op::ADD_REG_REG(5, 6, 5),
                    // cos(sin(1/x * 1/y) + cos(1/x)) -> 5
                    op::SIN(5, 5),
                    // sin(sin(1/x) + cos(1/y)) - cos(sin(1/x * 1/y) + cos(1/x)) -> 1
                    op::SUB_REG_REG(3, 5, 1),
                    op::EXT(0),
                ]
                .into()
            }
        }
    }

    #[cfg(feature="native-codegen")]
    pub fn bytecode(&self) -> Vec<compiler::jit::Instr> {
        match self {
            Program::OneDivX => compiler::bytecode! [
                DIV[imm(1), 0] -> 0,
                SUB[0, 1] -> 0,
            ]
            .to_vec(),
            Program::Sin => compiler::bytecode! [
                SIN[0] -> 0,
                SUB[0, 1] -> 0,
            ]
            .to_vec(),
            Program::Cos => compiler::bytecode! [
                COS[0] -> 0,
                SUB[0, 1] -> 0,
            ]
            .to_vec(),
            Program::Tan => compiler::bytecode! [
                TAN[0] -> 0,
                SUB[0, 1] -> 0,
            ]
            .to_vec(),
            Program::Sin1DivX => compiler::bytecode! [
                DIV[imm(1), 0] -> 0,
                SIN[0] -> 0,
                SUB[0, 1] -> 0,
            ]
            .to_vec(),
            Program::Cos1DivX => compiler::bytecode! [
                DIV[imm(1), 0] -> 0,
                COS[0] -> 0,
                SUB[0, 1] -> 0,
            ]
            .to_vec(),
            Program::Dense1 => compiler::bytecode! [
                ADD[0, 1] -> 2,
                POW[imm(3), 2] -> 2,
                SIN[0] -> 0,
                SIN[1] -> 1,
                SIN[2] -> 2,
                ADD[0, 1] -> 0,
                POW[imm(3), 0] -> 0,
                SUB[0, 2] -> 0,
            ]
            .to_vec(),
            Program::Dense2 => compiler::bytecode! [
                DIV[imm(1.0), 0] -> 0,
                DIV[imm(1.0), 1] -> 1,
                SIN[0] -> 2,
                COS[1] -> 3,
                ADD[2, 3] -> 2,
                SIN[2] -> 2,
                MUL[0, 1] -> 4,
                SIN[4] -> 4,
                COS[0] -> 5,
                ADD[4, 5] -> 4,
                COS[4] -> 4,
                SUB[2, 4] -> 0,
            ]
            .to_vec(),
            Program::Dense3 => compiler::bytecode! [
                DIV[imm(1.0), 0] -> 0,
                DIV[imm(1.0), 1] -> 1,
                SIN[0] -> 2,
                SIN[1] -> 3,
                ADD[2, 3] -> 2,
                SIN[2] -> 2,
                MUL[0, 1] -> 4,
                SIN[4] -> 4,
                SIN[0] -> 5,
                ADD[4, 5] -> 4,
                SIN[4] -> 4,
                SUB[2, 4] -> 0,
            ]
            .to_vec(),
        }
    }
}

struct JitFunction {
    #[cfg(feature="native-codegen")]
    native_f64_to_f64: extern "C" fn(f64, f64) -> f64,
    #[cfg(feature="native-codegen")]
    native_f64x2_to_f64x2: extern "C" fn(*mut [f64; 2], [f64; 2], [f64; 2]),
    #[cfg(feature="native-codegen")]
    native_f64x8_to_f64x8: extern "C" fn(*const [f64; 8], *const [f64; 8], *mut [f64; 8]),

    op_codes: Vec<vm::Opcode>,
}

impl JitFunction {
    fn new(program: Program) -> Self {
        #[cfg(feature="native-codegen")]
        {
            let jit_config = jit::CompConfig::default();
            let mut jit = jit::JITCompiler::init();
        }

        Self {
        #[cfg(feature="native-codegen")]
        native_f64_to_f64: jit.compile_for_f64("jit_fn", &program.bytecode(), &jit_config).fn_ptr,
        #[cfg(feature="native-codegen")]
        native_f64x2_to_f64x2: jit.compile_for_f64x2("jit_fn2", &program.bytecode(), &jit_config).fn_ptr,
        #[cfg(feature="native-codegen")]
        native_f64x8_to_f64x8: jit.compile_for_f64x2x4("jit_fn4", &[], &jit_config).fn_ptr,
        op_codes: program.opcode(),
        }
    }

    fn f64_to_f64(&self, a: f64, b: f64) -> f64 {
        let mut out = 0.0;

        #[cfg(feature="native-codegen")]
        {
            out = (self.native_f64_to_f64)(a, b)
        }
        #[cfg(not(feature="native-codegen"))]
        {
            let mut vm = vm::VM::with_instr_table(vm::F64InstrTable);
            out = vm.call([a, b, 0.0], &self.op_codes);
        }

        out
    }

    fn f64x2_to_f64x2(&self, a: [f64;2], b: [f64;2]) -> [f64;2] {
        let mut out = [0.0;2];
        #[cfg(feature="native-codegen")]
        {
            (self.native_f64x2_to_f64x2)(&mut out, a, b);
        } 
        #[cfg(not(feature="native-codegen"))]
        {
            let mut vm = vm::VM::with_instr_table(vm::F64InstrTable);
            out[0] = vm.call([a[0], b[0], 0.0], &self.op_codes);
            out[1] = vm.call([a[1], b[1], 0.0], &self.op_codes);
        }
        out
    }

    fn f64x8_to_f64x8(&self, a: [f64;8], b: [f64;8]) -> [f64;8] {
        let mut out = [0.0;8];
         
        #[cfg(feature="native-codegen")]
        {
            (self.native_f64x8_to_f64x8)(&a, &b, &mut out);
        }
        #[cfg(not(feature="native-codegen"))]
        {
            let mut vm = vm::VM::with_instr_table(vm::F64InstrTable);
            for i in 0..8 {
                out[i] = vm.call([a[i], b[i], 0.0], &self.op_codes);
            }
        }

        out
    }

    fn intrvl(&self, min: DVec2, max: DVec2) -> vm::Range {
        let min = min.extend(0.0);
        let max = max.extend(0.0);
        let mut vm = vm::VM::with_instr_table(vm::RangeInstrTable);
        for i in 0..3 {
            vm.reg[i + 1] = (min[i], max[i]).into();
        }

        vm.eval(&self.op_codes);
        vm.reg[1]
    }
}

// type JITParam = [f64; 2];
// type Impl2DFuncf64x2 = extern "C" fn(*mut [f64; 2], JITParam, JITParam);
type Impl2DFunc = (
    extern "C" fn(f64, f64) -> f64,
    extern "C" fn(*mut [f64; 2], [f64; 2], [f64; 2]),
    extern "C" fn(*const [f64; 8], *const [f64; 8], *mut [f64; 8]),
);

fn build_grid(config: &Iso2DConfig, f: &JitFunction) -> BitGrid {
    let res = 2u32.pow(config.intrvl_depth);

    let min = config.min;
    let max = config.max;
    let size = max - min;

    let mut grid = BitGrid::new(res as u32, res as u32);

    for j in 0..res {
        let y0 = (j as f64 / res as f64) * size.y + min.y;
        let y1 = ((j + 1) as f64 / res as f64) * size.y + min.y;
        for i in 0..res {
            let x0 = (i as f64 / res as f64) * size.x + min.x;
            let x1 = ((i + 1) as f64 / res as f64) * size.x + min.x;

            let q_min = DVec2::new(x0, y0);
            let q_max = DVec2::new(x1, y1);

            let intrvl = f.intrvl(q_min, q_max);
            if intrvl.contains_zero() || intrvl.is_empty() {
                grid.set(i, j);
            }
        }
    }

    grid
}

// atan(0.5)
const SAMPLE_ANGLE: f64 = 0.4636476090008061;
const SIN_SAMPLE_ANGLE: f64 = 0.4472135954999579;
const COS_SAMPLE_ANGLE: f64 = 0.8944271909999159;
const SQRT_5_FRAC_2: f64 = 1.118033988749895;
const FRAC_2_SQRT_5: f64 = COS_SAMPLE_ANGLE;

#[inline]
fn sample_transpose(p: DVec2) -> DVec2 {
    (p * COS_SAMPLE_ANGLE + DVec2::new(-p.y, p.x) * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2
}

#[inline]
fn inv_sample_transpose(p: DVec2) -> DVec2 {
    (p * COS_SAMPLE_ANGLE + DVec2::new(-p.y, p.x) * -SIN_SAMPLE_ANGLE) * FRAC_2_SQRT_5
}

#[inline]
fn hash_u64(i: u32, j: u32, seed: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    i.hash(&mut hasher);
    j.hash(&mut hasher);
    seed.hash(&mut hasher);
    hasher.finish()
}

#[inline]
fn to_unit_f64(h: u64) -> f64 {
    (h >> 1) as f64 / ((1u64 << 63) as f64)
}

// fn subdiv_sample_grid(
//     grid: &BitGrid,
//     config: &Iso2DConfig,
//     f: &JitFunction,
// ) -> (Vec<Vertex>, Vec<(DVec2, DVec2)>) {
//     let cell_depth = config.intrvl_depth as u32;
//     let subdiv_depth = config.subdiv_depth as u32;
//     let cell_res = 1 << cell_depth;
//     let sub_res = 1 << subdiv_depth;
//     let full_res = cell_res * sub_res;

//     let size = config.max - config.min;
//     let inv_full = 1.0 / (full_res as f64);
//     let cell_size = size / cell_res as f64;
//     let subdiv_size = size / (full_res as f64);

//     let mut verts = Vec::new();
//     let mut segments = Vec::new();

//     const MAX_SUB_DEPTH: usize = 7;
//     assert!(MAX_SUB_DEPTH >= subdiv_depth as usize);
//     let mut prev_row = [0.0f64; { 1 << MAX_SUB_DEPTH + 1 }];
//     let mut curr_row = [0.0f64; { 1 << MAX_SUB_DEPTH + 1 }];

//     #[inline]
//     fn jitter(i: u32, j: u32, alpha: f64) -> DVec2 {
//         let h1 = to_unit_f64(hash_u64(i, j, 0));
//         let h2 = to_unit_f64(hash_u64(i, j, 1));
//         let dx = (h1 - 0.5) * alpha;
//         let dy = (h2 - 0.5) * alpha;
//         DVec2::new(dx, dy)
//     }

//     let alpha = config.grad_tol;

//     fn rotate(x: DVec2, origin: DVec2) -> DVec2 {
//         x
//         // let a = 0.5f64.atan();
//         // let v = x - origin;
//         // let (sin_a, cos_a) = a.sin_cos();
//         // DVec2::new(v.x*cos_a - v.y*sin_a, v.y*cos_a + v.x*sin_a) + origin
//     }

//     for (cx, cy) in grid.iter() {
//         let w_cell_min = config.min + DVec2::new(cx as f64, cy as f64) * cell_size;
//         let s_cell_min = DVec2::new(cx as f64, cy as f64) * sub_res as f64 * inv_full - 0.5;

//         // sample first row
//         for i in 0..=sub_res as usize {
//             let glob_i = cx * sub_res + i as u32;
//             let glob_j = cy * sub_res;
//             let mut pt = DVec2::new(i as f64, 0.0) * subdiv_size + w_cell_min;
//             let o = pt + subdiv_size / 2.0;
//             pt = rotate(pt, o);
//             pt += jitter(glob_i, glob_j, alpha) * size * inv_full;
//             curr_row[i] = f.f64_to_f64(pt.x, pt.y);
//         }

//         for j in 1..=sub_res as usize {
//             std::mem::swap(&mut curr_row, &mut prev_row);

//             for i in 0..=sub_res as usize {
//                 let glob_i = cx * sub_res + i as u32;
//                 let glob_j = cy * sub_res + j as u32 - 1;
//                 let mut pt = DVec2::new(i as f64, j as f64) * size * inv_full + w_cell_min;
//                 let o = pt + subdiv_size / 2.0;
//                 pt = rotate(pt, o);
//                 pt += jitter(glob_i, glob_j, alpha) * size * inv_full;
//                 curr_row[i] = f.f64_to_f64(pt.x, pt.y);
//             }

//             for i in 1..=sub_res as usize {
//                 // screen pos
//                 let p_max = s_cell_min + DVec2::new(i as f64, j as f64) * inv_full;
//                 let p_min = p_max - inv_full;

//                 let glob_i = cx * sub_res + i as u32 - 1;
//                 let glob_j = cy * sub_res + j as u32 - 1;

//                 let p00 = p_min + jitter(glob_i, glob_j, alpha) * inv_full;
//                 let p10 = p_min.with_x(p_max.x) + jitter(glob_i + 1, glob_j, alpha) * inv_full;
//                 let p11 = p_max + jitter(glob_i + 1, glob_j + 1, alpha) * inv_full;
//                 let p01 = p_min.with_y(p_max.y) + jitter(glob_i, glob_j + 1, alpha) * inv_full;

//                 let o = (p00 + p11) / 2.0;
//                 let p00 = rotate(p00, o);
//                 let p10 = rotate(p10, o);
//                 let p11 = rotate(p11, o);
//                 let p01 = rotate(p01, o);

//                 if config.debug {
//                     let [r, g, b] = [0, 1, 2].map(|s| to_unit_f64(hash_u64(cx, cy, s)) as f32);
//                     verts.extend(dbg_rect(p_min, p_max, (r, g, b).into()));
//                 }

//                 // corner positions
//                 let screen_pts = [p00, p10, p11, p01];

//                 // corner values
//                 let values = [prev_row[i - 1], prev_row[i], curr_row[i], curr_row[i - 1]];

//                 let mut zero_pts = [DVec2::ZERO; 4];
//                 let mut count = 0;

//                 for k in 0..4 {
//                     let n_k = (k + 1) & 3;
//                     let (v1, v2) = (values[k], values[n_k]);
//                     let (p1, p2) = (screen_pts[k], screen_pts[n_k]);
//                     if v1.signum() != v2.signum() && !v1.is_nan() {
//                         let t = v1 / (v1 - v2);
//                         zero_pts[count] = p1.lerp(p2, t);
//                         count += 1;
//                     }
//                 }

//                 if count == 2 {
//                     segments.push((zero_pts[0], zero_pts[1]));
//                 } else if count > 2 {
//                     let mut avg = DVec2::ZERO;
//                     for p in &zero_pts[..count] {
//                         avg += *p;
//                     }
//                     avg /= count as f64;
//                     for &p in &zero_pts[..count] {
//                         segments.push((p, avg));
//                     }
//                 }
//             }
//         }
//     }

//     (verts, segments)
// }

fn dbg_rect_sample(min: DVec2, max: DVec2, c: Vec3) -> [Vertex; 6] {
    let col = c.extend(1.0);
    let s_pts = [min, min.with_x(max.x), max, min.with_y(max.y)]
        .map(|p| sample_transpose(p).as_vec2().extend(0.0).extend(0.0) - 0.5);

    [
        Vertex { pos: s_pts[0], col },
        Vertex { pos: s_pts[1], col },
        Vertex { pos: s_pts[2], col },
        Vertex { pos: s_pts[0], col },
        Vertex { pos: s_pts[2], col },
        Vertex { pos: s_pts[3], col },
    ]
}

fn dbg_rect(min: DVec2, max: DVec2, c: Vec3) -> [Vertex; 6] {
    let col = c.extend(1.0);
    let s_pts = [min, min.with_x(max.x), max, min.with_y(max.y)]
        .map(|p| p.as_vec2().extend(0.0).extend(0.0));

    [
        Vertex { pos: s_pts[0], col },
        Vertex { pos: s_pts[1], col },
        Vertex { pos: s_pts[2], col },
        Vertex { pos: s_pts[0], col },
        Vertex { pos: s_pts[2], col },
        Vertex { pos: s_pts[3], col },
    ]
}

fn aabb_overlap(min1: DVec2, max1: DVec2, min2: DVec2, max2: DVec2) -> bool {
    !(max1.x < min2.x || min1.x > max2.x || max1.y < min2.y || min1.y > max2.y)
}

// Edge indices (0–3 correspond to the four edges of the cell):
//  0: bottom  (between corner 0 and 1)
//  1: right   (between corner 1 and 2)
//  2: top     (between corner 2 and 3)
//  3: left    (between corner 3 and 0)
const EDGE_LOOKUP: [[(usize, usize); 2]; 16] = [
    // for each of the 16 case indices, the pairs of edges to connect
    // { (e1, e2), (e3, e4) } – unused slots are set to (0,0)
    [(0, 0), (0, 0)], // 0000: no crossings
    [(3, 0), (0, 0)], // 0001
    [(0, 1), (0, 0)], // 0010
    [(3, 1), (0, 0)], // 0011
    [(1, 2), (0, 0)], // 0100
    [(3, 2), (1, 0)], // 0101 (ambiguous saddle)
    [(0, 2), (0, 0)], // 0110
    [(3, 2), (0, 0)], // 0111
    [(2, 3), (0, 0)], // 1000
    [(0, 2), (0, 0)], // 1001
    [(1, 3), (0, 1)], // 1010 (ambiguous saddle)
    [(1, 2), (0, 0)], // 1011
    [(1, 3), (0, 0)], // 1100
    [(0, 1), (0, 0)], // 1101
    [(0, 3), (0, 0)], // 1110
    [(0, 0), (0, 0)], // 1111: fully inside
];

fn subdiv_sample_grid_rot_par(
    grid: &BitGrid,
    config: &Iso2DConfig,
    f: &JitFunction,
) -> (Vec<Vertex>, Vec<(DVec2, DVec2)>) {
    let mut verts = Vec::new();
    // let mut segments = Vec::new();

    let cell_depth = config.intrvl_depth as u32;
    let sub_depth = config.subdiv_depth as u32;

    let cell_res = 1 << cell_depth;
    let sub_res = 1 << sub_depth;
    let full_res = cell_res * sub_res;

    let cell_res_inv = 1.0 / cell_res as f64;
    let full_res_inv = 1.0 / full_res as f64;

    let size = config.max - config.min;

    const MAX_SUB_DEPTH: usize = 7;
    assert!(MAX_SUB_DEPTH >= sub_depth as usize);

    let segments: Vec<_> = grid
        .iter()
        .par_bridge()
        .flat_map(|(cx, cy)| {
            let mut segments = Vec::new();
            let cell_bound_min = DVec2::new(cx as f64, cy as f64) * cell_res_inv - 0.5;
            let cell_bound_max = cell_bound_min + cell_res_inv;

            let mut prev_row = [0.0f64; { 1 << MAX_SUB_DEPTH + 1 }];
            let mut curr_row = [0.0f64; { 1 << MAX_SUB_DEPTH + 1 }];

            let [r, g, b] = [0, 1, 2].map(|s| to_unit_f64(hash_u64(cx, cy, s)) as f32);
            let sample_col = Vec3::new(r, g, b);

            // map the four corners back into rotated-grid units
            let s_rot_corners = [(cx, cy), (cx + 1, cy), (cx + 1, cy + 1), (cx, cy + 1)]
                .map(|c| inv_sample_transpose(DVec2::new(c.0 as f64, c.1 as f64)));

            let mut s_rot_min = DVec2::INFINITY;
            let mut s_rot_max = DVec2::NEG_INFINITY;
            for corner in s_rot_corners {
                s_rot_min = s_rot_min.min(corner);
                s_rot_max = s_rot_max.max(corner);
            }

            let min_indx = (s_rot_min * sub_res as f64).floor().as_i64vec2();

            let max_indx = if cx + 1 < grid.width && grid.get(cx + 1, cy)
                || cy + 1 < grid.height && grid.get(cx, cy + 1)
            {
                (s_rot_max * sub_res as f64).ceil().as_i64vec2()
            } else {
                (s_rot_max * sub_res as f64).floor().as_i64vec2()
            };

            // sample first row
            for i in min_indx.x..=max_indx.x {
                let f_idx = I64Vec2::new(i, min_indx.y).as_dvec2();
                let s_sub_min = f_idx * full_res_inv;
                // let s_sub_max = s_sub_min + full_res_inv;

                let sample_pt = sample_transpose(f_idx * full_res_inv) * size + config.min;
                curr_row[(i - min_indx.x) as usize] = f.f64_to_f64(sample_pt.x, sample_pt.y);
            }

            // skip first row
            for j in min_indx.y + 1..=max_indx.y {
                std::mem::swap(&mut prev_row, &mut curr_row);

                // sample current row

                for i in (min_indx.x..=max_indx.x).step_by(2) {
                    let l = (i - min_indx.x) as usize;

                    //(p * COS_SAMPLE_ANGLE + DVec2::new(-p.y, p.x) * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2
                    let x0 = i as f64 * full_res_inv;
                    let x1 = x0 + full_res_inv;
                    let y0 = j as f64 * full_res_inv;

                    let rx0 =
                        (x0 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx1 =
                        (x1 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let ry0 =
                        (y0 * COS_SAMPLE_ANGLE + x0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry1 =
                        (y0 * COS_SAMPLE_ANGLE + x1 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;

                    let out = f.f64x2_to_f64x2([rx0, rx1], [ry0, ry1]);
                    curr_row[l] = out[0];
                    curr_row[l + 1] = out[1];
                    // curr_row[l+1] = f.1(sample_pt.x, sample_pt.y);
                }
                for i in min_indx.x + 1..=max_indx.x {
                    let l = (i - min_indx.x) as usize;
                    let p_max = DVec2::new(i as f64, j as f64) * full_res_inv;
                    let p_min = p_max - full_res_inv;

                    let screen_pts = [p_min, p_min.with_x(p_max.x), p_max, p_min.with_y(p_max.y)]
                        .map(|p| sample_transpose(p) - 0.5);

                    let values =
                        [prev_row[l - 1], prev_row[l], curr_row[l], curr_row[l - 1]]
                            .map(|v| if v.is_nan() { f64::MIN } else { v });

                    let mut ms_code = 0;
                    for (k, &v) in values.iter().enumerate() {
                        if v > 0.0 {
                            ms_code |= 1 << k;
                        }
                    }

                    if ms_code == 5 || ms_code == 10 {
                        let avg = values.into_iter().sum::<f64>() * 0.25;
                        if avg > 0.0 {
                            ms_code = 15 - ms_code;
                        }
                    }

                    let mut edge_duals = [DVec2::ZERO; 4];
                    for edge in 0..4 {
                        let i0 = edge;
                        let i1 = (edge + 1) & 3;
                        let v0 = values[i0];
                        let v1 = values[i1];

                        // if (v0 <= 0.0 && v1 > 0.0) || (v0 > 0.0 && v1 <= 0.0)
                        if v0.is_finite() && v1.is_finite() && v0 * v1 < 0.0 {
                            let t = v0 / (v0 - v1);
                            edge_duals[edge] = screen_pts[i0].lerp(screen_pts[i1], t);
                        }
                    }

                    for (e1, e2) in EDGE_LOOKUP[ms_code] {
                        if e1 == e2 {
                            continue;
                        };
                        let (p1, p2) = (edge_duals[e1], edge_duals[e2]);
                        segments.push((p1, p2));
                    }
                }
            }

            // if config.debug {
            //     let c_min = DVec2::new(cx as f64, cy as f64) * cell_res_inv - 0.5;
            //     let c_max = c_min + cell_res_inv;
            //     verts.extend(dbg_rect(c_min, c_max, Vec3::new(1.0, 1.0, 1.0)));
            // }
            segments
        })
        .collect();

    (verts, segments)
}

fn subdiv_sample_grid_rot(
    grid: &BitGrid,
    config: &Iso2DConfig,
    f: &JitFunction,
) -> (Vec<Vertex>, Vec<(DVec2, DVec2)>) {
    let mut verts = Vec::new();
    let mut segments = Vec::new();

    let cell_depth = config.intrvl_depth as u32;
    let sub_depth = config.subdiv_depth as u32;

    let cell_res = 1 << cell_depth;
    let sub_res = 1 << sub_depth;
    let full_res = cell_res * sub_res;

    let cell_res_inv = 1.0 / cell_res as f64;
    let full_res_inv = 1.0 / full_res as f64;

    let size = config.max - config.min;

    const MAX_SUB_DEPTH: usize = 7;
    assert!(MAX_SUB_DEPTH >= sub_depth as usize);
    let mut prev_row = [0.0f64; { 1 << MAX_SUB_DEPTH + 1 }];
    let mut curr_row = [0.0f64; { 1 << MAX_SUB_DEPTH + 1 }];

    for (cx, cy) in grid.iter() {
        let cell_bound_min = DVec2::new(cx as f64, cy as f64) * cell_res_inv - 0.5;
        let cell_bound_max = cell_bound_min + cell_res_inv;

        let [r, g, b] = [0, 1, 2].map(|s| to_unit_f64(hash_u64(cx, cy, s)) as f32);
        let sample_col = Vec3::new(r, g, b);

        // map the four corners back into rotated-grid units
        let s_rot_corners = [(cx, cy), (cx + 1, cy), (cx + 1, cy + 1), (cx, cy + 1)]
            .map(|c| inv_sample_transpose(DVec2::new(c.0 as f64, c.1 as f64)));

        let mut s_rot_min = DVec2::INFINITY;
        let mut s_rot_max = DVec2::NEG_INFINITY;
        for corner in s_rot_corners {
            s_rot_min = s_rot_min.min(corner);
            s_rot_max = s_rot_max.max(corner);
        }

        let min_indx = (s_rot_min * sub_res as f64).floor().as_i64vec2();

        let max_indx = if cx + 1 < grid.width && grid.get(cx + 1, cy)
            || cy + 1 < grid.height && grid.get(cx, cy + 1)
        {
            (s_rot_max * sub_res as f64).ceil().as_i64vec2()
        } else {
            (s_rot_max * sub_res as f64).floor().as_i64vec2()
        };

        // sample first row
        for i in min_indx.x..=max_indx.x {
            let f_idx = I64Vec2::new(i, min_indx.y).as_dvec2();
            let s_sub_min = f_idx * full_res_inv;
            // let s_sub_max = s_sub_min + full_res_inv;

            if config.debug {
                if i != max_indx.x {
                    let max = s_sub_min + full_res_inv / 1.0;
                    verts.extend(dbg_rect_sample(s_sub_min, max, sample_col));
                }
            }

            let sample_pt = sample_transpose(f_idx * full_res_inv) * size + config.min;
            curr_row[(i - min_indx.x) as usize] = f.f64_to_f64(sample_pt.x, sample_pt.y);
        }

        // skip first row
        for j in min_indx.y + 1..=max_indx.y {
            std::mem::swap(&mut prev_row, &mut curr_row);

            // sample current row
            if config.simd {
                for i in (min_indx.x..=max_indx.x).step_by(8) {
                    let l = (i - min_indx.x) as usize;

                    //(p * COS_SAMPLE_ANGLE + DVec2::new(-p.y, p.x) * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2
                    let x0 = i as f64 * full_res_inv;
                    let x1 = x0 + full_res_inv;
                    let x2 = x1 + full_res_inv;
                    let x3 = x2 + full_res_inv;
                    let x4 = x3 + full_res_inv;
                    let x5 = x4 + full_res_inv;
                    let x6 = x5 + full_res_inv;
                    let x7 = x6 + full_res_inv;
                    let y0 = j as f64 * full_res_inv;

                    let rx0 =
                        (x0 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx1 =
                        (x1 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx2 =
                        (x2 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx3 =
                        (x3 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx4 =
                        (x4 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx5 =
                        (x5 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx6 =
                        (x6 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx7 =
                        (x7 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;

                    let ry0 =
                        (y0 * COS_SAMPLE_ANGLE + x0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry1 =
                        (y0 * COS_SAMPLE_ANGLE + x1 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry2 =
                        (y0 * COS_SAMPLE_ANGLE + x2 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry3 =
                        (y0 * COS_SAMPLE_ANGLE + x3 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry4 =
                        (y0 * COS_SAMPLE_ANGLE + x4 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry5 =
                        (y0 * COS_SAMPLE_ANGLE + x5 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry6 =
                        (y0 * COS_SAMPLE_ANGLE + x6 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry7 =
                        (y0 * COS_SAMPLE_ANGLE + x7 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;

                    // let mut out = [0.0; 8];
                    // f.1(&mut out, [rx0, rx1], [ry0, ry1]);
                    let out = f.f64x8_to_f64x8(
                        [rx0, rx1, rx2, rx3, rx4, rx5, rx6, rx7],
                        [ry0, ry1, ry2, ry3, ry4, ry5, ry6, ry7],
                    );
                    curr_row[l] = out[0];
                    curr_row[l + 1] = out[1];
                    curr_row[l + 2] = out[2];
                    curr_row[l + 3] = out[3];
                    curr_row[l + 4] = out[4];
                    curr_row[l + 5] = out[5];
                    curr_row[l + 6] = out[6];
                    curr_row[l + 7] = out[7];
                    // curr_row[l+1] = f.1(sample_pt.x, sample_pt.y);
                }
            } else {
                for i in (min_indx.x..=max_indx.x).step_by(2) {
                    let l = (i - min_indx.x) as usize;

                    //(p * COS_SAMPLE_ANGLE + DVec2::new(-p.y, p.x) * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2
                    let x0 = i as f64 * full_res_inv;
                    let x1 = x0 + full_res_inv;
                    let y0 = j as f64 * full_res_inv;

                    let rx0 =
                        (x0 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let rx1 =
                        (x1 * COS_SAMPLE_ANGLE - y0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.x
                            + config.min.x;
                    let ry0 =
                        (y0 * COS_SAMPLE_ANGLE + x0 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;
                    let ry1 =
                        (y0 * COS_SAMPLE_ANGLE + x1 * SIN_SAMPLE_ANGLE) * SQRT_5_FRAC_2 * size.y
                            + config.min.y;

                    let out = f.f64x2_to_f64x2([rx0, rx1], [ry0, ry1]);
                    curr_row[l] = out[0];
                    curr_row[l + 1] = out[1];
                    // curr_row[l+1] = f.1(sample_pt.x, sample_pt.y);
                }
            }
            // else {
            //     for i in min_indx.x..=max_indx.x {
            //         let l = (i - min_indx.x) as usize;

            //         let idx_min = DVec2::new(i as f64, j as f64);
            //         let sample_pt = sample_transpose(idx_min * full_res_inv) * size + config.min;

            //         curr_row[l] = f.0(sample_pt.x, sample_pt.y);

            //         if config.debug {
            //             if j != max_indx.y && i != max_indx.x {
            //                 let s_sub_min = idx_min * full_res_inv;
            //                 let s_sub_max = s_sub_min + full_res_inv / 1.0;
            //                 verts.extend(dbg_rect_sample(s_sub_min, s_sub_max, sample_col));
            //             }
            //         }
            //     }
            // }

            for i in min_indx.x + 1..=max_indx.x {
                let l = (i - min_indx.x) as usize;
                let p_max = DVec2::new(i as f64, j as f64) * full_res_inv;
                let p_min = p_max - full_res_inv;

                let screen_pts = [p_min, p_min.with_x(p_max.x), p_max, p_min.with_y(p_max.y)]
                    .map(|p| sample_transpose(p) - 0.5);

                let values =
                    [prev_row[l - 1], prev_row[l], curr_row[l], curr_row[l - 1]]
                        .map(|v| if v.is_nan() { f64::MIN } else { v });

                let mut ms_code = 0;
                for (k, &v) in values.iter().enumerate() {
                    if v > 0.0 {
                        ms_code |= 1 << k;
                    }
                }

                if ms_code == 5 || ms_code == 10 {
                    let avg = values.into_iter().sum::<f64>() * 0.25;
                    if avg > 0.0 {
                        ms_code = 15 - ms_code;
                    }
                }

                let mut edge_duals = [DVec2::ZERO; 4];
                for edge in 0..4 {
                    let i0 = edge;
                    let i1 = (edge + 1) & 3;
                    let v0 = values[i0];
                    let v1 = values[i1];

                    // if (v0 <= 0.0 && v1 > 0.0) || (v0 > 0.0 && v1 <= 0.0)
                    if v0.is_finite() && v1.is_finite() && v0 * v1 < 0.0 {
                        let t = v0 / (v0 - v1);
                        edge_duals[edge] = screen_pts[i0].lerp(screen_pts[i1], t);
                    }
                }

                for (e1, e2) in EDGE_LOOKUP[ms_code] {
                    if e1 == e2 {
                        continue;
                    };
                    let (p1, p2) = (edge_duals[e1], edge_duals[e2]);
                    segments.push((p1, p2));
                }
            }
        }

        // if config.debug {
        //     let c_min = DVec2::new(cx as f64, cy as f64) * cell_res_inv - 0.5;
        //     let c_max = c_min + cell_res_inv;
        //     verts.extend(dbg_rect(c_min, c_max, Vec3::new(1.0, 1.0, 1.0)));
        // }
    }

    (verts, segments)
}

pub(crate) fn build_2d(config: Iso2DConfig) -> (Vec<Vertex>, Vec<LineSegmentInst>) {
    if config.max.is_nan() || config.max.is_nan() {
        return (vec![], vec![]);
    }

    let f = JitFunction::new(config.program);
    let grid = build_grid(&config, &f);
    // let (verts, segments) = if config.simd {
    //     subdiv_sample_grid(&grid, &config, jit_f)
    // } else {
    // };
    let (verts, segments) = subdiv_sample_grid_rot_par(&grid, &config, &f);

    let segments = segments
        .into_iter()
        .map(|(a, b)| LineSegmentInst {
            a: a.as_vec2().extend(0.0),
            b: b.as_vec2().extend(0.0),
        })
        .collect();

    (verts, segments)
}
