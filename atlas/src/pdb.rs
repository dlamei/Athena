use std::fmt;

use glam::{Mat3, Mat4, Vec3, Vec4};

pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
    len: usize,
}

#[derive(Clone)]
struct Data<'a> {
    category: &'a str,
    dict: Vec<(&'a str, &'a str)>,
}

impl<'a> Data<'a> {
    fn get_value(&'a self, field: &str) -> Option<&'a str> {
        self.dict
            .iter()
            .find_map(|(key, value)| if *key == field { Some(*value) } else { None })
    }

    fn get_f32(&'a self, field: &str) -> Option<f32> {
        self.get_value(field).map(|s| s.parse().unwrap())
    }
}

impl fmt::Debug for Data<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:\n", self.category)?;
        for (k, v) in &self.dict {
            write!(f, "{k}\t{v}\n");
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Loop<'a> {
    category: &'a str,
    columns: Vec<&'a str>,
    data: Vec<Vec<&'a str>>,
}

impl<'a> Loop<'a> {
    fn get_column(&'a self, column: &str) -> Option<Vec<&'a str>> {
        let indx = self.columns.iter().position(|c| *c == column)?;

        let mut column = vec![];
        for row in &self.data {
            column.push(row[indx]);
        }

        Some(column)
    }

    fn parse_column<T: std::str::FromStr>(&'a self, column: &str) -> Option<Vec<T>> {
        Some(
            self.get_column(column)?
                .iter()
                .map(|s| s.parse().ok().unwrap())
                .collect(),
        )
    }

    fn get_f32_column(&'a self, column: &str) -> Option<Vec<f32>> {
        Some(
            self.get_column(column)?
                .iter()
                .map(|s| s.parse().unwrap())
                .collect(),
        )
    }
}

impl fmt::Debug for Loop<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:\n", self.category)?;
        for c in &self.columns {
            write!(f, "\t{c}")?;
        }

        for r in &self.data {
            for c in r {
                write!(f, "\t{c}")?;
            }
            write!(f, "\n");
        }
        Ok(())
    }
}

#[derive(Clone)]
enum Block<'a> {
    Data(Data<'a>),
    Loop(Loop<'a>),
}

impl fmt::Debug for Block<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Block::Data(x) => write!(f, "{x:?}"),
            Block::Loop(x) => write!(f, "{x:?}"),
        }
    }
}

#[derive(Clone)]
struct CIFData<'a> {
    title: &'a str,
    blocks: Vec<Block<'a>>,
}

impl fmt::Debug for CIFData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n", self.title)?;

        for b in &self.blocks {
            write!(f, "{b:?}\n")?;
        }

        Ok(())
    }
}

