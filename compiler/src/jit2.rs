use std::{
    cell::{RefCell, RefMut},
    fmt, ops,
    sync::Arc,
};

use cranelift::prelude::*;
use cranelift_codegen::ir;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use rustc_hash::FxHashMap;

use crate::jit::{BinOp, Instr, Oprnd, UnOp};

macro_rules! extrn {
    ($($tt:tt)*) => {
        #[unsafe(no_mangle)]
        pub extern "C"
            $($tt)*
    }
}

#[inline(always)]
const fn min_max_2(a: f64, b: f64) -> (f64, f64) {
    if a < b { (a, b) } else { (b, a) }
}

#[inline(always)]
const fn min_max_4(a: f64, b: f64, c: f64, d: f64) -> (f64, f64) {
    let (a, b) = min_max_2(a, b);
    let (c, d) = min_max_2(c, d);
    (min_max_2(a, c).0, min_max_2(b, d).1)
}

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct Intrvl {
    pub lo: f64,
    pub hi: f64,
}

impl std::fmt::Display for Intrvl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.lo, self.hi)
    }
}

impl From<(f64, f64)> for Intrvl {
    fn from(value: (f64, f64)) -> Self {
        Intrvl::new(value.0, value.1)
    }
}

impl Intrvl {
    pub const NULL: Self = Self {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    };

    pub const UNDEF: Self = Self {
        lo: f64::NAN,
        hi: f64::NAN,
    };

    pub const INF: Self = Self {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    };

    pub const ONE: Self = Self::new_const(1.0);
    pub const MINUS_ONE: Self = Self::new_const(-1.0);
    pub const TWO: Self = Self::new_const(2.0);

    #[inline(always)]
    pub fn new(l: f64, u: f64) -> Self {
        if l > u {
            let mid = (l + u) / 2.0;
            Intrvl { hi: mid, lo: mid }
        } else {
            Intrvl { lo: l, hi: u }
        }
    }

    #[inline(always)]
    pub fn imm(imm: f64) -> Self {
        Intrvl { lo: imm, hi: imm }
    }

    #[inline(always)]
    pub const fn new_const(v: f64) -> Self {
        Intrvl { lo: v, hi: v }
    }

    #[inline(always)]
    pub fn add(self, other: Self) -> Self {
        Self::of_add(self, other)
    }
    #[inline(always)]
    pub fn sub(self, other: Self) -> Self {
        Self::of_sub(self, other)
    }
    #[inline(always)]
    pub fn mul(self, other: Self) -> Self {
        Self::of_mul(self, other)
    }
    #[inline(always)]
    pub fn div(self, other: Self) -> Self {
        Self::of_div(self, other)
    }
    #[inline(always)]
    pub fn pow(self, other: Self) -> Self {
        Self::of_pow(self, other)
    }
    #[inline(always)]
    pub fn sin(self) -> Self {
        Self::of_sin(self)
    }
    #[inline(always)]
    pub fn cos(self) -> Self {
        Self::of_cos(self)
    }
    #[inline(always)]
    pub fn tan(self) -> Self {
        Self::of_tan(self)
    }
    #[inline(always)]
    pub fn ln(self) -> Self {
        Self::of_ln(self)
    }

    #[inline(always)]
    pub fn of_sin(a: Intrvl) -> Self {
        const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
        const THREE_HALVES_PI: f64 = 3.0 * HALF_PI;
        const TWO_PI: f64 = 2.0 * std::f64::consts::PI;

        if a.is_empty() {
            return Self::UNDEF;
        } else if a.dist() >= TWO_PI {
            return (-1.0, 1.0).into();
        }

        let l = a.lo.rem_euclid(TWO_PI);
        let u = a.hi.rem_euclid(TWO_PI);

        let contains_half_pi =
            (l <= HALF_PI && HALF_PI <= u) || (l > u && (HALF_PI >= l || HALF_PI <= u));
        let contains_three_halves_pi = (l <= THREE_HALVES_PI && THREE_HALVES_PI <= u)
            || (l > u && (THREE_HALVES_PI >= l || THREE_HALVES_PI <= u));

        if contains_three_halves_pi && contains_half_pi {
            (-1.0, 1.0)
        } else if contains_three_halves_pi && !contains_half_pi {
            (-1.0, l.sin().max(u.sin()))
        } else if !contains_three_halves_pi && contains_half_pi {
            (l.sin().min(u.sin()), 1.0)
        } else {
            let ls = l.sin();
            let us = u.sin();
            min_max_2(ls, us)
        }
        .into()
    }

    pub fn of_cos(a: Intrvl) -> Self {
        const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
        const TWO_PI: f64 = 2.0 * std::f64::consts::PI;
        const PI: f64 = std::f64::consts::PI;

        if a.is_empty() {
            return Self::UNDEF;
        } else if a.dist() >= TWO_PI {
            return (-1.0, 1.0).into();
        }

        let l = a.lo.rem_euclid(TWO_PI);
        let u = a.hi.rem_euclid(TWO_PI);

        let contains_zero = (l <= 0.0 && 0.0 <= u) || (l > u && (0.0 >= l || 0.0 <= u));
        let contains_pi = (l <= PI && PI <= u) || (l > u && (PI >= l || PI <= u));

        let min = if contains_pi {
            -1.0
        } else {
            l.cos().min(u.cos())
        };
        let max = if contains_zero {
            1.0
        } else {
            l.cos().max(u.cos())
        };

        if contains_pi && contains_zero {
            (-1.0, 1.0)
        } else if contains_pi && !contains_zero {
            (-1.0, l.cos().max(u.cos()))
        } else if !contains_pi && contains_zero {
            (l.cos().min(u.cos()), 1.0)
        } else {
            let lc = l.cos();
            let uc = u.cos();
            min_max_2(lc, uc)
        }
        .into()
    }

    pub fn of_tan(a: Intrvl) -> Self {
        use std::f64::consts::FRAC_PI_2 as HALF_PI;
        use std::f64::consts::PI;

        if a.is_empty() {
            return Self::UNDEF;
        } else if a.dist() >= PI {
            return Self::UNDEF; // Intervals >= π always contain an asymptote
        }

        let l = a.lo.rem_euclid(PI);
        let u = a.hi.rem_euclid(PI);

        // Check if π/2 is in the interval (accounting for wrapping)
        let contains_half_pi =
            (l <= HALF_PI && HALF_PI <= u) || (l > u && (HALF_PI >= l || HALF_PI <= u));

        if contains_half_pi {
            Self::UNDEF
        } else {
            let min = a.lo.tan().min(a.hi.tan());
            let max = a.lo.tan().max(a.hi.tan());
            (min, max).into()
        }
    }

