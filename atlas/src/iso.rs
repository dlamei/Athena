///https://people.engr.tamu.edu/schaefer/research/iso_simplicial.pdf

pub const MAX_DEPTH: u8 = 15;

use std::{collections::BinaryHeap, fmt, sync::Arc};

use egui_probe::EguiProbe;
use ordered_float::OrderedFloat;

use glam::{DVec3, Vec2, Vec3};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    ui,
    vm::{self, float, op},
};

macro_rules! get_octants {
    ($loc:expr, $lvl:expr) => {
        // $loc >> $lvl * 4 & 0xF
        // $loc >> (16 - 1 - $lvl) * 4 & 0xF
        $loc >> $lvl * 4 & 0xF
    };
}

struct FmtSlice<'a, T>(&'a [T]);

impl<T: fmt::Display> fmt::Display for FmtSlice<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(first) = iter.next() {
            write!(f, "[{first}")?;
        }

        for next in iter {
            write!(f, ", {next}")?;
        }

        write!(f, "]")
    }
}
#[derive(Debug, Clone)]
pub struct ImplicitFn2 {
    pub program: Arc<Vec<vm::Opcode>>,
}

impl ImplicitFn2 {
    #[inline]
    pub fn eval_f64x4_vec(&mut self, input: Vec<DVec3>) -> Vec<float> {
        let mut vm = vm::VM::with_instr_table(vm::simd::F64x4VecInstrTable);
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];
        let len = input.len();

        for inp in input {
            x.push(inp[0]);
            y.push(inp[1]);
            z.push(inp[2]);
        }

        let simd_x = vm::simd::F64x4Vec::from(&x);
        let simd_y = vm::simd::F64x4Vec::from(&y);
        let simd_z = vm::simd::F64x4Vec::from(&z);

        vm.reg[1] = simd_x;
        vm.reg[2] = simd_y;
        vm.reg[3] = simd_z;

        vm.set_vec_size(len);
        vm.eval(&self.program);
        vm.take_reg(1, len)
    }

    #[inline]
    pub fn eval_range_vec(&mut self, input: Vec<(DVec3, DVec3)>) -> Vec<vm::Range> {
        let mut vm = vm::VM::with_instr_table(vm::RangeVecInstrTable);
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];
        let len = input.len();

        for (min, max) in input {
            x.push(vm::Range::from((min[0], max[0])));
            y.push(vm::Range::from((min[1], max[1])));
            z.push(vm::Range::from((min[2], max[2])));
        }

        vm.reg[1] = x.into();
        vm.reg[2] = y.into();
        vm.reg[3] = z.into();

        vm.set_vec_size(len);
        vm.eval(&self.program);
        vm.take_reg(1)
    }
}

#[derive(Debug, Clone)]
pub struct ImplicitFn {
    vm_f64: vm::VM<f64>,
    vm_f64_vec: vm::VM<vm::F64Vec>,
    vm_f64x4_vec: vm::VM<vm::simd::F64x4Vec>,
    vm_range: vm::VM<vm::Range>,
    vm_range_vec: vm::VM<vm::RangeVec>,
    vm_range_deriv: vm::VM<vm::RangeDeriv>,
    vm_deriv: vm::VM<vm::F64Deriv>,
    pub program: Vec<vm::Opcode>,
}

impl ImplicitFn {
    pub fn new(program: Vec<vm::Opcode>) -> Self {
        Self {
            vm_f64: vm::VM::with_instr_table(vm::F64InstrTable),
            vm_f64_vec: vm::VM::with_instr_table(vm::F64VecInstrTable),
            vm_f64x4_vec: vm::VM::with_instr_table(vm::simd::F64x4VecInstrTable),
            vm_range_vec: vm::VM::with_instr_table(vm::RangeVecInstrTable),
            vm_range: vm::VM::with_instr_table(vm::RangeInstrTable),
            vm_range_deriv: vm::VM::with_instr_table(vm::RangeDerivInstrTable),
            vm_deriv: vm::VM::with_instr_table(vm::F64DerivInstrTable),
            program,
        }
    }
    #[inline]
    pub fn eval_f64(&mut self, arg: DVec3) -> f64 {
        self.vm_f64.call([arg.x, arg.y, arg.z], &self.program)
    }

    pub fn eval_grad_range(&mut self, min: DVec3, max: DVec3) -> (vm::Range, vm::Range, vm::Range) {
        let x_rng = vm::Range::new(min.x, max.x);
        let y_rng = vm::Range::new(min.y, max.y);
        let z_rng = vm::Range::new(min.z, max.z);

        let dx = vm::RangeDeriv::var(x_rng);
        let dy = vm::RangeDeriv::var(y_rng);
        let dz = vm::RangeDeriv::var(z_rng);
        let x = vm::RangeDeriv::cnst(x_rng);
        let y = vm::RangeDeriv::cnst(y_rng);
        let z = vm::RangeDeriv::cnst(z_rng);

        let grad_x = self.vm_range_deriv.call([dx, y, z], &self.program).grad;
        let grad_y = self.vm_range_deriv.call([x, dy, z], &self.program).grad;
        let grad_z = self.vm_range_deriv.call([x, y, dz], &self.program).grad;

        (grad_x, grad_y, grad_z)
    }

    pub fn eval_grad(&mut self, arg: DVec3) -> DVec3 {
        let dx = vm::F64Deriv::var(arg.x);
        let dy = vm::F64Deriv::var(arg.y);
        let dz = vm::F64Deriv::var(arg.z);
        let x = vm::F64Deriv::cnst(arg.x);
        let y = vm::F64Deriv::cnst(arg.y);
        let z = vm::F64Deriv::cnst(arg.z);

        let grad_x = self.vm_deriv.call([dx, y, z], &self.program).grad;
        let grad_y = self.vm_deriv.call([x, dy, z], &self.program).grad;
        let grad_z = self.vm_deriv.call([x, y, dz], &self.program).grad;

        (grad_x, grad_y, grad_z).into()
    }

    #[inline]
    pub fn eval_range_vec(&mut self, input: Vec<(DVec3, DVec3)>) -> Vec<vm::Range> {
        let vm = &mut self.vm_range_vec;
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];
        let len = input.len();

        for (min, max) in input {
            x.push(vm::Range::from((min[0], max[0])));
            y.push(vm::Range::from((min[1], max[1])));
            z.push(vm::Range::from((min[2], max[2])));
        }

        vm.reg[1] = x.into();
        vm.reg[2] = y.into();
        vm.reg[3] = z.into();

