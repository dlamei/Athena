use std::fmt;

use crate::{
    iso::{self, ImplicitFn},
    vm::{self, op},
};
use egui_probe::EguiProbe;
use glam::{DMat2, DMat3, DVec2, DVec3, Vec3};
use rustc_hash::{FxHashMap, FxHashSet};

type Index = u32;
const NO_NEIGHBOR: u32 = u32::MAX;

#[inline]
pub fn zero_cross(p0: DVec2, v0: f64, p1: DVec2, v1: f64) -> DVec2 {
    let denom = v0 - v1;
    let k0 = -v1 / denom;
    let k1 = v0 / denom;
    k0 * p0 + k1 * p1
}

///// QefSolver accumulates plane constraints and solves for the point minimizing the quadratic error.
//pub struct QefSolver {
//    // Accumulated matrix: AᵀA, where each row of A is a plane normal.
//    ata: DMat3,
//    // Accumulated vector: Aᵀb, where bᵢ = nᵢ ⋅ pᵢ.
//    atb: DVec3,
//    // Accumulated mass point (sum of all pᵢ).
//    mass_point: DVec3,
//    // Count of added constraints.
//    num_points: u32,
//}

//impl QefSolver {
//    /// Creates a new QEF solver instance.
//    pub fn new() -> Self {
//        Self {
//            ata: DMat3::ZERO,
//            atb: DVec3::ZERO,
//            mass_point: DVec3::ZERO,
//            num_points: 0,
//        }
//    }

//    /// Adds a plane constraint defined by a point `p` on the surface and its unit normal `n`.
//    ///
//    /// The constraint is defined as: n ⋅ (x - p) = 0,
//    /// which can be rearranged to: n ⋅ x = n ⋅ p.
//    pub fn add(&mut self, p: DVec3, n: DVec3) {
//        // Ensure the normal is normalized.
//        let n = n.normalize();
//        // Compute constant: d = n ⋅ p.
//        let d = n.dot(p);
//        // Accumulate the symmetric matrix ATA += n * nᵀ.
//        self.ata += DMat3::from_cols_array(&[
//            n.x * n.x, n.x * n.y, n.x * n.z,
//            n.y * n.x, n.y * n.y, n.y * n.z,
//            n.z * n.x, n.z * n.y, n.z * n.z,
//        ]);
//        // Accumulate the vector ATb += d * n.
//        self.atb += n * d;
//        // Accumulate mass point for fallback.
//        self.mass_point += p;
//        self.num_points += 1;
//    }

//    /// Solves the QEF by minimizing the sum of squared distances from x to all planes.
//    ///
//    /// Returns the optimal position.
//    /// If no constraints were added, it returns Vec3::ZERO.
//    /// If the system is underconstrained or singular, it falls back to the mass point.
//    pub fn solve(&self) -> DVec3 {
//        if self.num_points == 0 {
//            return DVec3::ZERO;
//        }

//        // Compute the mass point (the average of all points).
//        let mass_point = self.mass_point / self.num_points as f64;

//        // Try to compute the inverse of ATA.
//        // If the matrix is near-singular, inverse() may return None.
//        println!("{}", self.ata);
//        let det = self.ata.determinant();
//        // if let Some(ata_inv) = self.ata.inverse() {
//        if det != 0.0 {
//            let ata_inv = self.ata.inverse();
//            // Optimal solution: x = (ATA)⁻¹ ATb.
//            ata_inv * self.atb
//        } else {
//            // Fallback: return the mass point if ATA is singular.
//            mass_point
//        }
//    }
//}

pub struct QefSolver2D {
    // Accumulated matrix: AᵀA (2×2)
    ata: DMat2,
    // Accumulated vector: Aᵀb (2×1)
    atb: DVec2,
    // Mass point accumulation
    mass_point: DVec2,
    // Number of constraints
    num_points: u32,
}

impl QefSolver2D {
    /// Creates a new QEF solver instance.
    pub fn new() -> Self {
        Self {
            ata: DMat2::ZERO,
            atb: DVec2::ZERO,
            mass_point: DVec2::ZERO,
            num_points: 0,
        }
    }

