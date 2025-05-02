use std::cell::OnceCell;

use compiler::bytecode;
use compiler::jit;
use glam::DVec3;
use glam::{DVec2, Vec3};
use rayon::prelude::*;

use crate::{iso::ImplicitFn, iso2};

mod bits {
    #[inline]
    pub const fn part1by1(mut x: u32) -> u64 {
        x &= 0x1fffffff; // mask to 29 bits
        let mut x = x as u64;
        x = (x | (x << 16)) & 0x0000FFFF0000FFFF;
        x = (x | (x << 8)) & 0x00FF00FF00FF00FF;
        x = (x | (x << 4)) & 0x0F0F0F0F0F0F0F0F;
        x = (x | (x << 2)) & 0x3333333333333333;
        x = (x | (x << 1)) & 0x5555555555555555;
        x
    }

    #[inline]
    pub const fn compact1by1(mut x: u64) -> u32 {
        x &= 0x5555555555555555;
        x = (x ^ (x >> 1)) & 0x3333333333333333;
        x = (x ^ (x >> 2)) & 0x0F0F0F0F0F0F0F0F;
        x = (x ^ (x >> 4)) & 0x00FF00FF00FF00FF;
        x = (x ^ (x >> 8)) & 0x0000FFFF0000FFFF;
        x = (x ^ (x >> 16)) & 0x00000000FFFFFFFF;
        x as u32
    }

    #[inline]
    pub const fn interleave(x: u32, y: u32) -> u64 {
        part1by1(x) | (part1by1(y) << 1)
    }

