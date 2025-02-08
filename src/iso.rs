use std::{collections::VecDeque, ops};

use crate::vm::{self, float};

////https://people.engr.tamu.edu/schaefer/research/iso_simplicial.pdf

//#[derive(Debug, Clone)]
//pub struct ImplicitFn<const N: usize> {
//    pub program: Vec<vm::Opcode>,
//    pub vm: vm::VM,
//}

//impl<const N: usize> ImplicitFn<N> {
//    pub fn eval_f64(&mut self, input: IsoVec<N>) -> float {
//        let vm = &mut self.vm;
//        for i in 0..N {
//            vm.registers[i + 1] = input[i];
//        }

//        vm.eval(&self.program);
//        vm.registers[1]
//    }

//    pub fn eval_range(&mut self, min: IsoVec<N>, max: IsoVec<N>) -> vm::Range {
//        let vm = &mut self.vm;

//        for i in 0..N {
//            vm.registers_range[i + 1] = (min[i], max[i]).into();
//        }

//        vm.eval_range(&self.program);
//        vm.registers_range[1]
//    }
//}

//#[derive(Copy, Clone, Debug, PartialEq)]
//pub struct IsoVec<const N: usize> {
//    v: [float; N],
//}

//impl<const N: usize> Default for IsoVec<N> {
//    fn default() -> Self {
//        Self {
//            v: [Default::default(); N],
//        }
//    }
//}

//impl From<glam::Vec3> for IsoVec<3> {
//    fn from(vec: glam::Vec3) -> Self {
//        Self {
//            v: [vec.x as float, vec.y as float, vec.z as float],
//        }
//    }
//}

//impl From<IsoVec<3>> for glam::Vec3 {
//    fn from(vec: IsoVec<3>) -> Self {
//        glam::Vec3::new(vec[0] as f32, vec[1] as f32, vec[2] as f32)
//    }
//}

//impl From<glam::Vec2> for IsoVec<2> {
//    fn from(vec: glam::Vec2) -> Self {
//        Self {
//            v: [vec.x as float, vec.y as float],
//        }
//    }
//}

//impl From<IsoVec<2>> for glam::Vec2 {
//    fn from(vec: IsoVec<2>) -> Self {
//        glam::Vec2::new(vec[0] as f32, vec[1] as f32)
//    }
//}

//impl<const N: usize> ops::Add<float> for IsoVec<N> {
//    type Output = Self;

//    fn add(mut self, rhs: float) -> Self::Output {
//        for i in 0..N {
//            self[i] += rhs;
//        }
//        self
//    }
//}

//impl<const N: usize> ops::AddAssign<float> for IsoVec<N> {
//    fn add_assign(&mut self, rhs: float) {
//        *self = *self + rhs
//    }
//}

//impl<const N: usize> ops::Add<IsoVec<N>> for float {
//    type Output = IsoVec<N>;

//    fn add(self, rhs: IsoVec<N>) -> Self::Output {
//        rhs + self
//    }
//}

//impl<const N: usize> ops::Add<IsoVec<N>> for IsoVec<N> {
//    type Output = Self;

//    fn add(mut self, rhs: Self) -> Self::Output {
//        for i in 0..N {
//            self[i] += rhs[i]
//        }
//        self
//    }
//}

//impl<const N: usize> ops::Sub<float> for IsoVec<N> {
//    type Output = Self;

//    fn sub(mut self, rhs: float) -> Self::Output {
//        for i in 0..N {
//            self[i] -= rhs
//        }
//        self
//    }
//}

//impl<const N: usize> ops::SubAssign<float> for IsoVec<N> {
//    fn sub_assign(&mut self, rhs: float) {
//        *self = *self - rhs
//    }
//}

//impl<const N: usize> ops::Sub<IsoVec<N>> for float {
//    type Output = IsoVec<N>;

//    fn sub(self, rhs: IsoVec<N>) -> Self::Output {
//        -rhs + self
//    }
//}

//impl<const N: usize> ops::Sub<IsoVec<N>> for IsoVec<N> {
//    type Output = Self;

//    fn sub(mut self, rhs: Self) -> Self::Output {
//        for i in 0..N {
//            self[i] -= rhs[i]
//        }
//        self
//    }
//}

//impl<const N: usize> ops::Mul<float> for IsoVec<N> {
//    type Output = Self;

//    fn mul(mut self, rhs: float) -> Self::Output {
//        for i in 0..N {
//            self[i] *= rhs;
//        }
//        self
//    }
//}

//impl<const N: usize> ops::Mul<IsoVec<N>> for float {
//    type Output = IsoVec<N>;

//    fn mul(self, rhs: IsoVec<N>) -> Self::Output {
//        rhs * self
//    }
//}

//impl<const N: usize> ops::Div<float> for IsoVec<N> {
//    type Output = Self;

//    fn div(mut self, rhs: float) -> Self::Output {
//        for i in 0..N {
//            self[i] /= rhs;
//        }
//        self
//    }
//}

//impl<const N: usize> ops::Neg for IsoVec<N> {
//    type Output = Self;

//    fn neg(self) -> Self::Output {
//        -1.0 * self
//    }
//}

//impl<const N: usize> ops::Index<usize> for IsoVec<N> {
//    type Output = float;

//    fn index(&self, index: usize) -> &Self::Output {
//        &self.v[index]
//    }
//}