    /// Adds a constraint using a point `p` on the line and the unit normal `n`.
    ///
    /// The constraint is defined as `n ⋅ (x - p) = 0`.
    pub fn add(&mut self, p: DVec2, n: DVec2) {
        let n = n.normalize();
        let d = n.dot(p);

        // Accumulate AᵀA (2×2 matrix)
        self.ata += DMat2::from_cols(
            DVec2::new(n.x * n.x, n.x * n.y),
            DVec2::new(n.y * n.x, n.y * n.y),
        );

        // Accumulate Aᵀb (2×1 vector)
        self.atb += n * d;

        // Accumulate mass point
        self.mass_point += p;
        self.num_points += 1;
    }

    /// Solves the QEF by minimizing the quadratic error.
    ///
    /// Returns the optimal (x, y) position.
    pub fn solve(&self) -> DVec2 {
        if self.num_points == 0 {
            return DVec2::ZERO;
        }

        let mass_point = self.mass_point / self.num_points as f64;

        let det = self.ata.determinant();
        // if let Some(ata_inv) = self.ata.inverse()
        if det != 0.0 {
            // Solve x = (AᵀA)⁻¹ Aᵀb
            self.ata.inverse() * self.atb
        } else {
            // Fallback: Use the mass point if ATA is singular
            mass_point
        }
    }
}

// #[inline(always)]
// pub fn zero_cross(p1: SurfacePoint, p2: SurfacePoint, f: &mut ImplicitFn) -> Vec3 {
//     let denom = p1.val - p2.val;
//     let k1 = -p2.val / denom;
//     let k2 = p1.val / denom;
//     k1 as f32 * p1.pos + k2 as f32 * p2.pos
// }

mod dir {
    pub const N: usize = 0;
    pub const E: usize = 1;
    pub const S: usize = 2;
    pub const W: usize = 3;

    pub fn rev(dir: usize) -> usize {
        (dir + 2) % 4
    }
}

mod dir_diag {
    pub const SW: u32 = 0;
    pub const SE: u32 = 1;
    pub const NW: u32 = 2;
    pub const NE: u32 = 3;
}