    #[inline(always)]
    pub fn of_add(a: Intrvl, b: Intrvl) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }
        (a.lo + b.lo, a.hi + b.hi).into()
    }

    #[inline(always)]
    pub fn of_sub(a: Intrvl, b: Intrvl) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }
        (a.lo - b.hi, a.hi - b.lo).into()
    }

    #[inline(always)]
    pub fn of_mul(a: Intrvl, b: Intrvl) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }
        let res = min_max_4(a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi);

        if res.0.is_nan() || res.1.is_nan() {
            return Self::UNDEF;
        }
        res.into()
    }

    #[inline(always)]
    pub fn of_div(a: Intrvl, b: Intrvl) -> Self {
        // 1 / b

        let denom = if !b.contains_zero() {
            (1.0 / b.hi, 1.0 / b.lo)
        } else if b.hi == 0.0 {
            (f64::NEG_INFINITY, 1.0 / b.lo)
        } else if b.lo == 0.0 {
            (1.0 / b.hi, f64::INFINITY)
        } else {
            return Intrvl::UNDEF;
        };

        Self::of_mul(a, denom.into())

        // if a.is_undef() || b.is_undef() || (a.contains_zero() && b.contains_zero()) {
        //     return Self::UNDEF;
        // }
        // min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u).into()
    }

    #[inline(always)]
    pub fn of_pow(a: Intrvl, b: Intrvl) -> Self {
        if a.lo >= 0.0 {
            // a >= 0
            if a.lo > 0.0 {
                // a > 0
                Intrvl::from_tuple(min_max_4(
                    a.lo.powf(b.lo),
                    a.hi.powf(b.hi),
                    a.hi.powf(b.lo),
                    a.lo.powf(b.hi),
                ))
            } else if !b.contains_zero() {
                Intrvl::new(0.0, a.hi.powf(b.hi))
            } else {
                Intrvl::UNDEF
            }
        } else if a.hi < 0.0 {
            // a < 0
            if b.is_const_int() {
                let b = b.lo;

                Intrvl::from_tuple(min_max_2(a.hi.powf(b), a.lo.powf(b)))
            } else {
                Intrvl::UNDEF
            }
        } else if b.is_const_int() {
            let b = b.lo;

            if (b % 2.0).abs() < f64::EPSILON {
                Intrvl::new(0.0, a.lo.abs().max(a.hi).powf(b))
            } else {
                Intrvl::new(a.lo.powf(b), a.lo.abs().max(a.hi).powf(b))
            }
        } else {
            Intrvl::UNDEF
        }
    }

    #[inline(always)]
    pub fn of_ln(a: Intrvl) -> Self {
        if a.is_empty() || a.lo <= 0.0 {
            return a;
        }

        (a.lo.ln(), a.hi.ln()).into()
    }

    #[inline(always)]
    pub fn from_tuple(b: (f64, f64)) -> Self {
        Self::new(b.0, b.1)
    }

    #[inline(always)]
    pub fn is_inf(&self) -> bool {
        self.lo == f64::NEG_INFINITY && self.hi == f64::INFINITY
    }

    #[inline(always)]
    pub const fn is_finite(&self) -> bool {
        self.lo.is_finite() && self.hi.is_finite()
    }

    #[inline(always)]
    pub const fn is_const(&self) -> bool {
        (self.lo - self.hi).abs() < f64::EPSILON
    }

    #[inline(always)]
    pub fn is_const_int(&self) -> bool {
        self.is_const() && self.lo.fract() == 0.0
    }

    #[inline(always)]
    pub const fn is_pos(&self) -> bool {
        self.lo > 0.0 && self.hi > 0.0
    }

    #[inline(always)]
    pub const fn is_neg(&self) -> bool {
        self.lo < 0.0 && self.hi < 0.0
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        !(self.lo.is_nan() || self.hi.is_nan() || self == &Self::NULL)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.lo.is_nan() || self.hi.is_nan() || self == &Self::NULL
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self == &Self::NULL
    }

    #[inline(always)]
    pub const fn contains_zero(&self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }

    #[inline(always)]
    pub fn dist(&self) -> f64 {
        self.hi - self.lo
    }
}

pub(crate) type FnDecl = (
    &'static str,
    *const u8,
    &'static [FnParam],
    &'static [FnParam],
);

mod f64_util {
    use super::*;

    pub extern "C" fn pow(b: f64, e: f64) -> f64 {
        b.powf(e)
    }

    pub extern "C" fn is_even(f: f64) -> i8 {
        (f % 2.0 == 0.0) as i8
    }

    pub const GLOB_FN_DECLS: &'static [FnDecl] = &[
        (
            "is_even_f64",
            is_even as *const u8,
            &[FnParam::F64],
            &[FnParam::I8],
        ),
        (
            "rem_euclid_f64",
            f64::rem_euclid as *const u8,
            &[FnParam::F64, FnParam::F64],
            &[FnParam::F64],
        ),
        (
            "pow_f64",
            pow as *const u8,
            &[FnParam::F64, FnParam::F64],
            &[FnParam::F64],
        ),
        (
            "sin_f64",
            f64::sin as *const u8,
            &[FnParam::F64],
            &[FnParam::F64],
        ),
        (
            "cos_f64",
            f64::cos as *const u8,
            &[FnParam::F64],
            &[FnParam::F64],
        ),
        (
            "tan_f64",
            f64::tan as *const u8,
            &[FnParam::F64],
            &[FnParam::F64],
        ),
    ];
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct F64X2(pub f64, pub f64);

impl fmt::Display for F64X2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl ops::Add for F64X2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        F64X2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl ops::Sub for F64X2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        F64X2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl ops::Mul for F64X2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        F64X2(self.0 * rhs.0, self.1 * rhs.1)
    }
}

impl ops::Div for F64X2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        F64X2(self.0 / rhs.0, self.1 / rhs.1)
    }
}

impl F64X2 {
    pub const EPSILON: F64X2 = F64X2::splat(f64::EPSILON);
    pub const NAN: F64X2 = F64X2::splat(f64::NAN);

    pub const fn to_array(self) -> [f64; 2] {
        [self.0, self.1]
    }

    pub const fn from_array(arr: [f64; 2]) -> F64X2 {
        F64X2(arr[0], arr[1])
    }

    pub const fn splat(f: f64) -> F64X2 {
        F64X2(f, f)
    }

    pub fn pow(&self, e: &F64X2) -> F64X2 {
        F64X2(self.0.powf(e.0), self.1.powf(e.1))
    }

    pub fn sin(&self) -> F64X2 {
        // let arr = wide::f64x2::new(self.to_array()).sin().to_array();
        // F64X2::from_array(arr)
        F64X2(self.0.sin(), self.1.sin())
    }

    pub fn cos(&self) -> F64X2 {
        // let arr = wide::f64x2::new(self.to_array()).cos().to_array();
        // F64X2::from_array(arr)
        F64X2(self.0.cos(), self.1.cos())
    }

    pub fn tan(&self) -> F64X2 {
        // let arr = wide::f64x2::new(self.to_array()).tan().to_array();
        // F64X2::from_array(arr)
        F64X2(self.0.tan(), self.1.tan())
    }

    pub fn abs(&self) -> F64X2 {
        F64X2(self.0.abs(), self.1.abs())
    }

    pub fn is_nan(&self) -> bool {
        self.0.is_nan() && self.1.is_nan()
    }
}

mod intrvl_util {
    // use Intrvl;

    use super::*;