//impl<const N: usize> ops::IndexMut<usize> for IsoVec<N> {
//    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
//        &mut self.v[index]
//    }
//}

//impl<const N: usize> IsoVec<N> {
//    #[inline(always)]
//    pub fn abs(mut self) -> Self {
//        for i in 0..N {
//            self[i] = self[i].abs()
//        }
//        self
//    }

//    #[inline(always)]
//    pub fn max_element(&self) -> float {
//        let mut max = self.v[0];
//        for e in &self.v[1..] {
//            max = max.max(*e);
//        }
//        max
//    }
//}

//#[derive(Copy, Clone, Debug, PartialEq, Default)]
//pub struct EvalPoint<const N: usize> {
//    pub pos: IsoVec<N>,
//    pub val: float,
//}

////pub trait ImplicitFn<const N: usize>: Fn(IsoVec<N>) -> float {}
////impl<const N: usize, F: Fn(IsoVec<N>) -> float> ImplicitFn<N> for F {}

//impl<const N: usize> EvalPoint<N> {
//    pub fn eval(pos: IsoVec<N>, f: &mut ImplicitFn<N>) -> Self {
//        let val = f.eval_f64(pos);
//        Self { pos, val }
//    }

//    pub fn midpoint(p1: Self, p2: Self, f: &mut ImplicitFn<N>) -> Self {
//        let pos = (p1.pos + p2.pos) / 2.0;
//        let val = f.eval_f64(pos);
//        Self { pos, val }
//    }

//    pub fn zero_intersect(p1: Self, p2: Self, f: &mut ImplicitFn<N>) -> Self {
//        let denom = p1.val - p2.val;
//        let k1 = -p2.val / denom;
//        let k2 = p1.val / denom;
//        let pos = k1 * p1.pos + k2 * p2.pos;
//        let val = f.eval_f64(pos);
//        Self { pos, val }
//    }

//    pub fn cube_eval(
//        min: IsoVec<N>,
//        max: IsoVec<N>,
//        f: &mut ImplicitFn<N>,
//    ) -> CellCorners<EvalPoint<N>> {
//        for i in 0..N {
//            debug_assert!(min[i] <= max[i])
//        }
//        let width = max - min;
//        let mut points = CellCorners::with_dim(N, EvalPoint::default());
//        for i in 0..1 << N {
//            let mut pos = min;
//            for j in 0..N {
//                if (i >> j) & 1 == 1 {
//                    pos[j] += width[j]
//                }
//            }

//            points[i] = EvalPoint::eval(pos, f);
//        }

//        points
//    }

//    pub fn get_dual(cells: &CellCorners<EvalPoint<N>>, f: &mut ImplicitFn<N>) -> EvalPoint<N> {
//        let verts = cells.as_ref();
//        EvalPoint::midpoint(verts[0], verts[verts.len() - 1], f)
//    }

//    // TODO: tol
//    pub fn find_zero(
//        mut a: EvalPoint<N>,
//        mut b: EvalPoint<N>,
//        f: &mut ImplicitFn<N>,
//        tol: f64,
//    ) -> EvalPoint<N> {
//        EvalPoint::zero_intersect(a, b, f)

//        // if (p1.pos - p2.pos).abs().max_element() < tol {
//        //     EvalPoint::zero_intersect(p1, p2, f)
//        // } else {
//        //     let mid = EvalPoint::midpoint(p1, p2, f);
//        //     if mid.val.abs() == tol {
//        //         mid
//        //     } else if (mid.val > 0.0) == (p1.val > 0.0) {
//        //         Self::find_zero(mid, p2, f, tol)
//        //     } else {
//        //         Self::find_zero(p1, mid, f, tol)
//        //     }
//        // }
//    }
//}

//// f(x) = (f(b) - f(a))/(b - a) * (x - a) + f(a)
//// f(x) / (f(b) - f(a)) * (b - a) / (x - a) = f(a)

//#[derive(Clone, Debug, Copy)]
//pub enum CellCorners<T> {
//    D1([T; 1 << 1]),
//    D2([T; 1 << 2]),
//    D3([T; 1 << 3]),
//}

//impl<T> ops::Deref for CellCorners<T> {
//    type Target = [T];

//    fn deref(&self) -> &Self::Target {
//        match self {
//            CellCorners::D1(a) => a,
//            CellCorners::D2(a) => a,
//            CellCorners::D3(a) => a,
//        }
//    }
//}

//impl<T> ops::DerefMut for CellCorners<T> {
//    fn deref_mut(&mut self) -> &mut Self::Target {
//        match self {
//            CellCorners::D1(a) => a,
//            CellCorners::D2(a) => a,
//            CellCorners::D3(a) => a,
//        }
//    }
//}

//impl<T> CellCorners<T> {
//    pub fn map<S>(self, mut f: impl FnMut(T) -> S) -> CellCorners<S> {
//        match self {
//            CellCorners::D1(a) => CellCorners::D1(a.map(|x| f(x))),
//            CellCorners::D2(a) => CellCorners::D2(a.map(|x| f(x))),
//            CellCorners::D3(a) => CellCorners::D3(a.map(|x| f(x))),
//        }
//    }
//}