impl<'a> CIFData<'a> {
    fn get_data_block(&self, block_name: &str) -> Option<&Data<'a>> {
        if let Some(Block::Data(block)) = self.get_block(block_name) {
            Some(block)
        } else {
            None
        }
    }

    fn get_loop_block(&self, block_name: &str) -> Option<&Loop<'a>> {
        if let Some(Block::Loop(block)) = self.get_block(block_name) {
            Some(block)
        } else {
            None
        }
    }

    fn get_block(&self, block_name: &str) -> Option<&Block<'a>> {
        self.blocks.iter().find(|b| match b {
            Block::Data(data) => data.category == block_name,
            Block::Loop(loop_) => loop_.category == block_name,
        })
    }
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given input.
    pub fn new(input: &'a str) -> Self {
        Parser {
            input,
            pos: 0,
            len: input.len(),
        }
    }

    pub fn next_item(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        let start = self.pos;
        let ch = self.peek()?;

        // Determine how to consume based on delimiter
        let end = match ch {
            '\'' => {
                // Single-quoted string: consume until closing quote
                self.advance(); // skip opening '
                self.parse_until(|c| c == '\'')?;
                self.advance(); // include closing '
                self.pos
            }
            ';' => {
                // Multiline: consume until next semicolon
                self.advance(); // skip opening ;
                self.parse_until(|c| c == ';')?;
                self.advance(); // include closing ;
                self.pos
            }
            _ if !ch.is_whitespace() => {
                // Regular token: consume until whitespace
                self.parse_until(|c| c.is_whitespace())?;
                self.pos
            }
            _ => return None,
        };

        Some(&self.input[start..end])
    }

    fn peek_item(&mut self) -> Option<&'a str> {
        let tmp = self.store_state();
        let item = self.next_item();
        self.load_state(tmp);
        item
    }

    fn store_state(&self) -> usize {
        self.pos
    }

    fn load_state(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn parse_until<F>(&mut self, mut predicate: F) -> Option<()>
    where
        F: FnMut(char) -> bool,
    {
        while let Some(c) = self.peek() {
            if predicate(c) {
                break;
            }
            self.advance();
        }
        Some(())
    }

    /// Skip any whitespace characters, updating `pos`.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '#' {
                self.parse_until(|c| c == '\n');
            } else {
                break;
            }
        }
    }

    /// Peek at the current character without advancing.
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Advance `pos` by one character.
    fn advance(&mut self) {
        if let Some((i, _)) = self.input[self.pos..].char_indices().nth(1) {
            self.pos += i;
        } else {
            self.pos = self.input.len();
        }
    }

    fn parse_file(&mut self) -> CIFData {
        let title = self.next_item().unwrap();

        let mut cif_data = CIFData {
            title,
            blocks: vec![],
        };

        loop {
            let Some(item) = self.peek_item() else {
                break;
            };

            if item.starts_with("_") {
                let mut data = Data {
                    category: item.split_once(".").unwrap().0,
                    dict: vec![],
                };

                loop {
                    let Some(item) = self.peek_item() else { break };
                    let (category, key) = item.split_once(".").unwrap();
                    if category != data.category {
                        break;
                    }
                    self.next_item();

                    let value = self.next_item().unwrap();
                    data.dict.push((key, value));

                    if let Some(item) = self.peek_item() {
                        if !item.starts_with("_") {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                cif_data.blocks.push(Block::Data(data));
            } else if item.starts_with("loop_") {
                let _ = self.next_item().unwrap();
                let item = self.peek_item().unwrap();

                let mut loop_ = Loop {
                    category: item.split_once(".").unwrap().0,
                    columns: vec![],
                    data: vec![],
                };

                // parse columns
                loop {
                    let Some(item) = self.peek_item() else { break };
                    let (category, key) = item.split_once(".").unwrap();
                    if category != loop_.category {
                        break;
                    }
                    self.next_item();

                    loop_.columns.push(key);

                    if let Some(item) = self.peek_item() {
                        if !item.starts_with("_") {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let n_items = loop_.columns.len();

                loop {
                    let mut row = vec![];
                    for _ in 0..n_items {
                        row.push(self.next_item().unwrap());
                    }

                    loop_.data.push(row);

                    if let Some(item) = self.peek_item() {
                        if item.starts_with("_") || item.starts_with("loop_") {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                cif_data.blocks.push(Block::Loop(loop_));
            } else {
                // for _ in 0..20 {
                //     if let Some(item) = self.next_item() {
                //         println!("{}", item);
                //     }
                // }
                panic!();
            }
        }

        cif_data
    }
}


pub fn generate_sphere(
    radius: f32,
    lat_segments: u32,
    long_segments: u32,
) -> (Vec<AtomVertex>, Vec<u32>) {
    // let mut vertices: Vec<Vec3> = Vec::new();
    // let mut normals: Vec<Vec3> = Vec::new();
    let mut vert = vec![];
    let mut indices: Vec<u32> = Vec::new();

    for lat in 0..=lat_segments {
        let theta = std::f32::consts::PI * (lat as f32) / (lat_segments as f32);
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=long_segments {
            let phi = 2.0 * std::f32::consts::PI * (lon as f32) / (long_segments as f32);
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let nx = sin_theta * cos_phi;
            let ny = cos_theta;
            let nz = sin_theta * sin_phi;

            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;

            let pos = Vec3::new(x, y, z);
            let v = AtomVertex {
                pos: pos,
                norm: pos.normalize(),
            };
            vert.push(v);
        }
    }

    for lat in 0..lat_segments {
        for lon in 0..long_segments {
            let first = lat * (long_segments + 1) + lon;
            let second = first + long_segments + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vert, indices)
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct AtomVertex {
    pub pos: Vec3,
    pub norm: Vec3,
}

#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct AtomInstance {
    pub pos: Vec3,
    pub label_entity_id: u32,
}

pub fn normalize_to_unit_box(mut atoms: Vec<AtomInstance>) -> Vec<AtomInstance> {
    if atoms.is_empty() {
        return atoms;
    }

    // Find component-wise min & max
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for p in &atoms {
        min = min.min(p.pos);
        max = max.max(p.pos);
    }

    // Compute midpoint and half-range (avoid divide-by-zero)
    let midpoint = (min + max) * 0.5;
    let half_range = (max - min) * 0.5;
    let inv = Vec3::new(
        if half_range.x != 0.0 {
            1.0 / half_range.x
        } else {
            1.0
        },
        if half_range.y != 0.0 {
            1.0 / half_range.y
        } else {
            1.0
        },
        if half_range.z != 0.0 {
            1.0 / half_range.z
        } else {
            1.0
        },
    );

    atoms.iter_mut().for_each(|p| {
        p.pos = (p.pos - midpoint) * inv;
    });

    atoms

    // Normalize
    // points
    //     .into_iter()
    //     .map(|p| (p.pos - midpoint) * inv)
    //     .map(|pos| AtomInstance { pos })
    //     .collect()
}

#[derive(Debug)]
pub struct StructOper<'a> {
    pub typ: &'a str,
    pub rot_scale: Mat3,
    pub transl: Vec3,
}

fn parse_mat3_vec<'a>(data: &'a Loop<'a>, mat_name: &str) -> Vec<Mat3> {
    let m11: Vec<f32> = data
        .get_column(&format!("{}[1][1]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let m12: Vec<f32> = data
        .get_column(&format!("{}[1][2]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let m13: Vec<f32> = data
        .get_column(&format!("{}[1][3]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let m21: Vec<f32> = data
        .get_column(&format!("{}[2][1]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let m22: Vec<f32> = data
        .get_column(&format!("{}[2][2]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let m23: Vec<f32> = data
        .get_column(&format!("{}[2][3]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let m31: Vec<f32> = data
        .get_column(&format!("{}[3][1]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let m32: Vec<f32> = data
        .get_column(&format!("{}[3][2]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let m33: Vec<f32> = data
        .get_column(&format!("{}[3][3]", mat_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let mut mats = vec![];

    for i in 0..m11.len() {
        let mat = Mat3::from_cols(
            Vec3::new(m11[i], m21[i], m31[i]),
            Vec3::new(m12[i], m22[i], m32[i]),
            Vec3::new(m13[i], m23[i], m33[i]),
        );
        mats.push(mat);
    }

    mats
}

fn parse_vec3_vec<'a>(data: &'a Loop<'a>, vec_name: &str) -> Vec<Vec3> {
    let vec_0: Vec<f32> = data
        .get_column(&format!("{}[1]", vec_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let vec_1: Vec<f32> = data
        .get_column(&format!("{}[2]", vec_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let vec_2: Vec<f32> = data
        .get_column(&format!("{}[3]", vec_name))
        .unwrap()
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let mut vecs = vec![];

    for i in 0..vec_0.len() {
        vecs.push(Vec3::new(vec_0[i], vec_1[i], vec_2[i]));
    }

    vecs
}


pub fn apply_transforms(
    base_atoms: &[AtomInstance],
    transforms: &[Mat4]
) -> Vec<AtomInstance> {
    let mut instances = Vec::with_capacity(base_atoms.len() * transforms.len());
    for &mat in transforms {
        for atom in base_atoms {
            // Extend position to vec4 (homogeneous), multiply, then truncate
            let pos4 = mat * atom.pos.extend(1.0);
            instances.push(AtomInstance {
                pos: Vec3::new(pos4.x, pos4.y, pos4.z),
                label_entity_id: atom.label_entity_id,
            });
        }
    }
    instances
}

pub fn load_with_transforms(file_path: &str) -> (Vec<AtomInstance>, Vec<Mat4>) {
    // 1. Read CIF
    let content = std::fs::read_to_string(file_path).unwrap();
    let mut parser = Parser::new(&content);
    let cif = parser.parse_file();

    // 2. Cell parameters → Cartesian basis and inverse
    let cell = cif.get_data_block("_cell").unwrap();
    let (a, b, c, alpha, beta, gamma) = (
        cell.get_f32("length_a").unwrap(),
        cell.get_f32("length_b").unwrap(),
        cell.get_f32("length_c").unwrap(),
        cell.get_f32("angle_alpha").unwrap().to_radians(),
        cell.get_f32("angle_beta").unwrap().to_radians(),
        cell.get_f32("angle_gamma").unwrap().to_radians(),
    );
    let a_vec = Vec3::new(a, 0.0, 0.0);
    let b_vec = Vec3::new(b * gamma.cos(), b * gamma.sin(), 0.0);
    let c_x = c * beta.cos();
    let c_y = c * (alpha.cos() - beta.cos() * gamma.cos()) / gamma.sin();
    let c_z = ((c * c) - c_x * c_x - c_y * c_y).max(0.0).sqrt();
    let c_vec = Vec3::new(c_x, c_y, c_z);
    let cell_mat = Mat3::from_cols(a_vec, b_vec, c_vec);
    let inv_cell = cell_mat.inverse();

    // 3. Base Cartesian atoms
    let site = cif.get_loop_block("_atom_site").unwrap();
    let xs = site.get_f32_column("Cartn_x").unwrap();
    let ys = site.get_f32_column("Cartn_y").unwrap();
    let zs = site.get_f32_column("Cartn_z").unwrap();
    let syms = site.get_column("type_symbol").unwrap();
    let labels = site.parse_column::<u32>("label_entity_id").unwrap();
    let base_atoms = xs.into_iter()
        .zip(ys)
        .zip(zs)
        .zip(syms)
        .zip(labels)
        .map(|((((x, y), z), sym), lbl)| AtomInstance {
            pos: Vec3::new(x, y, z),
            label_entity_id: lbl,
        })
        .collect();

    // 4. Symmetry ops → build correct 4×4 transforms
    let ops = cif.get_loop_block("_pdbx_struct_oper_list").unwrap();
    let mats_frac = parse_mat3_vec(&ops, "matrix");
    let vecs_frac = parse_vec3_vec(&ops, "vector");
    let transforms = mats_frac.into_iter().zip(vecs_frac).map(|(m_frac, t_frac)| {
        // Compute rotation & translation in Cartesian
        let rot3 = cell_mat * m_frac * inv_cell;
        let trans3 = cell_mat * t_frac;
        // Assemble 4×4: columns: rot cols extended with 0, and translation column
        Mat4::from_cols(
            rot3.x_axis.extend(0.0),
            rot3.y_axis.extend(0.0),
            rot3.z_axis.extend(0.0),
            trans3.extend(1.0)
        )
    }).collect();

    (base_atoms, transforms)
}


pub fn load(file_path: &str) -> Vec<AtomInstance> {
    let cif_file = std::fs::read_to_string(file_path).unwrap();

    let mut parser = Parser::new(&cif_file);

    let cif = parser.parse_file();

    let cell = cif.get_data_block("_cell").unwrap();
    let a_len = cell.get_f32("length_a").unwrap();
    let b_len = cell.get_f32("length_b").unwrap();
    let c_len = cell.get_f32("length_c").unwrap();
    let alpha = cell.get_f32("angle_alpha").unwrap().to_radians();
    let beta = cell.get_f32("angle_beta").unwrap().to_radians();
    let gamma = cell.get_f32("angle_gamma").unwrap().to_radians();

    let a_vec = Vec3::new(a_len, 0.0, 0.0);
    let b_vec = Vec3::new(b_len * gamma.cos(), b_len * gamma.sin(), 0.0);
    let c_x = c_len * beta.cos();
    let c_y = c_len * (alpha.cos() - beta.cos() * gamma.cos()) / gamma.sin();
    let c_z = (c_len * c_len - c_x * c_x - c_y * c_y).max(0.0).sqrt();
    let c_vec = Vec3::new(c_x, c_y, c_z);

    // Cell matrix and its inverse for frac<->cart conversions
    let cell_mat = Mat3::from_cols(a_vec, b_vec, c_vec);
    let inv_cell = cell_mat.inverse();

    // --- 2. Read atomic Cartesian positions ---
    let atom_site = cif.get_loop_block("_atom_site").unwrap();

    let cart_x = atom_site.get_f32_column("Cartn_x").unwrap();
    let cart_y = atom_site.get_f32_column("Cartn_y").unwrap();
    let cart_z = atom_site.get_f32_column("Cartn_z").unwrap();
    let type_symbol = atom_site.get_column("type_symbol").unwrap();
    let label_entity_id = atom_site.parse_column::<u32>("label_entity_id").unwrap();

    let atoms_cart: Vec<_> = cart_x
        .iter()
        .zip(&cart_y)
        .zip(&cart_z)
        .map(|((&x, &y), &z)| Vec3::new(x, y, z))
        .zip(type_symbol)
        .zip(label_entity_id)
        .map(|((pos, symbol), label_entity_id)| {
            AtomInstance { pos, label_entity_id }
        })
        .collect();


    // --- 3. Parse symmetry operators ---
    let ops_loop = cif.get_loop_block("_pdbx_struct_oper_list").unwrap();
    let ids = ops_loop.get_column("id").unwrap();
    // Matrix elements: row-major in CIF as matrix[i][j]
    let mats = parse_mat3_vec(&ops_loop, "matrix");
    let vecs = parse_vec3_vec(&ops_loop, "vector");

    // Build operator list
    let mut operators = Vec::new();
    for i in 0..ids.len() {
        let mat = mats[i];
        let vec = vecs[i];
        operators.push((ids[i].to_string(), mat, vec));
    }

    // --- 4. Apply symmetry in fractional space and convert back ---
    let mut instances = Vec::new();
    for (op_id, mat_frac, trans_frac) in operators {
        for atom_cart in &atoms_cart {
            // Cartesian -> fractional
            let frac = inv_cell * atom_cart.pos;
            // Symmetry op in fractional
            let new_frac = mat_frac * frac + trans_frac;
            // Fractional -> Cartesian
            let new_cart = cell_mat * new_frac;

            instances.push(AtomInstance {
                pos: new_cart,
                label_entity_id: atom_cart.label_entity_id,
                // op_id: op_id.clone(),
            });
        }
    }

    // normalize_to_unit_box(instances)
    instances

    // atoms_pos
}
