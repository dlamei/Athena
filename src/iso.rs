use std::{collections::VecDeque, ops};

use crate::vm::{self, float};

////https://people.engr.tamu.edu/schaefer/research/iso_simplicial.pdf

pub mod v3 {
    use std::{
        collections::{HashMap, HashSet, VecDeque},
        fmt, ops,
    };

    use glam::{DVec3, Vec3};

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

    const DIR_X: Direction = 0b0001;
    const DIR_Y: Direction = 0b0010;
    const DIR_Z: Direction = 0b0100;

    const DIR_MIN_X: Direction = 0b1001;
    const DIR_MIN_Y: Direction = 0b1010;
    const DIR_MIN_Z: Direction = 0b1100;

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

        let depth = octant_depth(loc);

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

    // #[inline(always)]
    // pub fn octant_depth(loc: LocCode) -> u8 {
    //     let mut i = 0;
    //     let mut oct = get_octants!(loc, i) as u8;
    //     while oct != 0 {
    //         i += 1;
    //         oct = get_octants!(loc, i) as u8;
    //     }
    //     i
    // }

    #[inline(always)]
    pub fn octant_depth(mut loc: LocCode) -> u8 {
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

    /*
    #[inline(always)]
    pub fn same_level_neighbor(mut loc: LocCode, dir: Direction) -> LocCode {
        let depth = octant_depth(loc);

        for i in (1..=depth).rev() {
            let oct = get_octants!(loc, i - 1) as u8 - 1;
            let next_oct = (oct ^ dir) + 1;
            loc = set_nth_nibble!(loc, 16 - i, next_oct);

            if dir & 0b1000 == 0 {
                if oct & dir == 0 {
                    // neighbor is in current octant
                    return loc;
                }
            } else {
                if oct & (dir & 0b0111) != 0 {
                    // neighbor is in current octant
                    return loc;
                }
            }
        }

        // no neighbor exists
        0
    }
    */

    /*
    #[inline(always)]
    pub fn next_octant(mut loc: LocCode) -> LocCode {
        let depth = octant_depth(loc);

        for i in (1..=depth).rev() {
            let oct = get_octants!(loc, i - 1) as u8;

            let next_oct = (oct % 8) + 1;
            loc = set_nth_nibble!(loc, 16 - i, next_oct);

            if oct != 8 {
                return loc;
            }
        }
        0
    }
    */

    /*
    #[inline(always)]
    pub fn find_closest_octant(mut loc: LocCode, mat: &[LocCode]) -> LocCode {

        let end = mat.partition_point(|x| *x < loc);
        if mat[end] == loc {
            return loc
        }

        let mut mat = &mat[..end];

        while loc > 0 {
            loc >>= 4;

            let end = mat.partition_point(|x| *x < loc);
            if mat[end] == loc {
                return loc
            }

            mat = &mat[..end];
        }

        0
    }
    */

    /*
    pub fn find_le_neighbor(loc: LocCode, dir: Direction, mat: &[LocCode]) -> Vec<LocCode> {
        let neighbor = same_level_neighbor(loc, dir);
        if neighbor == 0 {
            return vec![0];
        }

        let start = mat.partition_point(|&x| x < neighbor);
        if mat[start] == neighbor {
            return vec![neighbor];
        }

        let next_oct = next_octant(loc);
        let end = mat[start..].partition_point(|&x| x < loc) + start;

        let depth = octant_depth(loc);
        let neighbors = mat[start..end]
            .iter()
            .copied()
            .filter(|x| {
                let mut x = x << depth * 4;

                while x != 0 {
                    let octs = (x >> (16 - 1) * 4) as u8 - 1;

                    if dir & 0b1000 == 0 {
                        if octs & dir == 0 {
                            // neighbor is in current octant
                            return false;
                        }
                    } else {
                        if octs & (dir & 0b0111) != 0 {
                            // neighbor is in current octant
                            return false;
                        }
                    }

                    x <<= 4;
                }

                true
            })
            .collect();

        neighbors
    }

    pub fn find_eq_neighbor(loc: LocCode, dir: Direction, mat: &[LocCode]) -> LocCode {
        let neighbor = same_level_neighbor(loc, dir);
        if neighbor == 0 {
            return 0;
        }
        if let Ok(_) = mat.binary_search(&loc) {
            loc
        } else {
            0
        }
    }

    pub fn find_ge_neighbor(loc: LocCode, dir: Direction, mat: &[LocCode]) -> LocCode {
        let neighbor = same_level_neighbor(loc, dir);
        if neighbor == 0 {
            return 0;
        }

        let ge_neighbor = find_ge_octant(loc, mat);
        ge_neighbor
    }

    pub fn find_ge_octant(mut loc: LocCode, mat: &[LocCode]) -> LocCode {
        let end = mat.partition_point(|&x| x < loc);
        if mat[end] == loc {
            return loc;
        }

        let mut part = &mat[..end];
        let depth = octant_depth(loc);

        for i in (1..=depth).rev() {
            loc = set_nth_nibble!(loc, 16 - i, 0);

            let end = part.partition_point(|&x| x < loc);
            if part[end] == loc {
                return loc;
            }
            part = &part[..end];
        }

        loc
    }
    */

    /*
    #[inline(always)]
    pub fn find_neighbor(loc: LocCode, dir: Direction, mat: &[LocCode]) -> LocCode {
        let same_lvl_neighbor = same_level_neighbor(loc, dir);

        if same_lvl_neighbor == 0 {
            println!("loc: {}, same_lvl: NONE", LocFmt(loc));
            return 0
        }

        let end = mat.partition_point(|&x| x < same_lvl_neighbor);

        println!("loc: {}, same_lvl: {}", LocFmt(loc), LocFmt(same_lvl_neighbor));
        if mat[end] == same_lvl_neighbor {
            return loc
        }

        return 0;

        let mut part = &mat[..end];

        let mut neighbor = same_lvl_neighbor;
        let depth = octant_depth(neighbor);
        // search for larger neighbor
        for i in (1..=depth).rev() {
            neighbor = set_nth_nibble!(neighbor, 16-i, 0);

            let end = part.partition_point(|&x| x < neighbor);
            if part[end] == neighbor {
                return neighbor
            }
            part = &part[..end];
        }


        // mat[start..] > same_lvl

        0
    }
    */

    #[inline(always)]
    pub fn dual_vertex(p1: Vec3, p2: Vec3) -> Vec3 {
        let mid = (p1 + p2) / 2.0;
        mid
    }

    #[derive(Debug, Clone)]
    pub struct NTree {
        pub cells: Vec<LocCode>,
    }

    impl NTree {
        pub fn build_3d_2(
            mut min: Vec3,
            mut max: Vec3,
            depth: u32,
            f: &mut ImplicitFn,
            tol: float,
        ) -> Self {
            let mut leafs = vec![1, 2, 3, 4, 5, 6, 7, 8];
            // leafs.extend(subdivide_octant(1));
            // let mut leafs: Vec<LocCode> = vec![1, 2, 3, 4, 5, 6, 7, 8];

            for _ in 0..depth {
                leafs = leafs
                    .into_iter()
                    .flat_map(|oct| subdivide_octant(oct))
                    .collect();
            }

            Self { cells: leafs }
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
                    } else {
                        leafs.push(*oct);
                    }
                }

                std::mem::swap(&mut curr_lvl, &mut prev_lvl);
            }
            leafs.extend(prev_lvl.iter());