        vm.set_vec_size(len);
        vm.eval(&self.program);
        vm.take_reg(1)
    }

    #[inline]
    pub fn eval_f64_vec(&mut self, input: Vec<DVec3>) -> Vec<float> {
        let vm = &mut self.vm_f64_vec;
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];
        let len = input.len();

        for inp in input {
            x.push(inp[0]);
            y.push(inp[1]);
            z.push(inp[2]);
        }

        vm.reg[1] = x.into();
        vm.reg[2] = y.into();
        vm.reg[3] = z.into();

        vm.set_vec_size(len);
        vm.eval(&self.program);
        vm.take_reg(1)
    }

    #[inline]
    pub fn eval_f64x4_vec(&mut self, input: Vec<DVec3>) -> Vec<float> {
        let vm = &mut self.vm_f64x4_vec;
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];
        let len = input.len();

        for inp in input {
            x.push(inp[0]);
            y.push(inp[1]);
            z.push(inp[2]);
        }

        let simd_x = vm::simd::F64x4Vec::from(&x);
        let simd_y = vm::simd::F64x4Vec::from(&y);
        let simd_z = vm::simd::F64x4Vec::from(&z);

        vm.reg[1] = simd_x;
        vm.reg[2] = simd_y;
        vm.reg[3] = simd_z;

        vm.set_vec_size(len);
        vm.eval(&self.program);
        vm.take_reg(1, len)
    }

    #[inline]
    pub fn eval_range(&mut self, min: DVec3, max: DVec3) -> vm::Range {
        let vm = &mut self.vm_range;

        for i in 0..3 {
            vm.reg[i + 1] = (min[i], max[i]).into();
        }

        vm.eval(&self.program);
        vm.reg[1]
    }
}

// 4 bits per level -> 1 byte per 2 levels
// 8 bytes -> max depth of 16
// type LocCode = u64;
type Cell = u64;
type Direction = u8;
type Corner = u64;

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LocFmt(Cell);

impl fmt::Display for LocFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for LocFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut str = String::new();
        for i in 0..16 {
            let oct = get_octants!(self.0, i);

            let char = match oct {
                0 => '_',
                1 => '1',
                2 => '2',
                3 => '3',
                4 => '4',
                5 => '5',
                6 => '6',
                7 => '7',
                8 => '8',
                _ => '?',
            };

            str.push(char);
        }

        write!(f, "{str}")
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CorFmt(Corner);

impl fmt::Display for CorFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for CorFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = self.0 & 0xFFFF;
        let y = (self.0 >> 16) & 0xFFFF;
        let z = (self.0 >> 32) & 0xFFFF;
        write!(f, "{x:x}, {y:x}, {z:x}")
    }
}

// level in [0, 15]

mod dir {
    use super::Direction;

    pub const POS_X: Direction = 0b0001;
    pub const POS_Y: Direction = 0b0010;
    pub const POS_Z: Direction = 0b0100;

    pub const NEG_X: Direction = 0b1001;
    pub const NEG_Y: Direction = 0b1010;
    pub const NEG_Z: Direction = 0b1100;

    pub const X_AXIS: (Direction, Direction) = (NEG_X, POS_X);
    pub const Y_AXIS: (Direction, Direction) = (NEG_Y, POS_Y);
    pub const Z_AXIS: (Direction, Direction) = (NEG_Z, POS_Z);
}

// TODO: direct only min max ?

pub(crate) mod cell {
    use super::*;

    #[inline]
    pub fn depth(mut loc: Cell) -> u8 {
        let mut i = 0;
        while loc != 0 {
            i += 1;
            loc >>= 4;
        }
        i
    }

    #[inline]
    pub fn in_dir(mut loc: Cell, dir: Direction) -> Cell {
        let mut shift = 0;

        loop {
            let mask = 0b1111 << (4 * shift);
            let oct = (loc & mask) >> (4 * shift);
            if oct == 0 {
                return 0;
            }
            let oct = oct - 1;
            let n = (oct ^ (dir & 0b111) as Cell) + 1;

            loc &= !(0b1111 << (4 * shift));
            loc |= n << (4 * shift);

            if dir & 0b1000 == 0 {
                if oct as u8 & dir == 0 {
                    return loc;
                }
            } else if oct as u8 & (dir & 0b0111) != 0 {
                return loc;
            }

            shift += 1;
        }
    }
    #[inline]
    pub fn parent_in_dir(mut loc: Cell, dir: Direction) -> Cell {
        let mut shift = 1;
        loop {
            let mask = 0b1111 << (4 * shift);
            let oct = (loc & mask) >> (4 * shift);
            if oct == 0 {
                return 0;
            }
            let oct = oct - 1;
            let n = (oct ^ (dir & 0b111) as Cell) + 1;

            loc &= !(0b1111 << (4 * shift));
            loc |= n << (4 * shift);

            if dir & 0b1000 == 0 {
                if oct as u8 & dir == 0 {
                    return loc;
                }
            } else if oct as u8 & (dir & 0b0111) != 0 {
                return loc;
            }

            shift += 1;
        }
    }
}

pub(crate) mod oct {
    use super::*;

    #[inline]
    pub fn subdivide(loc: Cell) -> [Cell; 8] {
        let octs = [1, 2, 3, 4, 5, 6, 7, 8];
        // let depth = octant_depth(loc);
        octs.map(|oct| (loc << 4) | oct)
        // octs.map(|oct| loc | oct << (16 - 1 - depth) * 4)
    }

    // TODO: inline & unroll by hand?
    #[inline]
    pub(crate) fn unit_bounds(loc: Cell) -> (Vec3, Vec3) {
        // let mut bounds = (Vec3::ZERO, Vec3::ONE);
        let mut min = Vec3::ZERO;
        let mut max = Vec3::ONE;

        let depth = cell::depth(loc);

        for i in 0..depth {
            // let oct = ((loc >> ((depth - i) * 4 & 0xF) as u8;
            let oct = ((loc >> ((depth - i - 1) * 4)) & 0xF) - 1;

            let half_size = (max - min) / 2.0;

            if oct & 1 == 1 {
                min.x += half_size.x
            }
            if (oct >> 1) & 1 == 1 {
                min.y += half_size.y
            }
            if (oct >> 2) & 1 == 1 {
                min.z += half_size.z
            }

            max = min + half_size;
        }
        (min, max)
    }

    #[inline]
    pub fn bounds(min: Vec3, max: Vec3, loc: Cell) -> (Vec3, Vec3) {
        let size = max - min;
        let (u_min, u_max) = unit_bounds(loc);
        (u_min * size + min, u_max * size + min)
    }