//impl<T: Copy> CellCorners<T> {
//    pub const fn with_dim(dim: usize, v: T) -> Self {
//        match dim {
//            1 => CellCorners::D1([v; 1 << 1]),
//            2 => CellCorners::D2([v; 1 << 2]),
//            3 => CellCorners::D3([v; 1 << 3]),
//            _ => panic!("unsupported dimension"),
//        }
//    }

//    pub fn get_subcell(&self, axis: u32, dir: bool) -> Option<Self> {
//        let m = 1 << axis;

//        match self {
//            CellCorners::D1(_) => None,
//            CellCorners::D2(a) => {
//                let mut sub_cell = [a[0]; 1 << 1];
//                let mut k = 0;
//                for (i, vert) in a.iter().enumerate() {
//                    if ((i & m) > 0) == dir {
//                        sub_cell[k] = *vert;
//                        k += 1;
//                    }
//                }
//                CellCorners::D1(sub_cell).into()
//            }
//            CellCorners::D3(a) => {
//                let mut sub_cell = [a[0]; 1 << 2];
//                let mut k = 0;
//                for (i, vert) in a.iter().enumerate() {
//                    if ((i & m) > 0) == dir {
//                        sub_cell[k] = *vert;
//                        k += 1;
//                    }
//                }
//                CellCorners::D2(sub_cell).into()
//            }
//        }
//    }
//}

//#[derive(Debug, Clone, Copy)]
//pub struct Cell<const N: usize> {
//    pub depth: u32,
//    pub children: Option<CellCorners<CellPtr>>,
//    pub parent: Option<CellPtr>,
//    pub child_dir: u32, // TODO u8
//    pub verts: CellCorners<EvalPoint<N>>,
//}

//impl<const N: usize> Default for Cell<N> {
//    fn default() -> Self {
//        Self {
//            depth: 0,
//            children: None,
//            parent: None,
//            child_dir: 0,
//            verts: CellCorners::with_dim(N, EvalPoint::default()),
//        }
//    }
//}

//#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
//pub struct CellPtr(pub(crate) usize);

//impl CellPtr {
//    // TODO: NonZero
//    pub const NONE: Self = Self(usize::MAX);
//    pub const ROOT: Self = Self(0);
//}

//#[derive(Debug, Clone)]
//pub struct QuadTree<const N: usize> {
//    pub cells: Vec<Cell<N>>,
//}

//impl<const N: usize> ops::Index<CellPtr> for QuadTree<N> {
//    type Output = Cell<N>;

//    fn index(&self, index: CellPtr) -> &Self::Output {
//        &self.cells[index.0]
//    }
//}

//impl<const N: usize> ops::IndexMut<CellPtr> for QuadTree<N> {
//    fn index_mut(&mut self, index: CellPtr) -> &mut Self::Output {
//        &mut self.cells[index.0]
//    }
//}

//impl<const N: usize> QuadTree<N> {
//    pub fn empty() -> Self {
//        Self { cells: vec![] }
//    }

//    pub fn build(
//        min: IsoVec<N>,
//        max: IsoVec<N>,
//        min_depth: u32,
//        max_cells: u32,
//        tol: float,
//        f: &mut ImplicitFn<N>,
//    ) -> Self {
//        let branch_fac = 1u32 << N;

//        let max_cells = branch_fac.pow(min_depth).max(max_cells);
//        let verts = EvalPoint::cube_eval(min, max, f);

//        let mut tree = Self::empty();

//        let root = tree.insert(Cell {
//            depth: 0,
//            children: None,
//            parent: None,
//            child_dir: 0,
//            verts,
//        });

//        let mut leaf_count = 1;
//        let mut quad_queue = VecDeque::from([root]);

//        while !quad_queue.is_empty() && leaf_count < max_cells {
//            let curr = quad_queue.pop_front().unwrap();
//            if tree.should_descend(curr, f, tol) {
//                let children = tree.compute_children(curr, f);
//                // todo: priority
//                children.into_iter().for_each(|c| quad_queue.push_back(*c));
//                leaf_count += branch_fac - 1;
//            }
//        }

//        tree
//    }

//    pub fn insert(&mut self, cell: Cell<N>) -> CellPtr {
//        self.cells.push(cell);
//        CellPtr(self.cells.len() - 1)
//    }

//    pub fn compute_children(
//        &mut self,
//        cell_ptr: CellPtr,
//        f: &mut ImplicitFn<N>,
//    ) -> &CellCorners<CellPtr> {
//        //) -> &[CellPtr; 1 << N] {
//        let cell = &self[cell_ptr];
//        assert!(cell.children.is_none());

//        let mut new_cells = CellCorners::with_dim(N, Cell::default());

//        for (i, vert) in cell.verts.iter().enumerate() {
//            let min = (cell.verts[0].pos + vert.pos) / 2.0;
//            let max = (cell.verts[(1 << N) - 1].pos + vert.pos) / 2.0;
//            let verts = EvalPoint::cube_eval(min, max, f);
//            new_cells[i] = Cell {
//                depth: cell.depth + 1,
//                children: None,
//                parent: Some(cell_ptr),
//                child_dir: i as u32,
//                verts,
//            };
//        }

//        self[cell_ptr].children = Some(new_cells.map(|cell| self.insert(cell)));
//        self[cell_ptr].children.as_ref().unwrap()
//    }