    pub const fn unravel(code: u64) -> (u32, u32) {
        let x = compact1by1(code);
        let y = compact1by1(code >> 1);
        (x, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct Morton(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct MortonCorner(u64);

impl MortonCorner {
    #[inline]
    fn from_xy(x: u32, y: u32) -> Self {
        Self(bits::interleave(x, y))
    }

    #[inline]
    fn to_xy(self) -> (u32, u32) {
        bits::unravel(self.0)
    }

    fn position(self, depth: u8, glob_min: DVec2, glob_max: DVec2) -> DVec2 {
        let (x, y) = self.to_xy();
        let size = glob_max - glob_min;
        let quad_size = size / 2f64.powi(depth as i32 + 1);
        DVec2::new(x as f64, y as f64) * quad_size + glob_min
    }
}

impl Morton {
    const UNDEF: Self = Self(u64::MAX);

    #[inline]
    fn from_xy(x: u32, y: u32) -> Self {
        Self(bits::interleave(x, y))
    }

    #[inline]
    fn to_xy(self) -> (u32, u32) {
        bits::unravel(self.0)
    }

    #[inline]
    fn interval(self, depth: u8, glob_min: DVec2, glob_max: DVec2) -> (DVec2, DVec2) {
        let (x, y) = self.to_xy();
        let size = glob_max - glob_min;

        let quad_pos = DVec2::new(x as f64, y as f64);
        let quad_size = size / 2f64.powi(depth as i32);
        let quad_min = quad_pos * quad_size + glob_min;
        let quad_max = quad_min + quad_size;

        (quad_min, quad_max)
    }

    fn subdivide(self) -> [Self; 4] {
        let (p_x, p_y) = self.to_xy();
        let (b_x, b_y) = (p_x << 1, p_y << 1);

        [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| {
            let c_x = b_x + dx;
            let c_y = b_y + dy;
            Self(bits::interleave(c_x, c_y))
        })
    }

    fn neighbors(self) -> [Self; 4] {
        let (x, y) = self.to_xy();

        [(0, 1), (1, 0), (0, -1), (-1, 0)].map(|(dx, dy)| {
            if dx < 0 && x == 0 {
                Self::UNDEF
            } else if dy < 0 && y == 0 {
                Self::UNDEF
            } else {
                Self::from_xy((x as i32 + dx) as u32, (y as i32 + dy) as u32)
            }
        })
    }

    fn corners(self) -> [MortonCorner; 4] {
        let (x, y) = self.to_xy();

        [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| {
            let c_x = x + dx;
            let c_y = y + dy;
            MortonCorner::from_xy(c_x, c_y)
        })
    }
}

#[inline]
pub fn zero_cross(p0: DVec2, v0: f64, p1: DVec2, v1: f64) -> DVec2 {
    let denom = v0 - v1;
    let k0 = -v1 / denom;
    let k1 = v0 / denom;
    k0 * p0 + k1 * p1
}

pub fn extract_iso_2(
    config: &iso2::Iso2DConfig,
    f: &mut ImplicitFn,
    jit_fn: Implicit2DFn,
) -> Vec<[Vec3; 2]> {
    let depth = config.intrvl_depth + 1;
    // The grid has 2^depth points per side.
    let size = 1 << depth;
    let num_corners = size * size;

    // Compute corner positions in row-major order.
    // The coordinate for (i, j) is interpolated between config.min and config.max.
    let mut corner_pos = Vec::with_capacity(num_corners);
    for i in 0..size {
        for j in 0..size {
            // Linear interpolation: when i or j is 0 we get config.min and when i or j is size-1 we get config.max.
            let t = DVec2::new(i as f64, j as f64) / DVec2::splat((size - 1) as f64);
            let pos = config.min + t * (config.max - config.min);
            corner_pos.push(pos);
        }
    }

    // Evaluate the implicit function at each corner.
    // We extend each 2D point to 3D by adding a zero, matching the input expected by f.
    let input: Vec<_> = corner_pos.iter().map(|p| p.extend(0.0)).collect();

    let start = std::time::Instant::now();
    let corner_eval = if !config.jit {
        f.eval_f64x4_vec(input)
    } else if config.simd {
        input
            .par_chunks_exact(2)
            .flat_map(|c| {
                let x = [c[0].x, c[1].x];
                let y = [c[0].y, c[1].y];
                let mut o = [0.0; 2];
                jit_fn.1(&mut o, x, y);
                o
            })
            .collect()
    } else {
        input.into_par_iter().map(|v| jit_fn.0(v.x, v.y)).collect()
    };
    let end = std::time::Instant::now();
    println!("{} for {}", (end - start).as_millis(), corner_eval.len());

    // Each cell (of which there are (size-1) x (size-1)) will have a dual value if any of its edges cross zero.
    // We'll store duals in a 2D array (flattened in row-major order) for the cells.
    let cell_count = (size - 1) * (size - 1);
    let mut cell_duals = vec![DVec2::NAN; cell_count];

    // For each cell, compute the averaged zero-crossing location (dual) if any edge shows a sign change.
    for i in 0..(size - 1) {
        for j in 0..(size - 1) {
            // Compute indices into corner_pos and corner_eval.
            // Corners are arranged as:
            // top-left: (i, j), top-right: (i, j+1),
            // bottom-left: (i+1, j), bottom-right: (i+1, j+1)
            let idx_tl = i * size + j;
            let idx_tr = idx_tl + 1;
            let idx_bl = (i + 1) * size + j;
            let idx_br = idx_bl + 1;

            // Gather the four corner values.
            let v_tl = corner_eval[idx_tl];
            let v_tr = corner_eval[idx_tr];
            let v_bl = corner_eval[idx_bl];
            let v_br = corner_eval[idx_br];

            // Gather the four positions.
            let p_tl = corner_pos[idx_tl];
            let p_tr = corner_pos[idx_tr];
            let p_bl = corner_pos[idx_bl];
            let p_br = corner_pos[idx_br];

            // Check each of the four edges for a sign change.
            // We consider the following edges:
            // top: p_tl -> p_tr, right: p_tr -> p_br,
            // bottom: p_br -> p_bl, left: p_bl -> p_tl.
            let edges = [
                ((p_tl, v_tl), (p_tr, v_tr)),
                ((p_tr, v_tr), (p_br, v_br)),
                ((p_br, v_br), (p_bl, v_bl)),
                ((p_bl, v_bl), (p_tl, v_tl)),
            ];

            let mut n_duals = 0;
            let mut sum_dual = DVec2::ZERO;
            for &((p0, v0), (p1, v1)) in edges.iter() {
                // If there is a sign change along this edge, compute the zero crossing.
                if v0.signum() != v1.signum() {
                    let dual = zero_cross(p0, v0, p1, v1);
                    sum_dual += dual;
                    n_duals += 1;
                }
            }

            // Store the averaged dual in the cell if any intersections were found.
            if n_duals > 0 {
                let avg_dual = sum_dual / n_duals as f64;
                cell_duals[i * (size - 1) + j] = avg_dual;
            }
        }
    }

    // Now build segments between neighboring duals.
    // To ensure each segment is only added once, we connect each cell's dual to its left and upper neighbor,
    // if those neighbors exist and have a valid (non-NaN) dual.
    let mut segments = Vec::new();
    for i in 0..(size - 1) {
        for j in 0..(size - 1) {
            let idx = i * (size - 1) + j;
            let curr = cell_duals[idx];
            if curr.is_nan() {
                continue;
            }
            // Connect to the left neighbor (same row, previous column).
            if j > 0 {
                let left = cell_duals[i * (size - 1) + (j - 1)];
                if !left.is_nan() {
                    segments.push((curr, left));
                }
            }
            // Connect to the upper neighbor (previous row, same column).
            if i > 0 {
                let up = cell_duals[(i - 1) * (size - 1) + j];
                if !up.is_nan() {
                    segments.push((curr, up));
                }
            }
        }
    }

    let res = segments
        .into_iter()
        .map(|(v0, v1)| [v0.as_vec2().extend(0.0), v1.as_vec2().extend(0.0)])
        .collect();

    res
}

type Implicit2DFn1 = extern "C" fn(f64, f64) -> f64;
type Implicit2DFn2 = extern "C" fn(*mut [f64; 2], [f64; 2], [f64; 2]) -> ();

type Implicit2DFn = (Implicit2DFn1, Implicit2DFn2);

fn tmp_jit_fn(ctx: &mut jit::JITCompiler, config: &iso2::Iso2DConfig) -> Implicit2DFn {
    let program = bytecode! [
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
    ];

    let program2 = bytecode! [
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
    ];

    let config = compiler::jit::CompConfig::default();
    let f1 = ctx.compile_for_f64("impl", &program, &config);
    let f2 = ctx.compile_for_f64x2("impl_simd", &program, &config);
    (f1.fn_ptr, f2.fn_ptr)
}

pub(crate) fn build_2d(config: iso2::Iso2DConfig) -> (Vec<[Vec3; 3]>, Vec<[Vec3; 2]>) {
    let mut f = ImplicitFn::new(config.program.opcode());

    let mut ctx = jit::JITCompiler::init();
    //let jit_fn = tmp_jit_fn(&mut ctx);

    let cell = OnceCell::new();
    let jit_fn = *cell.get_or_init(|| tmp_jit_fn(&mut ctx, &config));

    (vec![], extract_iso_2(&config, &mut f, jit_fn))
}

pub mod bench {
    use super::*;

    pub fn extract_iso_line(config: iso2::Iso2DConfig) {
        let f = ImplicitFn::new(config.program.opcode());

        let mut ctx = jit::JITCompiler::init();
        let cell = OnceCell::new();
        let jit_fn = *cell.get_or_init(|| tmp_jit_fn(&mut ctx, &config));

        let mut f = ImplicitFn::new(config.program.opcode());
        extract_iso_2(&config, &mut f, jit_fn);
    }
}