    pub fn unit_corners(loc: Cell) -> [Vec3; 8] {
        let mut out = [Default::default(); 8];
        let (min, max) = unit_bounds(loc);
        let size = max - min;

        for i in 0..8 {
            let mut pos = min;

            for j in 0..3 {
                if (i >> j) & 1 == 1 {
                    pos[j] += size[j];
                }
            }

            out[i] = pos;
        }

        out
    }

    #[inline]
    pub fn corners(min: Vec3, max: Vec3, loc: Cell) -> [Vec3; 8] {
        let size = max - min;
        let u_corners = unit_corners(loc);
        u_corners.map(|corner| corner * size + min)
    }

    #[inline]
    pub fn neighbors(o: Cell) -> [Cell; 26] {
        let n_mid = quad::neighbors(o);
        let n_top = n_mid.map(|n| cell::in_dir(n, dir::POS_Z));
        let n_bot = n_mid.map(|n| cell::in_dir(n, dir::NEG_Z));

        let up = cell::in_dir(o, dir::POS_Z);
        let down = cell::in_dir(o, dir::NEG_Z);

        let mut res = [0; 26];
        res[24] = down;
        res[25] = up;
        for i in 0..8 {
            res[i] = n_bot[i];
            res[i + 8] = n_mid[i];
            res[i + 16] = n_top[i];
        }

        res
    }

    #[inline]
    pub fn min_corner(loc: u64) -> Corner {
        let lvl = 1;
        // TODO only max depth 15!!!
        let mut oct_size = 1 << MAX_DEPTH;

        let mut c_x = 0 as Corner;
        let mut c_y = 0 as Corner;
        let mut c_z = 0 as Corner;

        let depth = cell::depth(loc);
        debug_assert!(depth <= MAX_DEPTH);

        for i in 0..depth {
            let octs = ((loc >> ((depth - 1 - i) * 4)) & 0xF) - 1;

            oct_size >>= 1;
            if octs & 0b001 != 0 {
                c_x += oct_size;
            }
            if octs & 0b010 != 0 {
                c_y += oct_size;
            }
            if octs & 0b100 != 0 {
                c_z += oct_size;
            }
        }

        c_x | (c_y << 16) | (c_z << 32)
    }

    #[inline]
    fn min_corner_pos(loc: u64) -> DVec3 {
        let depth = cell::depth(loc);
        debug_assert!(depth <= MAX_DEPTH);

        let mut c_x = 0.0;
        let mut c_y = 0.0;
        let mut c_z = 0.0;
        let mut cube_side = 1.0;

        for i in 0..depth {
            cube_side /= 2.0;
            let shift = (depth - 1 - i) * 4;
            // Extract the nibble for this level; subtract 1 so the value is in 0..7.
            let oct_indx = ((loc >> shift) & 0xF) - 1;
            if oct_indx & 0b001 != 0 {
                c_x += cube_side;
            }
            if oct_indx & 0b010 != 0 {
                c_y += cube_side;
            }
            if oct_indx & 0b100 != 0 {
                c_z += cube_side;
            }
        }
        DVec3::new(c_x, c_y, c_z)
    }
}

pub(crate) mod quad {
    use super::*;
    #[inline]
    pub fn subdivide(loc: Cell) -> [Cell; 4] {
        let octs = [1, 2, 3, 4];
        octs.map(|oct| (loc << 4) | oct)
    }

    #[inline]
    pub fn subdivide_until(loc: Cell, max_depth: u8) -> Vec<Cell> {
        let n_subdiv = max_depth - cell::depth(loc);
        let n_cells = 4_usize.pow(n_subdiv as u32);
        let mut res = Vec::with_capacity(n_cells);

        for i in 0..n_cells {
            let mut quad = loc << (4 * n_subdiv);
            let mut j = i;
            for k in 0..n_subdiv {
                let digit = (j % 4) as Cell + 1;
                j /= 4;
                quad |= digit << (4 * (n_subdiv - 1 - k));
            }
            res.push(quad);
        }

        res
    }

    #[inline]
    pub fn neighbors(q: Cell) -> [Cell; 8] {
        let n = cell::in_dir(q, dir::POS_Y);
        let e = cell::in_dir(q, dir::POS_X);
        let s = cell::in_dir(q, dir::NEG_Y);
        let w = cell::in_dir(q, dir::NEG_X);

        let ne = cell::in_dir(n, dir::POS_X);
        let nw = cell::in_dir(n, dir::NEG_X);
        let se = cell::in_dir(s, dir::POS_X);
        let sw = cell::in_dir(s, dir::NEG_X);

        [n, ne, e, se, s, sw, w, nw]
    }

    #[inline]
    pub fn parent_neighbors(q: Cell) -> [Cell; 8] {
        let n = cell::parent_in_dir(q, dir::POS_Y);
        let e = cell::parent_in_dir(q, dir::POS_X);
        let s = cell::parent_in_dir(q, dir::NEG_Y);
        let w = cell::parent_in_dir(q, dir::NEG_X);

        let ne = cell::parent_in_dir(n, dir::POS_X);
        let nw = cell::parent_in_dir(n, dir::NEG_X);
        let se = cell::parent_in_dir(s, dir::POS_X);
        let sw = cell::parent_in_dir(s, dir::NEG_X);

        [n, ne, e, se, s, sw, w, nw]
    }

    #[inline]
    pub fn corner_locations(loc: u64) -> [Corner; 4] {
        let min_corner = oct::min_corner(loc);
        let depth = cell::depth(loc);
        let oct_size = 1 << (MAX_DEPTH - depth);

        let min_x = min_corner & 0xFFFF;
        let min_y = (min_corner >> 16) & 0xFFFF;

        [
            min_x | (min_y << 16),
            (min_x + oct_size) | (min_y << 16),
            (min_x + oct_size) | ((min_y + oct_size) << 16),
            min_x | ((min_y + oct_size) << 16),
        ]
    }

    #[inline]
    pub fn corners(min: Vec3, max: Vec3, loc: Cell) -> [Vec3; 4] {
        let size = max - min;
        let oct_corners = oct::unit_corners(loc);
        let u_corners = [
            oct_corners[0],
            oct_corners[1],
            oct_corners[2],
            oct_corners[3],
        ];
        u_corners.map(|corner| corner * size + min)
    }
}

