use std::{cell::RefCell, collections::VecDeque, ops, rc::Rc};

use glam::Vec3;

const N: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct EvalPos {
    pub pos: Vec3,
    pub val: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dir {
    Pos,
    Neg,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub trait IsoVec {
    type F;
    type Cell<T>;
    const N: usize;
}



pub trait IsoSampleFn: Fn(Vec3) -> f32 {}
impl<F: Fn(Vec3) -> f32> IsoSampleFn for F {}

impl EvalPos {
    pub fn eval(pos: Vec3, f: impl IsoSampleFn) -> Self {
        let val = f(pos);
        Self { pos, val }
    }

    pub fn midpoint(p1: Self, p2: Self, f: impl IsoSampleFn) -> Self {
        let pos = (p1.pos + p2.pos) / 2.0;
        let val = f(pos);
        Self { pos, val }
    }

    pub fn zero_intersect(p1: Self, p2: Self, f: impl IsoSampleFn) -> Self {
        let denom = p1.val - p2.val;
        let k1 = -p2.val / denom;
        let k2 = p1.val / denom;
        let pos = k1 * p1.pos + k2 * p2.pos;
        let val = f(pos);
        Self { pos, val }
    }
}

// f(x) = (f(b) - f(a))/(b - a) * (x - a) + f(a)


pub fn bin_search_zero(p1: EvalPos, p2: EvalPos, f: impl IsoSampleFn, tol: f32) -> EvalPos {
    if (p1.pos - p2.pos).abs().max_element() < tol {
        EvalPos::zero_intersect(p1, p2, f)
    } else {
        let mid = EvalPos::midpoint(p1, p2, &f);
        if mid.val == 0.0 {
            mid
        } else if (mid.val > 0.0) == (p1.val > 0.0) {
            bin_search_zero(mid, p2, f, tol)
        } else {
            bin_search_zero(p1, mid, f, tol)
        }
    }
}
pub fn verts_from_extrema(min: Vec3, max: Vec3, f: impl IsoSampleFn) -> [EvalPos; 1 << N] {
    debug_assert!(min.x <= max.x && min.y <= max.y && min.z <= max.z);
    let width = max - min;

    let mut points = [Default::default(); 1 << N];

    for i in 0..1 << N {
        let mut pos = min;
        for j in 0..N {
            if (i >> j) & 1 == 1 {
                pos[j] += width[j]
            }
        }

        points[i] = EvalPos::eval(pos, &f);
    }

    points
}

#[derive(Debug, Clone, Default)]
pub struct SubCell {
    verts: smallvec::SmallVec<[EvalPos; 1 << N]>,
}

impl SubCell {
    pub fn get_subcell(&self, axis: u32, dir: bool) -> SubCell {
        let m = 1 << axis;
        let mut subcell = SubCell::default();

        for (i, vert) in self.verts.iter().enumerate() {
            if ((i & m) > 0) == dir {
                subcell.verts.push(*vert);
            }
        }

        subcell
    }

    pub fn get_dual(&self, f: impl IsoSampleFn) -> EvalPos {
        EvalPos::midpoint(self.verts[0], *self.verts.last().unwrap(), f)
    }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct Cell {
    pub depth: u32,
    pub children: Option<[CellPtr; 1 << N]>,
    pub parent: Option<CellPtr>,
    pub child_dir: u32, // TODO u8
    pub verts: [EvalPos; 1 << N],
}

impl Cell {
    //pub fn get_subcell(&self, axis: u32, dir: bool) -> SubCell {
    //    let m = 1 << axis;
    //    let mut subcell = SubCell::default();

    //    for (i, vert) in self.verts.iter().enumerate() {
    //        if ((i & m) > 0) == dir {
    //            subcell.verts.push(vert);
    //        }
    //    }

    //    subcell
    //}
    pub fn as_subcell(&self) -> SubCell {
        SubCell {
            verts: self.verts.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellPtr(usize);

impl CellPtr {
    // TODO: NonZero
    pub const NONE: Self = Self(usize::MAX);
    pub const ROOT: Self = Self(0);
}

#[derive(Debug, Clone)]
pub struct QuadTree {
    pub cells: Vec<Cell>,
}

impl ops::Index<CellPtr> for QuadTree {
    type Output = Cell;

    fn index(&self, index: CellPtr) -> &Self::Output {
        &self.cells[index.0]
    }
}

impl ops::IndexMut<CellPtr> for QuadTree {
    fn index_mut(&mut self, index: CellPtr) -> &mut Self::Output {
        &mut self.cells[index.0]
    }
}

impl QuadTree {
    pub fn empty() -> Self {
        Self { cells: vec![] }
    }

    pub fn build(
        min: Vec3,
        max: Vec3,
        min_depth: u32,
        max_cells: u32,
        tol: f32,
        f: impl IsoSampleFn,
    ) -> Self {
        let branch_fac = 1u32 << N;

        let max_cells = branch_fac.pow(min_depth).max(max_cells);
        let verts = verts_from_extrema(min, max, &f);

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
            if tree[curr].depth < min_depth || tree.should_descend(curr, tol) {
                let children = tree.compute_children(curr, &f);
                // todo: priority
                children.into_iter().for_each(|c| quad_queue.push_back(*c));
                leaf_count += branch_fac - 1;
            }
        }

        tree
    }

    pub fn insert(&mut self, cell: Cell) -> CellPtr {
        self.cells.push(cell);
        CellPtr(self.cells.len() - 1)
    }

    pub fn compute_children(
        &mut self,
        cell_ptr: CellPtr,
        f: impl IsoSampleFn,
    ) -> &[CellPtr; 1 << N] {
        let cell = &self[cell_ptr];
        assert!(cell.children.is_none());

        let mut new_cells = [Default::default(); 1 << N];

        for (i, vert) in cell.verts.iter().enumerate() {
            let min = (cell.verts[0].pos + vert.pos) / 2.0;
            let max = (cell.verts[(1 << N) - 1].pos + vert.pos) / 2.0;
            let verts = verts_from_extrema(min, max, &f);
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
                .into_iter()
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

    pub fn should_descend(&self, cell_ptr: CellPtr, tol: f32) -> bool {
        let cell = &self[cell_ptr];

        // TODO : abs()?
        if (cell.verts[(1 << N) - 1].pos - cell.verts[0].pos)
            .max_element()
            .abs()
            < 10.0 * tol
        {
            false
        } else if cell.verts.iter().all(|v| v.val.is_nan()) {
            false
        } else if cell.verts.iter().any(|v| v.val.is_nan()) {
            true
        } else {
            // TODO: grad, second-deriv
            cell.verts[1..]
                .iter()
                .any(|v| v.val.signum() != cell.verts[0].val.signum())
        }
    }
}

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
        0b0000 => TETRAHEDRON_TABLE[0], // falsey
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

pub fn march_indices(simplex: &[EvalPos]) -> &'static [(u32, u32)] {
    let mut id = 0;
    for p in simplex {
        id = 2 * id + (p.val > 0.0) as u32;
    }

    if let Some(res) = tetrahedron_table(id) {
        res
    } else {
        tetrahedron_table(id ^ 0b1111).unwrap()
    }
}

pub enum Primitive {
    Tri([Vec3; 3]),
    Quad([Vec3; 3], [Vec3; 3]),
}

pub fn march_simplex(simplex: &[EvalPos], f: impl IsoSampleFn, tol: f32) -> Option<Primitive> {
    let indices = march_indices(simplex);

    if indices.is_empty() {
        return None;
    }

    let mut pts = vec![];
    for (i, j) in indices {
        let intersec = bin_search_zero(simplex[*i as usize], simplex[*j as usize], &f, tol);
        pts.push(intersec.pos);
    }

    if pts.len() == 3 {
        Primitive::Tri([pts[0], pts[1], pts[2]])
    } else {
        Primitive::Quad([pts[0], pts[1], pts[3]], [pts[1], pts[2], pts[3]])
    }
    .into()
}

pub struct SimplexGen<F> {
    tree: QuadTree,
    sample_fn: F,
}

impl<F: IsoSampleFn> SimplexGen<F> {
    pub fn get_simplices_from(&self, cell_ptr: CellPtr) -> Vec<[EvalPos; 4]> {
        let cell = self.tree[cell_ptr];

        if let Some(children) = cell.children {
            children
                .into_iter()
                .flat_map(|child| self.get_simplices_from(child))
                .collect()
        } else {
            let mut evals = vec![];
            for axis in [0, 1, 2] {
                for dir in [false, true] {
                    if let Some(adj) = self.tree.walk_leaves_in_dir(cell_ptr, axis, dir) {
                        evals.extend(
                            adj.into_iter().flat_map(|leaf| {
                                self.get_simplices_between(cell_ptr, leaf, axis, dir)
                            }),
                        )
                    } else {
                        let sub = cell.as_subcell();
                        evals.extend(
                            self.get_simplices_between_face(
                                sub.clone(),
                                sub.get_subcell(axis, dir),
                            ),
                        )
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
    ) -> Vec<[EvalPos; 4]> {
        let mut a = self.tree[a_p];
        let mut b = self.tree[b_p];

        if a.depth > b.depth {
            std::mem::swap(&mut a, &mut b);
            dir = !dir;
        }

        let face = b.as_subcell().get_subcell(axis, !dir);

        [a, b]
            .into_iter()
            .flat_map(|volume| self.get_simplices_between_face(volume.as_subcell(), face.clone()))
            .collect()
        //for vol in [a, b] {
        //}
    }

    pub fn get_simplices_between_face(&self, vol: SubCell, face: SubCell) -> Vec<[EvalPos; 4]> {
        let vd = vol.get_dual(&self.sample_fn);
        let fd = face.get_dual(&self.sample_fn);

        (0..4)
            .into_iter()
            .flat_map(move |i| {
                let edge = face.get_subcell(i % 2, (i / 2) as u32 == 0);
                let ed = edge.get_dual(&self.sample_fn);
                edge.verts
                    .iter()
                    .map(move |v| [vd, fd, ed, *v])
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    //pub fn get_simplices_between_face(&self, vol: SubCell, face: SubCell) -> impl Iterator<Item = [EvalPos;4]> + use<'_, F> {
    //    let vd = vol.get_dual(&self.sample_fn);
    //    let fd = face.get_dual(&self.sample_fn);

    //    (0..4).into_iter().flat_map(move |i| {

    //        let edge = face.get_subcell(i%2, (i/2) as u32 == 0);
    //        let ed = edge.get_dual(&self.sample_fn);

    //        let res: Vec<_> = edge.verts.iter().map(move |v| {
    //            [
    //                vd,
    //                fd,
    //                ed,
    //                *v,
    //            ]
    //        }).collect();
    //        res
    //    })
    //}
}

pub fn get_isosurface(
    min: Vec3,
    max: Vec3,
    min_depth: u32,
    max_cells: u32,
    f: impl IsoSampleFn,
) -> (Vec<[Vec3; 3]>, QuadTree) {
    let tol = 1e-5;

    let tree = QuadTree::build(min, max, min_depth, max_cells, tol, &f);

    let gen = SimplexGen {
        tree,
        sample_fn: &f,
    };

    let simplicies = gen.get_simplices_from(CellPtr::ROOT);

    let mut faces = vec![];

    for simplex in simplicies {
        match march_simplex(&simplex, &f, tol) {
            Some(Primitive::Tri(t)) => faces.push(t),
            Some(Primitive::Quad(t1, t2)) => {
                faces.push(t1);
                faces.push(t2);
            }
            None => (),
        }
    }

    (faces, gen.tree)
}

#[cfg(test)]
mod test {
    use super::*;
    use glam::Vec3;

    #[test]
    fn verts_from_extrema_test() {
        let min = Vec3::new(0.0, 0.0, 0.0);
        let max = Vec3::new(1.0, 1.0, 1.0);

        let f = |v: Vec3| v.x + v.y + v.z;

        let res = verts_from_extrema(min, max, f);

        assert_eq!(
            res,
            [
                EvalPos::eval((0.0, 0.0, 0.0).into(), f), // 0: dl
                EvalPos::eval((1.0, 0.0, 0.0).into(), f), // 1: dr
                EvalPos::eval((0.0, 1.0, 0.0).into(), f), // 2: dfl
                EvalPos::eval((1.0, 1.0, 0.0).into(), f), // 3: dfr
                EvalPos::eval((0.0, 0.0, 1.0).into(), f), // 4: upl
                EvalPos::eval((1.0, 0.0, 1.0).into(), f), // 5: upr
                EvalPos::eval((0.0, 1.0, 1.0).into(), f), // 6: upfl
                EvalPos::eval((1.0, 1.0, 1.0).into(), f), // 7: upfr
            ]
        )
    }
}