            Self { cells: leafs }
        }

        #[inline(never)]
        pub fn march_tetrahedra2(
            &self,
            min: Vec3,
            max: Vec3,
            mut f: &mut ImplicitFn,
        ) -> Vec<[Vec3; 3]> {
            // let mut tetras = vec![];
            let mut tris = vec![];

            let dmin = min.as_dvec3();
            let dmax = max.as_dvec3();

            let corner_lookup: HashMap<_, _> = {
                let mut corners = HashSet::new();
                for oct in &self.cells {
                    let locs = corner_locations(*oct);
                    corners.extend(locs);
                }

                let pos: Vec<_> = corners
                    .iter()
                    .map(|c| corner_position(*c, dmin, dmax))
                    .collect();

                let surf_points: Vec<_> = f
                    .eval_f64_vec(pos.clone())
                    .into_iter()
                    .zip(pos)
                    .map(|(val, pos)| SurfacePoint {
                        pos: pos.as_vec3(),
                        val,
                    })
                    .collect();

                corners.into_iter().zip(surf_points).collect()
            };

            let dvol_lookup: HashMap<_, _> = {
                let mut corners = HashSet::new();
                for oct in &self.cells {
                    let locs = corner_locations(*oct);
                    let a = locs[0].min(locs[7]);
                    let b = locs[0].max(locs[7]);
                    corners.insert((a, b));
                }
                let dpos: Vec<_> = corners
                    .iter()
                    .map(|c| {
                        let a = corner_position(c.0, dmin, dmax);
                        let b = corner_position(c.1, dmin, dmax);
                        (a + b) / 2.0
                    })
                    .collect();

                let dvols: Vec<_> = f
                    .eval_f64_vec(dpos.clone())
                    .into_iter()
                    .zip(dpos)
                    .map(|(val, pos)| SurfacePoint {
                        pos: pos.as_vec3(),
                        val,
                    })
                    .collect();

                corners.into_iter().zip(dvols).collect()
            };

            let dface_lookup: HashMap<_, _> = {
                // let mut lookup = HashMap::new();
                let mut corners = HashSet::new();
                for oct in &self.cells {
                    let coords = corner_locations(*oct);
                    let pos = coords.map(|corner| corner_position(corner, dmin, dmax).as_vec3());

                    let indxs = [[0, 3], [0, 5], [0, 6], [4, 7], [2, 7], [1, 7]];

                    for [i1, i3] in indxs {
                        let a = coords[i1].min(coords[i3]);
                        let b = coords[i1].max(coords[i3]);
                        corners.insert((a, b));
                        // let face_dual = SurfacePoint::new(dual_vertex(pos[i1], pos[i3]), &mut f);
                        // lookup.insert((a, b), face_dual);
                    }
                }

                let dpos: Vec<_> = corners
                    .iter()
                    .map(|c| {
                        let a = corner_position(c.0, dmin, dmax);
                        let b = corner_position(c.1, dmin, dmax);
                        (a + b) / 2.0
                    })
                    .collect();

                let dfaces: Vec<_> = f
                    .eval_f64_vec(dpos.clone())
                    .into_iter()
                    .zip(dpos)
                    .map(|(val, pos)| SurfacePoint {
                        pos: pos.as_vec3(),
                        val,
                    })
                    .collect();

                corners.into_iter().zip(dfaces).collect()
            };

            for oct in &self.cells {
                let mut c = [SurfacePoint::default(); 8];

                let corner_loc = corner_locations(*oct);
                for i in 0..8 {
                    c[i] = *corner_lookup.get(&corner_loc[i]).unwrap();
                }

                //let vol_dual = SurfacePoint::new(dual_vertex(c[0].pos, c[7].pos), &mut f);
                let val_dual_corner = {
                    let a = corner_loc[0].min(corner_loc[7]);
                    let b = corner_loc[0].max(corner_loc[7]);
                    (a, b)
                };
                let vol_dual = *dvol_lookup.get(&val_dual_corner).unwrap();

                let faces = [
                    [c[0], c[2], c[3], c[1]],
                    [c[0], c[1], c[5], c[4]],
                    [c[0], c[4], c[6], c[2]],
                    [c[4], c[6], c[7], c[5]],
                    [c[2], c[3], c[7], c[6]],
                    [c[1], c[5], c[7], c[3]],
                ];

                let face_indx = [
                    [0, 2, 3, 1],
                    [0, 1, 5, 4],
                    [0, 4, 6, 2],
                    [4, 6, 7, 5],
                    [2, 3, 7, 6],
                    [1, 5, 7, 3],
                ];

                for [i0, i1, i2, i3] in face_indx {
                    let a = corner_loc[i0].min(corner_loc[i2]);
                    let b = corner_loc[i0].max(corner_loc[i2]);

                    let f0 = c[i0];
                    let f1 = c[i1];
                    let f2 = c[i2];
                    let f3 = c[i3];

                    let face_dual = *dface_lookup.get(&(a, b)).unwrap();
                    // let face_dual = SurfacePoint::new(dual_vertex(f0.pos, f2.pos), &mut f);
                    // let face_dual = SurfacePoint {
                    //     pos: dual_vertex(f0.pos, f2.pos),
                    //     val: (f0.val + f2.val) / 2.0,
                    // };

                    let edges = [[f0, f1], [f1, f2], [f2, f3], [f3, f0]];

                    for [e0, e1] in edges {
                        let tetra = [e0, e1, face_dual, vol_dual];

                        march_tetrahedron(tetra, f, &mut tris);
                    }
                }

                // for [f0, f1, f2, f3] in faces {
                //     let face_dual = SurfacePoint::new(dual_vertex(f0.pos, f2.pos), &mut f);

                //     let edges = [[f0, f1], [f1, f2], [f2, f3], [f3, f0]];

                //     for [e0, e1] in edges {
                //         let tetra = [e0, e1, face_dual, vol_dual];

                //         march_tetrahedron(tetra, f, &mut tris);
                //     }
                // }
            }

            tris
        }

        #[inline(never)]
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
                // let c = octant_corners(min, max, *oct).map(|p| SurfacePoint::new(p, &mut f));

                /*
                let oct_corners = octant_corners(min, max, *oct);
                let mut c = [SurfacePoint::default(); 8];
                for i in 0..8 {
                    c[i] = SurfacePoint { pos: oct_corners[i], val: eval[i] }
                }
                */

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

    fn octant_min_corner(mut loc: u64) -> Corner {
        let mut lvl = 1;
        // TODO only max depth 15!!!
        let mut oct_size = 1 << 15;

        let mut c_x = 0 as Corner;
        let mut c_y = 0 as Corner;
        let mut c_z = 0 as Corner;

        let mut depth = octant_depth(loc);
        debug_assert!(depth <= 15);

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
        let depth = octant_depth(loc);
        debug_assert!(depth <= 15);

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
        let resolution = (1 << 15) as f64;
        DVec3::new(c_x / resolution, c_y / resolution, c_z / resolution)
    }

    #[inline(always)]
    pub fn corner_position(c: Corner, min: DVec3, max: DVec3) -> DVec3 {
        let pos = corner_unit_position(c);
        let size = max - min;
        pos * size + min
    }

    #[inline(always)]
    pub fn corner_locations(loc: u64) -> [Corner; 8] {
        let min_corner = octant_min_corner(loc);
        let depth = octant_depth(loc);
        let oct_size = 1 << (15 - depth);

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
    struct VolCorner {
        loc: LocCode,
        faces: [LocCode; 6],
    }

    struct PlaneCorner {
        loc: LocCode,
        edges: [LocCode; 4]
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
    pub fn find_zero(p1: SurfacePoint, p2: SurfacePoint, f: &mut ImplicitFn) -> Vec3 {
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
                .map(|(i, j)| find_zero(tetra[i as usize], tetra[j as usize], f));

                tris.push([p0, p1, p3]);
                tris.push([p1, p2, p3]);

                return;
            }
        }
        .map(|(i, j)| find_zero(tetra[i as usize], tetra[j as usize], f));
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

    pub fn build(
        min: Vec3,
        max: Vec3,
        min_depth: u32,
        program: &[op::Opcode],
        tol: f64,
    ) -> (Vec<[Vec3; 3]>, NTree) {
        let mut f = ImplicitFn::new(program.to_vec());
        let tree = NTree::build_3d(min, max, min_depth, &mut f, tol);
        let tris = tree.march_tetrahedra2(min, max, &mut f);
        (tris, tree)
    }
}