// #[inline(never)]
pub fn lines_to_triangles(segments: &[(Vec3, Vec3)], thickness: f32) -> Vec<[Vec3; 3]> {
    let mut triangles = Vec::new();

    for &(a, b) in segments {
        let dir = b - a;
        if dir.length_squared() < 1e-6 {
            continue;
        }

        let w = dir.cross(Vec3::Z).normalize() * thickness / 2.0;

        let v0 = a + w;
        let v1 = a - w;
        let v2 = b + w;
        let v3 = b - w;

        triangles.push([v0, v1, v2]);
        triangles.push([v2, v1, v3]);
        // triangles.push([a, a, b]);
    }

    triangles
}

#[inline]
fn dual_vertex(p1: Vec3, p2: Vec3) -> Vec3 {
    (p1 + p2) / 2.0
}

fn make_u128_edge(mut v1: u64, mut v2: u64) -> u128 {
    if v1 < v2 {
        (v1, v2) = (v2, v1);
    }
    (v1 as u128) | ((v2 as u128) << 64)
}

fn corners_from_edge(edge: u128) -> (u64, u64) {
    (edge as u64, (edge >> 64) as u64)
}

fn corner_unit_position(corner_code: u64) -> DVec3 {
    let mask: u64 = 0xFFFF;
    let c_x = (corner_code & mask) as f64;
    let c_y = ((corner_code >> 16) & mask) as f64;
    let c_z = ((corner_code >> 32) & mask) as f64;
    let resolution = (1 << MAX_DEPTH) as f64;
    DVec3::new(c_x / resolution, c_y / resolution, c_z / resolution)
}

#[inline]
pub fn corner_position(c: Corner, min: DVec3, max: DVec3) -> DVec3 {
    let pos = corner_unit_position(c);
    let size = max - min;
    pos * size + min
}

#[inline]
pub fn corner_locations(loc: u64) -> [Corner; 8] {
    let min_corner = oct::min_corner(loc);
    let depth = cell::depth(loc);
    let oct_size = 1 << (MAX_DEPTH - depth);

    let min_x = min_corner & 0xFFFF;
    let min_y = (min_corner >> 16) & 0xFFFF;
    let min_z = (min_corner >> 32) & 0xFFFF;

    [
        min_x | (min_y << 16) | (min_z << 32),
        (min_x + oct_size) | (min_y << 16) | (min_z << 32),
        min_x | ((min_y + oct_size) << 16) | (min_z << 32),
        (min_x + oct_size) | ((min_y + oct_size) << 16) | (min_z << 32),
        min_x | (min_y << 16) | ((min_z + oct_size) << 32),
        (min_x + oct_size) | (min_y << 16) | ((min_z + oct_size) << 32),
        min_x | ((min_y + oct_size) << 16) | ((min_z + oct_size) << 32),
        (min_x + oct_size) | ((min_y + oct_size) << 16) | ((min_z + oct_size) << 32),
    ]
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
struct SurfacePoint {
    pos: Vec3,
    val: f64,
}

impl SurfacePoint {
    #[inline]
    fn new(pos: Vec3, f: &mut ImplicitFn) -> Self {
        let val = f.eval_f64(pos.into());
        Self { pos, val }
    }
}

#[inline]
pub fn zero_cross(p1: SurfacePoint, p2: SurfacePoint, f: &mut ImplicitFn) -> Vec3 {
    let denom = p1.val - p2.val;
    let k1 = -p2.val / denom;
    let k2 = p1.val / denom;
    k1 as f32 * p1.pos + k2 as f32 * p2.pos
}

#[inline]
fn march_tetrahedron(tetra: [SurfacePoint; 4], f: &mut ImplicitFn, tris: &mut Vec<[Vec3; 3]>) {
    let mut id = 0u32;
    for t in tetra {
        id = 2 * id + (t.val > 0.0) as u32;
    }

    let [p0, p1, p2] = match id {
        0b0001 | 0b1110 => [(0u8, 3u8), (1u8, 3u8), (2u8, 3u8)],
        0b0010 | 0b1101 => [(0u8, 2u8), (1u8, 2u8), (3u8, 2u8)],
        0b0100 | 0b1011 => [(0u8, 1u8), (2u8, 1u8), (3u8, 1u8)],
        0b1000 | 0b0111 => [(1u8, 0u8), (2u8, 0u8), (3u8, 0u8)],
        id => {
            let [p0, p1, p2, p3] = match id {
                0b0011 | 0b1100 => [(0u8, 2u8), (2u8, 1u8), (1u8, 3u8), (3u8, 0u8)],
                0b0110 | 0b1001 => [(0u8, 1u8), (1u8, 3u8), (3u8, 2u8), (2u8, 0u8)],
                0b0101 | 0b1010 => [(0u8, 1u8), (1u8, 2u8), (2u8, 3u8), (3u8, 0u8)],
                _ => return,
            }
            .map(|(i, j)| zero_cross(tetra[i as usize], tetra[j as usize], f));

            tris.push([p0, p1, p3]);
            tris.push([p1, p2, p3]);

            return;
        }
    }
    .map(|(i, j)| zero_cross(tetra[i as usize], tetra[j as usize], f));
    tris.push([p0, p1, p2]);

    // let pts: Vec<_> = indxs.iter().map(|(i, j)| find_zero(tetra[*i as usize], tetra[*j as usize], f)).collect();

    // if pts.len() == 3 {
    //     tris.push([pts[0], pts[1], pts[2]]);
    // }
    // if pts.len() == 4 {
    //     tris.push([pts[0], pts[1], pts[3]]);
    //     tris.push([pts[1], pts[2], pts[3]]);
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OctCell {
    grad: OrderedFloat<f64>,
    has_undef: bool,
    has_zero: bool,
    depth: u32,
    loc: Cell,
}

impl OctCell {
    fn root(loc: Cell) -> Self {
        Self {
            grad: OrderedFloat(f64::MAX),
            has_undef: false,
            loc,
            depth: 0,
            has_zero: false,
        }
    }
}

impl Ord for OctCell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.depth == other.depth {
            Ord::cmp(&self.grad, &other.grad)
        } else {
            Ord::cmp(&self.depth, &other.depth).reverse()
        }
    }
}
impl PartialOrd for OctCell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

const fn neighbor_indx_from_xy(xy: (i8, i8)) -> u8 {
    match xy {
        (0, 0) => 0,
        (0, 1) => 1,
        (1, 0) => 2,
        (0, -1) => 3,
        (-1, 0) => 4,
        (1, 1) => 5,
        (-1, 1) => 6,
        (1, -1) => 7,
        (-1, -1) => 8,
        _ => panic!(),
    }
}

