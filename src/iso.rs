use crate::vm::{self, float};
use std::{collections::VecDeque, ops};

////https://people.engr.tamu.edu/schaefer/research/iso_simplicial.pdf

pub mod v3 {

    pub const MAX_DEPTH: u8 = 15;

    use std::{
        collections::{BinaryHeap, HashMap, HashSet, VecDeque},
        fmt, ops,
    };

    use ordered_float::OrderedFloat;

    use fxhash::{FxHashMap, FxHashSet};
    use glam::{DVec3, Vec2, Vec3};

    use crate::vm::{self, float, op};

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

            while let Some(next) = iter.next() {
                write!(f, ", {next}")?;
            }

            write!(f, "]")
        }
    }

    pub struct ImplicitFn {
        vm_f64: vm::VM<f64>,
        vm_f64_vec: vm::VM<vm::F64Vec>,
        vm_range: vm::VM<vm::Range>,
        vm_range_deriv: vm::VM<vm::RangeDeriv>,
        program: Vec<vm::Opcode>,
    }

    impl ImplicitFn {
        pub fn new(program: Vec<vm::Opcode>) -> Self {
            Self {
                vm_f64: vm::VM::with_instr_table(vm::F64InstrTable),
                vm_f64_vec: vm::VM::with_instr_table(vm::F64VecInstrTable),
                vm_range: vm::VM::with_instr_table(vm::RangeInstrTable),
                vm_range_deriv: vm::VM::with_instr_table(vm::RangeDerivInstrTable),
                program,
            }
        }
        #[inline(always)]
        pub fn eval_f64(&mut self, arg: DVec3) -> f64 {
            self.vm_f64.call([arg.x, arg.y, arg.z], &self.program)
        }

        pub fn eval_grad_range(
            &mut self,
            min: DVec3,
            max: DVec3,
        ) -> (vm::Range, vm::Range, vm::Range) {
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

        #[inline(always)]
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

        #[inline(always)]
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
    type LocCode = u64;

    #[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct LocFmt(LocCode);

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
            let y = self.0 >> 16 & 0xFFFF;
            let z = self.0 >> 32 & 0xFFFF;
            write!(f, "{x:x}, {y:x}, {z:x}")
        }
    }

    // level in [0, 15]
    type Level = u8;

    type Direction = u8;

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

    #[inline(always)]
    fn local_octant_bounds(p_bounds: (Vec3, Vec3), oct: u8) -> (Vec3, Vec3) {
        debug_assert_ne!(oct, 0);

        let oct = oct - 1;

        let (p_min, p_max) = p_bounds;
        let half_size = (p_max - p_min) / 2.0;

        let mut min = p_min;
        if oct >> 0 & 1 == 1 {
            min.x += half_size.x
        }
        if oct >> 1 & 1 == 1 {
            min.y += half_size.y
        }
        if oct >> 2 & 1 == 1 {
            min.z += half_size.z
        }

        (min, min + half_size)
    }

    pub fn dbg_loc_code(loc: LocCode) -> String {
        let mut str = String::new();
        let mut i = 0;
        let mut oct = get_octants!(loc, i) as u8;
        while oct != 0 {
            str.push_str(&format!("{oct} "));
            i += 1;
            oct = get_octants!(loc, i) as u8;
        }

        str.trim().to_string()
    }

    // TODO: inline & unroll by hand?
    #[inline(always)]
    fn octant_unit_bounds(mut loc: LocCode) -> (Vec3, Vec3) {
        // let mut bounds = (Vec3::ZERO, Vec3::ONE);
        let mut min = Vec3::ZERO;
        let mut max = Vec3::ONE;

        let depth = cell_depth(loc);

        for i in 0..depth {
            // let oct = ((loc >> ((depth - i) * 4 & 0xF) as u8;
            let oct = ((loc >> (depth - i - 1) * 4) & 0xF) - 1;
            // bounds = local_octant_bounds(bounds, oct as u8);

            let half_size = (max - min) / 2.0;

            if oct >> 0 & 1 == 1 {
                min.x += half_size.x
            }
            if oct >> 1 & 1 == 1 {
                min.y += half_size.y
            }
            if oct >> 2 & 1 == 1 {
                min.z += half_size.z
            }

            max = min + half_size;
        }
        (min, max)
    }

    #[inline(always)]
    pub fn octant_bounds(min: Vec3, max: Vec3, loc: LocCode) -> (Vec3, Vec3) {
        let size = max - min;
        let (u_min, u_max) = octant_unit_bounds(loc);
        (u_min * size + min, u_max * size + min)
    }

    pub fn octant_unit_corners(loc: LocCode) -> [Vec3; 8] {
        let mut out = [Default::default(); 8];
        let (min, max) = octant_unit_bounds(loc);
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

    #[inline(always)]
    pub fn octant_corners(min: Vec3, max: Vec3, loc: LocCode) -> [Vec3; 8] {
        let size = max - min;
        let u_corners = octant_unit_corners(loc);
        u_corners.map(|corner| corner * size + min)
    }

    #[inline(always)]
    pub fn quad_corners(min: Vec3, max: Vec3, loc: LocCode) -> [Vec3; 4] {
        let size = max - min;
        let oct_corners = octant_unit_corners(loc);
        let u_corners = [
            oct_corners[0],
            oct_corners[1],
            oct_corners[2],
            oct_corners[3],
        ];
        u_corners.map(|corner| corner * size + min)
    }

    #[inline(always)]
    pub fn cell_depth(mut loc: LocCode) -> u8 {
        let mut i = 0;
        while loc != 0 {
            i += 1;
            loc >>= 4;
        }
        i
    }

    #[inline(always)]
    pub fn subdivide_octant(loc: LocCode) -> [LocCode; 8] {
        let octs = [1, 2, 3, 4, 5, 6, 7, 8];
        // let depth = octant_depth(loc);
        octs.map(|oct| (loc << 4) | oct)
        // octs.map(|oct| loc | oct << (16 - 1 - depth) * 4)
    }

    #[inline(always)]
    pub fn subdivide_quadrant(loc: LocCode) -> [LocCode; 4] {
        let octs = [1, 2, 3, 4];
        // let depth = octant_depth(loc);
        octs.map(|oct| (loc << 4) | oct)
        // octs.map(|oct| loc | oct << (16 - 1 - depth) * 4)
    }

    macro_rules! set_nth_nibble {
        ($num:expr, $n:expr, $val:expr) => {{
            let shift = $n * 4;
            let mask: u64 = 0xF << shift;
            ($num & !mask) | (($val as u64 & 0xF) << shift)
        }};
    }

    macro_rules! keep_upper_nibbles {
        ($val:expr, $n:expr) => {{
            $val & u64::MAX << (16 - $n) * 4;
        }};
    }

    #[inline(always)]
    pub fn dual_vertex(p1: Vec3, p2: Vec3) -> Vec3 {
        let mid = (p1 + p2) / 2.0;
        mid
    }

    #[derive(Debug, Clone)]
    pub struct NTree {
        pub cells: Vec<LocCode>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct OctCell {
        grad: OrderedFloat<f64>,
        has_undef: bool,
        depth: u32,
        loc: LocCode,
    }

    impl OctCell {
        fn root(loc: LocCode) -> Self {
            Self {
                grad: OrderedFloat(f64::MAX),
                has_undef: false,
                loc,
                depth: 0,
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

            // if self.depth.abs_diff(other.depth) > 1 {
            //     Ord::cmp(&self.depth, &other.depth).reverse()
            // } else {
            //     Ord::cmp(&self.grad, &other.grad)
            // }
            // match Ord::cmp(&self.depth, &other.depth).reverse() {
            //     std::cmp::Ordering::Equal => Ord::cmp(&self.grad, &other.grad),
            //     ord => ord,
            // }
        }
    }
    impl PartialOrd for OctCell {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(Ord::cmp(self, other))
        }
    }

    impl NTree {
        pub fn build_2d(min: Vec2, max: Vec2, depth: u32, f: &mut ImplicitFn, tol: float) -> Self {
            assert!(depth <= MAX_DEPTH as u32);
            let max_cells: u32 = 4u32.pow(depth);

            let min = min.extend(0.0);
            let max = max.extend(0.0);

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

                let (mut o_min, mut o_max) = octant_bounds(min, max, quad.loc);
                o_min.z = 0.0;
                o_max.z = 0.0;
                let range = f.eval_range(o_min.into(), o_max.into());

                if (o_min - o_max).length_squared() < 1e-3 as f32 {
                    let mut quad = quad;
                    quad.has_undef = range.is_undef();
                    quads.push(quad);
                    continue
                }


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

                if grad > tol &&  range.contains_zero() || range.is_undef() {
                    if quad.depth <= depth {
                        cells_todo.extend(subdivide_quadrant(quad.loc).map(|loc| OctCell {
                            // has_undef: range.is_undef(),
                            has_undef: false,
                            grad: grad.into(),
                            depth: quad.depth + 1,
                            loc,
                        }));
                    } 
                } else {
                    let mut quad = quad;
                    quad.has_undef = range.is_undef();
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

            // for c in cells_todo.iter_mut() {
            let cells_todo: Vec<_> = cells_todo.into_iter().map(|mut c| {
                let (mut o_min, mut o_max) = octant_bounds(min, max, c.loc);
                o_min.z = 0.0;
                o_max.z = 0.0;
                let range = f.eval_range(o_min.into(), o_max.into());
                c.has_undef = range.is_undef();
                c
            }).collect();

            quads.extend(cells_todo.into_iter().filter(|c| !c.has_undef));
            // let mut cells: Vec<_> = quads.iter().filter_map(|c| if !c.has_undef { Some(c.loc) } else { None }).collect();
            // let mut cells: Vec<_> = quads.into_iter().chain(cells_todo).map(|c| c.loc).collect();
            let mut cells: Vec<_> = quads.iter().filter_map(|c| if !c.has_undef { Some(c.loc) } else { None }).collect();
            cells.sort();
            cells.reverse();

            Self { cells }
        }

        pub fn build_3d_2(
            min: Vec3,
            max: Vec3,
            depth: u32,
            f: &mut ImplicitFn,
            tol: float,
        ) -> Self {
            assert!(depth <= MAX_DEPTH as u32);
            let max_cells: u32 = 8u32.pow(depth);

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
                let (o_min, o_max) = octant_bounds(min, max, oct.loc);
                let range = f.eval_range(o_min.into(), o_max.into());

                if !range.is_undef() && !range.contains_zero() {
                    continue;
                }

                let (xgrad, ygrad, zgrad) = f.eval_grad_range(o_min.into(), o_max.into());
                let grad = DVec3::new(xgrad.dist(), ygrad.dist(), zgrad.dist());

                if grad.length() > tol && range.contains_zero() || range.is_undef() {
                    // curr_lvl.extend(subdivide_octant(*oct))
                    cells_todo.extend(subdivide_octant(oct.loc).map(|loc| OctCell {
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
            octs.extend(cells_todo.into_iter());
            let cells = octs.iter().map(|c| c.loc).collect();


            Self { cells }
        }

        pub fn build_3d(min: Vec3, max: Vec3, depth: u32, f: &mut ImplicitFn, tol: float) -> Self {
            let mut leafs = vec![];

            let mut buff_1: Vec<LocCode> = vec![1, 2, 3, 4, 5, 6, 7, 8];
            let mut buff_2: Vec<LocCode> = vec![];

            let mut prev_lvl = &mut buff_1;
            let mut curr_lvl = &mut buff_2;

            for _ in 0..depth {
                curr_lvl.clear();

                for oct in prev_lvl.iter() {
                    let (o_min, o_max) = octant_bounds(min, max, *oct);
                    let range = f.eval_range(o_min.into(), o_max.into());

                    let (xgrad, ygrad, zgrad) = f.eval_grad_range(o_min.into(), o_max.into());
                    let grad = DVec3::new(xgrad.dist(), ygrad.dist(), zgrad.dist());

                    if grad.length() > tol && range.contains_zero() || range.is_undef() {
                        curr_lvl.extend(subdivide_octant(*oct))
                    } else if range.contains_zero() {
                        leafs.push(*oct);
                    }
                }

                std::mem::swap(&mut curr_lvl, &mut prev_lvl);
            }
            leafs.extend(prev_lvl.iter());

            Self { cells: leafs }
        }

        // fn generate_line_triangles(graph: &FxHashMap<u128, (Vec<Vec3>, Vec3)>, thickness: f32) -> Vec<[Vec3; 3]> {
        //     let mut tris = Vec::new();
        //     let half_thickness = thickness / 2.0;

        //     for (_, (verts, p)) in graph {
        //         for q in verts {
        //             let up = Vec3::Z;
        //             let dir = q - p;
        //             let w = (dir.cross(up)).normalize() * thickness / 2.0;

        //             let a = p + w;
        //             let c = p - w;
        //             let b = q - w;
        //             let d = q + w;

        //             tris.extend([
        //                 [a, b, c],
        //                 [a, c, d],
        //             ]);
        //         }
        //     }

        //     tris
        // }

        #[inline(always)]
        pub fn cell_in_dir(mut loc: LocCode, dir: Direction) -> LocCode {
            let mut shift = 0;

            loop {
                let mask = 0b1111 << 4 * shift;
                let oct = (loc & mask) >> 4 * shift;
                if oct == 0 {
                    return 0;
                }
                let oct = oct - 1;
                let n = (oct ^ (dir & 0b111) as LocCode) + 1;

                loc &= !(0b1111 << 4 * shift);
                loc |= n << 4 * shift;

                if dir & 0b1000 == 0 {
                    if oct as u8 & dir == 0 {
                        return loc;
                    }
                } else {
                    if oct as u8 & (dir & 0b0111) != 0 {
                        return loc;
                    }
                }

                shift += 1;
            }
        }

        pub fn parent_cell_in_dir(mut loc: LocCode, dir: Direction) -> LocCode {
            let mut shift = 1;
            loop {
                let mask = 0b1111 << 4 * shift;
                let oct = (loc & mask) >> 4 * shift;
                if oct == 0 {
                    return 0;
                }
                let oct = oct - 1;
                let n = (oct ^ (dir & 0b111) as LocCode) + 1;

                loc &= !(0b1111 << 4 * shift);
                loc |= n << 4 * shift;

                if dir & 0b1000 == 0 {
                    if oct as u8 & dir == 0 {
                        return loc;
                    }
                } else {
                    if oct as u8 & (dir & 0b0111) != 0 {
                        return loc;
                    }
                }

                shift += 1;
            }
        }

        pub fn quad_neighbors(q: LocCode) -> [LocCode; 8] {
            let n = Self::cell_in_dir(q, dir::POS_Y);
            let e = Self::cell_in_dir(q, dir::POS_X);
            let s = Self::cell_in_dir(q, dir::NEG_Y);
            let w = Self::cell_in_dir(q, dir::NEG_X);

            let ne = Self::cell_in_dir(n, dir::POS_X);
            let nw = Self::cell_in_dir(n, dir::NEG_X);
            let se = Self::cell_in_dir(s, dir::POS_X);
            let sw = Self::cell_in_dir(s, dir::NEG_X);

            [n, ne, e, se, s, sw, w, nw]
        }

        pub fn quad_parent_neighbors(q: LocCode) -> [LocCode; 8] {
            let n = Self::parent_cell_in_dir(q, dir::POS_Y);
            let e = Self::parent_cell_in_dir(q, dir::POS_X);
            let s = Self::parent_cell_in_dir(q, dir::NEG_Y);
            let w = Self::parent_cell_in_dir(q, dir::NEG_X);

            let ne = Self::parent_cell_in_dir(n, dir::POS_X);
            let nw = Self::parent_cell_in_dir(n, dir::NEG_X);
            let se = Self::parent_cell_in_dir(s, dir::POS_X);
            let sw = Self::parent_cell_in_dir(s, dir::NEG_X);

            [n, ne, e, se, s, sw, w, nw]
        }

        pub fn oct_neighbors(o: LocCode) -> [LocCode; 26] {
            let n_mid = Self::quad_neighbors(o);
            let n_top = n_mid.map(|n| Self::cell_in_dir(n, dir::POS_Z));
            let n_bot = n_mid.map(|n| Self::cell_in_dir(n, dir::NEG_Z));

            let up = Self::cell_in_dir(o, dir::POS_Z);
            let down = Self::cell_in_dir(o, dir::NEG_Z);

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

        fn connect_quads(cells: &FxHashMap<LocCode, Vec3>, thickness: f32) -> Vec<[Vec3; 3]> {
            let mut lines = vec![];
            let mut visited = FxHashSet::<u64>::default();

            let mut max_depth = 0;
            for (cell, _) in cells {
                max_depth = max_depth.max(cell_depth(*cell));
            }

            let mut adaptive_cells = FxHashMap::<LocCode, Vec3>::default();
            for (cell, vert) in cells {
                let mut new_cells = vec![*cell];
                for _ in cell_depth(*cell)..max_depth {
                    new_cells = new_cells.drain(..).flat_map(|q| subdivide_quadrant(q)).collect();
                }

                for c in new_cells {
                    adaptive_cells.insert(c, *vert);
                }
            }
            println!("{}", adaptive_cells.len());

            let todo: Vec<_> = cells.iter().map(|(cell, _)| *cell).collect();

            for cell in todo {
                if visited.contains(&cell) {
                    continue;
                } else {
                    visited.insert(cell);
                }
                let v0 = cells.get(&cell).unwrap();

                for n in Self::quad_neighbors(cell) {
                    if n == 0 {
                        continue;
                    }
                    if let Some(v1) = cells.get(&n) {
                        lines.push((*v0, *v1));
                    } else {
                        let mut tmp = vec![n];
                        for _ in cell_depth(n)..max_depth {
                            tmp = tmp.drain(..).flat_map(|q| subdivide_quadrant(q)).collect();
                        }

                        for m in tmp {
                            if let Some(v1) = cells.get(&m) {
                                lines.push((*v0, *v1));
                            }
                        }
                    }
                }

                // if lines.len() <= 2 {
                //     // for dir in [dir::POS_X, dir::NEG_X, dir::POS_Y, dir::NEG_Y] {
                //     //     let n = Self::parent_cell_in_dir(cell, dir);
                //     for n in Self::quad_parent_neighbors(cell) {
                //         if let Some(v1) = cells.get(&n) {
                //             lines.push((*v0, *v1));
                //         }
                //     }
                // }
            }
            // for cell in cells {
            //     if visited.contains(&cell) {
            //         continue;
            //     } else {
            //         visited.insert(cell);
            //     }
            // }

            Self::lines_to_triangles(&lines, thickness)
        }

        pub fn lines_to_triangles(segments: &[(Vec3, Vec3)], thickness: f32) -> Vec<[Vec3; 3]> {
            let mut triangles = Vec::new();

            for &(a, b) in segments {
                let dir = b - a;
                if dir.length_squared() < 1e-6 {
                    continue;
                }

                // let perp = Self::find_perpendicular(dir).normalize() * thickness;
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

        fn find_perpendicular(v: Vec3) -> Vec3 {
            let up = Vec3::Z;
            let perp = v.cross(up);

            if perp.length_squared() < 1e-6 {
                v.cross(Vec3::X)
            } else {
                perp
            }
        }

        fn quad_edges(quad: u64) -> [u128; 4] {
            let c = quad_corner_locations(quad);
            [
                make_u128_edge(c[0], c[1]),
                make_u128_edge(c[1], c[2]),
                make_u128_edge(c[2], c[3]),
                make_u128_edge(c[3], c[0]),
            ]
        }

        pub fn dual_contour_2d(
            &self,
            min: Vec2,
            max: Vec2,
            mut f: &mut ImplicitFn,
        ) -> Vec<[Vec3; 3]> {
            // let mut tris = vec![];
            let min = min.extend(0.0);
            let max = max.extend(0.0);

            let mut max_depth = 0;
            for c in &self.cells {
                max_depth = max_depth.max(cell_depth(*c));
            }

            // let mut graph = FxHashMap::<u128, (Vec<(u64, Vec3)>, Vec3)>::default();
            let mut verts = FxHashMap::<LocCode, Vec3>::default();

            for quad in &self.cells {
                let sp = quad_corners(min, max, *quad).map(|pos| SurfacePoint {
                    pos,
                    val: f.eval_f64(pos.as_dvec3()),
                });

                let corner_codes = quad_corner_locations(*quad);
                let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];

                // let mut intersects = vec![];

                let mut intersects = vec![];

                for edge in edges {
                    let (v1, v2) = (sp[edge.0], sp[edge.1]);
                    if v2.val.signum() != v1.val.signum()
                    {
                        let (c1, c2) = (corner_codes[edge.0], corner_codes[edge.1]);
                        let e = make_u128_edge(c1, c2);
                        intersects.push(zero_cross(v1, v2, f));
                        // intersects.push((*quad, (e, zero_cross(v1, v2, f))));
                    }
                }

                if !intersects.is_empty() {
                    let mut vert = Vec3::ZERO;
                    for i in &intersects {
                        vert += i;
                    }
                    vert /= intersects.len() as f32;
                    // let (min, max) = octant_bounds(min, max, *quad);
                    // let vert = vert.clamp

//                     let mut balanced_cells = vec![*quad];
//                     for _ in cell_depth(*quad)..max_depth {
//                         balanced_cells = balanced_cells.drain(..).flat_map(|q| subdivide_quadrant(q)).collect();
//                     }

//                     for c in balanced_cells {
//                         verts.insert(c, vert);
//                     }
                    verts.insert(*quad, vert);
                }
            }

            Self::connect_quads(&verts, 0.01)

            // let lines: Vec<_> = graph.into_iter().flat_map(|(_, (verts, intersec))| {
            //     verts.into_iter().map(move |v| (v, intersec))
            // }).collect();

            // Self::lines_to_triangles(&lines, 0.1)
            // Self::generate_line_triangles(&graph, 0.01)
            //Self::connect_edges(&graph, 0.01)
        }

        #[inline(never)]
        pub fn dual_contour(&self, min: Vec3, max: Vec3, mut f: &mut ImplicitFn) -> Vec<[Vec3; 3]> {
            let mut tris = vec![];

            let dmin = min.as_dvec3();
            let dmax = max.as_dvec3();

            for oct in &self.cells {
                let c: [SurfacePoint; 8] = corner_locations(*oct)
                    .map(|c| corner_position(c, dmin, dmax))
                    .map(|pos| SurfacePoint {
                        pos: pos.as_vec3(),
                        val: f.eval_f64(pos),
                    });

                const EDGES: [(usize, usize); 12] = [
                    (0, 1),
                    (1, 2),
                    (2, 3),
                    (3, 0),
                    (4, 5),
                    (5, 6),
                    (6, 7),
                    (7, 4),
                    (0, 4),
                    (1, 5),
                    (2, 6),
                    (3, 7),
                ];

                let mut intersects = vec![];

                for (i, j) in EDGES {
                    let (a, b) = (c[i], c[j]);

                    if a.val * b.val < 0.0 {
                        intersects.push(zero_cross(a, b, f));
                    }
                }

                if !intersects.is_empty() {
                    let cell_vert = intersects.iter().fold(Vec3::ZERO, |acc, p| acc + p)
                        / intersects.len() as f32;

                    for i in 1..intersects.len() - 1 {
                        tris.push([cell_vert, intersects[i], intersects[i + 1]])
                    }
                }
            }

            tris
        }

        pub fn march_tetrahedra(
            &self,
            min: Vec3,
            max: Vec3,
            mut f: &mut ImplicitFn,
        ) -> Vec<[Vec3; 3]> {
            // let mut tetras = vec![];
            let mut tris = vec![];

            let mut corner_points = vec![];
            for oct in &self.cells {
                corner_points.extend(octant_corners(min, max, *oct).map(|vec| vec.as_dvec3()));
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

                let vol_dual = SurfacePoint::new(dual_vertex(c[0].pos, c[7].pos), &mut f);

                let faces = [
                    [c[0], c[2], c[3], c[1]],
                    [c[0], c[1], c[5], c[4]],
                    [c[0], c[4], c[6], c[2]],
                    [c[4], c[6], c[7], c[5]],
                    [c[2], c[3], c[7], c[6]],
                    [c[1], c[5], c[7], c[3]],
                ];

                for [f0, f1, f2, f3] in faces {
                    let face_dual = SurfacePoint::new(dual_vertex(f0.pos, f2.pos), &mut f);

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
    pub type Corner = u64;

    fn make_u128_edge(mut v1: u64, mut v2: u64) -> u128 {
        if v1 < v2 {
            (v1, v2) = (v2, v1);
        }
        (v1 as u128) | (v2 as u128) << 8
    }

    fn corners_from_edge(edge: u128) -> (u64, u64) {
        (edge as u64, (edge >> 8) as u64)
    }

    fn octant_min_corner(mut loc: u64) -> Corner {
        let mut lvl = 1;
        // TODO only max depth 15!!!
        let mut oct_size = 1 << MAX_DEPTH;

        let mut c_x = 0 as Corner;
        let mut c_y = 0 as Corner;
        let mut c_z = 0 as Corner;

        let mut depth = cell_depth(loc);
        debug_assert!(depth <= MAX_DEPTH);

        for i in 0..depth {
            let octs = (loc >> (depth - 1 - i) * 4 & 0xF) - 1;

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

        let c_loc = c_x | c_y << 16 | c_z << 32;
        c_loc
    }

    fn octant_min_corner_pos(loc: u64) -> DVec3 {
        let depth = cell_depth(loc);
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

    fn corner_unit_position(corner_code: u64) -> DVec3 {
        let mask: u64 = 0xFFFF;
        let c_x = (corner_code & mask) as f64;
        let c_y = ((corner_code >> 16) & mask) as f64;
        let c_z = ((corner_code >> 32) & mask) as f64;
        let resolution = (1 << MAX_DEPTH) as f64;
        DVec3::new(c_x / resolution, c_y / resolution, c_z / resolution)
    }

    #[inline(always)]
    pub fn corner_position(c: Corner, min: DVec3, max: DVec3) -> DVec3 {
        let pos = corner_unit_position(c);
        let size = max - min;
        pos * size + min
    }

    #[inline(always)]
    pub fn quad_corner_locations(loc: u64) -> [Corner; 4] {
        let min_corner = octant_min_corner(loc);
        let depth = cell_depth(loc);
        let oct_size = 1 << (MAX_DEPTH - depth);

        let min_x = min_corner & 0xFFFF;
        let min_y = (min_corner >> 16) & 0xFFFF;

        [
            min_x | (min_y << 16),
            (min_x + oct_size) | (min_y << 16),
            min_x | ((min_y + oct_size) << 16),
            (min_x + oct_size) | ((min_y + oct_size) << 16),
        ]
    }

    #[inline(always)]
    pub fn corner_locations(loc: u64) -> [Corner; 8] {
        let min_corner = octant_min_corner(loc);
        let depth = cell_depth(loc);
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
        #[inline(always)]
        fn new(pos: Vec3, f: &mut ImplicitFn) -> Self {
            let val = f.eval_f64(pos.into());
            Self { pos, val }
        }
    }

    #[inline(always)]
    pub fn zero_cross(p1: SurfacePoint, p2: SurfacePoint, f: &mut ImplicitFn) -> Vec3 {
        let denom = p1.val - p2.val;
        let k1 = -p2.val / denom;
        let k2 = p1.val / denom;
        k1 as f32 * p1.pos + k2 as f32 * p2.pos
    }

    #[inline(always)]
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

    pub fn build_3d(
        min: Vec3,
        max: Vec3,
        min_depth: u32,
        program: &[op::Opcode],
        tol: f64,
    ) -> (Vec<[Vec3; 3]>, NTree) {
        let mut f = ImplicitFn::new(program.to_vec());
        let tree = NTree::build_3d_2(min, max, min_depth, &mut f, tol);
        let tris = tree.march_tetrahedra(min, max, &mut f);
        (tris, tree)
    }

    pub fn build_2d(
        min: Vec2,
        max: Vec2,
        min_depth: u32,
        program: &[op::Opcode],
        tol: f64,
    ) -> (Vec<[Vec3; 3]>, NTree) {
        let mut f = ImplicitFn::new(program.to_vec());
        let tree = NTree::build_2d(min, max, min_depth, &mut f, tol);
        let tris = tree.dual_contour_2d(min, max, &mut f);
        (tris, tree)
    }
}

/*
pub mod line {
    use glam::Vec2;
    use ordered_float::OrderedFloat;
    use std::{collections::HashMap, ops, rc::Rc};

    use crate::vm::{self, float, op};

    use super::{CellCorners, CellPtr, ImplicitFn};

    type QuadTree = super::QuadTree<2>;
    type EvalPoint = super::EvalPoint<2>;
    type IsoVec = super::IsoVec<2>;
    type Cell = super::Cell<2>;

    #[derive(Debug, Clone, Copy)]
    struct Triangle {
        verts: [EvalPoint; 3],
        next: Option<TriPtr>,
        next_bisec_point: Option<EvalPoint>,
        prev: Option<TriPtr>,
        visited: bool,
    }

    impl Triangle {
        fn new(p1: EvalPoint, p2: EvalPoint, p3: EvalPoint) -> Self {
            Self {
                verts: [p1, p2, p3],
                next: None,
                next_bisec_point: None,
                prev: None,
                visited: false,
            }
        }

        fn triangle_4(
            a: EvalPoint,
            b: EvalPoint,
            c: EvalPoint,
            d: EvalPoint,
            mid: EvalPoint,
        ) -> [Triangle; 4] {
            [
                Triangle::new(a, b, mid),
                Triangle::new(b, c, mid),
                Triangle::new(c, d, mid),
                Triangle::new(d, a, mid),
            ]
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Point(OrderedFloat<float>, OrderedFloat<float>);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TriPtr(usize);

    impl TriPtr {
        const ROOT: Self = TriPtr(0);
    }

    struct Triangulator<'a> {
        triangles: Vec<Triangle>,
        f: &'a mut ImplicitFn<2>,
        tol: float,
        tree: QuadTree,
        hanging_next: HashMap<Point, TriPtr>,
    }

    impl ops::Index<TriPtr> for Triangulator<'_> {
        type Output = Triangle;

        fn index(&self, index: TriPtr) -> &Self::Output {
            &self.triangles[index.0]
        }
    }

    impl ops::IndexMut<TriPtr> for Triangulator<'_> {
        fn index_mut(&mut self, index: TriPtr) -> &mut Self::Output {
            &mut self.triangles[index.0]
        }
    }

    impl Triangulator<'_> {
        pub fn insert(&mut self, t: Triangle) -> TriPtr {
            let ptr = self.triangles.len();
            self.triangles.push(t);
            TriPtr(ptr)
        }

        pub fn tri_inside(&mut self, c_p: CellPtr) {
            let c = self.tree[c_p];
            if let Some(children) = c.children {
                for c in children.as_ref() {
                    self.tri_inside(*c);
                }

                self.tri_crossing_row(children[0], children[1]);
                self.tri_crossing_row(children[2], children[3]);
                self.tri_crossing_col(children[0], children[2]);
                self.tri_crossing_col(children[1], children[3]);
            }
        }

        pub fn tri_crossing_row(&mut self, a_p: CellPtr, b_p: CellPtr) {
            let a = self.tree[a_p];
            let b = self.tree[b_p];

            if let (Some(c1), Some(c2)) = (a.children, b.children) {
                self.tri_crossing_row(c1[1], c2[0]);
                self.tri_crossing_row(c1[3], c2[2]);
            } else if let Some(c) = a.children {
                self.tri_crossing_row(c[1], b_p);
                self.tri_crossing_row(c[3], b_p);
            } else if let Some(c) = b.children {
                self.tri_crossing_row(a_p, c[0]);
                self.tri_crossing_row(a_p, c[2]);
            } else {
                let fd_a = EvalPoint::get_dual(&a.verts, self.f);
                let fd_b = EvalPoint::get_dual(&b.verts, self.f);

                let tris = if a.depth < b.depth {
                    let ed = self.edge_dual(b.verts[2], b.verts[0]);
                    Triangle::triangle_4(b.verts[2], fd_b, b.verts[0], fd_a, ed)
                } else {
                    let ed = self.edge_dual(a.verts[3], a.verts[1]);
                    Triangle::triangle_4(a.verts[3], fd_b, a.verts[1], fd_a, ed)
                };

                self.add_4_tris(tris);
            }
        }

        pub fn tri_crossing_col(&mut self, a_p: CellPtr, b_p: CellPtr) {
            let a = self.tree[a_p];
            let b = self.tree[b_p];

            if let (Some(c1), Some(c2)) = (a.children, b.children) {
                self.tri_crossing_col(c1[2], c2[0]);
                self.tri_crossing_col(c1[3], c2[1]);
            } else if let Some(c) = a.children {
                self.tri_crossing_col(c[2], b_p);
                self.tri_crossing_col(c[3], b_p);
            } else if let Some(c) = b.children {
                self.tri_crossing_col(a_p, c[0]);
                self.tri_crossing_col(a_p, c[1]);
            } else {
                let fd_a = EvalPoint::get_dual(&a.verts, self.f);
                let fd_b = EvalPoint::get_dual(&b.verts, self.f);

                let tris = if a.depth < b.depth {
                    let ed = self.edge_dual(b.verts[0], b.verts[1]);
                    Triangle::triangle_4(b.verts[0], fd_b, b.verts[1], fd_a, ed)
                } else {
                    let ed = self.edge_dual(a.verts[2], a.verts[3]);
                    Triangle::triangle_4(a.verts[2], fd_b, a.verts[3], fd_a, ed)
                };

                self.add_4_tris(tris);
            }
        }

        fn add_4_tris(&mut self, triangles: [Triangle; 4]) {
            let tris = triangles.map(|t| self.insert(t));

            for i in 0..4 {
                self.next_sandwich_tri(tris[i], tris[(i + 1) % 4], tris[(i + 2) % 4]);
            }
        }

        fn set_next(&mut self, t1: TriPtr, t2: TriPtr, pos: EvalPoint, neg: EvalPoint) {
            if !(pos.val > 0.0 && 0.0 >= neg.val) {
                return;
            }

            let int = EvalPoint::find_zero(pos, neg, self.f, self.tol);

            self[t1].next_bisec_point = Some(int);
            self[t1].next = Some(t2);

            self[t2].prev = Some(t1);

            //t1.next_bisec_point = int;
            //t1.next = t2;
        }

        fn next_sandwich_tri(&mut self, a_p: TriPtr, b_p: TriPtr, c_p: TriPtr) {
            let b = &self[b_p];

            let mid = b.verts[2];
            let x = b.verts[0];
            let y = b.verts[1];

            if mid.val > 0.0 && 0.0 >= y.val {
                self.set_next(b_p, c_p, mid, y);
            }

            if x.val > 0.0 && 0.0 >= mid.val {
                self.set_next(b_p, a_p, x, mid)
            }

            let id = x.pos + y.pos;
            let id = Point(id[0].into(), id[1].into());

            if y.val > 0.0 && 0.0 >= x.val {
                if self.hanging_next.contains_key(&id) {
                    let t = self.hanging_next.remove(&id).unwrap();
                    self.set_next(b_p, t, y, x);
                } else {
                    self.hanging_next.insert(id, b_p);
                }
            } else if y.val <= 0.0 && 0.0 < x.val {
                if self.hanging_next.contains_key(&id) {
                    let t = self.hanging_next.remove(&id).unwrap();
                    self.set_next(t, b_p, x, y);
                } else {
                    self.hanging_next.insert(id, b_p);
                }
            }
        }

        pub fn edge_dual(&mut self, p1: EvalPoint, p2: EvalPoint) -> EvalPoint {
            if (p1.val > 0.0) != (p2.val > 0.0) {
                EvalPoint::midpoint(p1, p2, self.f)
            } else {
                let dt = 0.001;

                let df1 = self.f.eval_f64(p1.pos * (1.0 - dt) + p2.pos * dt);
                let df2 = self.f.eval_f64(p1.pos + p2.pos * (1.0 - dt));

                if (df1 > 0.0) == (df2 > 0.0) {
                    EvalPoint::midpoint(p1, p2, self.f)
                } else {
                    let v1 = EvalPoint {
                        pos: p1.pos,
                        val: df1,
                    };
                    let v2 = EvalPoint {
                        pos: p2.pos,
                        val: df2,
                    };
                    EvalPoint::zero_intersect(v1, v2, self.f)
                }
            }
        }

        pub fn trace(&mut self) -> Vec<Vec<Vec2>> {
            let mut curves = vec![];

            for t_p in 0..self.triangles.len() {
                let tri = self.triangles[t_p];
                if !tri.visited && tri.next.is_some() {
                    let mut active_curve = vec![];
                    //self.march_triangle(TriPtr(t_p));
                    Self::march_triangle(&mut self.triangles, t_p, &mut active_curve);

                    curves.push(active_curve);
                }
            }

            curves
                .into_iter()
                .map(|curve| {
                    curve
                        .into_iter()
                        .map(|p| Vec2::new(p.pos[0] as f32, p.pos[1] as f32))
                        .collect()
                })
                .collect()
        }

        pub fn march_triangle(
            tris: &mut Vec<Triangle>,
            t_p: usize,
            active_curve: &mut Vec<EvalPoint>,
        ) {
            let start_tri = t_p;
            let mut tri = &mut tris[t_p];
            let mut closed_loop = false;

            while let Some(prev) = tri.prev {
                tri = &mut tris[prev.0];
                if prev.0 == start_tri {
                    closed_loop = true;
                    break;
                }
            }

            while !tri.visited {
                if let Some(nbp) = tri.next_bisec_point {
                    active_curve.push(nbp);
                }

                tri.visited = true;

                if let Some(t) = tri.next {
                    tri = &mut tris[t.0];
                } else {
                    break;
                }
            }

            if closed_loop {
                active_curve.push(active_curve[0]);
            }
        }

        /*
        pub fn march_triangle(&mut self, t_p: TriPtr) {
            let start_tri = t_p;
            let mut tri = self[t_p];
            let mut closed_loop = false;

            while let Some(prev) = tri.prev {
                tri = self[prev];
                if prev == start_tri {
                    closed_loop = true;
                    break
                }
            }

            while !tri.visited {
                if let Some(nbp) = tri.next_bisec_point {
                    self.active_curve.push(nbp);
                }

                tri.visited = true;

                if let Some(t) = tri.next {
                    tri = self[t];
                } else {
                    break
                }
            }

            if closed_loop {
                self.active_curve.append(self.active_curve[0]);
            }
        }
        */
    }

    pub fn build(
        min: Vec2,
        max: Vec2,
        min_depth: u32,
        max_cells: u32,
        // implicit_fn: impl Fn(Vec2) -> float,
        program: &[op::Opcode],
        tol: float,
    ) -> (Vec<Vec<Vec2>>, QuadTree) {
        //let f = |v: IsoVec| implicit_fn(v.into());

        // let program = vec![
        //     op::POW_IMM_RHS(3.0, 1, 1),
        //     op::SIN(1, 1),
        //     op::POW_LHS_IMM(1, -1.0, 1),
        //     op::SUB_LHS_RHS(1, 2, 1),
        //     op::SIN(2, 2),
        //     op::ADD_LHS_RHS(1, 2, 1),
        //     op::EXT(0),
        // ];

        let mut f = ImplicitFn {
            program: program.to_vec(),
            vm: vm::VM::new(),
        };

        let tree = QuadTree::build(min.into(), max.into(), min_depth, max_cells, tol, &mut f);

        let mut triangulator = Triangulator {
            triangles: vec![],
            f: &mut f,
            tol,
            tree,
            hanging_next: Default::default(),
        };

        triangulator.tri_inside(CellPtr::ROOT);
        let points = triangulator.trace();
        (points, triangulator.tree)
    }
}
*/

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn quad_neighbors() {
        let quad = 0b0001_0100;
        for n in v3::NTree::get_quad_neighbors(quad) {
            println!("{}, {}", n >> 4, n & 0b1111);
            // println!("{n:0b}");
        }
        panic!()
    }
}