//pub mod line {
//    use glam::Vec2;
//    use ordered_float::OrderedFloat;
//    use std::{collections::HashMap, ops, rc::Rc};

//    use crate::vm::{self, float, op};

//    use super::{CellCorners, CellPtr, ImplicitFn};

//    type QuadTree = super::QuadTree<2>;
//    type EvalPoint = super::EvalPoint<2>;
//    type IsoVec = super::IsoVec<2>;
//    type Cell = super::Cell<2>;

//    #[derive(Debug, Clone, Copy)]
//    struct Triangle {
//        verts: [EvalPoint; 3],
//        next: Option<TriPtr>,
//        next_bisec_point: Option<EvalPoint>,
//        prev: Option<TriPtr>,
//        visited: bool,
//    }

//    impl Triangle {
//        fn new(p1: EvalPoint, p2: EvalPoint, p3: EvalPoint) -> Self {
//            Self {
//                verts: [p1, p2, p3],
//                next: None,
//                next_bisec_point: None,
//                prev: None,
//                visited: false,
//            }
//        }

//        fn triangle_4(
//            a: EvalPoint,
//            b: EvalPoint,
//            c: EvalPoint,
//            d: EvalPoint,
//            mid: EvalPoint,
//        ) -> [Triangle; 4] {
//            [
//                Triangle::new(a, b, mid),
//                Triangle::new(b, c, mid),
//                Triangle::new(c, d, mid),
//                Triangle::new(d, a, mid),
//            ]
//        }
//    }