pub fn neighbor_indx_from_dir(min: Vec3, max: Vec3, p: Vec3, d: Vec3) -> [u8; 3] {
    use std::f32::consts::PI;
    const PI_4: f32 = PI / 4.0;

    const N: [(i8, i8); 3] = [(-1i8, 1i8), (0i8, 1i8), (1i8, 1i8)];
    const E: [(i8, i8); 3] = [(1i8, 1i8), (1i8, 0i8), (1i8, -1i8)];
    const S: [(i8, i8); 3] = [(1i8, -1i8), (0i8, -1i8), (-1i8, -1i8)];
    const W: [(i8, i8); 3] = [(-1i8, -1i8), (-1i8, 0i8), (-1i8, 1i8)];

    let a = d.angle_between(Vec3::X);
    if d.y > 0.0 {
        if a < PI_4 {
            E
        } else if a < 2.0 * PI - PI_4 {
            N
        } else {
            W
        }
    } else if a < PI_4 {
        E
    } else if a < 2.0 * PI - PI_4 {
        S
    } else {
        W
    }
    .map(neighbor_indx_from_xy)
}

#[derive(Debug, Clone)]
pub struct NTree {
    pub cells: Vec<Cell>,
}

impl NTree {
    pub fn build_2d_2(config: Iso2DConfig, f: &mut ImplicitFn) -> Self {
        assert!(config.depth <= MAX_DEPTH as u32);
        let max_cells: u32 = 4u32.pow(config.depth);

        let min = config.min.extend(0.0);
        let max = config.max.extend(0.0);

        let mut quads = vec![];
        let mut cells_todo = BinaryHeap::default();
        cells_todo.extend([
            OctCell::root(1),
            OctCell::root(2),
            OctCell::root(3),
            OctCell::root(4),
        ]);

        let mut n_cells = 0;
        while let Some(quad) = cells_todo.pop() {
            let (mut o_min, mut o_max) = oct::bounds(min, max, quad.loc);
            o_min.z = 0.0;
            o_max.z = 0.0;
            let range = f.eval_range(o_min.into(), o_max.into());

            //             if (o_min - o_max).length_squared() < (max-min).length_squared()/1000000.0 as f32 {
            //                 let mut quad = quad;
            //                 quad.has_undef = range.is_undef();
            //                 quad.has_zero = range.contains_zero();
            //                 quads.push(quad);
            //                 continue;
            //             }

            // if oct.depth > depth as u32 {
            //     let mut oct = oct;
            //     oct.has_undef = range.is_undef();
            //     quads.push(oct);
            //     continue;
            // }
            // if oct.depth > depth as u32 {
            //     leafs.push(oct.loc);
            //     continue;
            // }

            // if !range.is_undef() && !range.contains_zero() {
            //     leafs.push(oct.loc);
            //     continue;
            // }

            let (xgrad, ygrad, _) = f.eval_grad_range(o_min.into(), o_max.into());
            let grad = DVec3::new(xgrad.dist(), ygrad.dist(), 0.0).length();

            if grad > config.grad_tol && range.contains_zero() || range.is_undef() {
                // if quad.depth <= depth
                cells_todo.extend(quad::subdivide(quad.loc).map(|loc| OctCell {
                    // has_undef: range.is_undef(),
                    has_zero: range.contains_zero(),
                    has_undef: false,
                    grad: grad.into(),
                    depth: quad.depth + 1,
                    loc,
                }));
            } else {
                let mut quad = quad;
                quad.has_undef = range.is_undef();
                quad.has_zero = range.contains_zero();
                quads.push(quad);
                // if !quad.has_undef {
                //     quads.push(quad);
                // }
            }

            n_cells += 1;
            if max_cells <= n_cells {
                break;
            }
        }

        // for c in cells_todo.iter_mut()
        let cells_todo: Vec<_> = cells_todo
            .into_iter()
            .map(|mut c| {
                let (mut o_min, mut o_max) = oct::bounds(min, max, c.loc);
                o_min.z = 0.0;
                o_max.z = 0.0;
                let range = f.eval_range(o_min.into(), o_max.into());
                c.has_undef = range.is_undef();
                c
            })
            .collect();

        quads.extend(cells_todo.into_iter().filter(|c| !c.has_undef));
        // let mut cells: Vec<_> = quads.iter().filter_map(|c| if !c.has_undef { Some(c.loc) } else { None }).collect();
        // let mut cells: Vec<_> = quads.into_iter().chain(cells_todo).map(|c| c.loc).collect();
        let mut cells: Vec<_> = quads
            .iter()
            .filter_map(|c| if c.has_zero { Some(c.loc) } else { None })
            .collect();
        cells.sort();
        cells.reverse();

        Self { cells }
    }

    pub fn build_2d(config: Iso2DConfig, f: &mut ImplicitFn) -> Self {
        assert!(config.depth <= MAX_DEPTH as u32);

        let min = config.min.extend(0.0);
        let max = config.max.extend(0.0);

        let mut cells_todo: Vec<Cell> = vec![1, 2, 3, 4];

        for i in 1..=config.depth {
            let bounds: Vec<_> = cells_todo
                .iter()
                .map(|loc| {
                    let (min, max) = oct::bounds(min, max, *loc);
                    (min.as_dvec3(), max.as_dvec3())
                })
                .collect();
            let ranges = f.eval_range_vec(bounds);

            cells_todo = cells_todo
                .into_iter()
                .zip(ranges)
                .filter_map(|(loc, range)| {
                    if i == config.depth {
                        if range.contains_zero() {
                            return Some(loc);
                        }
                    } else if range.contains_zero() || range.is_undef() {
                        return Some(loc);
                    }
                    None
                })
                .flat_map(quad::subdivide)
                .collect();
        }

        Self { cells: cells_todo }
    }