    pub extern "C" fn add(l: F64X2, r: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(l.0, l.1).add(Intrvl::new(r.0, r.1));
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn sub(l: F64X2, r: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(l.0, l.1).sub(Intrvl::new(r.0, r.1));
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn mul(l: F64X2, r: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(l.0, l.1).mul(Intrvl::new(r.0, r.1));
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn div(l: F64X2, r: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(l.0, l.1).div(Intrvl::new(r.0, r.1));
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn pow(b: F64X2, e: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(b.0, b.1).pow(Intrvl::new(e.0, e.1));
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn sin(v: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(v.0, v.1).sin();
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn cos(v: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(v.0, v.1).cos();
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub extern "C" fn tan(v: F64X2) -> F64X2 {
        let intrvl = Intrvl::new(v.0, v.1).tan();
        F64X2(intrvl.lo, intrvl.hi)
    }

    pub const GLOB_FN_DECLS: &'static [FnDecl] = &[
        (
            "add_intrvl",
            add as *const u8,
            &[FnParam::SRet, FnParam::F64X2, FnParam::F64X2],
            &[],
        ),
        (
            "sub_intrvl",
            sub as *const u8,
            &[FnParam::SRet, FnParam::F64X2, FnParam::F64X2],
            &[],
        ),
        (
            "mul_intrvl",
            mul as *const u8,
            &[FnParam::SRet, FnParam::F64X2, FnParam::F64X2],
            &[],
        ),
        (
            "div_intrvl",
            div as *const u8,
            &[FnParam::SRet, FnParam::F64X2, FnParam::F64X2],
            &[],
        ),
        (
            "pow_intrvl",
            pow as *const u8,
            &[FnParam::SRet, FnParam::F64X2, FnParam::F64X2],
            &[],
        ),
        (
            "sin_intrvl",
            sin as *const u8,
            &[FnParam::SRet, FnParam::F64X2],
            &[],
        ),
        (
            "cos_intrvl",
            cos as *const u8,
            &[FnParam::SRet, FnParam::F64X2],
            &[],
        ),
        (
            "tan_intrvl",
            tan as *const u8,
            &[FnParam::SRet, FnParam::F64X2],
            &[],
        ),
    ];
}

mod f64x2_util {
    use super::*;

    pub extern "C" fn pow_f64x2(b: F64X2, e: F64X2) -> F64X2 {
        b.pow(&e)
    }

    pub extern "C" fn sin_f64x2(v: F64X2) -> F64X2 {
        v.sin()
    }

    pub extern "C" fn cos_f64x2(v: F64X2) -> F64X2 {
        v.cos()
    }

    pub extern "C" fn tan_f64x2(v: F64X2) -> F64X2 {
        v.tan()
    }

    pub const GLOB_FN_DECLS: &'static [FnDecl] = &[
        (
            "pow_f64x2",
            pow_f64x2 as *const u8,
            &[FnParam::SRet, FnParam::F64X2, FnParam::F64X2],
            &[],
        ),
        (
            "sin_f64x2",
            sin_f64x2 as *const u8,
            &[FnParam::SRet, FnParam::F64X2],
            &[],
        ),
        (
            "cos_f64x2",
            cos_f64x2 as *const u8,
            &[FnParam::SRet, FnParam::F64X2],
            &[],
        ),
        (
            "tan_f64x2",
            tan_f64x2 as *const u8,
            &[FnParam::SRet, FnParam::F64X2],
            &[],
        ),
    ];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FnParam {
    I8,
    F64,
    F64X2,
    SRet,
}

type FnRefTable = FxHashMap<&'static str, ir::FuncRef>;

pub struct JIT<'a> {
    pub builder_ctx: FunctionBuilderContext,
    pub ctx: RefCell<cranelift_codegen::Context>,
    pub module: RefCell<JITModule>,
    pub emit_asm: bool,

    pub glob_fns: FxHashMap<&'static str, FuncId>,
    pub asm: RefCell<Option<String>>,

    // calling a compiled function after dropping this struct would be invalid
    _fn_ptr_lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> ops::Drop for JIT<'a> {
    fn drop(&mut self) {
        // we restrict the lifetime of returned pointers or require unsafe from the user
        let module = self.module.replace(JITModule::new(
            JITBuilder::new(cranelift_module::default_libcall_names()).unwrap(),
        ));
        unsafe { module.free_memory() }
    }
}

/// Bind a function pointer to some lifetime by its return argument
///
/// https://internals.rust-lang.org/t/lt-a-lang-item-because-we-dont-have-derefmove-lifetimes-for-fn/15169
#[repr(transparent)]
pub struct Lt<'a, T = ()>(pub T, pub core::marker::PhantomData<&'a ()>);

impl<'a, T> ops::Deref for Lt<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T> ops::DerefMut for Lt<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> JIT<'a> {
    pub fn init() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        flag_builder.set("opt_level", "speed").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        {
            JIT::def_function_symbols(&mut builder, f64_util::GLOB_FN_DECLS);
            JIT::def_function_symbols(&mut builder, f64x2_util::GLOB_FN_DECLS);
            JIT::def_function_symbols(&mut builder, intrvl_util::GLOB_FN_DECLS);
        }

        let mut module = JITModule::new(builder);

        let mut glob_fns = FxHashMap::default();
        {
            glob_fns.extend(JIT::decl_functions(&mut module, f64_util::GLOB_FN_DECLS));
            glob_fns.extend(JIT::decl_functions(&mut module, f64x2_util::GLOB_FN_DECLS));
            glob_fns.extend(JIT::decl_functions(&mut module, intrvl_util::GLOB_FN_DECLS));
        }

        Self {
            builder_ctx: FunctionBuilderContext::new(),
            ctx: RefCell::new(module.make_context()),
            module: RefCell::new(module),
            emit_asm: false,
            glob_fns,
            asm: RefCell::new(None),
            _fn_ptr_lifetime: std::marker::PhantomData,
        }
    }

    fn def_function_symbols(b: &mut JITBuilder, fn_decls: &[FnDecl]) {
        for (s, ptr, _, _) in fn_decls {
            b.symbol(*s, *ptr);
        }
    }

    fn decl_functions(
        module: &mut JITModule,
        fn_decls: &[FnDecl],
    ) -> FxHashMap<&'static str, FuncId> {
        let mut decls = FxHashMap::default();
        let ptr_ty = module.target_config().pointer_type();

        for (name, _, params, returns) in fn_decls {
            let mut sig = module.make_signature();

            for param in *params {
                match param {
                    FnParam::I8 => sig.params.push(AbiParam::new(types::I8)),
                    FnParam::F64 => sig.params.push(AbiParam::new(types::F64)),
                    FnParam::F64X2 => sig.params.push(AbiParam::new(types::F64X2)),
                    FnParam::SRet => sig
                        .params
                        .push(AbiParam::special(ptr_ty, ir::ArgumentPurpose::StructReturn)),
                }
            }

            for ret in *returns {
                match ret {
                    FnParam::I8 => sig.returns.push(AbiParam::new(types::I8)),
                    FnParam::F64 => sig.returns.push(AbiParam::new(types::F64)),
                    FnParam::F64X2 => sig.returns.push(AbiParam::new(types::F64X2)),
                    FnParam::SRet => panic!("sret is only allowed as argument"),
                }
            }

            let id = module
                .declare_function(name, Linkage::Import, &sig)
                .unwrap();
            decls.insert(*name, id);
        }

        decls
    }

    fn decl_functions_in_function(
        module: &mut JITModule,
        func: &mut ir::Function,
        fn_decls: &FxHashMap<&'static str, FuncId>,
    ) -> FnRefTable {
        fn_decls
            .iter()
            .map(|(name, id)| (*name, module.declare_func_in_func(*id, func)))
            .collect()
    }

    pub unsafe fn compile_static_2f64_f64(
        &self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(f64, f64) -> f64 {
        let non_static = self.compile_2f64_f64(fn_name, bytecode);
        unsafe { std::mem::transmute(non_static) }
    }

    pub fn compile_2f64_f64(
        &self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(f64, f64) -> Lt<'a, f64> {
        let mut ctx_mut = self.ctx.borrow_mut();
        let mut module_mut = self.module.borrow_mut();

        ctx_mut.set_disasm(self.emit_asm);

        // Signature

        let mut sig = module_mut.make_signature();

        for _ in 0..2 {
            sig.params.push(AbiParam::new(types::F64));
        }

        sig.returns.push(AbiParam::new(types::F64));
        ctx_mut.func.signature = sig;

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut fb = FunctionBuilder::new(&mut ctx_mut.func, &mut fn_ctx);

        // entry block

        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        fb.seal_block(entry);

        let fn_refs = Self::decl_functions_in_function(&mut *module_mut, fb.func, &self.glob_fns);

        let nan = fb.ins().f64const(f64::NAN);

        // registers alloc

        let mut vars = vec![];
        for i in 0..16 {
            let v = Variable::from_u32(i);
            fb.declare_var(v, types::F64);
            fb.def_var(v, nan);
            vars.push(v);
        }

        for i in 0..2 {
            let x = fb.block_params(entry)[i as usize];
            fb.def_var(vars[i as usize], x);
        }

        Self::asmbl_f64_body(bytecode, &mut fb, &fn_refs, &vars);

        // return
        let ret = fb.use_var(vars[0]);
        fb.ins().return_(&[ret]);
        fb.finalize();

        let fn_id = module_mut
            .declare_function(fn_name, Linkage::Local, &ctx_mut.func.signature)
            .unwrap();
        module_mut.define_function(fn_id, &mut *ctx_mut).unwrap();
        module_mut.finalize_definitions().unwrap();

        if self.emit_asm {
            *self.asm.borrow_mut() = Some(
                ctx_mut
                    .compiled_code()
                    .unwrap()
                    .vcode
                    .clone()
                    .unwrap()
                    .to_string(),
            );
        }

        module_mut.clear_context(&mut *ctx_mut);
        let fn_ptr = module_mut.get_finalized_function(fn_id);
        unsafe { std::mem::transmute(fn_ptr) }
    }

    fn asmbl_f64_body(
        bytecode: &[Instr],
        fb: &mut FunctionBuilder,
        fn_refs: &FnRefTable,
        vars: &[Variable],
    ) {
        let use_oprnd = |oprnd: Oprnd, fb: &mut FunctionBuilder| match oprnd {
            Oprnd::Reg(indx) => fb.use_var(vars[indx as usize]),
            Oprnd::Imm(imm) => fb.ins().f64const(imm),
        };

        let call_fn = |name: &str, v: &[Value], fb: &mut FunctionBuilder| {
            let fn_ref = fn_refs[name];
            let call = fb.ins().call(fn_ref, v);
            fb.inst_results(call)[0]
        };

        for &instr in bytecode {
            match instr {
                Instr::UnOp { op, val, dst } => {
                    let dst = dst as usize;
                    let val = use_oprnd(val, fb);

                    let res = match op {
                        UnOp::MOV => val,
                        UnOp::SIN => call_fn("sin_f64", &[val], fb),
                        UnOp::COS => call_fn("cos_f64", &[val], fb),
                        UnOp::TAN => call_fn("tan_f64", &[val], fb),
                    };

                    fb.def_var(vars[dst], res);
                }
                Instr::BinOp { op, lhs, rhs, dst } => {
                    let dst = dst as usize;
                    let lhs = use_oprnd(lhs, fb);
                    let rhs = use_oprnd(rhs, fb);

                    let res = match op {
                        BinOp::ADD => fb.ins().fadd(lhs, rhs),
                        BinOp::SUB => fb.ins().fsub(lhs, rhs),
                        BinOp::MUL => fb.ins().fmul(lhs, rhs),
                        BinOp::DIV => fb.ins().fdiv(lhs, rhs),
                        BinOp::POW => call_fn("pow_f64", &[lhs, rhs], fb),
                    };

                    fb.def_var(vars[dst], res);
                }
            }
        }
    }

    pub unsafe fn compile_static_2f64x2_f64(
        &self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(F64X2, F64X2, *mut F64X2) {
        let non_static = self.compile_2f64x2_f64(fn_name, bytecode);
        unsafe { std::mem::transmute(non_static) }
    }

    pub fn compile_2f64x2_f64(
        &self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(F64X2, F64X2, *mut F64X2) -> Lt<'a> {
        let mut ctx_mut = self.ctx.borrow_mut();
        let mut module_mut = self.module.borrow_mut();

        ctx_mut.set_disasm(self.emit_asm);
        let ptr_ty = module_mut.target_config().pointer_type();

        let mut sig = module_mut.make_signature();

        for _ in 0..2 {
            sig.params.push(AbiParam::new(types::F64X2));
        }

        sig.params
            .push(AbiParam::special(ptr_ty, ir::ArgumentPurpose::StructReturn));

        ctx_mut.func.signature = sig;

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut fb = FunctionBuilder::new(&mut ctx_mut.func, &mut fn_ctx);

        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        fb.seal_block(entry);

        let fn_refs = Self::decl_functions_in_function(&mut *module_mut, fb.func, &self.glob_fns);

        let nan = fb.ins().f64const(f64::NAN);
        let nan = fb.ins().splat(types::F64X2, nan);

        let mut vars = vec![];
        for i in 0..16 {
            let v = Variable::from_u32(i);
            fb.declare_var(v, types::F64X2);
            fb.def_var(v, nan);
            vars.push(v);
        }

        for i in 0..2 {
            let x = fb.block_params(entry)[i as usize];
            fb.def_var(vars[i as usize], x);
        }

        let out_ptr = fb.block_params(entry)[2];

        Self::asmbl_f64x2_body(bytecode, &mut fb, &fn_refs, &vars, ptr_ty);

        let ret = fb.use_var(vars[0]);
        fb.ins().store(ir::MemFlags::new(), ret, out_ptr, 0);
        fb.ins().return_(&[]);
        fb.finalize();

        let fn_id = module_mut
            .declare_function(fn_name, Linkage::Local, &ctx_mut.func.signature)
            .unwrap();

        module_mut.define_function(fn_id, &mut *ctx_mut).unwrap();
        module_mut.finalize_definitions().unwrap();

        if self.emit_asm {
            *self.asm.borrow_mut() = Some(
                ctx_mut
                    .compiled_code()
                    .unwrap()
                    .vcode
                    .clone()
                    .unwrap()
                    .to_string(),
            );
        }

        module_mut.clear_context(&mut *ctx_mut);
        let fn_ptr = module_mut.get_finalized_function(fn_id);
        unsafe { std::mem::transmute(fn_ptr) }
    }

    fn asmbl_f64x2_body(
        bytecode: &[Instr],
        fb: &mut FunctionBuilder,
        fn_refs: &FnRefTable,
        vars: &[Variable],
        ptr_ty: Type,
    ) {
        let use_oprnd = |oprnd: Oprnd, fb: &mut FunctionBuilder| match oprnd {
            Oprnd::Reg(indx) => fb.use_var(vars[indx as usize]),
            Oprnd::Imm(imm) => {
                let imm = fb.ins().f64const(imm);
                fb.ins().splat(types::F64X2, imm)
            }
        };

        let call_fn = |name: &str, v: &[Value], fb: &mut FunctionBuilder| {
            let ret_slot = fb.create_sized_stack_slot(ir::StackSlotData {
                kind: ir::StackSlotKind::ExplicitSlot,
                size: 2 * 8,
                align_shift: 0,
            });

            let fn_ref = fn_refs[name];
            let ret_addr = fb.ins().stack_addr(ptr_ty, ret_slot, 0);
            let mut args = vec![ret_addr];
            args.extend_from_slice(v);

            let _ = fb.ins().call(fn_ref, &args);
            let res = fb
                .ins()
                .load(types::F64X2, ir::MemFlags::new(), ret_addr, 0);
            res
        };

        for &instr in bytecode {
            match instr {
                Instr::UnOp { op, val, dst } => {
                    let dst = dst as usize;
                    let val = use_oprnd(val, fb);

                    let res = match op {
                        UnOp::MOV => val,
                        UnOp::SIN => call_fn("sin_f64x2", &[val], fb),
                        UnOp::COS => call_fn("cos_f64x2", &[val], fb),
                        UnOp::TAN => call_fn("tan_f64x2", &[val], fb),
                    };

                    fb.def_var(vars[dst], res);
                }
                Instr::BinOp { op, lhs, rhs, dst } => {
                    let dst = dst as usize;
                    let lhs = use_oprnd(lhs, fb);
                    let rhs = use_oprnd(rhs, fb);

                    let res = match op {
                        BinOp::ADD => fb.ins().fadd(lhs, rhs),
                        BinOp::SUB => fb.ins().fsub(lhs, rhs),
                        BinOp::MUL => fb.ins().fmul(lhs, rhs),
                        BinOp::DIV => fb.ins().fdiv(lhs, rhs),
                        BinOp::POW => call_fn("pow_f64x2", &[lhs, rhs], fb),
                    };

                    fb.def_var(vars[dst], res);
                }
            }
        }
    }

    pub unsafe fn compile_static_2intrvl_intrvl(
        &self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(F64X2, F64X2, *mut F64X2) {
        let non_static = self.compile_2intrvl_intrvl(fn_name, bytecode);
        unsafe { std::mem::transmute(non_static) }
    }

    pub fn compile_2intrvl_intrvl(
        &self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(F64X2, F64X2, *mut F64X2) -> Lt<'a> {
        let mut ctx_mut = self.ctx.borrow_mut();
        let mut module_mut = self.module.borrow_mut();

        ctx_mut.set_disasm(self.emit_asm);
        let ptr_ty = module_mut.target_config().pointer_type();

        let mut sig = module_mut.make_signature();

        for _ in 0..2 {
            sig.params.push(AbiParam::new(types::F64X2));
        }

        sig.params
            .push(AbiParam::special(ptr_ty, ir::ArgumentPurpose::StructReturn));

        ctx_mut.func.signature = sig;

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut fb = FunctionBuilder::new(&mut ctx_mut.func, &mut fn_ctx);

        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        fb.seal_block(entry);

        let fn_refs = Self::decl_functions_in_function(&mut *module_mut, fb.func, &self.glob_fns);

        let nan = fb.ins().f64const(f64::NAN);
        let nan = fb.ins().splat(types::F64X2, nan);

        let mut vars = vec![];
        for i in 0..16 {
            let v = Variable::from_u32(i);
            fb.declare_var(v, types::F64X2);
            fb.def_var(v, nan);
            vars.push(v);
        }

        for i in 0..2 {
            let x = fb.block_params(entry)[i as usize];
            fb.def_var(vars[i as usize], x);
        }

        let out_ptr = fb.block_params(entry)[2];

        Self::asmbl_intrvl_body(bytecode, &mut fb, &fn_refs, &vars, ptr_ty);

        let ret = fb.use_var(vars[0]);
        fb.ins().store(ir::MemFlags::new(), ret, out_ptr, 0);
        fb.ins().return_(&[]);
        fb.finalize();

        let fn_id = module_mut
            .declare_function(fn_name, Linkage::Local, &ctx_mut.func.signature)
            .unwrap();

        module_mut.define_function(fn_id, &mut *ctx_mut).unwrap();
        module_mut.finalize_definitions().unwrap();

        if self.emit_asm {
            *self.asm.borrow_mut() = Some(
                ctx_mut
                    .compiled_code()
                    .unwrap()
                    .vcode
                    .clone()
                    .unwrap()
                    .to_string(),
            );
        }

        module_mut.clear_context(&mut *ctx_mut);
        let fn_ptr = module_mut.get_finalized_function(fn_id);
        unsafe { std::mem::transmute(fn_ptr) }
    }

    fn asmbl_intrvl_body(
        bytecode: &[Instr],
        fb: &mut FunctionBuilder,
        fn_refs: &FnRefTable,
        vars: &[Variable],
        ptr_ty: Type,
    ) {
        let use_oprnd = |oprnd: Oprnd, fb: &mut FunctionBuilder| match oprnd {
            Oprnd::Reg(indx) => fb.use_var(vars[indx as usize]),
            Oprnd::Imm(imm) => {
                let imm = fb.ins().f64const(imm);
                fb.ins().splat(types::F64X2, imm)
            }
        };

        let call_fn = |name: &str, v: &[Value], fb: &mut FunctionBuilder| {
            let ret_slot = fb.create_sized_stack_slot(ir::StackSlotData {
                kind: ir::StackSlotKind::ExplicitSlot,
                size: 2 * 8,
                align_shift: 0,
            });

            let fn_ref = fn_refs[name];
            let ret_addr = fb.ins().stack_addr(ptr_ty, ret_slot, 0);
            let mut args = vec![ret_addr];
            args.extend_from_slice(v);

            let _ = fb.ins().call(fn_ref, &args);
            let res = fb
                .ins()
                .load(types::F64X2, ir::MemFlags::new(), ret_addr, 0);
            res
        };

        for &instr in bytecode {
            match instr {
                Instr::UnOp { op, val, dst } => {
                    let dst = dst as usize;
                    let val = use_oprnd(val, fb);

                    let res = match op {
                        UnOp::MOV => val,
                        UnOp::SIN => Self::asmbl_sin_intrvl(val, fb, fn_refs),
                        UnOp::COS => Self::asmbl_cos_intrvl(val, fb, fn_refs),
                        UnOp::TAN => call_fn("tan_intrvl", &[val], fb),
                    };

                    fb.def_var(vars[dst], res);
                }
                Instr::BinOp { op, lhs, rhs, dst } => {
                    let dst = dst as usize;
                    let lhs = use_oprnd(lhs, fb);
                    let rhs = use_oprnd(rhs, fb);

                    let res = match op {
                        BinOp::ADD => Self::asmbl_add_intrvl(lhs, rhs, fb),
                        BinOp::SUB => Self::asmbl_sub_intrvl(lhs, rhs, fb),
                        BinOp::MUL => Self::asmbl_mul_intrvl(lhs, rhs, fb),
                        BinOp::DIV => Self::asmbl_div_intrvl(lhs, rhs, fb),
                        BinOp::POW => Self::asmbl_pow_intrvl(lhs, rhs, fb, fn_refs),
                    };

                    fb.def_var(vars[dst], res);
                }
            }
        }
    }

    fn asmbl_call_f64(
        name: &str,
        v: &[Value],
        fb: &mut FunctionBuilder,
        fn_refs: &FnRefTable,
    ) -> Value {
        let fn_ref = *fn_refs
            .get(name)
            .expect(&format!("could not find function: {name}"));
        let call = fb.ins().call(fn_ref, v);
        fb.inst_results(call)[0]
    }

    fn asmbl_add_intrvl(lhs: Value, rhs: Value, fb: &mut FunctionBuilder) -> Value {
        fb.ins().fadd(lhs, rhs)
    }

    fn asmbl_sub_intrvl(lhs: Value, rhs: Value, fb: &mut FunctionBuilder) -> Value {
        let l_lo = fb.ins().extractlane(lhs, 0);
        let l_hi = fb.ins().extractlane(lhs, 1);
        let r_lo = fb.ins().extractlane(rhs, 0);
        let r_hi = fb.ins().extractlane(rhs, 1);

        let lo = fb.ins().fsub(l_lo, r_hi);
        let hi = fb.ins().fsub(l_hi, r_lo);

        let res = fb.ins().splat(types::F64X2, lo);
        fb.ins().insertlane(res, hi, 1)
    }

    fn asmbl_if_else(
        cond: Value,
        fb: &mut FunctionBuilder,
        then_block_fn: impl FnOnce(&mut FunctionBuilder),
        else_block_fn: impl FnOnce(&mut FunctionBuilder),
    ) {
        let then_block = fb.create_block();
        let else_block = fb.create_block();

        fb.ins().brif(cond, then_block, &[], else_block, &[]);

        {
            fb.switch_to_block(then_block);
            fb.seal_block(then_block);
            then_block_fn(fb);
        }

        {
            fb.switch_to_block(else_block);
            fb.seal_block(else_block);
            else_block_fn(fb);
        }
    }

    fn asmbl_mul_intrvl(lhs: Value, rhs: Value, fb: &mut FunctionBuilder) -> Value {
        let l_lo = fb.ins().extractlane(lhs, 0);
        let l_hi = fb.ins().extractlane(lhs, 1);
        let r_lo = fb.ins().extractlane(rhs, 0);
        let r_hi = fb.ins().extractlane(rhs, 1);

        let a = fb.ins().fmul(l_lo, r_lo);
        let b = fb.ins().fmul(l_lo, r_hi);
        let c = fb.ins().fmul(l_hi, r_lo);
        let d = fb.ins().fmul(l_hi, r_hi);

        let lo = {
            let ab = fb.ins().fmin(a, b);
            let cd = fb.ins().fmin(c, d);
            fb.ins().fmin(ab, cd)
        };

        let hi = {
            let ab = fb.ins().fmax(a, b);
            let cd = fb.ins().fmax(c, d);
            fb.ins().fmax(ab, cd)
        };

        let res = fb.ins().splat(types::F64X2, lo);
        fb.ins().insertlane(res, hi, 1)
    }

    fn asmbl_div_intrvl(lhs: Value, rhs: Value, fb: &mut FunctionBuilder) -> Value {
        let one_s = fb.ins().f64const(1.0);
        let one = fb.ins().splat(types::F64X2, one_s);
        let inv_rhs = fb.ins().fdiv(one, rhs);

        Self::asmbl_mul_intrvl(lhs, inv_rhs, fb)
    }

    fn asmbl_f64x2(v0: Value, v1: Value, fb: &mut FunctionBuilder) -> Value {
        let vec = fb.ins().splat(types::F64X2, v0);
        fb.ins().insertlane(vec, v1, 1)
    }

    fn asmbl_sin_intrvl(val: Value, fb: &mut FunctionBuilder, fn_refs: &FnRefTable) -> Value {
        let half_pi = fb.ins().f64const(std::f64::consts::FRAC_PI_2);
        let three_half = fb.ins().f64const(3.0 * std::f64::consts::FRAC_PI_2);
        let two_pi = fb.ins().f64const(2.0 * std::f64::consts::PI);
        let one = fb.ins().f64const(1.0);
        let neg_one = fb.ins().f64const(-1.0);

        let lo = fb.ins().extractlane(val, 0);
        let hi = fb.ins().extractlane(val, 1);

        // width check for full range
        let dist = fb.ins().fsub(hi, lo);
        let is_ge_2pi = fb.ins().fcmp(FloatCC::GreaterThanOrEqual, dist, two_pi);

        let blk_full = fb.create_block();
        let blk_norm = fb.create_block();
        let blk_exit = fb.create_block();
        fb.ins().brif(is_ge_2pi, blk_full, &[], blk_norm, &[]);

        // Full‑range case
        fb.switch_to_block(blk_full);
        fb.seal_block(blk_full);
        let full_vec = Self::asmbl_f64x2(neg_one, one, fb);
        fb.ins().jump(blk_exit, &[full_vec]);

        // Normal case
        fb.switch_to_block(blk_norm);
        fb.seal_block(blk_norm);

        // Reduce lo/hi modulo 2π
        let l = Self::asmbl_call_f64("rem_euclid_f64", &[lo, two_pi], fb, fn_refs);
        let u = Self::asmbl_call_f64("rem_euclid_f64", &[hi, two_pi], fb, fn_refs);

        // Compute sin(l) and sin(u)
        let ls = Self::asmbl_call_f64("sin_f64", &[l], fb, fn_refs);
        let us = Self::asmbl_call_f64("sin_f64", &[u], fb, fn_refs);

        // Detect wrap-around if l > u
        let wrap = fb.ins().fcmp(FloatCC::GreaterThan, l, u);

        // HALF_PI membership
        let le_l_half = fb.ins().fcmp(FloatCC::LessThanOrEqual, l, half_pi);
        let le_half_u = fb.ins().fcmp(FloatCC::LessThanOrEqual, half_pi, u);
        let straight_half = fb.ins().band(le_l_half, le_half_u);
        let wrap_half = fb.ins().bor(le_l_half, le_half_u);
        let tmp1 = fb.ins().band(wrap, wrap_half);
        let contains_half = fb.ins().bor(straight_half, tmp1);

        // THREE_HALVES_PI membership
        let le_l_3h = fb.ins().fcmp(FloatCC::LessThanOrEqual, l, three_half);
        let le_3h_u = fb.ins().fcmp(FloatCC::LessThanOrEqual, three_half, u);
        let straight_3half = fb.ins().band(le_l_3h, le_3h_u);
        let wrap_3half = fb.ins().bor(le_l_3h, le_3h_u);
        let tmp2 = fb.ins().band(wrap, wrap_3half);
        let contains_3half = fb.ins().bor(straight_3half, tmp2);

        // Dispatch subcases
        let blk_a = fb.create_block(); // both π/2 & 3π/2
        let blk_b_hdr = fb.create_block(); // header for “3π/2 only”
        let blk_b_case = fb.create_block(); // body for case B
        let blk_c = fb.create_block(); // π/2 only
        let blk_d = fb.create_block(); // neither

        // Case A?
        let a_flag = fb.ins().band(contains_half, contains_3half);
        fb.ins().brif(a_flag, blk_a, &[], blk_b_hdr, &[]);
        fb.seal_block(blk_a);

        // Case B header: contains_3half && !contains_half
        fb.switch_to_block(blk_b_hdr);
        let not_contains_half = fb.ins().bnot(contains_half);
        let b_flag = fb.ins().band(contains_3half, not_contains_half);
        fb.ins().brif(b_flag, blk_b_case, &[], blk_c, &[]);
        fb.seal_block(blk_b_hdr);

        // Case A body
        fb.switch_to_block(blk_a);
        let vec_a = Self::asmbl_f64x2(neg_one, one, fb);
        fb.ins().jump(blk_exit, &[vec_a]);

        // Case B body: [-1, max(ls,us)]
        fb.switch_to_block(blk_b_case);
        fb.seal_block(blk_b_case);
        let max_ls_us = fb.ins().fmax(ls, us);
        let vec_b = Self::asmbl_f64x2(neg_one, max_ls_us, fb);
        fb.ins().jump(blk_exit, &[vec_b]);

        // Case C: [min(ls,us), 1]
        fb.switch_to_block(blk_c);
        fb.seal_block(blk_c);
        let min_ls_us = fb.ins().fmin(ls, us);
        let vec_c = Self::asmbl_f64x2(min_ls_us, one, fb);
        fb.ins().jump(blk_exit, &[vec_c]);

        // Case D: [min(ls,us), max(ls,us)]
        fb.switch_to_block(blk_d);
        fb.seal_block(blk_d);
        let lo_d = fb.ins().fmin(ls, us);
        let hi_d = fb.ins().fmax(ls, us);
        let vec_d = Self::asmbl_f64x2(lo_d, hi_d, fb);
        fb.ins().jump(blk_exit, &[vec_d]);

        // Merge
        fb.switch_to_block(blk_exit);
        fb.append_block_param(blk_exit, types::F64X2);
        fb.seal_block(blk_exit);

        fb.block_params(blk_exit)[0]
    }

    fn asmbl_cos_intrvl(val: Value, fb: &mut FunctionBuilder, fn_refs: &FnRefTable) -> Value {
        let zero = fb.ins().f64const(0.0);
        let one = fb.ins().f64const(1.0);
        let neg_one = fb.ins().f64const(-1.0);
        let pi = fb.ins().f64const(std::f64::consts::PI);
        let two_pi = fb.ins().f64const(2.0 * std::f64::consts::PI);

        let lo = fb.ins().extractlane(val, 0);
        let hi = fb.ins().extractlane(val, 1);

        let dist = fb.ins().fsub(hi, lo);
        let full_cc = fb.ins().fcmp(FloatCC::GreaterThanOrEqual, dist, two_pi);

        let blk_full = fb.create_block();
        let blk_norm = fb.create_block();
        let blk_exit = fb.create_block();
        fb.ins().brif(full_cc, blk_full, &[], blk_norm, &[]);

        fb.switch_to_block(blk_full);
        fb.seal_block(blk_full);
        let full_vec = Self::asmbl_f64x2(neg_one, one, fb);
        fb.ins().jump(blk_exit, &[full_vec]);

        fb.switch_to_block(blk_norm);
        fb.seal_block(blk_norm);

        let l = Self::asmbl_call_f64("rem_euclid_f64", &[lo, two_pi], fb, fn_refs);
        let u = Self::asmbl_call_f64("rem_euclid_f64", &[hi, two_pi], fb, fn_refs);

        let lc = Self::asmbl_call_f64("cos_f64", &[l], fb, fn_refs);
        let uc = Self::asmbl_call_f64("cos_f64", &[u], fb, fn_refs);

        let wrap = fb.ins().fcmp(FloatCC::GreaterThan, l, u);

        let le_l_0 = fb.ins().fcmp(FloatCC::LessThanOrEqual, l, zero);
        let le_0_u = fb.ins().fcmp(FloatCC::LessThanOrEqual, zero, u);
        let straight0 = fb.ins().band(le_l_0, le_0_u);
        let wrap0 = fb.ins().bor(le_l_0, le_0_u);
        let tmp0 = fb.ins().band(wrap, wrap0);
        let contains_zero = fb.ins().bor(straight0, tmp0);

        let le_l_pi = fb.ins().fcmp(FloatCC::LessThanOrEqual, l, pi);
        let le_pi_u = fb.ins().fcmp(FloatCC::LessThanOrEqual, pi, u);
        let straightP = fb.ins().band(le_l_pi, le_pi_u);
        let wrapP = fb.ins().bor(le_l_pi, le_pi_u);
        let tmpP = fb.ins().band(wrap, wrapP);
        let contains_pi = fb.ins().bor(straightP, tmpP);

        let blk_a_hdr = fb.create_block(); // header for case A?
        let blk_a = fb.create_block(); // case A body (π & 0 both contained)
        let blk_b_hdr = fb.create_block(); // header for case B?
        let blk_b = fb.create_block(); // case B body (π only)
        let blk_c = fb.create_block(); // case C (0 only)
        let blk_d = fb.create_block(); // case D neither

        let a_flag = fb.ins().band(contains_pi, contains_zero);
        fb.ins().brif(a_flag, blk_a, &[], blk_b_hdr, &[]);
        fb.seal_block(blk_a);

        fb.switch_to_block(blk_b_hdr);
        let not_zero = fb.ins().bnot(contains_zero);
        let b_flag = fb.ins().band(contains_pi, not_zero);
        fb.ins().brif(b_flag, blk_b, &[], blk_c, &[]);
        fb.seal_block(blk_b_hdr);

        fb.switch_to_block(blk_a);
        let vec_a = Self::asmbl_f64x2(neg_one, one, fb);
        fb.ins().jump(blk_exit, &[vec_a]);

        fb.switch_to_block(blk_b);
        fb.seal_block(blk_b);
        let max_lc_uc = fb.ins().fmax(lc, uc);
        let vec_b = Self::asmbl_f64x2(neg_one, max_lc_uc, fb);
        fb.ins().jump(blk_exit, &[vec_b]);

        fb.switch_to_block(blk_c);
        fb.seal_block(blk_c);
        let min_lc_uc = fb.ins().fmin(lc, uc);
        let vec_c = Self::asmbl_f64x2(min_lc_uc, one, fb);
        fb.ins().jump(blk_exit, &[vec_c]);

        fb.switch_to_block(blk_d);
        fb.seal_block(blk_d);
        let lo_d = fb.ins().fmin(lc, uc);
        let hi_d = fb.ins().fmax(lc, uc);
        let vec_d = Self::asmbl_f64x2(lo_d, hi_d, fb);
        fb.ins().jump(blk_exit, &[vec_d]);

        fb.switch_to_block(blk_exit);
        fb.append_block_param(blk_exit, types::F64X2);
        fb.seal_block(blk_exit);

        fb.block_params(blk_exit)[0]
    }

    fn asmbl_pow_intrvl(
        a_vec: Value,
        b_vec: Value,
        fb: &mut FunctionBuilder,
        fn_refs: &FnRefTable,
    ) -> Value {
        let zero = fb.ins().f64const(0.0);
        let eps = fb.ins().f64const(std::f64::EPSILON);

        let a_lo = fb.ins().extractlane(a_vec, 0);
        let a_hi = fb.ins().extractlane(a_vec, 1);
        let b_lo = fb.ins().extractlane(b_vec, 0);
        let b_hi = fb.ins().extractlane(b_vec, 1);

        let blk_a_nonneg = fb.create_block(); // a.lo >= 0
        let blk_a_neg = fb.create_block(); // a.hi < 0
        let blk_a_mixed_int = fb.create_block(); // mixed sign but b const int
        let blk_undef = fb.create_block(); // catch‐all
        let blk_exit = fb.create_block();

        // println!("blk_a_nonneg: {:?}", blk_a_nonneg);
        // println!("blk_a_neg: {:?}", blk_a_neg);
        // println!("blk_a_mixed_int: {:?}", blk_a_mixed_int);
        // println!("blk_undef: {:?}", blk_undef);
        // println!("blk_exit: {:?}", blk_exit);

        let a_lo_ge_zero = fb.ins().fcmp(FloatCC::GreaterThanOrEqual, a_lo, zero);
        fb.ins()
            .brif(a_lo_ge_zero, blk_a_nonneg, &[], blk_a_neg, &[]);
        fb.seal_block(blk_a_nonneg);

        fb.switch_to_block(blk_a_nonneg);
        let a_lo_gt_zero = fb.ins().fcmp(FloatCC::GreaterThan, a_lo, zero);
        let blk_a_pos = fb.create_block(); // a > 0
        let blk_a_zero = fb.create_block(); // a.lo == 0
        // println!("blk_a_pos: {:?}", blk_a_pos);
        // println!("blk_a_zero: {:?}", blk_a_zero);
        fb.ins().brif(a_lo_gt_zero, blk_a_pos, &[], blk_a_zero, &[]);
        fb.seal_block(blk_a_pos);

        fb.switch_to_block(blk_a_pos);
        let p1 = Self::asmbl_call_f64("pow_f64", &[a_lo, b_lo], fb, fn_refs);
        let p2 = Self::asmbl_call_f64("pow_f64", &[a_hi, b_hi], fb, fn_refs);
        let p3 = Self::asmbl_call_f64("pow_f64", &[a_hi, b_lo], fb, fn_refs);
        let p4 = Self::asmbl_call_f64("pow_f64", &[a_lo, b_hi], fb, fn_refs);
        let m1 = fb.ins().fmin(p1, p2);
        let m2 = fb.ins().fmin(p3, p4);
        let lo_res = fb.ins().fmin(m1, m2);
        let x1 = fb.ins().fmax(p1, p2);
        let x2 = fb.ins().fmax(p3, p4);
        let hi_res = fb.ins().fmax(x1, x2);
        let vec_a1 = Self::asmbl_f64x2(lo_res, hi_res, fb);
        fb.ins().jump(blk_exit, &[vec_a1]);

        fb.switch_to_block(blk_a_zero);
        fb.seal_block(blk_a_zero);
        let b_lo_le0 = fb.ins().fcmp(FloatCC::LessThanOrEqual, b_lo, zero);
        let b_hi_ge0 = fb.ins().fcmp(FloatCC::GreaterThanOrEqual, b_hi, zero);
        let b_contains_zero = fb.ins().band(b_lo_le0, b_hi_ge0);
        let blk_a_good = fb.create_block();
        // println!("blk_a_good: {:?}", blk_a_pos);
        fb.ins()
            .brif(b_contains_zero, blk_undef, &[], blk_a_good, &[]);
        fb.seal_block(blk_a_good);

        fb.switch_to_block(blk_a_good);
        let hi_pow = Self::asmbl_call_f64("pow_f64", &[a_hi, b_hi], fb, fn_refs);
        let vec_a2 = Self::asmbl_f64x2(zero, hi_pow, fb);
        fb.ins().jump(blk_exit, &[vec_a2]);

        fb.switch_to_block(blk_a_neg);
        fb.seal_block(blk_a_neg);
        let b_eq = fb.ins().fcmp(FloatCC::Equal, b_lo, b_hi);
        let floor_call = fb.ins().floor(b_lo);
        let fract = fb.ins().fsub(b_lo, floor_call);
        let fract_le_eps = fb.ins().fcmp(FloatCC::LessThanOrEqual, fract, eps);
        let is_const_int = fb.ins().band(b_eq, fract_le_eps);
        let blk_b_good = fb.create_block();
        // println!("blk_b_good: {:?}", blk_b_good);
        fb.ins().brif(is_const_int, blk_b_good, &[], blk_undef, &[]);
        fb.seal_block(blk_b_good);

        fb.switch_to_block(blk_b_good);
        let p_hi = Self::asmbl_call_f64("pow_f64", &[a_hi, b_lo], fb, fn_refs);
        let p_lo = Self::asmbl_call_f64("pow_f64", &[a_lo, b_lo], fb, fn_refs);
        let lo_b = fb.ins().fmin(p_hi, p_lo);
        let hi_b = fb.ins().fmax(p_hi, p_lo);
        let vec_b = Self::asmbl_f64x2(lo_b, hi_b, fb);
        fb.ins().jump(blk_exit, &[vec_b]);

        let blk_c_body = fb.create_block();
        // println!("blk_c_body: {:?}", blk_c_body);

        fb.switch_to_block(blk_a_mixed_int);
        fb.seal_block(blk_a_mixed_int);
        fb.ins().brif(is_const_int, blk_c_body, &[], blk_undef, &[]);

        fb.switch_to_block(blk_c_body);
        fb.seal_block(blk_c_body);

        let is_even = Self::asmbl_call_f64("is_even_f64", &[b_lo], fb, fn_refs);
        let blk_c_even = fb.create_block();
        let blk_c_odd = fb.create_block();
        // println!("blk_c_even: {:?}", blk_c_even);
        // println!("blk_c_odd: {:?}", blk_c_odd);
        fb.ins().brif(is_even, blk_c_even, &[], blk_c_odd, &[]);

        fb.switch_to_block(blk_c_even);
        fb.seal_block(blk_c_even);
        let abs_lo = fb.ins().fabs(a_lo); // assemble intrinsic for fabs
        let abs_hi = fb.ins().fabs(a_hi);
        let max_abs = fb.ins().fmax(abs_lo, abs_hi);
        let pow_even = Self::asmbl_call_f64("pow_f64", &[max_abs, b_lo], fb, fn_refs);
        let vec_ce = Self::asmbl_f64x2(zero, pow_even, fb);
        fb.ins().jump(blk_exit, &[vec_ce]);

        fb.switch_to_block(blk_c_odd);
        fb.seal_block(blk_c_odd);
        let lo_co = Self::asmbl_call_f64("pow_f64", &[a_lo, b_lo], fb, fn_refs);
        let pow_co = Self::asmbl_call_f64("pow_f64", &[max_abs, b_lo], fb, fn_refs);
        let vec_co = Self::asmbl_f64x2(lo_co, pow_co, fb);
        fb.ins().jump(blk_exit, &[vec_co]);

        fb.switch_to_block(blk_undef);
        fb.seal_block(blk_undef);
        let nan = fb.ins().f64const(f64::NAN);
        let vec_u = Self::asmbl_f64x2(nan, nan, fb);
        fb.ins().jump(blk_exit, &[vec_u]);

        fb.switch_to_block(blk_exit);
        fb.append_block_param(blk_exit, types::F64X2);
        fb.seal_block(blk_exit);
        fb.block_params(blk_exit)[0]
    }
}

#[cfg(test)]
mod test {
    use rand::seq::IndexedRandom;

    use crate::jit::{Program, Reg};

    use super::*;

    fn random_instr(dst: Reg) -> Instr {
        let mut rng = rand::rng();
        if rand::random_bool(0.4) {
            let op = *[UnOp::MOV, UnOp::SIN, UnOp::COS, UnOp::TAN]
                .choose(&mut rng)
                .unwrap();
            let val = Oprnd::Reg(rand::random_range(0..2));
            Instr::UnOp { op, val, dst }
        } else {
            let op = *[BinOp::ADD, BinOp::SUB, BinOp::MUL, BinOp::DIV, BinOp::POW]
                .choose(&mut rng)
                .unwrap();
            let lhs = Oprnd::Reg(rand::random_range(0..2));
            let rhs = if rand::random_bool(0.5) {
                Oprnd::Reg(rand::random_range(0..2))
            } else {
                let imm = match op {
                    BinOp::DIV => rand::random_range(0.1..5.0),
                    _ => rand::random_range(-5.0..5.0),
                };
                Oprnd::Imm(imm)
            };
            Instr::BinOp { op, lhs, rhs, dst }
        }
    }

    fn gen_random_program(max_len: usize) -> Vec<Instr> {
        let len = rand::random_range(1..=max_len);
        (0..len)
            .map(|i| random_instr((i % 2) as Reg))
            .chain(std::iter::once(bytecode!(ADD[0, 1] -> 0)))
            .collect()
    }

    fn cmp_float(l: f64, r: f64, tol: f64) -> bool {
        l.is_nan() && r.is_nan() || (l - r).abs() < tol || l == r
    }

    #[test]
    fn fuzz_cmp_f64_vs_f64x2() {
        const N: usize = 500;
        const MAX_LEN: usize = 20;
        const TOL: f64 = f64::EPSILON * 10.0;

        for _ in 0..N {
            let jit = JIT::init();

            let prog = gen_random_program(MAX_LEN);
            println!("{:?}", prog);
            let f_scalar = jit.compile_2f64_f64("scalar", &prog);
            let f_simd = jit.compile_2f64x2_f64("simd", &prog);

            let prog = Program::from(prog);

            let x1 = rand::random_range(-10.0..10.0);
            let y1 = rand::random_range(-10.0..10.0);
            let x2 = rand::random_range(-10.0..10.0);
            let y2 = rand::random_range(-10.0..10.0);

            let res_s0 = *f_scalar(x1, y1);
            let res_s1 = *f_scalar(x2, y2);

            let mut res_v = F64X2(0.0, 0.0);
            f_simd(F64X2(x1, x2), F64X2(y1, y2), &mut res_v);

            assert!(
                cmp_float(res_v.0, res_s0, TOL),
                "Lane0 mismatch:\nprog=\n{prog}\nx={x1}, y={y1}\nres_v.0={}, res_s0={}",
                res_v.0,
                res_s0
            );
            assert!(
                cmp_float(res_v.1, res_s1, TOL),
                "Lane1 mismatch:\nprog=\n{prog}\nx={x2}, y={y2}\nres_v.1={}, res_s1={}",
                res_v.1,
                res_s1
            );
        }
    }

    #[test]
    fn f64_binary() {
        let x = 1.5f64;
        let y = 2.5f64;

        let code = bytecode! [
            SIN[0] -> 0,
            COS[1] -> 1,
            MUL[1, 0] -> 0,
            TAN[0] -> 0,
            MUL[0, imm(-1.)] -> 0,
            POW[0, 1] -> 0,
            SUB[1, 0] -> 0,
            DIV[0, imm(3.)] -> 0,
        ];

        let mut a = x.sin();
        let b = y.cos();
        a = b * a;
        a = a.tan();
        a = -1.0 * a;
        a = a.powf(b);
        a = b - a;
        a = a / 3.;

        let mut jit = JIT::init();
        jit.emit_asm = true;
        let func = jit.compile_2f64_f64("binary_fn", &code);

        let res = *func(x, y);
        assert_eq!(res, a, "{res} != {a}");
    }

    #[test]
    fn f64x2_binary() {
        let x = F64X2(1.5, 2.3);
        let y = F64X2(2.5, 1.4);

        let code = bytecode! [
            SIN[0] -> 0,
            COS[1] -> 1,
            ADD[0, 1] -> 0,
            TAN[0] -> 0,
            MUL[0, 0] -> 0,
            POW[0, imm(2.3)] -> 0,
            DIV[imm(1.0), 0] -> 0,
        ];

        let mut a = x.sin();
        let b = y.cos();
        a = a + b;
        a = a.tan();
        a = a * a;
        a = a.pow(&F64X2(2.3, 2.3));
        a = F64X2(1.0, 1.0) / a;

        let mut jit = JIT::init();

        let mut res = F64X2(0., 0.);
        let func = jit.compile_2f64x2_f64("binary_fn", &code);
        func(x, y, &mut res);

        let diff = (res - a).abs();
        assert!(diff.0 < f64::EPSILON * 10.0, "{}", diff.0);
        assert!(diff.1 < f64::EPSILON * 10.0, "{}", diff.1);
    }

    #[test]
    fn intrvl_binary() {
        let i1 = Intrvl::new(1.5, 2.3);
        let i2 = Intrvl::new(2.5, 5.4);
        // let i1 = Intrvl(4.23396129404815, 4.60541682970345), (4.475363341986881, 4.475363341986881)

        let code = bytecode! [
            SIN[0] -> 0,
            SUB[0, 1] -> 0,
        ];

        let a = i1.sin().sub(i2);
        let a = F64X2(a.lo, a.hi);

        let jit = JIT::init();

        let mut res = F64X2(0., 0.);
        let func = jit.compile_2intrvl_intrvl("binary_fn", &code);
        func(F64X2(i1.lo, i1.hi), F64X2(i2.lo, i2.hi), &mut res);

        let diff = (res - a).abs();
        assert!(diff.0 < f64::EPSILON * 10.0, "{}", diff.0);
        assert!(diff.1 < f64::EPSILON * 10.0, "{}", diff.1);
    }
}