//    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
//    struct Point(OrderedFloat<float>, OrderedFloat<float>);

//    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
//    struct TriPtr(usize);

//    impl TriPtr {
//        const ROOT: Self = TriPtr(0);
//    }

//    struct Triangulator<'a> {
//        triangles: Vec<Triangle>,
//        f: &'a mut ImplicitFn<2>,
//        tol: float,
//        tree: QuadTree,
//        hanging_next: HashMap<Point, TriPtr>,
//    }

//    impl ops::Index<TriPtr> for Triangulator<'_> {
//        type Output = Triangle;

//        fn index(&self, index: TriPtr) -> &Self::Output {
//            &self.triangles[index.0]
//        }
//    }

//    impl ops::IndexMut<TriPtr> for Triangulator<'_> {
//        fn index_mut(&mut self, index: TriPtr) -> &mut Self::Output {
//            &mut self.triangles[index.0]
//        }
//    }

//    impl Triangulator<'_> {
//        pub fn insert(&mut self, t: Triangle) -> TriPtr {
//            let ptr = self.triangles.len();
//            self.triangles.push(t);
//            TriPtr(ptr)
//        }

//        pub fn tri_inside(&mut self, c_p: CellPtr) {
//            let c = self.tree[c_p];
//            if let Some(children) = c.children {
//                for c in children.as_ref() {
//                    self.tri_inside(*c);
//                }