    pub fn build_3d_2(config: Iso3DConfig, f: &mut ImplicitFn) -> Self {
        assert!(config.depth <= MAX_DEPTH as u32);
        let max_cells: u32 = 8u32.pow(config.depth);

        let mut octs = vec![];
        let mut cells_todo = BinaryHeap::default();
        cells_todo.extend([
            OctCell::root(1),
            OctCell::root(2),
            OctCell::root(3),
            OctCell::root(4),
            OctCell::root(5),
            OctCell::root(6),
            OctCell::root(7),
            OctCell::root(8),
        ]);

        let mut n_cells = 0;
        while let Some(oct) = cells_todo.pop() {
            let (o_min, o_max) = oct::bounds(config.min, config.max, oct.loc);
            let range = f.eval_range(o_min.into(), o_max.into());

            if !range.is_undef() && !range.contains_zero() {
                continue;
            }

            let (xgrad, ygrad, zgrad) = f.eval_grad_range(o_min.into(), o_max.into());
            let grad = DVec3::new(xgrad.dist(), ygrad.dist(), zgrad.dist());

            if grad.length() > config.tol && range.contains_zero() || range.is_undef() {
                // curr_lvl.extend(subdivide_octant(*oct))
                cells_todo.extend(oct::subdivide(oct.loc).map(|loc| OctCell {
                    has_zero: range.contains_zero(),
                    has_undef: range.is_undef(),
                    grad: grad.length().into(),
                    depth: oct.depth + 1,
                    loc,
                }));
            } else {
                octs.push(oct);
            }

            n_cells += 1;
            // if max_cells <= n_cells {
            //     break;
            // }
        }
        octs.extend(cells_todo);
        let cells = octs.iter().map(|c| c.loc).collect();

        Self { cells }
    }

    // pub fn build_3d(min: Vec3, max: Vec3, depth: u32, f: &mut ImplicitFn, tol: float) -> Self {
    //     let mut leafs = vec![];

    //     let mut buff_1: Vec<Cell> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    //     let mut buff_2: Vec<Cell> = vec![];

    //     let mut prev_lvl = &mut buff_1;
    //     let mut curr_lvl = &mut buff_2;

    //     for _ in 0..depth {
    //         curr_lvl.clear();

    //         for oct in prev_lvl.iter() {
    //             let (o_min, o_max) = oct::bounds(min, max, *oct);
    //             let range = f.eval_range(o_min.into(), o_max.into());

    //             let (xgrad, ygrad, zgrad) = f.eval_grad_range(o_min.into(), o_max.into());
    //             let grad = DVec3::new(xgrad.dist(), ygrad.dist(), zgrad.dist());

    //             if grad.length() > tol && range.contains_zero() || range.is_undef() {
    //                 curr_lvl.extend(oct::subdivide(*oct))
    //             } else if range.contains_zero() {
    //                 leafs.push(*oct);
    //             }
    //         }

    //         std::mem::swap(&mut curr_lvl, &mut prev_lvl);
    //     }
    //     leafs.extend(prev_lvl.iter());

    //     Self { cells: leafs }
    // }

    const fn neighbor_indx_from_marks(marks: u8) -> &'static [u8] {
        // const fn neighbor_indx_from_marks(marks: [bool; 4]) -> &'static [u8] {
        const N: u8 = neighbor_indx_from_xy((0, 1));
        const NE: u8 = neighbor_indx_from_xy((1, 1));
        const E: u8 = neighbor_indx_from_xy((1, 0));
        const SE: u8 = neighbor_indx_from_xy((1, 1));
        const S: u8 = neighbor_indx_from_xy((0, -1));
        const SW: u8 = neighbor_indx_from_xy((-1, -1));
        const W: u8 = neighbor_indx_from_xy((-1, 0));
        const NW: u8 = neighbor_indx_from_xy((-1, 1));

