use std::sync::Arc;

use glam::{DVec2, DVec3, Vec3, Vec4};
use crate::{iso2::{self, Iso2DConfig}, vm, Vertex};

#[derive(Debug, Clone, PartialEq)]
struct ImplicitFn {
    pub bin: Arc<Vec<vm::Opcode>>,
}

impl ImplicitFn {
    fn new<T: IntoIterator<Item = vm::Opcode>>(bin: T) -> Self {
        let bin = Arc::new(Vec::from_iter(bin.into_iter()));
        Self { bin }
    }

    fn eval_intrvl(&self, arg: [DVec3; 2]) -> vm::Range {
        let mut vm = vm::VM::with_instr_table(vm::RangeInstrTable);
        let [min, max] = arg;

        for i in 0..3 {
            vm.reg[i + 1] = (min[i], max[i]).into();
        }

        vm.eval(&self.bin);
        vm.reg[1]
    }

    fn eval_intrvl_v(&self, arg: &[[DVec3; 2]]) -> Vec<vm::Range> {
        let mut vm = vm::VM::with_instr_table(vm::RangeVecInstrTable);
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];
        let len = arg.len();

        for [min, max] in arg {
            x.push(vm::Range::from((min[0], max[0])));
            y.push(vm::Range::from((min[1], max[1])));
            z.push(vm::Range::from((min[2], max[2])));
        }

        vm.reg[1] = x.into();
        vm.reg[2] = y.into();
        vm.reg[3] = z.into();

        vm.set_vec_size(len);
        vm.eval(&self.bin);
        vm.take_reg(1)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Quad {
    code: u64,
}

impl Quad {
    #[inline]
    fn root() -> Self {
        Self {
            code: 0,
        }
    }

    #[inline]
    fn domain(&self, min: DVec2, max: DVec2) -> [DVec2; 2] {
        let (u_min, u_max) = iso2::quad_unit_bounds(self.code);
        let size = max - min;
        [u_min * size + min, u_max * size + min]
    }

    #[inline]
    fn corners(&self, min: DVec2, max: DVec2) -> [DVec2; 4] {
        let [q_min, q_max] = self.domain(min, max);
        let c0 = q_min;
        let c2 = q_max;
        let c1 = c0.with_x(c2.x);
        let c3 = c0.with_y(c2.y);
        [c0, c1, c2, c3]
    }

    #[inline]
    fn subdivide(&self) -> [Self; 4] {
        let c = self.code << 4;
        let nw = Self {
            code: c | 3,
        };
        let ne = Self {
            code: c | 4,
        };
        let se = Self {
            code: c | 2,
        };
        let sw = Self {
            code: c | 1,
        };

        [sw, se, nw, ne]
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DbgTree {
    pub quads: Vec<Quad>,
    pub min: DVec2,
    pub max: DVec2,
}

impl DbgTree {
    fn build(config: Iso2DConfig, f: &ImplicitFn) -> Self {
        let min = config.min;
        let max = config.max;

        let mut quads = vec![Quad::root()];

        for depth in 0..=config.depth {

            let quad_domains: Vec<_> = quads
                .iter()
                .map(|q| {
                    q.domain(min, max).map(|v| v.extend(0.0))
                }).collect();

            let quad_range = f.eval_intrvl_v(&quad_domains);

            quads = quads.iter().zip(quad_range).filter_map(|(q, r)| {
                if r.contains_zero() || r.is_undef() {
                    Some(q.subdivide())
                } else {
                    None
                }
            }).flatten().collect();
        }


        Self { quads, min, max }
    }
}

pub(crate) fn build_2d(config: iso2::Iso2DConfig) -> Vec<Vertex> {

    let f = ImplicitFn::new(config.program.opcode());
    let tree = DbgTree::build(config, &f);

    let quads: Vec<_> = tree.quads.into_iter().map(|q| {
        let domain = q.domain(config.min, config.max).map(|v| v.extend(0.0));
        let domain_dist = domain[0].distance(domain[1]) as f32;
        let range = f.eval_intrvl(domain);

        if range.is_undef() {
            (q, f32::NAN)
        } else {
            (q, range.dist() as f32 / domain_dist)
        } 
    }).collect();

    let mut max_var = 0.0;
    for (_, var) in &quads {
        max_var = var.max(max_var);
    }

    let mut verts: Vec<_> = quads.into_iter().flat_map(|(q, var)| {
        let [min, max] = q.domain(config.min, config.max).map(|v| v.as_vec2().extend(0.0).extend(0.0));
        let (ne, se, sw, nw) = (max, min.with_x(max.x), min, min.with_y(max.y));

        let col = if var.is_nan() {
            Vec3::ZERO.with_x(1.0).extend(1.0)
        } else {
            Vec4::splat(var / max_var)
        };

        [sw, se, ne, sw, ne, nw].map(|pos| Vertex { pos, col })
    }).collect();

    let size = (config.max - config.min).extend(1.0).extend(1.0).as_vec4();
    let center = ((config.max + config.min) / 2.0).extend(0.0).extend(0.0).as_vec4();

    for v in &mut verts {
        v.pos -= center;
        v.pos /= size;
    }

    verts
}
