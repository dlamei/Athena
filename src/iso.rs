use std::{collections::VecDeque, ops};

use crate::vm;

#[derive(Debug, Clone)]
pub struct ImplicitFn<const N: usize> {
    pub program: Vec<vm::Opcode>,
    pub vm: vm::VM,
}

impl<const N: usize> ImplicitFn<N> {

    pub fn eval_f32(&mut self, input: IsoVec<N>) -> f32 {
        let vm = &mut self.vm;
        for i in 0..N {
            vm.registers[i+1] = input[i];
        }

        vm.eval(&self.program);
        vm.registers[1]
    }

    pub fn eval_range(&mut self, min: IsoVec<N>, max: IsoVec<N>) -> vm::Range {
        let vm = &mut self.vm;

        for i in 0..N {
            vm.registers_range[i+1] = (min[i], max[i]).into();
        }

        vm.eval_range(&self.program);
        vm.registers_range[1]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IsoVec<const N: usize> {
    v: [f32; N],
}

impl<const N: usize> Default for IsoVec<N> {
    fn default() -> Self {
        Self {
            v: [Default::default(); N],
        }
    }
}

impl From<glam::Vec3> for IsoVec<3> {
    fn from(vec: glam::Vec3) -> Self {
        Self {
            v: [vec.x, vec.y, vec.z],
        }
    }
}

impl From<IsoVec<3>> for glam::Vec3 {
    fn from(vec: IsoVec<3>) -> Self {
        glam::Vec3::new(vec[0], vec[1], vec[2])
    }
}

impl From<glam::Vec2> for IsoVec<2> {
    fn from(vec: glam::Vec2) -> Self {
        Self { v: [vec.x, vec.y] }
    }
}

impl From<IsoVec<2>> for glam::Vec2 {
    fn from(vec: IsoVec<2>) -> Self {
        glam::Vec2::new(vec[0], vec[1])
    }
}

impl<const N: usize> ops::Add<f32> for IsoVec<N> {
    type Output = Self;

    fn add(mut self, rhs: f32) -> Self::Output {
        for i in 0..N {
            self[i] += rhs;
        }
        self
    }
}

impl<const N: usize> ops::AddAssign<f32> for IsoVec<N> {
    fn add_assign(&mut self, rhs: f32) {
        *self = *self + rhs
    }
}

impl<const N: usize> ops::Add<IsoVec<N>> for f32 {
    type Output = IsoVec<N>;

    fn add(self, rhs: IsoVec<N>) -> Self::Output {
        rhs + self
    }
}

impl<const N: usize> ops::Add<IsoVec<N>> for IsoVec<N> {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        for i in 0..N {
            self[i] += rhs[i]
        }
        self
    }
}

impl<const N: usize> ops::Sub<f32> for IsoVec<N> {
    type Output = Self;

    fn sub(mut self, rhs: f32) -> Self::Output {
        for i in 0..N {
            self[i] -= rhs
        }
        self
    }
}

impl<const N: usize> ops::SubAssign<f32> for IsoVec<N> {
    fn sub_assign(&mut self, rhs: f32) {
        *self = *self - rhs
    }
}

impl<const N: usize> ops::Sub<IsoVec<N>> for f32 {
    type Output = IsoVec<N>;

    fn sub(self, rhs: IsoVec<N>) -> Self::Output {
        -rhs + self
    }
}

impl<const N: usize> ops::Sub<IsoVec<N>> for IsoVec<N> {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        for i in 0..N {
            self[i] -= rhs[i]
        }
        self
    }
}

impl<const N: usize> ops::Mul<f32> for IsoVec<N> {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        for i in 0..N {
            self[i] *= rhs;
        }
        self
    }
}

impl<const N: usize> ops::Mul<IsoVec<N>> for f32 {
    type Output = IsoVec<N>;

    fn mul(self, rhs: IsoVec<N>) -> Self::Output {
        rhs * self
    }
}

impl<const N: usize> ops::Div<f32> for IsoVec<N> {
    type Output = Self;