        match marks {
            0b1111 => &[N, NE, E, SE, S, SW, W, NW],
            0b1110 => &[N, NE, E, SE, S],
            0b1101 => &[N, NE, E, W, NW],
            0b1011 => &[N, S, SW, W, NW],
            0b0111 => &[E, SE, S, SW, W],
            0b1100 => &[N, NE, E],
            0b0110 => &[E, SE, S],
            0b0011 => &[S, SW, W],
            0b1001 => &[W, NW, N],
            0b1010 => &[N, S],
            0b0101 => &[E, W],
            0b1000 => &[N],
            0b0100 => &[E],
            0b0010 => &[S],
            0b0001 => &[W],
            0b0000 => &[],
            _ => unreachable!(),
        }
    }

    // #[inline(never)]
    fn connect_quads(
        cells: FxHashMap<Cell, (Vec3, u8)>,
        // mut cells: FxHashMap<LocCode, (Vec3, [bool; 4])>,
        // thickness: f32,
        // min: Vec3,
        // max: Vec3,
        config: Iso2DConfig,
    ) -> Vec<[Vec3; 2]> {
        let mut lines = vec![];
        let mut visited = FxHashSet::<u64>::default();

        let mut max_depth = 0;
        for (cell, (_, _)) in &cells {
            max_depth = max_depth.max(cell::depth(*cell));
        }

        // let mut adaptive_cells = FxHashMap::<LocCode, Vec3>::default();
        let mut adapt_src_cells = FxHashMap::<u64, Vec3>::default();
        // let mut dst_cells = FxHashMap::<u64, (Vec3, [bool; 4])>::default();
        for (cell, (vert, marks)) in &cells {
            let (vert, marks) = (*vert, *marks);

            if cell::depth(*cell) != max_depth {
                let new_cells = quad::subdivide_until(*cell, max_depth);
                adapt_src_cells.extend(new_cells.into_iter().map(|c| (c, vert)));
            }
        }

        for (cell, (v0, marks)) in &cells {
            let (cell, v0, marks) = (*cell, *v0, *marks);
            visited.insert(cell);

            let neighbors = quad::neighbors(cell);
            let neighbor_indxs = Self::neighbor_indx_from_marks(marks);

            for i in neighbor_indxs {
                let n = neighbors[*i as usize - 1];
                if n == 0 || visited.contains(&n) {
                    continue;
                }

                if let Some(v1) = cells.get(&n) {
                    lines.push((v0, v1.0));
                } else if let Some(v1) = adapt_src_cells.get(&n) {
                    lines.push((v0, *v1));
                }
            }
        }

        // lines_to_triangles(&lines, thickness)
        lines.into_iter().map(|(a, b)| [a, b]).collect()
    }

    // #[inline(never)]
    pub fn dual_contour_2d(&self, config: Iso2DConfig, f: &mut ImplicitFn) -> Vec<[Vec3; 2]> {
        let min = config.min.extend(0.0);
        let max = config.max.extend(0.0);

        let mut max_depth = 0;
        for c in &self.cells {
            max_depth = max_depth.max(cell::depth(*c));
        }

        // let mut graph = FxHashMap::<u128, (Vec<(u64, Vec3)>, Vec3)>::default();
        let mut verts = FxHashMap::<Cell, (Vec3, u8)>::default();
        // let mut lines = vec![];

        let mut corners = FxHashSet::<Corner>::default();
        // let mut edges = FxHashSet::<u128>::default();
        for c in &self.cells {
            let corner_loc = quad::corner_locations(*c);
            // let edge_corner_indx = [(0, 1), (1, 2), (2, 3), (3, 0)];
            corners.extend(corner_loc);
        }

        let corner_pos: Vec<_> = corners
            .iter()
            .map(|c| corner_position(*c, min.into(), max.into()))
            .collect();
        let corner_vals = f.eval_f64_vec(corner_pos.clone());

        let corner_lookup: FxHashMap<Corner, SurfacePoint> = corners
            .into_iter()
            .zip(
                corner_pos
                    .into_iter()
                    .zip(corner_vals)
                    .map(|(pos, val)| SurfacePoint {
                        pos: pos.as_vec3(),
                        val,
                    }),
            )
            .collect();

        // let mut corner_lookup = FxHashMap::<Corner, SurfacePoint>::default();
        // for c in corners {
        //     let pos = corner_position(c, min.into(), max.into());
        //     corner_lookup.insert(
        //         c,
        //         SurfacePoint {
        //             pos: pos.as_vec3(),
        //             val: f.eval_f64(pos),
        //         },
        //     );
        // }

        for quad in &self.cells {
            let corn_loc = quad::corner_locations(*quad);
            let mut sp = [SurfacePoint::default(); 4];

            for i in 0..4 {
                sp[i] = *corner_lookup.get(&corn_loc[i]).unwrap();
            }

            let edge_indxs = [(0, 1), (1, 2), (2, 3), (3, 0)];

            let min = sp[0].pos;
            let max = sp[2].pos;

            let mut int_pt = [Vec3::ZERO; 4];
            let mut int_mark = 0u8;
            let mut n_intersec = 0;

            let mut mid = Vec3::ZERO;
            for s in sp {
                mid += s.pos;
            }
            mid /= 4.0;

            for (i1, i2) in edge_indxs {
                let (v1, v2) = (sp[i1], sp[i2]);
                if v2.val.signum() != v1.val.signum() {
                    let cross_pt = zero_cross(v1, v2, f);
                    int_pt[i1] = cross_pt;
                    int_mark |= 0b1 << (3 - i1);
                    n_intersec += 1;

                    if cross_pt.distance_squared(v1.pos) < config.connect_tol as f32 {
                        let j1 = if i1 == 0 { 3 } else { i1 - 1 };
                        int_mark |= 0b1 << (3 - j1);
                    } else if cross_pt.distance_squared(v2.pos) < config.connect_tol as f32 {
                        let j2 = if i2 == 3 { 0 } else { i2 + 1 };
                        int_mark |= 0b1 << (3 - j2);
                    }
                    // lines.push((mid, zero_cross(v1, v2, f)));
                }
            }

            if n_intersec != 0 {
                let mut vert = Vec3::ZERO;
                for i in &int_pt {
                    vert += i;
                }
                vert /= n_intersec as f32;
                // let vert = vert.clamp(min, max);
                verts.insert(*quad, (vert, int_mark));
            }
        }

        Self::connect_quads(verts, config)
    }

    pub fn march_tetrahedra(&self, config: Iso3DConfig, f: &mut ImplicitFn) -> Vec<[Vec3; 3]> {
        // let mut tetras = vec![];
        let mut tris = vec![];

        let mut corner_points = vec![];
        for oct in &self.cells {
            corner_points
                .extend(oct::corners(config.min, config.max, *oct).map(|vec| vec.as_dvec3()));
            // corner_points.push(octant_corners(min, max, *oct).map(|v| v.as_dvec3()));
        }

        // let point_evals: Vec<[f64; 8]> =
        //     f.eval_f64_vec(corner_points.clone().into_iter().flatten().collect())
        //     .chunks_exact(8)
        //     .map(|chunk| chunk.try_into().expect("static"))
        //     .collect();

        let point_evals = f.eval_f64_vec(corner_points.clone());

        debug_assert_eq!(corner_points.len(), point_evals.len());

        // let dmax = max.as_dvec3();
        // let dmin = min.as_dvec3();
        // let corner_locs: FxHashSet<Corner> = self.cells.iter().flat_map(|c| corner_locations(*c)).collect();
        // let corner_evals = f.eval_f64_vec(corner_locs.iter().map(|loc| corner_position(*loc, dmin, dmax)).collect());
        // let sp_lookup: FxHashMap<Corner, SurfacePoint> = corner_locs.into_iter().zip(corner_evals).map(|(loc, val)| {
        //     let pos = corner_position(loc).as_vec3();
        //     (loc, SurfacePoint { pos, val })
        // }).collect();

        // for oct in &self.cells
        // for (corners, evals) in corner_points.into_iter().zip(point_evals)
        for i in 0..point_evals.len() / 8 {
            // let oct_corners = octant_corners(min, max, *oct);

            let mut c = [SurfacePoint::default(); 8];
            for j in 0..8 {
                // c[i] = SurfacePoint::new(oct_corners[i], &mut f);
                c[j] = SurfacePoint {
                    pos: corner_points[i * 8 + j].as_vec3(),
                    val: point_evals[i * 8 + j],
                };

                // println!("{c1:?} vs {c2:?}");
                // assert_eq!(c1, c2);
                // c[i] = c1;
            }

            // let tetras = [
            //     [c[0], c[1], c[2], c[5]],
            //     [c[0], c[2], c[4], c[5]],
            //     [c[1], c[2], c[3], c[5]],
            //     [c[2], c[3], c[5], c[7]],
            //     [c[2], c[4], c[5], c[6]],
            //     [c[2], c[5], c[6], c[7]],
            // ];

            // let tetras = [
            //     [c[0], c[1], c[2], c[4]],
            //     [c[1], c[2], c[3], c[7]],
            //     [c[1], c[2], c[4], c[7]],
            //     [c[1], c[4], c[5], c[7]],
            //     [c[2], c[4], c[6], c[7]],
            // ];

            // for t in tetras {
            //     march_tetrahedron(t, f, &mut tris);
            // }

            let vol_dual = SurfacePoint::new(dual_vertex(c[0].pos, c[7].pos), f);

            let faces = [
                [c[0], c[2], c[3], c[1]],
                [c[0], c[1], c[5], c[4]],
                [c[0], c[4], c[6], c[2]],
                [c[4], c[6], c[7], c[5]],
                [c[2], c[3], c[7], c[6]],
                [c[1], c[5], c[7], c[3]],
            ];

            for [f0, f1, f2, f3] in faces {
                let face_dual = SurfacePoint::new(dual_vertex(f0.pos, f2.pos), f);

                let edges = [[f0, f1], [f1, f2], [f2, f3], [f3, f0]];

                for [e0, e1] in edges {
                    let tetra = [e0, e1, face_dual, vol_dual];

                    march_tetrahedron(tetra, f, &mut tris);
                    // tetras.push(tetra);

                    /*
                    let edge_dual = dual_vertex(e0, e1);

                    let tetra0 = [
                    e0, edge_dual, face_dual, vol_dual
                    ];
                    let tetra1 = [
                    e1, edge_dual, face_dual, vol_dual
                    ];

                    tetras.push(tetra0);
                    tetras.push(tetra1);
                    */
                }
            }
        }

        tris
    }
}

