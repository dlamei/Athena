use std::{collections::VecDeque, ops};

use glam::Vec2;

use crate::{
    iso::{self, CellCorners, CellPtr},
    vm,
};

fn quad_from_extrema(min: Vec2, max: Vec2) -> [Vec2; 4] {
    for i in 0..2 {
        debug_assert!(min[i] <= max[i])
    }
    let width = max - min;
    let mut points = [Vec2::ZERO; 4];
    for i in 0..1 << 2 {
        let mut pos = min;
        for j in 0..2 {
            if (i >> j) & 1 == 1 {
                pos[j] += width[j]
            }
        }

        points[i] = pos;
    }
    points
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitFn {
    /// store the implicit functin as bytecode with signature fn(x, y) -> f32
    ///
    /// store x and y in 1 / 2 registers before evaluating
    /// result is stored in register 1
    program: Vec<vm::Opcode>,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Cell {
    pub depth: u32,
    pub children: Option<CellCorners<CellPtr>>,
    pub parent: Option<CellPtr>,
    pub child_dir: u32, // TODO u8
    pub verts: [Vec2; 4],
}

#[derive(Debug, Clone)]
pub struct QuadTree {
    pub vm: vm::VM,
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
        Self {
            cells: vec![],
            vm: vm::VM::new(),
        }
    }

    pub fn build(min: Vec2, max: Vec2, min_depth: u32, max_cells: u32, tol: f32, f: &ImplicitFn) -> Self {
        let branch_fac = 1u32 << 2;
        let max_cells = branch_fac.pow(min_depth).max(max_cells);

        let mut tree = Self::empty();

        let verts = quad_from_extrema(min, max);
        let root = tree.insert(Cell {
            depth: 0,
            children: None,
            parent: None,
            verts,
            child_dir: 0,
        });

        let mut queue = VecDeque::from([root]);

        let mut leaf_count = 0;
        while !queue.is_empty() && leaf_count < max_cells {
            let cell_p = queue.pop_front().unwrap();
            //if tree.cell_contains_zero(cell_p, &f) {
            if tree.should_descend(cell_p, tol, &f) {
                let children = tree.compute_children(cell_p, &f);
                children.into_iter().for_each(|c| queue.push_back(*c));
                leaf_count += branch_fac - 1;
            }
        }

        tree
    }

    pub fn should_descend(&mut self, cell_ptr: CellPtr, tol: f32, f: &ImplicitFn) -> bool {
        let cell = &self[cell_ptr];

        // TODO : abs()?
        if (cell.verts[(1 << 2) - 1] - cell.verts[0])
            .max_element()
            .abs()
            < 10.0 * tol
        {
            false
        } else {
            self.cell_contains_zero(cell_ptr, &f)
        }
    }

    pub fn cell_contains_zero(&mut self, cell_ptr: CellPtr, f: &ImplicitFn) -> bool {
        let cell = &self[cell_ptr];

        let range = self.eval_range(cell.verts[0], cell.verts[3], f);
        range.contains_zero() || range.is_non_continuous()
    }

    pub fn insert(&mut self, cell: Cell) -> CellPtr {
        self.cells.push(cell);
        CellPtr(self.cells.len() - 1)
    }

    pub fn compute_children(
        &mut self,
        cell_ptr: iso::CellPtr,
        f: &ImplicitFn,
    ) -> &iso::CellCorners<iso::CellPtr> {
        //) -> &[CellPtr; 1 << N] {
        let cell = &self[cell_ptr];
        assert!(cell.children.is_none());

        let mut new_cells = CellCorners::with_dim(2, Cell::default());

        for (i, v_pos) in cell.verts.iter().enumerate() {
            let min = (cell.verts[0] + v_pos) / 2.0;
            let max = (cell.verts[3] + v_pos) / 2.0;
            //let verts = EvalPoint::cube_eval(min, max, &f);
            let verts = quad_from_extrema(min, max);
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

    pub fn eval_range(&mut self, min: Vec2, max: Vec2, f: &ImplicitFn) -> vm::Range {
        self.vm.registers_range[1] = (min.x, max.x).into();
        self.vm.registers_range[2] = (min.y, max.y).into();

        self.vm.eval_range(&f.program);
        self.vm.registers_range[1]
    }
}

pub fn build(min: Vec2, max: Vec2, min_depth: u32, max_cells:  u32) -> QuadTree {
    use vm::op;
    let tol = 1e-5;

    // let program = vec![
    //     op::ADD_LHS_RHS(1, 2, 3),
    //     op::POW_IMM_RHS(3.0, 3, 4),
    //     op::MUL_LHS_RHS(3, 4, 3),
    //     op::SIN(3, 3),
    //     op::SIN(1, 1),
    //     op::SIN(2, 2),
    //     op::ADD_LHS_RHS(1, 2, 1),
    //     op::SUB_LHS_RHS(1, 3, 1),
    //     op::EXT(0),
    // ];
    let program = vec![
        op::POW_IMM_RHS(3.0, 1, 1),
        op::SIN(1, 1),
        op::POW_LHS_IMM(1, -1.0, 1),
        op::SUB_LHS_RHS(1, 2, 1),
        op::SIN(2, 2),
        op::ADD_LHS_RHS(1, 2, 1),
        op::EXT(0),
    ];

    let f = ImplicitFn { program };

    let tree = QuadTree::build(min, max, min_depth, max_cells, tol, &f);
    tree
}