//    pub fn get_leaves_in_dir(&self, cell_ptr: CellPtr, axis: u32, dir: bool) -> Vec<CellPtr> {
//        if let Some(children) = self[cell_ptr].children {
//            let m = 1 << axis;
//            children
//                .iter()
//                .copied()
//                .filter(move |cell_ptr| ((self[*cell_ptr].depth & m) > 0) == dir)
//                .flat_map(move |cell_ptr| self.get_leaves_in_dir(cell_ptr, axis, dir))
//                .collect()
//        } else {
//            vec![cell_ptr]
//            //Box::new(std::iter::once(cell_ptr))
//        }
//    }

//    pub fn walk_in_dir(&self, cell_ptr: CellPtr, axis: u32, dir: bool) -> Option<CellPtr> {
//        let cell = &self[cell_ptr];
//        let m = 1 << axis;

//        if ((cell.child_dir & m) > 0) == dir {
//            let parent = cell.parent?;
//            let parent_walk = self.walk_in_dir(parent, axis, dir)?;
//            self[parent_walk]
//                .children
//                .map(|children| children[(cell.child_dir ^ m) as usize])
//        } else {
//            let parent = cell.parent?;
//            Some(self[parent].children.unwrap()[(cell.child_dir ^ m) as usize])
//        }
//    }

//    pub fn walk_leaves_in_dir(
//        &self,
//        cell_ptr: CellPtr,
//        axis: u32,
//        dir: bool,
//    ) -> Option<Vec<CellPtr>> {
//        let walk = self.walk_in_dir(cell_ptr, axis, dir)?;
//        Some(self.get_leaves_in_dir(walk, axis, dir))
//    }

//    pub fn should_descend(&self, cell_ptr: CellPtr, f: &mut ImplicitFn<N>, tol: float) -> bool {
//        let cell = &self[cell_ptr];

//        // TODO : abs()?
//        if (cell.verts[(1 << N) - 1].pos - cell.verts[0].pos)
//            .max_element()
//            .abs()
//            < 10.0 * tol
//        {
//            return false;
//        }

//        let range = f.eval_range(cell.verts[0].pos, cell.verts[(1 << N) - 1].pos);
//        range.contains_zero() || range.is_undef()

//        // if range.contains_zero() || range.is_non_continuous() {
//        //     true
//        // } else if cell.verts.iter().all(|v| v.val.is_nan()) {
//        //     false
//        // } else if cell.verts.iter().any(|v| v.val.is_nan()) {
//        //     true
//        // } else {
//        //     // TODO: grad, second-deriv
//        //     cell.verts[1..]
//        //         .iter()
//        //         .any(|v| v.val.signum() != cell.verts[0].val.signum())
//        // }
//    }
//}

pub mod v3 {
    use std::{collections::VecDeque, fmt, ops};

    use glam::{DVec3, Vec3};

    use crate::vm::{self, float, op, machines};

    macro_rules! get_octants {
        ($loc:expr, $lvl:expr) => {
            // $loc >> $lvl * 4 & 0xF
            $loc >> (16 - 1 - $lvl) * 4 & 0xF
        };
    }

    // struct RangeEvalFn {
    //     vm: machines::VmRange,
    //     program: Vec<vm::Opcode>,
    // }

    // impl RangeEvalFn {

    //     fn call(&mut self, min: DVec3, max: DVec3)  -> vm::Range {
    //         let vm = &mut self.vm;

    //         for i in 0..3 {
    //             vm.reg[i + 1] = (min[i], max[i]).into();
    //         }

    //         vm.eval(&self.program);
    //         vm.reg[1]
    //     }

    // }

    // struct F32EvalFn {
    //     vm: machines::VmF32,
    //     program: Vec<vm::Opcode>,
    // };

    // impl F32EvalFn {
    //     fn call(&mut self, input: DVec3) -> float {
    //         let vm = &mut self.vm;
    //         for i in 0..3 {
    //             vm.reg[i + 1] = input[i];
    //         }

    //         vm.eval(&self.program);
    //         vm.reg[1]
    //     }
    // }

    struct ImplicitFn {
        vm_f32: machines::VmF32,
        vm_range: machines::VmRange,
        program: Vec<vm::Opcode>,
    }

    impl ImplicitFn {
        fn new(program: Vec<vm::Opcode>) -> Self {
            Self {
                vm_f32: machines::VmF32::new(),
                vm_range: machines::VmRange::new(),
                program,
            }
        }

        fn eval_f64(&mut self, input: DVec3) -> float {
            let vm = &mut self.vm_f32;
            for i in 0..3 {
                vm.reg[i + 1] = input[i];
            }

            vm.eval(&self.program);
            vm.reg[1]
        }