//                self.tri_crossing_row(children[0], children[1]);
//                self.tri_crossing_row(children[2], children[3]);
//                self.tri_crossing_col(children[0], children[2]);
//                self.tri_crossing_col(children[1], children[3]);
//            }
//        }

//        pub fn tri_crossing_row(&mut self, a_p: CellPtr, b_p: CellPtr) {
//            let a = self.tree[a_p];
//            let b = self.tree[b_p];

//            if let (Some(c1), Some(c2)) = (a.children, b.children) {
//                self.tri_crossing_row(c1[1], c2[0]);
//                self.tri_crossing_row(c1[3], c2[2]);
//            } else if let Some(c) = a.children {
//                self.tri_crossing_row(c[1], b_p);
//                self.tri_crossing_row(c[3], b_p);
//            } else if let Some(c) = b.children {
//                self.tri_crossing_row(a_p, c[0]);
//                self.tri_crossing_row(a_p, c[2]);
//            } else {
//                let fd_a = EvalPoint::get_dual(&a.verts, self.f);
//                let fd_b = EvalPoint::get_dual(&b.verts, self.f);

//                let tris = if a.depth < b.depth {
//                    let ed = self.edge_dual(b.verts[2], b.verts[0]);
//                    Triangle::triangle_4(b.verts[2], fd_b, b.verts[0], fd_a, ed)
//                } else {
//                    let ed = self.edge_dual(a.verts[3], a.verts[1]);
//                    Triangle::triangle_4(a.verts[3], fd_b, a.verts[1], fd_a, ed)
//                };

//                self.add_4_tris(tris);
//            }
//        }