    fn div(mut self, rhs: f32) -> Self::Output {
        for i in 0..N {
            self[i] /= rhs;
        }
        self
    }
}

impl<const N: usize> ops::Neg for IsoVec<N> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        -1.0 * self
    }
}

impl<const N: usize> ops::Index<usize> for IsoVec<N> {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.v[index]
    }
}

impl<const N: usize> ops::IndexMut<usize> for IsoVec<N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.v[index]
    }
}

impl<const N: usize> IsoVec<N> {
    #[inline(always)]
    pub fn abs(mut self) -> Self {
        for i in 0..N {
            self[i] = self[i].abs()
        }
        self
    }

    #[inline(always)]
    pub fn max_element(&self) -> f32 {
        let mut max = self.v[0];
        for e in &self.v[1..] {
            max = max.max(*e);
        }
        max
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct EvalPoint<const N: usize> {
    pub pos: IsoVec<N>,
    pub val: f32,
}

//pub trait ImplicitFn<const N: usize>: Fn(IsoVec<N>) -> f32 {}
//impl<const N: usize, F: Fn(IsoVec<N>) -> f32> ImplicitFn<N> for F {}

impl<const N: usize> EvalPoint<N> {
    pub fn eval(pos: IsoVec<N>, f: &mut ImplicitFn<N>) -> Self {
        let val = f.eval_f32(pos);
        Self { pos, val }
    }

    pub fn midpoint(p1: Self, p2: Self, f: &mut ImplicitFn<N>) -> Self {
        let pos = (p1.pos + p2.pos) / 2.0;
        let val = f.eval_f32(pos);
        Self { pos, val }
    }

    pub fn zero_intersect(p1: Self, p2: Self, f: &mut ImplicitFn<N>) -> Self {
        let denom = p1.val - p2.val;
        let k1 = -p2.val / denom;
        let k2 = p1.val / denom;
        let pos = k1 * p1.pos + k2 * p2.pos;
        let val = f.eval_f32(pos);
        Self { pos, val }
    }

    pub fn cube_eval(
        min: IsoVec<N>,
        max: IsoVec<N>,
        f: &mut ImplicitFn<N>,
    ) -> CellCorners<EvalPoint<N>> {
        for i in 0..N {
            debug_assert!(min[i] <= max[i])
        }
        let width = max - min;
        let mut points = CellCorners::with_dim(N, EvalPoint::default());
        for i in 0..1 << N {
            let mut pos = min;
            for j in 0..N {
                if (i >> j) & 1 == 1 {
                    pos[j] += width[j]
                }
            }

            points[i] = EvalPoint::eval(pos, f);
        }

        points
    }

    pub fn get_dual(cells: &CellCorners<EvalPoint<N>>, f: &mut ImplicitFn<N>) -> EvalPoint<N> {
        let verts = cells.as_ref();
        EvalPoint::midpoint(verts[0], verts[verts.len() - 1], f)
    }

    // TODO: tol
    pub fn bin_search_zero(
        p1: EvalPoint<N>,
        p2: EvalPoint<N>,
        f: &mut ImplicitFn<N>,
        tol: f32,
    ) -> EvalPoint<N> {
        if (p1.pos - p2.pos).abs().max_element() < tol {
            EvalPoint::zero_intersect(p1, p2, f)
        } else {
            let mid = EvalPoint::midpoint(p1, p2, f);
            if mid.val.abs() == tol {
                mid
            } else if (mid.val > 0.0) == (p1.val > 0.0) {
                Self::bin_search_zero(mid, p2, f, tol)
            } else {
                Self::bin_search_zero(p1, mid, f, tol)
            }
        }
    }
}

// f(x) = (f(b) - f(a))/(b - a) * (x - a) + f(a)
// f(x) / (f(b) - f(a)) * (b - a) / (x - a) = f(a)

#[derive(Clone, Debug, Copy)]
pub enum CellCorners<T> {
    D1([T; 1 << 1]),
    D2([T; 1 << 2]),
    D3([T; 1 << 3]),
}

impl<T> ops::Deref for CellCorners<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            CellCorners::D1(a) => a,
            CellCorners::D2(a) => a,
            CellCorners::D3(a) => a,
        }
    }
}