        fn eval_range(&mut self, min: DVec3, max: DVec3)  -> vm::Range {
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

    impl From<LocCode> for LocFmt {
        fn from(value: LocCode) -> Self {
            Self(value)
        }
    }

    impl ops::Deref for LocFmt {
        type Target = u64;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl ops::DerefMut for LocFmt {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
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

    // const DIR_MIN_X: Direction = todo!();

    // macro_rules! build_location {
    //     ( $( $index:expr ),* $(,)? ) => {{
    //         let mut loc: u64 = 0;
    //         $(
    //             loc = (loc << 4) | (($index as u64) & 0xF);
    //         )*
    //             loc
    //     }};
    // }

    // pub(crate) use build_location;

    pub fn build_location(indxs: &[u8]) -> LocCode {
        let depth = indxs.len();
        let mut loc: LocCode = 0;
        for (i, indx) in indxs.iter().enumerate() {
            // assume indx in u4
            loc |= (*indx as LocCode) << (16 - 1 - i) * 4;
        }
        loc
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
    fn octant_unit_bounds(loc: LocCode) -> (Vec3, Vec3) {
        let mut bounds = (Vec3::ZERO, Vec3::ONE);

        let mut i = 0;
        let mut oct = get_octants!(loc, i) as u8;
        while oct != 0 {
            bounds = local_octant_bounds(bounds, oct);

            i += 1;
            oct = get_octants!(loc, i) as u8;
        }

        bounds
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
    pub fn octant_depth(loc: LocCode) -> u8 {
        let mut i = 0;
        let mut oct = get_octants!(loc, i) as u8;
        while oct != 0 {
            i += 1;
            oct = get_octants!(loc, i) as u8;
        }
        i
    }

    #[inline(always)]
    fn subdivide_octant(loc: LocCode) -> [LocCode; 8] {
        let octs = [1, 2, 3, 4, 5, 6, 7, 8];
        let depth = octant_depth(loc);
        octs.map(|oct| loc | oct << (16 - 1 - depth) * 4)
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
        pub fn build_3d(
            mut min: Vec3,
            mut max: Vec3,
            depth: u32,
            f: &mut ImplicitFn,
            tol: float,
        ) -> Self {
            let mut leafs: Vec<LocCode> = (1..=8u8)
                .into_iter()
                .map(|i| build_location(&[i]))
                .collect();

            for _ in 0..depth {
                leafs = leafs
                    .into_iter()
                    .flat_map(|oct| subdivide_octant(oct))
                    .collect();
            }

            let fmt_leafs: Vec<_> = leafs.iter().map(|l| LocFmt(*l)).collect();
            debug_assert!(fmt_leafs.is_sorted());

            let leafs = leafs
                .iter()
                .copied()
                // .filter(|oct| find_closest_octant(*oct) != 0)
                // .filter(|oct| same_lvl_neighbor(*oct, DIR_X) != 0)
                .filter(|oct| find_ge_neighbor(*oct, DIR_MIN_X, &leafs) != 0)
                // .map(|oct| next_octant(oct))
                // .filter(|oct| *oct != 0)
                .collect();

            Self { cells: leafs }
        }

        pub fn build_3d_2(
            mut min: Vec3,
            mut max: Vec3,
            depth: u32,
            f: &mut ImplicitFn,
            tol: float,
        ) -> Self {
            let mut leafs = vec![];

            let mut buff_1: Vec<LocCode> = (1..=8u8)
                .into_iter()
                .map(|i| build_location(&[i]))
                .collect();
            let mut buff_2: Vec<LocCode> = vec![];

            let mut prev_lvl = &mut buff_1;
            let mut curr_lvl = &mut buff_2;

            for _ in 0..depth {
                curr_lvl.clear();

                for oct in prev_lvl.iter() {
                    let (o_min, o_max) = octant_bounds(min, max, *oct);

                    if (o_min - o_max).abs().max_element() < (10.0 * tol as f32) {
                        leafs.push(*oct);
                    } else {
                        let range = f.eval_range(o_min.into(), o_max.into());
                        if range.contains_zero() || range.is_undef() {
                            curr_lvl.extend(subdivide_octant(*oct))
                        }
                    }
                }

                std::mem::swap(&mut curr_lvl, &mut prev_lvl);
            }
            leafs.extend(prev_lvl.iter());

            debug_assert!(leafs.is_sorted());
            Self { cells: leafs }
        }

        pub fn march_tetrahedra(
            &self,
            min: Vec3,
            max: Vec3,
            mut f: &mut ImplicitFn,
        ) -> Vec<[Vec3; 3]> {
            // let mut tetras = vec![];
            let mut tris = vec![];

            // let faces = [DIR_X, DIR_Y, DIR_Z, DIR_MIN_X, DIR_MIN_Y, DIR_MIN_Z];

            for oct in &self.cells {
                // let (o_min, o_max) = octant_bounds(min, max, oct);
                // let size = o_max - o_min;
                let c = octant_corners(min, max, *oct).map(|p| SurfacePoint::new(p, &mut f));


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

                        let edges = [
                            [f0, f1],
                            [f1, f2],
                            [f2, f3],
                            [f3, f0],
                        ];

                        for [e0, e1] in edges {
                            let tetra = [
                                e0, e1, face_dual, vol_dual
                            ];

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

    #[derive(Copy, Clone, Debug, PartialEq)]
    struct SurfacePoint {
        pos: Vec3,
        val: f64,
    }

    impl SurfacePoint {
        fn new(pos: Vec3, f: &mut ImplicitFn) -> Self {
            let val = f.eval_f64(pos.into());
            Self { pos, val }
        }
    }

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

            let [p0, p1, p2]  = match id {
                0b0001 | 0b1110 => [(0u8, 3u8), (1u8, 3u8), (2u8, 3u8)],
                0b0010 | 0b1101 => [(0u8, 2u8), (1u8, 2u8), (3u8, 2u8)],
                0b0100 | 0b1011 => [(0u8, 1u8), (2u8, 1u8), (3u8, 1u8)],
                0b1000 | 0b0111 => [(1u8, 0u8), (2u8, 0u8), (3u8, 0u8)],
                id => {
                    let [p0, p1, p2, p3] = match id {
                        0b0011 | 0b1100 => [(0u8, 2u8), (2u8, 1u8), (1u8, 3u8), (3u8, 0u8)],
                        0b0110 | 0b1001 => [(0u8, 1u8), (1u8, 3u8), (3u8, 2u8), (2u8, 0u8)],
                        0b0101 | 0b1010 => [(0u8, 1u8), (1u8, 2u8), (2u8, 3u8), (3u8, 0u8)],
                        _ => return
                    }.map(|(i, j)| find_zero(tetra[i as usize], tetra[j as usize], f));

                    tris.push([p0, p1, p3]);
                    tris.push([p1, p2, p3]);

                    return
                },
            }.map(|(i, j)| find_zero(tetra[i as usize], tetra[j as usize], f));
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

    pub fn march_tetrahedrons(tetras: &[[SurfacePoint;4]], mut f: &mut ImplicitFn) -> Vec<[Vec3; 3]> {
        let mut tris = vec![];
        for tetra in tetras {
            let mut id = 0u32;
            for t in tetra {
                id = 2 * id + (t.val > 0.0) as u32;
            }

            let indxs: &[_] = match id {
                0b0001 | 0b1110 => &[(0u8, 3u8), (1u8, 3u8), (2u8, 3u8)],
                0b0010 | 0b1101 => &[(0u8, 2u8), (1u8, 2u8), (3u8, 2u8)],
                0b0100 | 0b1011 => &[(0u8, 1u8), (2u8, 1u8), (3u8, 1u8)],
                0b1000 | 0b0111 => &[(1u8, 0u8), (2u8, 0u8), (3u8, 0u8)],
                0b0011 | 0b1100 => &[(0u8, 2u8), (2u8, 1u8), (1u8, 3u8), (3u8, 0u8)],
                0b0110 | 0b1001 => &[(0u8, 1u8), (1u8, 3u8), (3u8, 2u8), (2u8, 0u8)],
                0b0101 | 0b1010 => &[(0u8, 1u8), (1u8, 2u8), (2u8, 3u8), (3u8, 0u8)],
                _ => continue,
            };
            let pts: Vec<_> = indxs.iter().map(|(i, j)| find_zero(tetra[*i as usize], tetra[*j as usize], &mut f)).collect();

            if pts.len() == 3 {
                tris.push([pts[0], pts[1], pts[2]]);
            }
            if pts.len() == 4 {
                tris.push([pts[0], pts[1], pts[3]]);
                tris.push([pts[1], pts[2], pts[3]]);
            }
        }
        tris
    }

    pub fn build(
        min: Vec3,
        max: Vec3,
        min_depth: u32,
        program: &[op::Opcode],
        tol: f64,
    ) -> (Vec<[Vec3; 3]>, NTree) {
        let mut f = ImplicitFn::new(program.to_vec());
        let tree = NTree::build_3d_2(min, max, min_depth, &mut f, tol);
        let tris = tree.march_tetrahedra(min, max, &mut f);
        // let tris = march_tetrahedrons(&tetras, &mut f);
        (tris, tree)
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn loc_codes() {
            let loc = build_location(&[3, 4, 5, 1]);
            assert_eq!(&dbg_loc_code(loc), "3 4 5 1");

            assert_eq!(
                octant_unit_bounds(build_location(&[1])),
                (Vec3::splat(0.0), Vec3::splat(0.5))
            );
            assert_eq!(
                octant_unit_bounds(build_location(&[8])),
                (Vec3::splat(0.5), Vec3::splat(1.0))
            );
        }
    }
}

// pub mod v2 {
//     use std::{collections::VecDeque, ops};

//     use crate::vm::{self, float, op};

//     use super::{EvalPoint, ImplicitFn, IsoVec};

//     // TODO use f3 for position
//     type NVec<const N: usize> = IsoVec<N>;

//     #[derive(Debug, Clone, Copy, Default, PartialEq)]
//     pub struct Cell<const N: usize> {
//         // if cell is a leaf, set to zero, because the first
//         // cell is always the root and there are no cells that
//         // have root as a child
//         pub first_child: u32,
//         // volume of the cell defined by the min and max
//         pub min: NVec<N>,
//         pub max: NVec<N>,
//         pub depth: u8,
//     }

//     impl<const N: usize> Cell<N> {
//         pub fn get_corners(&self) -> Vec<NVec<N>> {
//             let mut out = vec![Default::default(); 1 << N];
//             let min = self.min;
//             let max = self.max;
//             let size = max - min;

//             for i in 0..1 << N {
//                 let mut pos = min;

//                 for j in 0..N {
//                     if (i >> j) & 1 == 1 {
//                         pos[j] += size[j];
//                     }
//                 }

//                 out[i] = pos;
//             }

//             out
//         }
//     }

//     // TODO: non-linear?
//     pub struct NTree<const N: usize> {
//         pub cells: Vec<Cell<N>>,
//     }

//     impl<const N: usize> ops::Index<u32> for NTree<N> {
//         type Output = Cell<N>;

//         fn index(&self, index: u32) -> &Self::Output {
//             &self.cells[index as usize]
//         }
//     }

//     impl<const N: usize> ops::IndexMut<u32> for NTree<N> {
//         fn index_mut(&mut self, index: u32) -> &mut Self::Output {
//             &mut self.cells[index as usize]
//         }
//     }

//     impl<const N: usize> NTree<N> {
//         pub fn empty() -> Self {
//             Self { cells: vec![] }
//         }

//         pub fn build(
//             min: NVec<N>,
//             max: NVec<N>,
//             depth: u32,
//             max_cells: u32,
//             tol: float,
//             f: &mut ImplicitFn<N>,
//         ) -> Self {
//             let branch_fac = 1u32 << N;
//             let max_cells = branch_fac.pow(depth).max(max_cells);

//             let mut tree = Self::empty();
//             let root = tree.insert(Cell {
//                 depth: 0,
//                 first_child: 0,
//                 min,
//                 max,
//             });

//             // let first_child = tree.subdivide_cell(min, max);

//             let mut cell_queue = VecDeque::from([root]);
//             let mut leaf_count = 1;
//             while let Some(cell_p) = cell_queue.pop_front() {
//                 let cell = &mut tree[cell_p];
//                 debug_assert!(cell.first_child == 0);
//                 let min = cell.min;
//                 let max = cell.max;
//                 let depth = cell.depth;

//                 let should_descend = tree.subdivide_cond(min, max, f, tol);
//                 if should_descend {
//                     let first_child = tree.subdivide_cell(min, max, depth + 1);
//                     tree[cell_p].first_child = first_child;

//                     cell_queue.extend(first_child..first_child + branch_fac);

//                     leaf_count += branch_fac - 1;
//                     if leaf_count >= max_cells {
//                         break;
//                     }
//                 }
//             }

//             tree
//         }

//         fn insert(&mut self, c: Cell<N>) -> u32 {
//             self.cells.push(c);
//             self.cells.len() as u32 - 1
//         }

//         pub fn subdivide_cond(
//             &self,
//             min: NVec<N>,
//             max: NVec<N>,
//             f: &mut ImplicitFn<N>,
//             tol: float,
//         ) -> bool {
//             if (min - max).abs().max_element() < 10.0 * tol {
//                 false
//             } else {
//                 let range = f.eval_range(min, max);
//                 range.contains_zero() || range.is_undef()
//             }
//         }

//         pub fn subdivide_cell(&mut self, min: NVec<N>, max: NVec<N>, depth: u8) -> u32 {
//             let first = self.cells.len();
//             self.cells.resize(first + (1 << N), Cell::default());

//             let half_size = (max - min) / 2.0;

//             for i in 0..1 << N {
//                 let mut c_min = min;

//                 for j in 0..N {
//                     if (i >> j) & 1 == 1 {
//                         c_min[j] += half_size[j];
//                     }
//                 }

//                 let c_max = c_min + half_size;
//                 self.cells[first + i] = Cell {
//                     depth: depth,
//                     first_child: 0,
//                     min: c_min,
//                     max: c_max,
//                 }
//             }

//             first as u32
//         }
//     }

//     pub fn build(
//         min: NVec<3>,
//         max: NVec<3>,
//         min_depth: u32,
//         max_cells: u32,
//         program: &[op::Opcode],
//         tol: f64,
//     ) -> NTree<3> {
//         let mut f = ImplicitFn {
//             program: program.to_vec(),
//             vm: vm::VM::new(),
//         };

//         let tree = NTree::build(min.into(), max.into(), min_depth, max_cells, tol, &mut f);
//         tree
//     }
// }

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

//pub mod surface {

//    use glam::Vec3;

//    use crate::vm::{self, float, op};

//    use super::{CellCorners, CellPtr, ImplicitFn};

//    type QuadTree = super::QuadTree<3>;
//    type EvalPoint = super::EvalPoint<3>;
//    type IsoVec = super::IsoVec<3>;

//    const TETRAHEDRON_TABLE: [&'static [(u32, u32)]; 8] = [
//        /*0b0000*/ &[], // falsey
//        /*0b0001*/ &[(0, 3), (1, 3), (2, 3)],
//        /*0b0010*/ &[(0, 2), (1, 2), (3, 2)],
//        /*0b0100*/ &[(0, 1), (2, 1), (3, 1)],
//        /*0b1000*/ &[(1, 0), (2, 0), (3, 0)],
//        /*0b0011*/ &[(0, 2), (2, 1), (1, 3), (3, 0)],
//        /*0b0110*/ &[(0, 1), (1, 3), (3, 2), (2, 0)],
//        /*0b0101*/ &[(0, 1), (1, 2), (2, 3), (3, 0)],
//    ];

//    const fn tetrahedron_table(id: u32) -> Option<&'static [(u32, u32)]> {
//        Some(match id {
//            //0b0000 => TETRAHEDRON_TABLE[0], // falsey
//            0b0001 => TETRAHEDRON_TABLE[1],
//            0b0010 => TETRAHEDRON_TABLE[2],
//            0b0100 => TETRAHEDRON_TABLE[3],
//            0b1000 => TETRAHEDRON_TABLE[4],
//            0b0011 => TETRAHEDRON_TABLE[5],
//            0b0110 => TETRAHEDRON_TABLE[6],
//            0b0101 => TETRAHEDRON_TABLE[7],
//            _ => return None,
//        })
//    }

//    pub fn march_indices(simplex: &[EvalPoint]) -> Option<&'static [(u32, u32)]> {
//        let mut id = 0;
//        for p in simplex {
//            id = 2 * id + (p.val > 0.0) as u32;
//        }

//        if let Some(res) = tetrahedron_table(id) {
//            Some(res)
//        } else {
//            tetrahedron_table(id ^ 0b1111)
//        }
//    }

//    pub enum Primitive {
//        Tri([Vec3; 3]),
//        Quad([Vec3; 3], [Vec3; 3]),
//    }

//    pub fn march_simplex(
//        simplex: &[EvalPoint],
//        f: &mut ImplicitFn<3>,
//        tol: float,
//    ) -> Option<Primitive> {
//        let Some(indices) = march_indices(simplex) else {
//            return None;
//        };

//        let mut pts = vec![];
//        for (i, j) in indices {
//            let intersec = EvalPoint::find_zero(simplex[*i as usize], simplex[*j as usize], f, tol);
//            pts.push(intersec.pos);
//        }

//        if pts.len() == 3 {
//            let p0 = pts[0].into();
//            let p1 = pts[1].into();
//            let p2 = pts[2].into();
//            Primitive::Tri([p0, p1, p2])
//        } else {
//            let p0 = pts[0].into();
//            let p1 = pts[1].into();
//            let p2 = pts[2].into();
//            let p3 = pts[3].into();
//            Primitive::Quad([p0, p1, p3], [p1, p2, p3])
//        }
//        .into()
//    }

//    //pub struct SimplexGen {
//    //    tree: QuadTree<3>,
//    //    //sample_fn: F,
//    //}

//    impl QuadTree {
//        pub fn get_simplices_from(
//            &self,
//            cell_ptr: CellPtr,
//            f: &mut ImplicitFn<3>,
//        ) -> Vec<[EvalPoint; 4]> {
//            let cell = self[cell_ptr];

//            if let Some(children) = cell.children {
//                children
//                    .iter()
//                    .copied()
//                    .flat_map(|child| self.get_simplices_from(child, f))
//                    .collect()
//            } else {
//                let mut evals = vec![];
//                for axis in [0, 1, 2] {
//                    for dir in [false, true] {
//                        if let Some(adj) = self.walk_leaves_in_dir(cell_ptr, axis, dir) {
//                            evals.extend(adj.into_iter().flat_map(|leaf| {
//                                self.get_simplices_between(cell_ptr, leaf, axis, dir, f)
//                            }))
//                        } else {
//                            let sub = cell.verts;
//                            evals.extend(self.get_simplices_between_face(
//                                sub.clone(),
//                                sub.get_subcell(axis, dir).unwrap(),
//                                f,
//                            ))
//                        }
//                    }
//                }
//                evals
//            }
//        }

//        pub fn get_simplices_between(
//            &self,
//            a_p: CellPtr,
//            b_p: CellPtr,
//            axis: u32,
//            mut dir: bool,
//            f: &mut ImplicitFn<3>,
//        ) -> Vec<[EvalPoint; 4]> {
//            let mut a = self[a_p];
//            let mut b = self[b_p];

//            if a.depth > b.depth {
//                std::mem::swap(&mut a, &mut b);
//                dir = !dir;
//            }

//            let face = b.verts.get_subcell(axis, !dir).unwrap();

//            [a, b]
//                .into_iter()
//                .flat_map(|volume| self.get_simplices_between_face(volume.verts, face.clone(), f))
//                .collect()
//            //for vol in [a, b] {
//            //}
//        }

//        pub fn get_simplices_between_face(
//            &self,
//            vol: CellCorners<EvalPoint>,
//            face: CellCorners<EvalPoint>,
//            f: &mut ImplicitFn<3>,
//        ) -> Vec<[EvalPoint; 4]> {
//            let vd = EvalPoint::get_dual(&vol, f);
//            let fd = EvalPoint::get_dual(&face, f);

//            (0..4)
//                .into_iter()
//                .flat_map(move |i| {
//                    let edge = face.get_subcell(i % 2, (i / 2) as u32 == 0).unwrap();
//                    let ed = EvalPoint::get_dual(&edge, f);
//                    edge.iter()
//                        .map(move |v| [vd, fd, ed, *v])
//                        .collect::<Vec<_>>()
//                })
//                .collect()
//        }
//    }

//    pub fn build(
//        min: Vec3,
//        max: Vec3,
//        min_depth: u32,
//        max_cells: u32,
//        program: &[op::Opcode],
//        tol: f64,
//    ) -> (Vec<[Vec3; 3]>, QuadTree) {
//        let mut f = ImplicitFn {
//            program: program.to_vec(),
//            vm: vm::VM::new(),
//        };

//        let tree = QuadTree::build(min.into(), max.into(), min_depth, max_cells, tol, &mut f);
//        let simplicies = tree.get_simplices_from(CellPtr::ROOT, &mut f);

//        let mut faces = vec![];

//        for simplex in simplicies {
//            match march_simplex(&simplex, &mut f, tol) {
//                Some(Primitive::Tri(t)) => faces.push(t),
//                Some(Primitive::Quad(t1, t2)) => {
//                    faces.push(t1);
//                    faces.push(t2);
//                }
//                None => (),
//            }
//        }

//        (faces, tree)
//    }
//}