//        pub fn tri_crossing_col(&mut self, a_p: CellPtr, b_p: CellPtr) {
//            let a = self.tree[a_p];
//            let b = self.tree[b_p];

//            if let (Some(c1), Some(c2)) = (a.children, b.children) {
//                self.tri_crossing_col(c1[2], c2[0]);
//                self.tri_crossing_col(c1[3], c2[1]);
//            } else if let Some(c) = a.children {
//                self.tri_crossing_col(c[2], b_p);
//                self.tri_crossing_col(c[3], b_p);
//            } else if let Some(c) = b.children {
//                self.tri_crossing_col(a_p, c[0]);
//                self.tri_crossing_col(a_p, c[1]);
//            } else {
//                let fd_a = EvalPoint::get_dual(&a.verts, self.f);
//                let fd_b = EvalPoint::get_dual(&b.verts, self.f);

//                let tris = if a.depth < b.depth {
//                    let ed = self.edge_dual(b.verts[0], b.verts[1]);
//                    Triangle::triangle_4(b.verts[0], fd_b, b.verts[1], fd_a, ed)
//                } else {
//                    let ed = self.edge_dual(a.verts[2], a.verts[3]);
//                    Triangle::triangle_4(a.verts[2], fd_b, a.verts[3], fd_a, ed)
//                };

//                self.add_4_tris(tris);
//            }
//        }

//        fn add_4_tris(&mut self, triangles: [Triangle; 4]) {
//            let tris = triangles.map(|t| self.insert(t));

//            for i in 0..4 {
//                self.next_sandwich_tri(tris[i], tris[(i + 1) % 4], tris[(i + 2) % 4]);
//            }
//        }

//        fn set_next(&mut self, t1: TriPtr, t2: TriPtr, pos: EvalPoint, neg: EvalPoint) {
//            if !(pos.val > 0.0 && 0.0 >= neg.val) {
//                return;
//            }

//            let int = EvalPoint::find_zero(pos, neg, self.f, self.tol);

//            self[t1].next_bisec_point = Some(int);
//            self[t1].next = Some(t2);

//            self[t2].prev = Some(t1);

//            //t1.next_bisec_point = int;
//            //t1.next = t2;
//        }

//        fn next_sandwich_tri(&mut self, a_p: TriPtr, b_p: TriPtr, c_p: TriPtr) {
//            let b = &self[b_p];

//            let mid = b.verts[2];
//            let x = b.verts[0];
//            let y = b.verts[1];

//            if mid.val > 0.0 && 0.0 >= y.val {
//                self.set_next(b_p, c_p, mid, y);
//            }

//            if x.val > 0.0 && 0.0 >= mid.val {
//                self.set_next(b_p, a_p, x, mid)
//            }

//            let id = x.pos + y.pos;
//            let id = Point(id[0].into(), id[1].into());

//            if y.val > 0.0 && 0.0 >= x.val {
//                if self.hanging_next.contains_key(&id) {
//                    let t = self.hanging_next.remove(&id).unwrap();
//                    self.set_next(b_p, t, y, x);
//                } else {
//                    self.hanging_next.insert(id, b_p);
//                }
//            } else if y.val <= 0.0 && 0.0 < x.val {
//                if self.hanging_next.contains_key(&id) {
//                    let t = self.hanging_next.remove(&id).unwrap();
//                    self.set_next(t, b_p, x, y);
//                } else {
//                    self.hanging_next.insert(id, b_p);
//                }
//            }
//        }

//        pub fn edge_dual(&mut self, p1: EvalPoint, p2: EvalPoint) -> EvalPoint {
//            if (p1.val > 0.0) != (p2.val > 0.0) {
//                EvalPoint::midpoint(p1, p2, self.f)
//            } else {
//                let dt = 0.001;

//                let df1 = self.f.eval_f64(p1.pos * (1.0 - dt) + p2.pos * dt);
//                let df2 = self.f.eval_f64(p1.pos + p2.pos * (1.0 - dt));