// on 16x16x16
#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub struct Iso3DConfig {
    #[egui_probe(with ui::vec3_probe)]
    pub min: Vec3,

    #[egui_probe(with ui::vec3_probe)]
    pub max: Vec3,

    pub depth: u32,

    #[egui_probe(with ui::f64_drag(0.01))]
    pub tol: f64,

    pub shade_smooth: bool,
}

pub fn build_3d(config: Iso3DConfig, program: &[op::Opcode]) -> (Vec<[Vec3; 3]>, NTree) {
    let mut f = ImplicitFn::new(program.to_vec());
    let tree = NTree::build_3d_2(config, &mut f);
    let tris = tree.march_tetrahedra(config, &mut f);
    (tris, tree)
}

#[derive(Debug, Clone, Copy, PartialEq, EguiProbe)]
pub struct Iso2DConfig {
    #[egui_probe(with ui::vec2_probe)]
    pub min: Vec2,

    #[egui_probe(with ui::vec2_probe)]
    pub max: Vec2,

    pub depth: u32,

    #[egui_probe(with ui::f32_drag(0.00001))]
    pub line_thickness: f32,

    #[egui_probe(with ui::f64_drag(0.01))]
    pub grad_tol: f64,

    #[egui_probe(with ui::f64_drag(0.01))]
    pub connect_tol: f64,
}

impl Default for Iso2DConfig {
    fn default() -> Self {
        Self {
            grad_tol: 0.0,
            connect_tol: 0.001,
            min: Vec2::ZERO,
            max: Vec2::ZERO,
            depth: 0,
            line_thickness: 0.0001,
        }
    }
}

pub(crate) fn build_2d(config: Iso2DConfig, program: &[op::Opcode]) -> (Vec<[Vec3; 2]>, NTree) {
    let mut f = ImplicitFn::new(program.to_vec());
    let tree = NTree::build_2d(config, &mut f);
    let tris = tree.dual_contour_2d(config, &mut f);
    (tris, tree)
}

pub mod bench {
    use super::*;

    pub fn extract_iso_line(config: crate::iso2::Iso2DConfig) -> Vec<[Vec3; 2]> {
        let mut f = ImplicitFn::new(config.program.opcode());
        let config = Iso2DConfig {
            grad_tol: config.grad_tol,
            connect_tol: config.connect_tol,
            min: config.min.as_vec2(),
            max: config.max.as_vec2(),
            depth: config.depth,
            line_thickness: config.line_thickness,
        };
        let tree = NTree::build_2d(config, &mut f);

        tree.dual_contour_2d(config, &mut f)
    }
}

// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use glam::Vec2;

//     // Use a common rectangle for these tests.
//     const BL: Vec2 = Vec2::new(0.0, 0.0);
//     const TR: Vec2 = Vec2::new(10.0, 10.0);

//     #[test]
//     fn test_horizontal_exit() {
//         // p at center, ray horizontal to the right.
//         let p = Vec2::new(5.0, 5.0);
//         let d = Vec2::new(1.0, 0.0);
//         let tol = 1e-6;
//         let result = v3::neighbor_xy_from_dir(BL, TR, p, d, tol);
//         let expected = [(1, 0), (0, 0), (0, 0)];
//         assert_eq!(result, expected, "Horizontal exit should return right neighbor only.");
//     }

//     #[test]
//     fn test_vertical_exit() {
//         // p at center, ray vertical upward.
//         let p = Vec2::new(5.0, 5.0);
//         let d = Vec2::new(0.0, 1.0);
//         let tol = 1e-6;
//         let result = v3::neighbor_xy_from_dir(BL, TR, p, d, tol);
//         let expected = [(0, 1), (0, 0), (0, 0)];
//         assert_eq!(result, expected, "Vertical upward exit should return top neighbor only.");
//     }

//     #[test]
//     fn test_exact_corner() {
//         // p at bottom-left corner with ray heading to top-right.
//         let p = Vec2::new(0.0, 0.0);
//         let d = Vec2::new(1.0, 1.0);
//         let tol = 1e-6;
//         let result = v3::neighbor_xy_from_dir(BL, TR, p, d, tol);
//         let expected = [(1, 0), (0, 1), (1, 1)];
//         assert_eq!(result, expected, "Exact corner exit should return three neighbors.");
//     }

//     #[test]
//     fn test_almost_corner_small_tol() {
//         // p = (1,1) with d = (9,10) so that t_x and t_y differ by 0.1.
//         // With a small tol, we expect only one neighbor.
//         let p = Vec2::new(1.0, 1.0);
//         let d = Vec2::new(9.0, 10.0);
//         let tol = 0.05; // smaller than the difference (0.1)
//         let result = v3::neighbor_xy_from_dir(BL, TR, p, d, tol);
//         // Which boundary is hit first? t_y = (10-1)/10 = 0.9 and t_x = (10-1)/9 ≈ 1.0.
//         // So we exit via the top, so neighbor (0,1) only.
//         let expected = [(0, 1), (0, 0), (0, 0)];
//         assert_eq!(result, expected, "With small tol, only one neighbor should be returned.");
//     }

//     #[test]
//     fn test_almost_corner_big_tol() {
//         // Same as previous but with a larger tol so that 0.1 difference is acceptable.
//         let p = Vec2::new(1.0, 1.0);
//         let d = Vec2::new(9.0, 10.0);
//         let tol = 0.2; // larger than 0.1 difference
//         let result = v3::neighbor_xy_from_dir(BL, TR, p, d, tol);
//         // Expect two neighbors: (0,1) and (1,0). Order: first the one for the boundary hit first.
//         // Since t_y < t_x (0.9 < 1.0), the primary neighbor is (0,1) and then (1,0).
//         let expected = [(0, 1), (1, 0), (0, 0)];
//         assert_eq!(result, expected, "With big tol, two neighbors should be returned.");
//     }
// }