impl<T> ops::DerefMut for CellCorners<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            CellCorners::D1(a) => a,
            CellCorners::D2(a) => a,
            CellCorners::D3(a) => a,
        }
    }
}

impl<T> CellCorners<T> {
    pub fn map<S>(self, mut f: impl FnMut(T) -> S) -> CellCorners<S> {
        match self {
            CellCorners::D1(a) => CellCorners::D1(a.map(|x| f(x))),
            CellCorners::D2(a) => CellCorners::D2(a.map(|x| f(x))),
            CellCorners::D3(a) => CellCorners::D3(a.map(|x| f(x))),
        }
    }
}

impl<T: Copy> CellCorners<T> {
    pub const fn with_dim(dim: usize, v: T) -> Self {
        match dim {
            1 => CellCorners::D1([v; 1 << 1]),
            2 => CellCorners::D2([v; 1 << 2]),
            3 => CellCorners::D3([v; 1 << 3]),
            _ => panic!("unsupported dimension"),
        }
    }

    pub fn get_subcell(&self, axis: u32, dir: bool) -> Option<Self> {
        let m = 1 << axis;

        match self {
            CellCorners::D1(_) => None,
            CellCorners::D2(a) => {
                let mut sub_cell = [a[0]; 1 << 1];
                let mut k = 0;
                for (i, vert) in a.iter().enumerate() {
                    if ((i & m) > 0) == dir {
                        sub_cell[k] = *vert;
                        k += 1;
                    }
                }
                CellCorners::D1(sub_cell).into()
            }
            CellCorners::D3(a) => {
                let mut sub_cell = [a[0]; 1 << 2];
                let mut k = 0;
                for (i, vert) in a.iter().enumerate() {
                    if ((i & m) > 0) == dir {
                        sub_cell[k] = *vert;
                        k += 1;
                    }
                }
                CellCorners::D2(sub_cell).into()
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cell<const N: usize> {
    pub depth: u32,
    pub children: Option<CellCorners<CellPtr>>,
    pub parent: Option<CellPtr>,
    pub child_dir: u32, // TODO u8
    pub verts: CellCorners<EvalPoint<N>>,
}

impl<const N: usize> Default for Cell<N> {
    fn default() -> Self {
        Self {
            depth: 0,
            children: None,
            parent: None,
            child_dir: 0,
            verts: CellCorners::with_dim(N, EvalPoint::default()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellPtr(pub(crate) usize);

impl CellPtr {
    // TODO: NonZero
    pub const NONE: Self = Self(usize::MAX);
    pub const ROOT: Self = Self(0);
}

#[derive(Debug, Clone)]
pub struct QuadTree<const N: usize> {
    pub cells: Vec<Cell<N>>,
}

impl<const N: usize> ops::Index<CellPtr> for QuadTree<N> {
    type Output = Cell<N>;

    fn index(&self, index: CellPtr) -> &Self::Output {
        &self.cells[index.0]
    }
}

impl<const N: usize> ops::IndexMut<CellPtr> for QuadTree<N> {
    fn index_mut(&mut self, index: CellPtr) -> &mut Self::Output {
        &mut self.cells[index.0]
    }
}

impl<const N: usize> QuadTree<N> {
    pub fn empty() -> Self {
        Self { cells: vec![] }
    }

    pub fn build(
        min: IsoVec<N>,
        max: IsoVec<N>,
        min_depth: u32,
        max_cells: u32,
        tol: f32,
        f: &mut ImplicitFn<N>,
    ) -> Self {
        let branch_fac = 1u32 << N;

        let max_cells = branch_fac.pow(min_depth).max(max_cells);
        let verts = EvalPoint::cube_eval(min, max, f);

        let mut tree = Self::empty();

        let root = tree.insert(Cell {
            depth: 0,
            children: None,
            parent: None,
            child_dir: 0,
            verts,
        });

        let mut leaf_count = 1;
        let mut quad_queue = VecDeque::from([root]);

        while !quad_queue.is_empty() && leaf_count < max_cells {
            let curr = quad_queue.pop_front().unwrap();
            if tree.should_descend(curr, f, tol) {
                let children = tree.compute_children(curr, f);
                // todo: priority
                children.into_iter().for_each(|c| quad_queue.push_back(*c));
                leaf_count += branch_fac - 1;
            }
        }

        tree
    }

    pub fn insert(&mut self, cell: Cell<N>) -> CellPtr {
        self.cells.push(cell);
        CellPtr(self.cells.len() - 1)
    }

    pub fn compute_children(
        &mut self,
        cell_ptr: CellPtr,
        f: &mut ImplicitFn<N>,
    ) -> &CellCorners<CellPtr> {
        //) -> &[CellPtr; 1 << N] {
        let cell = &self[cell_ptr];
        assert!(cell.children.is_none());

        let mut new_cells = CellCorners::with_dim(N, Cell::default());

        for (i, vert) in cell.verts.iter().enumerate() {
            let min = (cell.verts[0].pos + vert.pos) / 2.0;
            let max = (cell.verts[(1 << N) - 1].pos + vert.pos) / 2.0;
            let verts = EvalPoint::cube_eval(min, max, f);
            new_cells[i] = Cell {
                depth: cell.depth + 1,
                children: None,
                parent: Some(cell_ptr),
                child_dir: i as u32,
                verts,
            };
        }

        self[cell_ptr].children = Some(new_cells.map(|cell| self.insert(cell)));
        self[cell_ptr].children.as_ref().unwrap()
    }

    pub fn get_leaves_in_dir(&self, cell_ptr: CellPtr, axis: u32, dir: bool) -> Vec<CellPtr> {
        if let Some(children) = self[cell_ptr].children {
            let m = 1 << axis;
            children
                .iter()
                .copied()
                .filter(move |cell_ptr| ((self[*cell_ptr].depth & m) > 0) == dir)
                .flat_map(move |cell_ptr| self.get_leaves_in_dir(cell_ptr, axis, dir))
                .collect()
        } else {
            vec![cell_ptr]
            //Box::new(std::iter::once(cell_ptr))
        }
    }

    pub fn walk_in_dir(&self, cell_ptr: CellPtr, axis: u32, dir: bool) -> Option<CellPtr> {
        let cell = &self[cell_ptr];
        let m = 1 << axis;

        if ((cell.child_dir & m) > 0) == dir {
            let parent = cell.parent?;
            let parent_walk = self.walk_in_dir(parent, axis, dir)?;
            self[parent_walk]
                .children
                .map(|children| children[(cell.child_dir ^ m) as usize])
        } else {
            let parent = cell.parent?;
            Some(self[parent].children.unwrap()[(cell.child_dir ^ m) as usize])
        }
    }

    pub fn walk_leaves_in_dir(
        &self,
        cell_ptr: CellPtr,
        axis: u32,
        dir: bool,
    ) -> Option<Vec<CellPtr>> {
        let walk = self.walk_in_dir(cell_ptr, axis, dir)?;
        Some(self.get_leaves_in_dir(walk, axis, dir))
    }

    pub fn should_descend(&self, cell_ptr: CellPtr, f: &mut ImplicitFn<N>, tol: f32) -> bool {
        let cell = &self[cell_ptr];

        // TODO : abs()?
        if (cell.verts[(1 << N) - 1].pos - cell.verts[0].pos)
            .max_element()
            .abs()
            < 10.0 * tol
        {
            return false
        }

        let range = f.eval_range(cell.verts[0].pos, cell.verts[(1<<N) - 1].pos);
        range.contains_zero() || range.is_non_continuous()
        
        // if range.contains_zero() || range.is_non_continuous() {
        //     true
        // } else if cell.verts.iter().all(|v| v.val.is_nan()) {
        //     false
        // } else if cell.verts.iter().any(|v| v.val.is_nan()) {
        //     true
        // } else {
        //     // TODO: grad, second-deriv
        //     cell.verts[1..]
        //         .iter()
        //         .any(|v| v.val.signum() != cell.verts[0].val.signum())
        // }
    }
}

pub mod line {
    use glam::Vec2;
    use ordered_float::OrderedFloat;
    use std::{collections::HashMap, ops, rc::Rc};

    use crate::vm::{self, op};

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
    struct Point(OrderedFloat<f32>, OrderedFloat<f32>);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TriPtr(usize);

    impl TriPtr {
        const ROOT: Self = TriPtr(0);
    }

    struct Triangulator<'a> {
        triangles: Vec<Triangle>,
        f: &'a mut ImplicitFn<2>,
        tol: f32,
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

    impl  Triangulator<'_> {
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

            let int = EvalPoint::bin_search_zero(pos, neg, self.f, self.tol);

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

                let df1 = self.f.eval_f32(p1.pos * (1.0 - dt) + p2.pos * dt);
                let df2 = self.f.eval_f32(p1.pos + p2.pos * (1.0 - dt));

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
                        .map(|p| Vec2::new(p.pos[0], p.pos[1]))
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
        // implicit_fn: impl Fn(Vec2) -> f32,
        program: &[op::Opcode],
    ) -> (Vec<Vec<Vec2>>, QuadTree) {
        let tol = 1e-5;

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

pub mod surface {

    use glam::Vec3;

    use crate::vm::{self, op};

    use super::{CellCorners, CellPtr, ImplicitFn};

    type QuadTree = super::QuadTree<3>;
    type EvalPoint = super::EvalPoint<3>;
    type IsoVec = super::IsoVec<3>;

    const TETRAHEDRON_TABLE: [&'static [(u32, u32)]; 8] = [
        /*0b0000*/ &[], // falsey
        /*0b0001*/ &[(0, 3), (1, 3), (2, 3)],
        /*0b0010*/ &[(0, 2), (1, 2), (3, 2)],
        /*0b0100*/ &[(0, 1), (2, 1), (3, 1)],
        /*0b1000*/ &[(1, 0), (2, 0), (3, 0)],
        /*0b0011*/ &[(0, 2), (2, 1), (1, 3), (3, 0)],
        /*0b0110*/ &[(0, 1), (1, 3), (3, 2), (2, 0)],
        /*0b0101*/ &[(0, 1), (1, 2), (2, 3), (3, 0)],
    ];

    const fn tetrahedron_table(id: u32) -> Option<&'static [(u32, u32)]> {
        Some(match id {
            //0b0000 => TETRAHEDRON_TABLE[0], // falsey
            0b0001 => TETRAHEDRON_TABLE[1],
            0b0010 => TETRAHEDRON_TABLE[2],
            0b0100 => TETRAHEDRON_TABLE[3],
            0b1000 => TETRAHEDRON_TABLE[4],
            0b0011 => TETRAHEDRON_TABLE[5],
            0b0110 => TETRAHEDRON_TABLE[6],
            0b0101 => TETRAHEDRON_TABLE[7],
            _ => return None,
        })
    }

    pub fn march_indices(simplex: &[EvalPoint]) -> Option<&'static [(u32, u32)]> {
        let mut id = 0;
        for p in simplex {
            id = 2 * id + (p.val > 0.0) as u32;
        }

        if let Some(res) = tetrahedron_table(id) {
            Some(res)
        } else {
            tetrahedron_table(id ^ 0b1111)
        }
    }

    pub enum Primitive {
        Tri([Vec3; 3]),
        Quad([Vec3; 3], [Vec3; 3]),
    }

    pub fn march_simplex(
        simplex: &[EvalPoint],
        f: &mut ImplicitFn<3>,
        tol: f32,
    ) -> Option<Primitive> {
        let Some(indices) = march_indices(simplex) else {
            return None;
        };

        let mut pts = vec![];
        for (i, j) in indices {
            let intersec =
                EvalPoint::bin_search_zero(simplex[*i as usize], simplex[*j as usize], f, tol);
            pts.push(intersec.pos);
        }

        if pts.len() == 3 {
            let p0 = pts[0].into();
            let p1 = pts[1].into();
            let p2 = pts[2].into();
            Primitive::Tri([p0, p1, p2])
        } else {
            let p0 = pts[0].into();
            let p1 = pts[1].into();
            let p2 = pts[2].into();
            let p3 = pts[3].into();
            Primitive::Quad([p0, p1, p3], [p1, p2, p3])
        }
        .into()
    }

    //pub struct SimplexGen {
    //    tree: QuadTree<3>,
    //    //sample_fn: F,
    //}

    impl QuadTree {
        pub fn get_simplices_from(
            &self,
            cell_ptr: CellPtr,
            f: &mut ImplicitFn<3>,
        ) -> Vec<[EvalPoint; 4]> {
            let cell = self[cell_ptr];

            if let Some(children) = cell.children {
                children
                    .iter()
                    .copied()
                    .flat_map(|child| self.get_simplices_from(child, f))
                    .collect()
            } else {
                let mut evals = vec![];
                for axis in [0, 1, 2] {
                    for dir in [false, true] {
                        if let Some(adj) = self.walk_leaves_in_dir(cell_ptr, axis, dir) {
                            evals.extend(adj.into_iter().flat_map(|leaf| {
                                self.get_simplices_between(cell_ptr, leaf, axis, dir, f)
                            }))
                        } else {
                            let sub = cell.verts;
                            evals.extend(self.get_simplices_between_face(
                                sub.clone(),
                                sub.get_subcell(axis, dir).unwrap(),
                                f,
                            ))
                        }
                    }
                }
                evals
            }
        }

        pub fn get_simplices_between(
            &self,
            a_p: CellPtr,
            b_p: CellPtr,
            axis: u32,
            mut dir: bool,
            f: &mut ImplicitFn<3>,
        ) -> Vec<[EvalPoint; 4]> {
            let mut a = self[a_p];
            let mut b = self[b_p];

            if a.depth > b.depth {
                std::mem::swap(&mut a, &mut b);
                dir = !dir;
            }

            let face = b.verts.get_subcell(axis, !dir).unwrap();

            [a, b]
                .into_iter()
                .flat_map(|volume| self.get_simplices_between_face(volume.verts, face.clone(), f))
                .collect()
            //for vol in [a, b] {
            //}
        }

        pub fn get_simplices_between_face(
            &self,
            vol: CellCorners<EvalPoint>,
            face: CellCorners<EvalPoint>,
            f: &mut ImplicitFn<3>,
        ) -> Vec<[EvalPoint; 4]> {
            let vd = EvalPoint::get_dual(&vol, f);
            let fd = EvalPoint::get_dual(&face, f);

            (0..4)
                .into_iter()
                .flat_map(move |i| {
                    let edge = face.get_subcell(i % 2, (i / 2) as u32 == 0).unwrap();
                    let ed = EvalPoint::get_dual(&edge, f);
                    edge.iter()
                        .map(move |v| [vd, fd, ed, *v])
                        .collect::<Vec<_>>()
                })
                .collect()
        }
    }

    pub fn build(
        min: Vec3,
        max: Vec3,
        min_depth: u32,
        max_cells: u32,
        program: &[op::Opcode]
    ) -> (Vec<[Vec3; 3]>, QuadTree) {
        let tol = 1e-5;

        let mut f = ImplicitFn {
            program: program.to_vec(),
            vm: vm::VM::new()
        };

        let tree = QuadTree::build(min.into(), max.into(), min_depth, max_cells, tol, &mut f);
        let simplicies = tree.get_simplices_from(CellPtr::ROOT, &mut f);

        let mut faces = vec![];

        for simplex in simplicies {
            match march_simplex(&simplex, &mut f, tol) {
                Some(Primitive::Tri(t)) => faces.push(t),
                Some(Primitive::Quad(t1, t2)) => {
                    faces.push(t1);
                    faces.push(t2);
                }
                None => (),
            }
        }

        (faces, tree)
    }
}