//                if (df1 > 0.0) == (df2 > 0.0) {
//                    EvalPoint::midpoint(p1, p2, self.f)
//                } else {
//                    let v1 = EvalPoint {
//                        pos: p1.pos,
//                        val: df1,
//                    };
//                    let v2 = EvalPoint {
//                        pos: p2.pos,
//                        val: df2,
//                    };
//                    EvalPoint::zero_intersect(v1, v2, self.f)
//                }
//            }
//        }

//        pub fn trace(&mut self) -> Vec<Vec<Vec2>> {
//            let mut curves = vec![];

//            for t_p in 0..self.triangles.len() {
//                let tri = self.triangles[t_p];
//                if !tri.visited && tri.next.is_some() {
//                    let mut active_curve = vec![];
//                    //self.march_triangle(TriPtr(t_p));
//                    Self::march_triangle(&mut self.triangles, t_p, &mut active_curve);

//                    curves.push(active_curve);
//                }
//            }

//            curves
//                .into_iter()
//                .map(|curve| {
//                    curve
//                        .into_iter()
//                        .map(|p| Vec2::new(p.pos[0] as f32, p.pos[1] as f32))
//                        .collect()
//                })
//                .collect()
//        }

//        pub fn march_triangle(
//            tris: &mut Vec<Triangle>,
//            t_p: usize,
//            active_curve: &mut Vec<EvalPoint>,
//        ) {
//            let start_tri = t_p;
//            let mut tri = &mut tris[t_p];
//            let mut closed_loop = false;

//            while let Some(prev) = tri.prev {
//                tri = &mut tris[prev.0];
//                if prev.0 == start_tri {
//                    closed_loop = true;
//                    break;
//                }
//            }

//            while !tri.visited {
//                if let Some(nbp) = tri.next_bisec_point {
//                    active_curve.push(nbp);
//                }

//                tri.visited = true;

//                if let Some(t) = tri.next {
//                    tri = &mut tris[t.0];
//                } else {
//                    break;
//                }
//            }

//            if closed_loop {
//                active_curve.push(active_curve[0]);
//            }
//        }

//        /*
//        pub fn march_triangle(&mut self, t_p: TriPtr) {
//            let start_tri = t_p;
//            let mut tri = self[t_p];
//            let mut closed_loop = false;

//            while let Some(prev) = tri.prev {
//                tri = self[prev];
//                if prev == start_tri {
//                    closed_loop = true;
//                    break
//                }
//            }

//            while !tri.visited {
//                if let Some(nbp) = tri.next_bisec_point {
//                    self.active_curve.push(nbp);
//                }

//                tri.visited = true;

//                if let Some(t) = tri.next {
//                    tri = self[t];
//                } else {
//                    break
//                }
//            }

//            if closed_loop {
//                self.active_curve.append(self.active_curve[0]);
//            }
//        }
//        */
//    }

//    pub fn build(
//        min: Vec2,
//        max: Vec2,
//        min_depth: u32,
//        max_cells: u32,
//        // implicit_fn: impl Fn(Vec2) -> float,
//        program: &[op::Opcode],
//        tol: float,
//    ) -> (Vec<Vec<Vec2>>, QuadTree) {
//        //let f = |v: IsoVec| implicit_fn(v.into());

//        // let program = vec![
//        //     op::POW_IMM_RHS(3.0, 1, 1),
//        //     op::SIN(1, 1),
//        //     op::POW_LHS_IMM(1, -1.0, 1),
//        //     op::SUB_LHS_RHS(1, 2, 1),
//        //     op::SIN(2, 2),
//        //     op::ADD_LHS_RHS(1, 2, 1),
//        //     op::EXT(0),
//        // ];

//        let mut f = ImplicitFn {
//            program: program.to_vec(),
//            vm: vm::VM::new(),
//        };

//        let tree = QuadTree::build(min.into(), max.into(), min_depth, max_cells, tol, &mut f);

//        let mut triangulator = Triangulator {
//            triangles: vec![],
//            f: &mut f,
//            tol,
//            tree,
//            hanging_next: Default::default(),
//        };

//        triangulator.tri_inside(CellPtr::ROOT);
//        let points = triangulator.trace();
//        (points, triangulator.tree)
//    }
//}