#[inline]
fn quad_unit_bounds(mut code: u64) -> (DVec2, DVec2) {
    // let mut bounds = (Vec3::ZERO, Vec3::ONE);
    let mut min = DVec2::ZERO;
    let mut max = DVec2::ONE;

    let depth = iso::cell::depth(code);

    for i in 0..depth {
        // let oct = ((loc >> ((depth - i) * 4 & 0xF) as u8;
        let oct = ((code >> (depth - i - 1) * 4) & 0xF) - 1;

        let half_size = (max - min) / 2.0;

        if oct >> 0 & 1 == 1 {
            min.x += half_size.x
        }
        if oct >> 1 & 1 == 1 {
            min.y += half_size.y
        }
        // if oct >> 2 & 1 == 1 {
        //     min.z += half_size.z
        // }

        max = min + half_size;
    }
    (min, max)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Quad {
    code: u64,
    neighbors: [Index; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LeafQuad {
    code: u64,
    neighbors: [Index; 4],
    marked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DualQuad {
    vertex: DVec2,
    neighbors: [Index; 4],
    marked: bool,
    discard: bool,
}

impl LeafQuad {
    fn as_quad(self) -> Quad {
        Quad {
            code: self.code,
            neighbors: self.neighbors,
        }
    }

    fn as_dual(self, vertex: DVec2) -> DualQuad {
        DualQuad {
            vertex,
            neighbors: self.neighbors,
            marked: self.marked,
            discard: false,
        }
    }
}

impl Quad {
    #[inline]
    fn root() -> Self {
        Self {
            code: 0,
            neighbors: [NO_NEIGHBOR; 4],
        }
    }

    fn as_leaf(self) -> LeafQuad {
        let mut neighbors = self.neighbors;
        neighbors.sort_unstable();
        LeafQuad {
            code: self.code,
            neighbors,
            marked: false,
        }
    }

    #[inline]
    fn bounds(&self, min: DVec2, max: DVec2) -> (DVec2, DVec2) {
        let (u_min, u_max) = quad_unit_bounds(self.code);
        let size = max - min;
        (u_min * size + min, u_max * size + min)
    }

    #[inline]
    fn corners(&self, min: DVec2, max: DVec2) -> [DVec2; 4] {
        let (q_min, q_max) = self.bounds(min, max);
        let c0 = q_min;
        let c2 = q_max;
        let c1 = c0.with_x(c2.x);
        let c3 = c0.with_y(c2.y);
        [c0, c1, c2, c3]
    }

    #[inline]
    fn subdivide(&self) -> [Self; 4] {
        let c = self.code << 4;
        let neighbors = [NO_NEIGHBOR; 4];
        // let (min, max) = (self.min, self.max);
        // let mid = (min + max) / 2.0;

        // let nw = Self::init(min.with_y(mid.y), max.with_x(mid.x), c | 0);
        // let ne = Self::init(mid, max, c | 1);
        // let se = Self::init(min.with_x(mid.x), max.with_y(mid.y), c | 2);
        // let sw = Self::init(min, mid, c | 3);
        let nw = Self {
            code: c | 3,
            neighbors,
        };
        let ne = Self {
            code: c | 4,
            neighbors,
        };
        let se = Self {
            code: c | 2,
            neighbors,
        };
        let sw = Self {
            code: c | 1,
            neighbors,
        };

        [sw, se, nw, ne]
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct TreeGraph {
    quads: Vec<LeafQuad>,
    min: DVec2,
    max: DVec2,
}

// impl fmt::Display for TreeGraph {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let mut cell_indx = 0;
//         for q in &self.quads {
//             write!(f, "{cell_indx} ({}): ({}, {}) -> [ ", q.depth, q.min, q.max)?;
//             cell_indx += 1;
//             for n in q.neighbors {
//                 if n != NO_NEIGHBOR {
//                     write!(f, "{} ", n)?;
//                 } else {
//                     write!(f, "_ ")?;
//                 }
//             }
//             write!(f, "]\n")?;
//         }
//         Ok(())
//     }
// }

impl TreeGraph {
    #[inline(never)]
    pub fn build(config: &Iso2DConfig, f: &mut ImplicitFn) -> Self {
        let min = config.min;
        let max = config.max;

        // let mut quads = Quad::root().subdivide().to_vec();
        let mut quads = vec![Quad::root()];
        let mut par_quads = vec![];

        // let should_descend = |vals: [f64; 4], range: crate::vm::Range| {
        //     let [s0, s1, s2, s3] = vals.map(|v| v.signum());
        //     range.is_undef()
        //         || range.contains_zero()
        //         || s0 != s1
        //         || s1 != s2
        //         || s2 != s3
        //         || s3 != s0
        // };

        for depth in 0..=config.depth {
            // let quad_corner_pos: Vec<_> = quads
            //     .iter()
            //     .map(|q| q.corners(min, max).map(|c| c.extend(0.0)))
            //     .collect();

            // let quad_bounds: Vec<_> = quad_corner_pos
            //     .iter()
            //     .map(|corners| (corners[0], corners[2]))
            //     .collect();

            let quad_bounds: Vec<_> = quads
                .iter()
                .map(|q| {
                    let (min, max) = q.bounds(min, max);
                    (min.extend(0.0), max.extend(0.0))
                })
                .collect();

            let quad_ranges = f.eval_range_vec(quad_bounds);
            // let quad_evals: Vec<_> = f
            //     .eval_f64_vec(quad_corner_pos.into_iter().flatten().collect())
            //     .chunks_exact(4)
            //     .map(|v| [v[0], v[1], v[2], v[3]])
            //     .collect();

            // let mut indx_map = Vec::with_capacity(quads.len());
            let mut indx_map = vec![NO_NEIGHBOR; quads.len()];
            // let mut indx_map: Vec<_> = quad_ranges.into_iter().map(|r| if r.contains_zero() { 0 } else { NO_NEIGHBOR }).collect();

            std::mem::swap(&mut quads, &mut par_quads);
            quads.clear();

            for p_indx in 0..par_quads.len() {
                let range = quad_ranges[p_indx];
                // let evals = quad_evals[p_indx];

                // if !should_descend(evals, range) {
                //     continue;
                // }
                if !(range.contains_zero() || range.is_undef()) {
                    // indx_map.push(NO_NEIGHBOR);
                    continue;
                }
                // else if depth == config.depth - 1 && range.is_undef() {
                //     continue;
                // }

                let p = par_quads[p_indx];
                let [sw, se, nw, ne] = p.subdivide();

                let sw_indx = quads.len() as Index;
                let se_indx = sw_indx + dir_diag::SE;
                let nw_indx = sw_indx + dir_diag::NW;
                let ne_indx = sw_indx + dir_diag::NE;
                quads.extend([sw, se, nw, ne]);
                // indx_map.push(sw_indx);
                indx_map[p_indx] = sw_indx;

                let sw_n = &mut quads[sw_indx as usize].neighbors;
                sw_n[dir::N] = nw_indx;
                sw_n[dir::E] = se_indx;

                let se_n = &mut quads[se_indx as usize].neighbors;
                se_n[dir::N] = ne_indx;
                se_n[dir::W] = sw_indx;

                let nw_n = &mut quads[nw_indx as usize].neighbors;
                nw_n[dir::S] = sw_indx;
                nw_n[dir::E] = ne_indx;

                let ne_n = &mut quads[ne_indx as usize].neighbors;
                ne_n[dir::S] = se_indx;
                ne_n[dir::W] = nw_indx;
            }

            for p_indx in 0..par_quads.len() {
                let sw_indx = indx_map[p_indx];
                // cell was discarded in this loop
                if sw_indx == NO_NEIGHBOR {
                    continue;
                }

                let p_n = &par_quads[p_indx].neighbors;
                let se_indx = sw_indx + dir_diag::SE;
                let nw_indx = sw_indx + dir_diag::NW;
                let ne_indx = sw_indx + dir_diag::NE;

                // no neighbor found
                if p_n[dir::N] != NO_NEIGHBOR {
                    let outer_indx = indx_map[p_n[dir::N] as usize];
                    // neighbor was discarded this loop
                    if outer_indx != NO_NEIGHBOR {
                        quads[nw_indx as usize].neighbors[dir::N] = outer_indx + dir_diag::SW;
                        quads[ne_indx as usize].neighbors[dir::N] = outer_indx + dir_diag::SE;
                    }
                }
                if p_n[dir::E] != NO_NEIGHBOR {
                    let outer_indx = indx_map[p_n[dir::E] as usize];
                    if outer_indx != NO_NEIGHBOR {
                        quads[ne_indx as usize].neighbors[dir::E] = outer_indx + dir_diag::NW;
                        quads[se_indx as usize].neighbors[dir::E] = outer_indx + dir_diag::SW;
                    }
                }
                if p_n[dir::S] != NO_NEIGHBOR {
                    let outer_indx = indx_map[p_n[dir::S] as usize];
                    if outer_indx != NO_NEIGHBOR {
                        quads[se_indx as usize].neighbors[dir::S] = outer_indx + dir_diag::NE;
                        quads[sw_indx as usize].neighbors[dir::S] = outer_indx + dir_diag::NW;
                    }
                }
                if p_n[dir::W] != NO_NEIGHBOR {
                    let outer_indx = indx_map[p_n[dir::W] as usize];
                    if outer_indx != NO_NEIGHBOR {
                        quads[nw_indx as usize].neighbors[dir::W] = outer_indx + dir_diag::NE;
                        quads[sw_indx as usize].neighbors[dir::W] = outer_indx + dir_diag::SE;
                    }
                }
            }
        }

        // let quad_corner_pos: Vec<_> = quads
        //     .iter()
        //     .map(|q| q.corners(min, max).map(|c| c.extend(0.0)))
        //     .collect();

        // let quad_bounds: Vec<_> = quad_corner_pos
        //     .iter()
        //     .map(|corners| (corners[0], corners[2]))
        //     .collect();
        let quad_bounds: Vec<_> = quads
            .iter()
            .map(|q| {
                let (min, max) = q.bounds(min, max);
                (min.extend(0.0), max.extend(0.0))
            })
            .collect();

        let quad_ranges = f.eval_range_vec(quad_bounds);

        // mark cells that should not be connected to eachother, e.g. cells containing undef values

        // let quad_evals: Vec<_> = f
        //     .eval_f64_vec(quad_corner_pos.into_iter().flatten().collect())
        //     .chunks_exact(4)
        //     .map(|v| [v[0], v[1], v[2], v[3]])
        //     .collect();

        let mut indx_remap = vec![NO_NEIGHBOR; quads.len()];
        let mut final_quads = vec![];

        for i in 0..quads.len() {
            let range = quad_ranges[i];
            // let evals = quad_evals[i];
            // let [s0, s1, s2, s3] = evals.map(|v| v.signum());
            // if s0 != s1 || s1 != s2 || s2 != s3 || s3 != s0 {
            if !(range.contains_zero() || range.is_undef()) {
                continue;
            }

            indx_remap[i] = final_quads.len() as u32;
            let mut leaf = quads[i].as_leaf();
            leaf.marked = range.is_undef();
            final_quads.push(leaf);
        }

        let mut quads = final_quads;
        for q in &mut quads {
            for i in 0..4 {
                let n = q.neighbors[i];
                if n == NO_NEIGHBOR {
                    break;
                }
                q.neighbors[i] = indx_remap[n as usize];
            }
            q.neighbors.sort_unstable();
        }

        Self { quads, min, max }
    }

    #[inline(never)]
    fn collapse(self, config: &Iso2DConfig, f: &mut ImplicitFn) -> Vec<[DVec3; 2]> {
        let min = self.min.extend(0.0);
        let max = self.max.extend(0.0);

        let mut corners = FxHashSet::<u64>::default();
        for q in &self.quads {
            corners.extend(iso::quad::corner_locations(q.code));
        }

        let corner_pos: Vec<_> = corners
            .iter()
            .map(|&q| iso::corner_position(q, min, max))
            .collect();
        let corner_vals = f.eval_f64_vec(corner_pos.clone());
        // let corner_grads: Vec<_> = corner_pos.clone().into_iter().map(|pos| f.eval_grad(pos)).collect();

        let corner_lookup: FxHashMap<u64, (DVec2, f64 /*, DVec3*/)> = corners
            .into_iter()
            .zip(
                corner_pos
                    .into_iter()
                    .map(|pos| pos.truncate())
                    .zip(corner_vals), // .zip(corner_grads)
                                       // .map(|((p, v), g)| (p, v, g))
            )
            .collect();

        let mut quad_duals = vec![];

        for q in self.quads {
            let corner_code = iso::quad::corner_locations(q.code);
            let mut corner_pos = [DVec2::ZERO; 4];
            let mut corner_evals = [0f64; 4];
            // let mut corner_grads = [DVec3::ZERO; 4];

            for i in 0..4 {
                let &(pos, eval) = corner_lookup.get(&corner_code[i]).unwrap();
                corner_pos[i] = pos;
                corner_evals[i] = eval;
                // corner_grads[i] = grad;
            }

            let q_min = corner_pos[0];
            let q_max = corner_pos[2];

            let mut edge_duals = [DVec2::ZERO; 4];
            let mut n_duals = 0;

            let mut qef = QefSolver2D::new();

            for (i0, i1) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
                let (v0, v1) = (corner_evals[i0], corner_evals[i1]);
                let (p0, p1) = (corner_pos[i0], corner_pos[i1]);
                // let (n0, n1) = (corner_grads[i0], corner_grads[i1]);
                if v0.signum() != v1.signum() {
                    n_duals += 1;
                    let dual = zero_cross(p0, v0, p1, v1);
                    edge_duals[i0] = dual;
                    if matches!(config.dual_vertex, DualVertex::QEF) {
                        let grad = f.eval_grad(dual.extend(0.0)).truncate();
                        qef.add(dual, grad);
                    }
                }
            }

            if matches!(config.dual_vertex, DualVertex::AllMidPoints) {
                quad_duals.push(q.as_dual((q_min + q_max) / 2.0));
                continue;
            }

            if n_duals != 0 {
                let mass_point = (q_min + q_max) / 2.0;
                let vert = match config.dual_vertex {
                    DualVertex::MidPoint => mass_point,
                    DualVertex::AvgEdgeDual => {
                        let mut vert = DVec2::ZERO;
                        for d in edge_duals {
                            vert += d;
                        }
                        vert /= n_duals as f64;
                        vert
                    }
                    DualVertex::QEF => {
                        let bias_strength = 0.0000000001;
                        qef.add(mass_point, DVec2::new(bias_strength, 0.0));
                        qef.add(mass_point, DVec2::new(0.0, bias_strength));
                        let vert = qef.solve();
                        vert
                    }
                    DualVertex::AllMidPoints => todo!(),
                };
                // quad_duals.push((Some(vert.extend(0.0)), q.neighbors, q.marked));
                quad_duals.push(q.as_dual(vert));
            } else {
                let mut dual = q.as_dual(DVec2::NAN);
                dual.discard = true;
                quad_duals.push(dual);
                // quad_duals.push((None, q.neighbors, q.marked))
            }
        }

        let mut segments = vec![];

        for i in 0..quad_duals.len() {
            // let (dual, neighbors, marked) = quad_duals[i];
            let dual = quad_duals[i];
            if dual.discard || dual.marked { continue };
            quad_duals[i].discard = true;

            // let Some(dual) = dual else { continue };
            // quad_duals[i].0 = None;
            let mut connected = false;
            for n in dual.neighbors {
                if n == NO_NEIGHBOR {
                    break;
                }
                let n_dual = quad_duals[n as usize];
                if !n_dual.discard && !n_dual.marked {
                    segments.push([dual.vertex.extend(0.0), n_dual.vertex.extend(0.0)]);
                    connected = true;
                } else if n_dual.discard && !n_dual.vertex.is_nan() {
                    // was already connected
                    connected = true;
                }
            }

            if !connected && !dual.marked {
                for n in dual.neighbors {
                    if n == NO_NEIGHBOR {
                        break;
                    }
                    let n_dual = quad_duals[n as usize];
                    if !n_dual.discard && n_dual.marked {
                        segments.push([dual.vertex.extend(0.0), n_dual.vertex.extend(0.0)]);
                    }
                }
            }
        }


        segments
        // let mut quad_duals: Vec<_> = self
        //     .quads
        //     .iter()
        //     .map(|q| {
        //         let corner_code = iso::quad::corner_locations(q.code);
        //         let mut corner_pos = [DVec2::ZERO; 4];
        //         let mut corner_evals = [0f64; 4];
        //         // let mut corner_grads = [DVec3::ZERO; 4];

        //         for i in 0..4 {
        //             let &(pos, eval) = corner_lookup.get(&corner_code[i]).unwrap();
        //             corner_pos[i] = pos;
        //             corner_evals[i] = eval;
        //             // corner_grads[i] = grad;
        //         }

        //         let q_min = corner_pos[0];
        //         let q_max = corner_pos[2];

        //         let mut edge_duals = [DVec2::ZERO; 4];
        //         let mut n_duals = 0;

        //         let mut qef = QefSolver2D::new();

        //         for (i0, i1) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        //             let (v0, v1) = (corner_evals[i0], corner_evals[i1]);
        //             let (p0, p1) = (corner_pos[i0], corner_pos[i1]);
        //             // let (n0, n1) = (corner_grads[i0], corner_grads[i1]);
        //             if v0.signum() != v1.signum() {
        //                 n_duals += 1;
        //                 let dual = zero_cross(p0, v0, p1, v1);
        //                 edge_duals[i0] = dual;
        //                 if matches!(config.dual_vertex, DualVertex::QEF) {
        //                     let grad = f.eval_grad(dual.extend(0.0)).truncate();
        //                     qef.add(dual, grad);
        //                 }
        //             }
        //         }

        //         if matches!(config.dual_vertex, DualVertex::AllMidPoints) {
        //             return (
        //                 Some(((q_min + q_max) / 2.0).extend(0.0)),
        //                 q.neighbors,
        //                 q.marked,
        //             );
        //         }

        //         if n_duals != 0 {
        //             let mass_point = (q_min + q_max) / 2.0;
        //             let vert = match config.dual_vertex {
        //                 DualVertex::MidPoint => mass_point,
        //                 DualVertex::AvgEdgeDual => {
        //                     let mut vert = DVec2::ZERO;
        //                     for d in edge_duals {
        //                         vert += d;
        //                     }
        //                     vert /= n_duals as f64;
        //                     vert
        //                 }
        //                 DualVertex::QEF => {
        //                     let bias_strength = 0.0000000001;
        //                     qef.add(mass_point, DVec2::new(bias_strength, 0.0));
        //                     qef.add(mass_point, DVec2::new(0.0, bias_strength));
        //                     let vert = qef.solve();
        //                     vert
        //                     // if vert.x >= q_min.x && vert.y >= q_min.y && vert.x <= q_max.x && vert.y <= q_max.y {
        //                     //     vert
        //                     // } else {
        //                     //     let mut vert = DVec2::ZERO;
        //                     //     for d in edge_duals {
        //                     //         vert += d;
        //                     //     }
        //                     //     vert /= n_duals as f64;
        //                     //     vert
        //                     // }
        //                 }
        //                 DualVertex::AllMidPoints => todo!(),
        //             };
        //             (Some(vert.extend(0.0)), q.neighbors, q.marked)
        //             // let mut vert = DVec2::ZERO;
        //             // for d in edge_duals {
        //             //     vert += d;
        //             // }
        //             // vert /= n_duals as f64;
        //             // let vert = vert.clamp(corner_pos[0], corner_pos[2]);
        //             // let vert = vert.extend(0.0);
        //             // (Some(vert), q.neighbors)
        //         } else {
        //             (None, q.neighbors, q.marked)
        //         }
        //     })
        //     .collect();

    }
}

#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub enum DualVertex {
    MidPoint,
    AvgEdgeDual,
    QEF,
    AllMidPoints,
}

#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub struct Iso2DConfig {
    #[egui_probe(with crate::ui::dvec2_probe)]
    pub min: DVec2,

    #[egui_probe(with crate::ui::dvec2_probe)]
    pub max: DVec2,

    pub depth: u32,

    #[egui_probe(with crate::ui::f32_drag(0.00001))]
    pub line_thickness: f32,

    #[egui_probe(with crate::ui::f64_drag(0.01))]
    pub grad_tol: f64,

    #[egui_probe(with crate::ui::f64_drag(0.01))]
    pub connect_tol: f64,

    pub dual_vertex: DualVertex,
    pub program: Program,
}

#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub enum Program {
    Tan,
    Sin1DivX,
    Dense,
}

impl Program {
    pub fn opcode(&self) -> Vec<vm::Opcode> {
        match self {
            Program::Tan => [op::TAN(1, 1), op::SUB_LHS_RHS(2, 1, 1), op::EXT(0)].to_vec(),
            Program::Sin1DivX => [
                op::DIV_IMM_RHS(1.0, 1, 1),
                op::SIN(1, 1),
                op::SUB_LHS_RHS(2, 1, 1),
                op::EXT(0),
            ]
            .to_vec(),
            Program::Dense => {
                [
                    op::ADD_LHS_RHS(1, 2, 3),
                    op::POW_IMM_RHS(3.0, 3, 3),
                    op::SIN(3, 3),
                    op::SIN(1, 1),
                    op::SIN(2, 2),
                    op::ADD_LHS_RHS(1, 2, 1),
                    op::POW_IMM_RHS(3.0, 1, 1),
                    op::SUB_LHS_RHS(1, 3, 1),
                    // op::ADD_LHS_RHS(1, 2, 3),
                    // op::SIN(3, 3),
                    // op::SIN(1, 1),
                    // op::COS(2, 2),
                    // op::ADD_LHS_RHS(1, 2, 1),
                    // op::SUB_LHS_RHS(1, 3, 1),
                    op::EXT(0),
                ]
                .to_vec()
            }
        }
    }
}

impl Default for Iso2DConfig {
    fn default() -> Self {
        Self {
            grad_tol: 0.0,
            connect_tol: 0.001,
            min: DVec2::ZERO,
            max: DVec2::ZERO,
            depth: 0,
            line_thickness: 0.0001,
            dual_vertex: DualVertex::AvgEdgeDual,
            program: Program::Tan,
        }
    }
}

pub fn build_2d(config: Iso2DConfig) -> (Vec<[Vec3; 3]>, Vec<[Vec3; 2]>) {
    let mut f = ImplicitFn::new(config.program.opcode());
    let tree_graph = TreeGraph::build(&config, &mut f);

    let mut tree_tris = vec![];
    for quad in &tree_graph.quads {
        let quad = quad.as_quad();
        let (min, max) = quad.bounds(config.min, config.max);
        let v0 = min.extend(0.0).as_vec3();
        let v2 = max.extend(0.0).as_vec3();
        let v1 = v0.with_x(v2.x);
        let v3 = v0.with_y(v2.y);
        tree_tris.push([v0, v1, v2]);
        tree_tris.push([v0, v2, v3]);
    }

    let lines: Vec<_> = tree_graph
        .collapse(&config, &mut f)
        .into_iter()
        .map(|s| s.map(|v| v.as_vec3()))
        .collect();

    (tree_tris, lines)
}
