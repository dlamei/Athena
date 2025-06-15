use std::{cmp, fmt, ops, rc::Rc};

use crate::{config::EvalStrategy, log_fn};
use itertools::Itertools;
use num::rational::Ratio;

use crate::{
    config::{AddStrategy, MulStrategy, PowStrategy, noctua_global_config},
    flat_deque::FlatDeque,
    real::{Real, Sign},
};

mod ordering_abbreviations {
    use std::cmp::Ordering;

    pub const GE: Ordering = Ordering::Greater;
    pub const LE: Ordering = Ordering::Less;
    pub const EQ: Ordering = Ordering::Equal;
}

macro_rules! bits {
    ($n:literal) => { 1 << $n };
    ($i:ident) => { Meta::$i.bits() };
    ($($x:tt)|+) => {
        $(bits!($x) | )* 0
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Meta: u32 {
        /// The sign of the expression is only stored as a flag. Negative integers are just
        /// unsigned with this flag set. Absence of this bit implies a plus sign
        const SIGN_MINUS        = bits!(0);

        const HAS_UNDEF         = bits!(1);
        const IS_NUMERIC        = bits!(2);
        const IS_INTEGER        = bits!(3 | IS_RATIONAL | IS_REAL);
        const IS_RATIONAL       = bits!(4 | IS_NUMERIC);

        const IS_ZERO           = bits!(5 | IS_INTEGER | IS_EVEN);
        const IS_NON_ZERO       = bits!(6 | IS_NUMERIC);

        const IS_EVEN           = bits!(7 | IS_INTEGER);
        const IS_ODD            = bits!(8 | IS_INTEGER);

        /// different to [Meta::SIGN_PLUS] because expressions with a positive sign can still be
        /// negative
        const IS_POSITIVE       = bits!(9 | IS_NON_ZERO);
        /// different to [Meta::SIGN_MINUS] because expressions with a negative sign can still be
        /// positive
        const IS_NEGATIVE       = bits!(10 | IS_NON_ZERO);

        const IS_REAL           = bits!(11 | IS_NUMERIC);

    }
}

macro_rules! bitop {
    ($l:ident |= $r:expr) => {
        $l = $l.union($r);
    };
}

fn add_signed_ratio(
    (s_l, r_l): (Sign, Ratio<u32>),
    (s_r, r_r): (Sign, Ratio<u32>),
) -> (Sign, Ratio<u32>) {
    if s_l == s_r {
        let sum = r_l + r_r;
        if *sum.numer() == 0 {
            return (Sign::Plus, Ratio::ZERO);
        }
        return (s_l, sum);
    }

    match r_l.cmp(&r_r) {
        cmp::Ordering::Equal => (Sign::Plus, Ratio::ZERO),
        cmp::Ordering::Greater => {
            let diff = r_l - r_r;
            (s_l, diff)
        }
        cmp::Ordering::Less => {
            let diff = r_r - r_l;
            (s_r, diff)
        }
    }
}

fn mul_signed_ratio(
    (s_l, r_l): (Sign, Ratio<u32>),
    (s_r, r_r): (Sign, Ratio<u32>),
) -> (Sign, Ratio<u32>) {
    (s_l * s_r, r_l * r_r)
}

/// Compute `(sb * b)^(se * e)` for rational base `b` and rational exponent `e`, extracting any integer‐power factor.
///
/// # Parameters
/// - `sb: Sign` and `b: Ratio<u32>`: sign and absolute value of the base (nonzero unless exponent is zero).
/// - `se: Sign` and `e: Ratio<u32>`: sign and absolute value of the exponent (nonnegative).
///
/// # Behavior
/// 1. Panics on `0^0` or `0^(negative)`. Returns `0` for `0^(positive)`.
/// 2. If exponent is negative, replace `b` with its reciprocal and make exponent positive.
/// 3. If `e = en/ed` is an integer (ed == 1), compute `b^en` (odd exponents keep `sb`, even force `Plus`).
/// 4. If `e > 1` (en > ed), write `e = quot + rem/ed`:
///    - Return `((sgn, b.pow(quot)), Some((Plus, rem/ed)))`.
///    - Sign is `Plus` if `quot` is even, else `sb`.
/// 5. If `0 < e < 1`, return `((sb, b), Some((Plus, e)))`.
///
/// # Returns
/// - A pair `((Sign, Ratio<u32>), Option<(Sign, Ratio<u32>)>)`:
///   - First is the integer‐power part (`b^quot` or `b`).
///   - Second is `None` if no fractional exponent remains, or `Some((Plus, rem/ed))` otherwise.
fn pow_rational(
    (mut sb, mut b): (Sign, Ratio<u32>),
    (se, e): (Sign, Ratio<u32>),
) -> ((Sign, Ratio<u32>), Option<(Sign, Ratio<u32>)>) {
    // Extract numerators/denominators as plain u32:
    let (mut bn, mut bd) = (*b.numer(), *b.denom());
    let (en, ed) = (*e.numer(), *e.denom());

    // 1) 0^0 is undefined
    if bn == 0 && en == 0 {
        panic!("0^0 is undefined");
    }

    if bn == 0 && se.is_minus() {
        panic!("0 raised to a negative exponent is undefined");
    }
    if bn == 0 {
        return ((Sign::Plus, Ratio::ZERO), None);
    }
    if en == 0 {
        return ((Sign::Plus, Ratio::ONE), None);
    }

    if se.is_minus() {
        std::mem::swap(&mut bn, &mut bd);
        b = Ratio::new(bn, bd);
        // se = se.flip();
    }

    if ed == 1 {
        if en % 2 == 0 {
            sb = Sign::Plus;
        }
        let int_exp = en as i32;
        let result = b.pow(int_exp);
        return ((sb, result), None);
    }

    if en > ed {
        let (quot, rem) = num::integer::div_rem(en, ed);
        if quot % 2 == 0 {
            sb = Sign::Plus;
        }
        let int_part = b.pow(quot as i32);

        let rem_rational = Ratio::new(rem, ed);
        return ((sb, int_part), Some((Sign::Plus, rem_rational)));
    }

    ((sb, b), Some((Sign::Plus, e)))
}

impl Meta {
    #[inline]
    pub const fn of_u32(u: u32) -> Self {
        let mut meta = Meta::IS_INTEGER;
        if u == 0 {
            return Meta::IS_ZERO;
        }

        meta = meta.union(Meta::IS_POSITIVE);
        if u % 2 == 0 {
            meta = meta.union(Meta::IS_EVEN);
        } else {
            meta = meta.union(Meta::IS_ODD);
        }

        meta
    }

    #[inline]
    pub const fn of_rational(r: Ratio<u32>) -> Self {
        if *r.denom() == 1 {
            Self::of_u32(*r.numer())
        } else {
            let mut res = Meta::empty();
            bitop!(res |= Meta::IS_RATIONAL);
            bitop!(res |= Meta::IS_POSITIVE);
            res
        }
    }

    pub const fn match_exclusive(a: (Meta, Meta), b: (Meta, Meta)) -> bool {
        b.0.has(a.0) && !b.0.has(a.1) && !b.1.has(a.0) && b.1.has(a.1)
            || b.1.has(a.0) && !b.1.has(a.1) && !b.0.has(a.0) && b.0.has(a.1)
    }

    #[inline]
    pub fn of_add2(l: Meta, r: Meta) -> Meta {
        Self::of_add(l, r)
    }

    #[inline]
    pub fn of_mul2(l: Meta, r: Meta) -> Meta {
        Self::of_mul(l, r)
    }

    #[inline]
    pub const fn of_add(l: Meta, r: Meta) -> Meta {
        use Meta as M;
        l.dbg_check_valid();
        r.dbg_check_valid();

        let mut res = Meta::empty();

        bitop!(res |= M::HAS_UNDEF.if_either(l, r));

        if l.has(M::IS_ZERO) {
            return r;
        } else if r.has(M::IS_ZERO) {
            return l;
        }

        bitop!(res |= M::IS_NUMERIC.if_both(l, r));
        bitop!(res |= M::IS_INTEGER.if_both(l, r));
        bitop!(res |= M::IS_RATIONAL.if_both(l, r));
        bitop!(res |= M::IS_REAL.if_both(l, r));
        bitop!(res |= M::IS_ZERO.if_both(l, r));
        bitop!(res |= M::IS_POSITIVE.if_both(l, r));
        bitop!(res |= M::IS_NEGATIVE.if_both(l, r));
        bitop!(res |= M::IS_EVEN.if_both(l, r));

        if M::IS_EVEN.in_both(l, r) && M::IS_ODD.in_both(l, r) {
            bitop!(res |= M::IS_EVEN);
        }

        if M::match_exclusive((M::IS_EVEN, M::IS_ODD), (l, r)) {
            bitop!(res |= M::IS_ODD);
        }

        if M::match_exclusive((M::IS_ZERO, M::IS_POSITIVE), (l, r)) {
            bitop!(res |= M::IS_POSITIVE);
        }

        if M::match_exclusive((M::IS_ZERO, M::IS_NEGATIVE), (l, r)) {
            bitop!(res |= M::IS_NEGATIVE);
        }

        res.dbg_check_valid();
        res
    }

    #[inline]
    pub const fn of_mul(l: Meta, r: Meta) -> Meta {
        use Meta as M;
        l.dbg_check_valid();
        r.dbg_check_valid();

        let mut res = Meta::empty();

        if M::IS_ZERO.in_either(l, r) {
            return M::of_u32(0);
        }

        bitop!(res |= M::IS_NUMERIC.if_both(l, r));
        bitop!(res |= M::IS_INTEGER.if_both(l, r));
        bitop!(res |= M::IS_REAL.if_both(l, r));
        bitop!(res |= M::IS_ODD.if_both(l, r));
        bitop!(res |= M::IS_NON_ZERO.if_both(l, r));

        bitop!(res |= M::HAS_UNDEF.if_either(l, r));
        bitop!(res |= M::IS_EVEN.if_either(l, r));

        if M::IS_POSITIVE.in_both(l, r) || M::IS_NEGATIVE.in_both(l, r) {
            bitop!(res |= M::IS_POSITIVE);
        }

        bitop!(res |= M::IS_NEGATIVE.if_both_exclusive(l, r));

        res.dbg_check_valid();
        res
    }

    #[inline]
    pub const fn of_div(l: Meta, r: Meta) -> Meta {
        let rhs = Meta::of_pow(r, Meta::of_neg(Meta::of_u32(1)));
        Meta::of_mul(l, rhs)
    }

    #[inline]
    pub const fn of_pow(b: Meta, e: Meta) -> Meta {
        use Meta as M;
        b.dbg_check_valid();
        e.dbg_check_valid();

        let mut res = Meta::empty();

        bitop!(res |= M::IS_NUMERIC.if_both(b, e));

        if M::HAS_UNDEF.in_either(b, e) || M::IS_ZERO.in_both(b, e) {
            bitop!(res |= M::HAS_UNDEF);
            return res;
        }

        // 2) If exponent is zero but base ≠ 0 => result = 1
        if e.has(M::IS_ZERO) && b.has(M::IS_NON_ZERO) {
            return M::of_u32(1);
        }

        if b.has(M::IS_ZERO) && e.has(M::IS_POSITIVE) {
            return M::of_u32(0);
        }

        if b.has(M::IS_ZERO) && e.has(M::IS_NEGATIVE) {
            return M::HAS_UNDEF;
        }

        if e.has(M::IS_POSITIVE) && b.has(M::IS_NUMERIC) {
            bitop!(res |= M::IS_NUMERIC);

            if b.has(M::IS_INTEGER) {
                bitop!(res |= M::IS_INTEGER);
            } else {
                if b.has(M::IS_RATIONAL) {
                    bitop!(res |= M::IS_RATIONAL);
                }
            }
            bitop!(res |= M::IS_REAL);
        }

        if e.has(M::IS_NEGATIVE) {
            if e.has(M::IS_NON_ZERO) {
                bitop!(res |= M::IS_NUMERIC);
                bitop!(res |= M::IS_RATIONAL);
                bitop!(res |= M::IS_REAL);
            }
        }

        if e.has(M::IS_EVEN) {
            if b.has(M::IS_NON_ZERO) && b.has(M::IS_REAL) {
                bitop!(res |= M::IS_POSITIVE);
            }

            if b.has(M::IS_NON_ZERO) && b.has(M::IS_NUMERIC) {
                bitop!(res |= M::IS_EVEN);
            }
        }

        if e.has(M::IS_ODD) {
            if b.has(M::IS_POSITIVE) {
                bitop!(res |= M::IS_POSITIVE);
            }
            if b.has(M::IS_NEGATIVE) {
                bitop!(res |= M::IS_NEGATIVE);
            }
            if b.has(M::IS_NON_ZERO) && b.has(M::IS_NUMERIC) {
                bitop!(res |= M::IS_NON_ZERO);
            }
        }

        res.dbg_check_valid();
        res
    }

    #[inline]
    pub const fn of_sin(x: Meta) -> Self {
        use Meta as M;
        x.dbg_check_valid();

        let mut res = M::empty();

        if x.has(M::HAS_UNDEF) {
            bitop!(res |= M::HAS_UNDEF);
        }

        if x.has(M::IS_ZERO) {
            return M::of_u32(0);
        }

        if x.has(M::IS_NUMERIC) {
            bitop!(res |= M::IS_NUMERIC);
            bitop!(res |= M::IS_REAL);
        }

        res.dbg_check_valid();
        res
    }

    #[inline]
    pub const fn of_cos(x: Meta) -> Self {
        use Meta as M;
        x.dbg_check_valid();

        let mut res = M::empty();

        if x.has(M::HAS_UNDEF) {
            bitop!(res |= M::HAS_UNDEF);
        }

        if x.has(M::IS_ZERO) {
            return M::of_u32(1);
        }

        if x.has(M::IS_NUMERIC) {
            bitop!(res |= M::IS_NUMERIC);
            bitop!(res |= M::IS_REAL);
        }

        res.dbg_check_valid();
        res
    }

    #[inline]
    pub const fn of_neg(x: Meta) -> Meta {
        use Meta as M;
        x.dbg_check_valid();

        let mut res = M::empty();

        bitop!(res |= M::HAS_UNDEF.if_in(x));
        bitop!(res |= M::IS_NUMERIC.if_in(x));
        bitop!(res |= M::IS_INTEGER.if_in(x));
        bitop!(res |= M::IS_RATIONAL.if_in(x));
        bitop!(res |= M::IS_ZERO.if_in(x));
        bitop!(res |= M::IS_NON_ZERO.if_in(x));
        bitop!(res |= M::IS_EVEN.if_in(x));
        bitop!(res |= M::IS_ODD.if_in(x));
        bitop!(res |= M::IS_REAL.if_in(x));

        if x.has(M::IS_POSITIVE) {
            bitop!(res |= M::IS_NEGATIVE);
        }
        if x.has(M::IS_NEGATIVE) {
            bitop!(res |= M::IS_POSITIVE);
        }
        if !x.has(M::SIGN_MINUS) {
            bitop!(res |= M::SIGN_MINUS);
        }

        res
    }

    #[inline]
    pub const fn if_in(self, l: Meta) -> Meta {
        if l.has(self) { self } else { Meta::empty() }
    }

    #[inline]
    pub const fn if_both(self, l: Meta, r: Meta) -> Meta {
        if self.in_both(l, r) {
            self
        } else {
            Meta::empty()
        }
    }

    #[inline]
    pub const fn if_either(self, l: Meta, r: Meta) -> Meta {
        if self.in_either(l, r) {
            self
        } else {
            Meta::empty()
        }
    }

    #[inline]
    pub const fn in_either(self, l: Meta, r: Meta) -> bool {
        l.has(self) || r.has(self)
    }

    #[inline]
    pub const fn in_both(self, l: Meta, r: Meta) -> bool {
        l.has(self) && r.has(self)
    }

    #[inline]
    pub const fn if_both_exclusive(self, l: Meta, r: Meta) -> Meta {
        if self.in_both_exclusive(l, r) {
            self
        } else {
            Meta::empty()
        }
    }

    #[inline]
    pub const fn in_both_exclusive(self, l: Meta, r: Meta) -> bool {
        l.has(self) ^ r.has(self)
    }

    #[inline]
    pub const fn has(&self, m: Meta) -> bool {
        self.contains(m)
    }

    #[inline]
    pub const fn dbg_check_valid(self) {
        #[cfg(debug_assertions)]
        self.check_valid();
    }

    pub const fn check_valid(self) {
        use Meta as M;

        // If undefined, skip all other checks
        if self.contains(M::HAS_UNDEF) {
            return;
        }

        // Mutual‐exclusion rules:
        //  • Cannot be both IS_POSITIVE and IS_NEGATIVE
        //  • Cannot be both IS_EVEN and IS_ODD
        //  • Cannot be both IS_ZERO and IS_NON_ZERO
        //  • IS_ZERO cannot coexist with IS_POSITIVE or IS_NEGATIVE
        if (self.contains(M::IS_POSITIVE) && self.contains(M::IS_NEGATIVE))
            || (self.contains(M::IS_EVEN) && self.contains(M::IS_ODD))
            || (self.contains(M::IS_ZERO) && self.contains(M::IS_NON_ZERO))
            || (self.contains(M::IS_ZERO)
                && (self.contains(M::IS_POSITIVE) || self.contains(M::IS_NEGATIVE)))
        {
            panic!("invalid Meta: consistency check failed");
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(ustr::Ustr);

impl Symbol {
    pub fn new(v: impl AsRef<str>) -> Self {
        Self(ustr::Ustr::from(v.as_ref()))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryFn {
    Sin,
    Cos,
    Tan,
    ASin,
    ACos,
    ATan,
}

impl UnaryFn {
    pub fn name(&self) -> &'static str {
        match self {
            UnaryFn::Sin => "sin",
            UnaryFn::Cos => "cos",
            UnaryFn::Tan => "tan",
            UnaryFn::ASin => "asin",
            UnaryFn::ACos => "acos",
            UnaryFn::ATan => "atan",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryFn {
    Pow,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NAryFn {
    Sum,
    Prod,
}

#[derive(Clone, PartialEq)]
pub enum ExprTyp {
    Undef,
    Rational(Ratio<u32>),
    Var(Symbol),

    Unary(UnaryFn, Rc<Expr>),
    Binary(BinaryFn, Rc<[Expr; 2]>),
    NAry(NAryFn, Rc<FlatDeque<Expr>>),
}

#[derive(Clone)]
pub struct Expr {
    typ: ExprTyp,
    meta: Meta,
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.typ == other.typ
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddMode {
    Frozen,
    #[default]
    Basic,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MulMode {
    Frozen,
    #[default]
    Basic,
    Expand,
    MergeBase,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PowMode {
    Frozen,
    #[default]
    Basic,
    Expand,
}

#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvalMode {
    pub add: AddMode,
    pub mul: MulMode,
    pub pow: PowMode,
}

impl fmt::Debug for EvalMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Eval[+: {:?}, *: {:?}, ^: {:?}]",
            self.add, self.mul, self.pow
        )
    }
}

impl EvalMode {
    pub const fn frozen() -> Self {
        Self {
            add: AddMode::Frozen,
            mul: MulMode::Frozen,
            pow: PowMode::Frozen,
        }
    }

    pub const fn basic() -> Self {
        Self {
            add: AddMode::Basic,
            mul: MulMode::Basic,
            pow: PowMode::Basic,
        }
    }

    pub const fn expand() -> Self {
        Self {
            add: AddMode::Basic,
            mul: MulMode::Expand,
            pow: PowMode::Expand,
        }
    }

    pub const fn with_mul(mut self, mul: MulMode) -> Self {
        self.mul = mul;
        self
    }
}

impl Expr {
    //////////////////////////////////////////////////////
    //////    Constructors
    //////////////////////////////////////////////////////

    #[inline]
    pub const fn undef() -> Expr {
        Expr {
            typ: ExprTyp::Undef,
            meta: Meta::HAS_UNDEF,
        }
    }

    #[inline]
    pub const fn rational(r: Ratio<u32>) -> Expr {
        Expr {
            typ: ExprTyp::Rational(r),
            meta: Meta::of_rational(r),
        }
    }

    #[inline]
    pub const fn signed_rational(s: Sign, r: Ratio<u32>) -> Expr {
        let mut e = Expr::rational(r);
        if s.is_minus() {
            e.meta = Meta::of_neg(e.meta);
        }
        e
    }

    #[inline]
    pub const fn u32(u: u32) -> Expr {
        Expr {
            typ: ExprTyp::Rational(Ratio::new_raw(u, 1)),
            meta: Meta::of_u32(u),
        }
    }

    #[inline]
    pub const fn i32(i: i32) -> Expr {
        let mut e = Expr::u32(i.unsigned_abs());
        if i < 0 {
            e.meta = Meta::of_neg(e.meta);
        }
        e
    }

    #[inline]
    pub fn var(s: &str) -> Expr {
        Expr {
            typ: ExprTyp::Var(Symbol::new(s)),
            meta: Meta::empty(),
        }
    }

    #[inline]
    pub fn nonzero_var(s: &str) -> Expr {
        Expr {
            typ: ExprTyp::Var(Symbol::new(s)),
            meta: Meta::IS_NON_ZERO,
        }
    }

    #[inline]
    pub fn sin(e: Expr) -> Expr {
        let meta = Meta::of_sin(e.meta);
        Expr {
            typ: ExprTyp::Unary(UnaryFn::Sin, e.into()),
            meta,
        }
    }

    #[inline]
    pub fn cos(e: Expr) -> Expr {
        let meta = Meta::of_cos(e.meta);
        Expr {
            typ: ExprTyp::Unary(UnaryFn::Cos, e.into()),
            meta,
        }
    }
    /// should be used when using the take_... functions
    #[inline]
    const fn placeholder() -> Expr {
        Expr {
            typ: ExprTyp::Undef,
            meta: Meta::empty(),
        }
    }

    #[inline]
    const fn infer_from_meta(m: Meta) -> Option<Expr> {
        if m.has(Meta::HAS_UNDEF) {
            Some(Expr::undef())
        } else if m.has(Meta::IS_ZERO) {
            Some(Expr::u32(0))
        } else {
            None
        }
    }

    #[inline]
    pub fn set_attrib(&mut self, m: Meta) -> &mut Expr {
        self.meta = m;
        self
    }

    //////////////////////////////////////////////////////
    //////    Accessors
    //////////////////////////////////////////////////////

    #[inline]
    pub fn sign(&self) -> Sign {
        if self.has_attrib(Meta::SIGN_MINUS) {
            Sign::Minus
        } else {
            Sign::Plus
        }
    }

    #[inline]
    pub fn n_operands(&self) -> usize {
        self.operands().len()
    }

    #[inline]
    pub fn operands(&self) -> &[Expr] {
        match &self.typ {
            ExprTyp::Undef | ExprTyp::Rational(_) | ExprTyp::Var(_) => std::slice::from_ref(self),
            ExprTyp::Unary(_, oprnd) => std::slice::from_ref(oprnd.as_ref()),
            ExprTyp::Binary(_, oprnds) => oprnds.as_slice(),
            ExprTyp::NAry(_, oprnds) => oprnds.as_slice(),
        }
    }

    #[inline]
    pub fn operands_mut(&mut self) -> &mut [Expr] {
        match self {
            Expr {
                typ: ExprTyp::Unary(_, oprnd),
                ..
            } => std::slice::from_mut(Rc::make_mut(oprnd)),
            Expr {
                typ: ExprTyp::Binary(_, oprnds),
                ..
            } => return Rc::make_mut(oprnds).as_mut_slice(),
            Expr {
                typ: ExprTyp::NAry(_, oprnds),
                ..
            } => return Rc::make_mut(oprnds).as_mut_slice(),
            e => std::slice::from_mut(e),
        }
    }

    #[inline]
    pub fn unary_operand_mut(&mut self) -> &mut Expr {
        match &mut self.typ {
            ExprTyp::Unary(_, oprnd) => Rc::make_mut(oprnd),
            _ => panic!(""),
        }
    }

    #[inline]
    pub fn binary_operand_mut(&mut self) -> &mut [Expr; 2] {
        match &mut self.typ {
            ExprTyp::Binary(_, oprnds) => Rc::make_mut(oprnds),
            _ => panic!(""),
        }
    }

    #[inline]
    pub fn nary_operand_mut(&mut self) -> &mut FlatDeque<Expr> {
        match &mut self.typ {
            ExprTyp::NAry(_, oprnds) => Rc::make_mut(oprnds),
            _ => panic!(""),
        }
    }

    const _ONE_REF: &'static Expr = &Expr::u32(1);

    /// returns the rational coefficient
    #[inline]
    fn rational_coeff(&self) -> (Sign, Ratio<u32>) {
        if self.is_prod() {
            let coeff = &self.operands()[0];
            if coeff.is_rational_const() {
                return coeff.as_rational().unwrap();
            }
        }

        (Sign::Plus, Ratio::ONE)
    }

    #[inline]
    fn term_ref(&self) -> &[Expr] {
        if self.is_prod() {
            let coeff = &self.operands()[0];
            if coeff.is_rational_const() {
                return &self.operands()[1..];
            }
        }

        &[]
    }

    /// if `self` is [`Expr::Pow`] return the base otherwise return `self`
    #[inline]
    pub fn base_ref(&self) -> &Expr {
        self.base_expon_ref().0
    }

    /// if `self` is [`Expr::Pow`] return the exponent otherwise return 1
    #[inline]
    pub fn exponent_ref(&self) -> &Expr {
        self.base_expon_ref().1
    }

    /// if `self` is [`BinaryFn::Pow`] return (base, exponent) otherwise (`self`, 1)
    #[inline]
    pub fn base_expon_ref(&self) -> (&Expr, &Expr) {
        match &self.typ {
            ExprTyp::Binary(BinaryFn::Pow, base_expon) => {
                let [base, expon] = base_expon.as_ref();
                (base, expon)
            }
            _ => (self, Expr::_ONE_REF),
        }
    }

    #[inline]
    pub fn as_int(&self) -> Option<(Sign, u32)> {
        self.as_rational().map(|(s, r)| (s, *r.numer()))
    }

    #[inline]
    pub fn as_rational(&self) -> Option<(Sign, Ratio<u32>)> {
        match self.typ {
            ExprTyp::Rational(ratio) => Some((self.sign(), ratio)),
            _ => None,
        }
    }

    #[inline]
    pub fn is_one(&self) -> bool {
        self.is_int32_const_and(|s, u| s.is_plus() && u == 1)
    }

    #[inline]
    pub fn is_int32_const_and(&self, f: impl FnOnce(Sign, u32) -> bool) -> bool {
        self.is_rational_const_and(|s, r| {
            if r.is_integer() {
                f(s, *r.numer())
            } else {
                false
            }
        })
    }

    #[inline]
    pub fn is_rational_const(&self) -> bool {
        self.is_rational_const_and(|_, _| true)
    }

    #[inline]
    pub fn is_rational_const_and(&self, f: impl FnOnce(Sign, Ratio<u32>) -> bool) -> bool {
        let sign = self.sign();
        match &self.typ {
            ExprTyp::Rational(ratio) => f(sign, *ratio),
            _ => false,
        }
    }

    /// return true if the expression is irreducable
    ///
    #[inline]
    pub fn is_atom(&self) -> bool {
        match &self.typ {
            ExprTyp::Undef | ExprTyp::Rational(_) | ExprTyp::Var(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_pow(&self) -> bool {
        self.is_binary_and(|f| f == BinaryFn::Pow)
    }
    #[inline]
    pub fn is_sum(&self) -> bool {
        self.is_nary_and(|f| f == NAryFn::Sum)
    }
    #[inline]
    pub fn is_prod(&self) -> bool {
        self.is_nary_and(|f| f == NAryFn::Prod)
    }

    #[inline]
    pub fn is_unary_and(&self, f: impl FnOnce(UnaryFn) -> bool) -> bool {
        match self.typ {
            ExprTyp::Unary(unary_fn, _) => f(unary_fn),
            _ => false,
        }
    }
    #[inline]
    pub fn is_binary_and(&self, f: impl FnOnce(BinaryFn) -> bool) -> bool {
        match self.typ {
            ExprTyp::Binary(binary_fn, _) => f(binary_fn),
            _ => false,
        }
    }
    #[inline]
    pub fn is_nary_and(&self, f: impl FnOnce(NAryFn) -> bool) -> bool {
        match self.typ {
            ExprTyp::NAry(nary_fn, _) => f(nary_fn),
            _ => false,
        }
    }

    #[inline]
    pub fn is_undef(&self) -> bool {
        self.has_attrib(Meta::HAS_UNDEF)
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.has_attrib(Meta::IS_ZERO)
    }

    #[inline]
    pub fn is_non_zero(&self) -> bool {
        self.has_attrib(Meta::IS_NON_ZERO)
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        self.has_attrib(Meta::IS_INTEGER)
    }

    #[inline]
    pub fn is_even(&self) -> bool {
        self.has_attrib(Meta::IS_EVEN)
    }

    #[inline]
    pub fn is_odd(&self) -> bool {
        self.has_attrib(Meta::IS_ODD)
    }

    #[inline]
    pub fn is_positive(&self) -> bool {
        self.has_attrib(Meta::IS_POSITIVE)
    }

    #[inline]
    pub fn is_negative(&self) -> bool {
        self.has_attrib(Meta::IS_NEGATIVE)
    }

    #[inline]
    pub fn has_attrib(&self, m: Meta) -> bool {
        self.meta.has(m)
    }

    #[inline]
    pub fn is_equal_typ(&self, other: &Expr) -> bool {
        std::mem::discriminant(&self.typ) == std::mem::discriminant(&other.typ)
            && match (&self.typ, &other.typ) {
                (ExprTyp::Unary(fn1, _), ExprTyp::Unary(fn2, _)) => fn1 == fn2,
                (ExprTyp::Binary(fn1, _), ExprTyp::Binary(fn2, _)) => fn1 == fn2,
                (ExprTyp::NAry(fn1, _), ExprTyp::NAry(fn2, _)) => fn1 == fn2,
                _ => true,
            }
    }

    //////////////////////////////////////////////////////
    //////    Operators / Modifiers
    //////////////////////////////////////////////////////

    pub fn add_with(mut self, rhs: Expr, mode: EvalMode) -> Expr {
        self.add_assign_with(rhs, mode);
        self
    }

    #[log_fn]
    pub fn add_assign_with(&mut self, mut rhs: Expr, mode: EvalMode) -> &mut Expr {
        let (l_meta, r_meta) = (self.meta, rhs.meta);

        if mode.add != AddMode::Frozen {
            if let Some(infer) = Expr::infer_from_meta(Meta::of_add2(l_meta, r_meta)) {
                *self = infer;
                return self;
            }

            if self.has_attrib(Meta::IS_ZERO) {
                *self = rhs;
                return self;
            } else if rhs.has_attrib(Meta::IS_ZERO) {
                return self;
            }
        }

        /// flip the sign of the root expression and then flip all the signs of the operands
        fn sum_distribute_sign(e: &mut Expr) {
            debug_assert!(e.is_sum());
            e.mul_sign_mut(Sign::Minus);
            e.operands_mut().iter_mut().for_each(|o| {
                o.mul_sign_mut(Sign::Minus);
            });
        }

        /// Ensure two sum‐expressions share a common sign by distributing the sign over
        /// the smaller (or minus if equal length) side. Returns the sign to factor out of `(l + r)`.
        fn sum_balance_signs(l: &mut Expr, r: &mut Expr) -> Sign {
            let len_l = l.n_operands();
            let len_r = r.n_operands();
            let sign_l = l.sign();
            let sign_r = r.sign();

            if sign_l == sign_r {
                return Sign::Plus;
            }

            if len_l == len_r {
                if sign_l.is_minus() {
                    sum_distribute_sign(l);
                } else {
                    sum_distribute_sign(r);
                }
                Sign::Plus
            } else if len_l > len_r {
                sum_distribute_sign(r);
                sign_l
            } else {
                sum_distribute_sign(l);
                sign_r
            }
        }

        if let (Some(r1), Some(r2)) = (self.as_rational(), rhs.as_rational()) {
            let sum = add_signed_ratio(r1, r2);
            *self = Expr::signed_rational(sum.0, sum.1);
            return self;
        }

        match mode.add {
            AddMode::Basic => {
                if self.is_sum() && rhs.is_sum() {
                    let sum_sign = sum_balance_signs(self, &mut rhs);
                    debug_assert_eq!(self.sign(), rhs.sign());
                    let (sum_l, sum_r) = (self.nary_operand_mut(), rhs.take_nary_operand());

                    sum_l.extend(sum_r);
                    if sum_sign != self.sign() {
                        self.mul_sign_mut(Sign::Minus);
                    }
                } else if self.is_sum() {
                    if self.sign().is_minus() {
                        rhs.mul_sign_mut(Sign::Minus);
                    }

                    let s = self.nary_operand_mut();
                    s.push_back(rhs);
                } else if rhs.is_sum() {
                    if rhs.sign().is_minus() {
                        self.mul_sign_mut(Sign::Minus);
                    }
                    let s = rhs.nary_operand_mut();
                    s.push_front(self.take_expr());
                    *self = rhs;
                } else {
                    *self = Expr {
                        typ: ExprTyp::NAry(NAryFn::Sum, Rc::new([self.take_expr(), rhs].into())),
                        meta: Meta::of_add2(l_meta, r_meta),
                    };
                }

                self.meta = Meta::of_add2(l_meta, r_meta);
            }
            AddMode::Frozen => {
                *self = Expr {
                    typ: ExprTyp::NAry(NAryFn::Sum, Rc::new([self.take_expr(), rhs].into())),
                    meta: Meta::of_add2(l_meta, r_meta),
                };
            }
        }

        self
    }

    pub fn mul_with(mut self, rhs: Expr, mode: EvalMode) -> Expr {
        self.mul_assign_with(rhs, mode);
        self
    }

    #[log_fn]
    pub fn mul_assign_with(&mut self, mut rhs: Expr, mode: EvalMode) -> &mut Expr {
        if mode.mul == MulMode::Frozen {
            let (l_meta, r_meta) = (self.meta, rhs.meta);
            *self = Expr {
                typ: ExprTyp::NAry(NAryFn::Prod, Rc::new([self.take_expr(), rhs].into())),
                meta: Meta::of_mul2(l_meta, r_meta),
            };
            return self;
        }

        if let Some(infer) = Expr::infer_from_meta(Meta::of_mul2(self.meta, rhs.meta)) {
            *self = infer;
            return self;
        }

        let prod_meta = Meta::of_mul2(self.meta, rhs.meta);
        if self.is_prod() && rhs.is_rational_const() {
            let (coeff, _) = self.make_mut_coeff_term();
            //  only rational prod
            coeff.mul_assign_with(rhs, EvalMode::basic());

            // we need to update the meta data when modifying operands
            return self.set_attrib(prod_meta);
        } else if rhs.is_prod() && self.is_rational_const() {
            let (coeff, _) = rhs.make_mut_coeff_term();
            //  only rational prod
            coeff.mul_assign_with(self.take_expr(), EvalMode::basic());
            *self = rhs;
            return self.set_attrib(prod_meta);
        }

        let (s1, _) = self.split_sign();
        let (s2, _) = rhs.split_sign();
        // wrap return value in prod_sign
        let prod_sign = s1 * s2;
        let (l_meta, r_meta) = (self.meta, rhs.meta);

        if self.is_one() {
            *self = rhs;
            return self.mul_sign_mut(prod_sign);
        } else if rhs.is_one() {
            return self.mul_sign_mut(prod_sign);
        } else if let (Some(r1), Some(r2)) = (self.as_rational(), rhs.as_rational()) {
            let prod = mul_signed_ratio(r1, r2);
            *self = Expr::signed_rational(prod.0, prod.1);
            return self.mul_sign_mut(prod_sign);
        }

        match mode.mul {
            MulMode::Basic => {
                if self.is_prod() && rhs.is_prod() {
                    self.nary_operand_mut().extend(rhs.take_nary_operand());
                } else if self.is_prod() {
                    self.nary_operand_mut().push_back(rhs);
                } else if rhs.is_prod() {
                    rhs.nary_operand_mut().push_front(self.take_expr());
                    *self = rhs;
                } else {
                    *self = Expr {
                        typ: ExprTyp::NAry(NAryFn::Prod, Rc::new([self.take_expr(), rhs].into())),
                        meta: Meta::empty(),
                    }
                }
                // update meta-data because we modify expressions in-place
                self.meta = Meta::of_mul2(l_meta, r_meta);
            }
            MulMode::Expand => {
                if self.is_sum() {
                    let mut sum = Expr::u32(0);

                    for mut term in self.take_nary_operand() {
                        term.mul_assign_with(rhs.clone(), mode);
                        sum.add_assign_with(term, mode);
                    }
                    *self = sum;
                } else if rhs.is_sum() {
                    let mut sum = Expr::u32(0);
                    for term in rhs.take_nary_operand() {
                        let mut prod = self.clone();
                        prod.mul_assign_with(term, mode);
                        sum.add_assign_with(prod, mode);
                    }
                    *self = sum;
                } else {
                    self.mul_assign_with(rhs, mode.with_mul(MulMode::MergeBase));
                }
            }

            MulMode::MergeBase => {
                if rhs.is_prod() {
                    for oprnd in rhs.take_nary_operand() {
                        self.mul_assign_with(oprnd, mode);
                    }
                } else if self.is_prod() {
                    if let Some(pow) = self
                        .nary_operand_mut()
                        .iter_mut()
                        .find(|a| a.base_ref() == rhs.base_ref())
                    {
                        let (l_base, l_expon) = pow.make_mut_base_expon();
                        let (_, r_expon) = rhs.make_mut_base_expon();
                        l_expon.add_assign_with(r_expon.take_expr(), mode);
                        // update meta because of in-place modification
                        pow.meta = Meta::of_pow(l_base.meta, l_expon.meta);
                    } else {
                        self.nary_operand_mut().push_back(rhs);
                    }
                } else if self.base_ref() == rhs.base_ref() {
                    let (_, l_expon) = self.make_mut_base_expon();
                    let (_, r_expon) = rhs.make_mut_base_expon();
                    l_expon.add_assign_with(r_expon.take_expr(), mode);
                } else {
                    *self = Expr {
                        typ: ExprTyp::NAry(NAryFn::Prod, Rc::new([self.take_expr(), rhs].into())),
                        meta: Meta::of_mul2(l_meta, r_meta),
                    };
                }

                self.meta = prod_meta;
            }
            MulMode::Frozen => unreachable!(),
        }

        self.mul_sign_mut(prod_sign);
        self
    }

    pub fn pow(mut self, expon: Expr) -> Expr {
        self.pow_with(expon, noctua_global_config().default_eval_mode)
    }

    pub fn pow_with(mut self, expon: Expr, mode: EvalMode) -> Expr {
        self.pow_assign_with(expon, mode);
        self
    }

    #[log_fn]
    pub fn pow_assign_with(&mut self, expon: Expr, mode: EvalMode) -> &mut Expr {
        let (b_meta, e_meta) = (self.meta, expon.meta);
        let pow_meta = Meta::of_pow(b_meta, e_meta);

        if mode.pow != PowMode::Frozen {
            if let Some(infer) = Expr::infer_from_meta(pow_meta) {
                *self = infer;
                return self;
            }

            if expon.is_int32_const_and(|_, i| i % 2 == 0) {
                if self.sign().is_minus() {
                    self.mul_sign_mut(Sign::Minus);
                }
            }

            if expon.is_one() {
                return self;
            }

            if self.is_pow() {
                let bb = self.base_ref().base_ref();
                let be = self.base_ref().exponent_ref();

                if bb.is_positive() || be.is_rational_const() && expon.is_rational_const() {
                    let (b_base, b_expon) = self.make_mut_base_expon();
                    let (mut b_base, mut b_expon) = (b_base.take_expr(), b_expon.take_expr());

                    b_expon.mul_assign_with(expon, mode);
                    *self = b_base;
                    self.pow_assign_with(b_expon, mode);
                    return self;
                }
            }
            // if self.is_positive() && self.base_ref().is_pow() {
            // || self.base_ref().is_pow() && self.exponent_ref().is_rational_const() && expon.is_rational_const() {
            //     let (_, b_expon) = self.make_mut_base_expon();
            //     b_expon.mul_assign_with(expon, mode);
            //     self.meta = pow_meta;
            //     return self;
            // }

            if let (Some(br), Some(er)) = (self.as_rational(), expon.as_rational()) {
                let (pow, rem) = pow_rational(br, er);

                *self = Expr::signed_rational(pow.0, pow.1);

                if let Some(rem) = rem {
                    let rem_expr = Expr::signed_rational(rem.0, rem.1);
                    let base_expr = Expr::signed_rational(br.0, br.1);
                    let rem_pow = Expr {
                        typ: ExprTyp::Binary(BinaryFn::Pow, [base_expr, rem_expr].into()),
                        meta: pow_meta,
                    };

                    self.add_assign_with(rem_pow, mode);
                    return self;
                } else {
                    return self;
                }
            }
        }

        match mode.pow {
            PowMode::Expand => {
                if self.is_prod() {
                    let mut prod = Expr::u32(1);
                    for mut op in self.take_nary_operand() {
                        op.pow_assign_with(expon.clone(), mode);
                        prod.mul_assign_with(op, mode);
                    }
                    *self = prod;
                } else if self.is_sum() && expon.is_int32_const_and(|s, u| s.is_plus() && u > 1) {
                    let orig_meta = self.meta;
                    let oprnds = self.nary_operand_mut();

                    let term = oprnds.pop_front().unwrap();
                    let mut rest = self.take_expr();
                    rest.meta = Meta::of_div(orig_meta, term.meta);

                    rest.inline_trivial_compound();

                    let n = expon.as_int().unwrap().1;

                    let mut sum = Expr::u32(0);

                    for k in 0..=n {
                        if k == 0 {
                            let mut a = term.clone();
                            a.pow_assign_with(expon.clone(), mode);
                            sum.add_assign_with(a, mode);
                        } else if k == n {
                            let mut b = rest.clone();
                            b.pow_assign_with(expon.clone(), mode);
                            sum.add_assign_with(b, mode);
                        } else {
                            let c = num::integer::binomial(n, k);
                            let mut a = term.clone();
                            let mut b = rest.clone();

                            a.pow_assign_with(Expr::u32(k), mode);
                            b.pow_assign_with(Expr::u32(n - k), mode);

                            a.mul_assign_with(Expr::u32(c), mode)
                                .mul_assign_with(b, mode);
                            sum.add_assign_with(a, mode);
                        }
                    }

                    *self = sum;
                }
            }
            PowMode::Basic | PowMode::Frozen => {
                *self = Expr {
                    typ: ExprTyp::Binary(BinaryFn::Pow, [self.take_expr(), expon].into()),
                    meta: pow_meta,
                };
            }
        }

        self
    }

    // pub fn reduce_mut(&mut self) -> &mut Self {
    //     self.operands_mut().iter_mut().for_each(|op| op.reduce_mut());

    //     match self.typ {
    //     }
    // }

    /// Order of the expressions in simplified form
    ///
    pub fn simple_order(&self, other: &Expr) -> cmp::Ordering {
        use ordering_abbreviations::*;

        let (lhs, rhs) = (self, other);

        fn cmp_slices<'a>(
            lhs: impl Iterator<Item = &'a Expr>,
            rhs: impl Iterator<Item = &'a Expr>,
        ) -> cmp::Ordering {
            let (mut l_iter, mut r_iter) = (lhs.into_iter(), rhs.into_iter());

            loop {
                match (l_iter.next(), r_iter.next()) {
                    (Some(l), Some(r)) => {
                        if l != r {
                            return l.simple_order(&r);
                        }
                    }
                    (Some(_), None) => return GE,
                    (None, Some(_)) => return LE,
                    (None, None) => return EQ,
                }
            }
        }

        fn expr<'a>(e: &'a Expr) -> impl Iterator<Item = &'a Expr> {
            std::iter::once(e)
        }

        const MINUS_ONE: &'static Expr = &Expr::u32(1);

        fn minus<'a>(e: &'a Expr) -> impl Iterator<Item = &'a Expr> {
            std::iter::once(MINUS_ONE).chain(std::iter::once(e))
        }

        fn oprnds<'a>(e: &'a Expr) -> impl Iterator<Item = &'a Expr> {
            e.operands().iter()
        }

        if lhs == rhs {
            return EQ;
        } else if lhs.is_atom() && rhs.is_atom() {
            // return match (&lhs.typ, &rhs.typ) {
            //     (ExprTyp::Undef, _) => LE,
            //     (_, ExprTyp::Undef) => GE,
            //     (ExprTyp::Var(v1), ExprTyp::Var(v2)) => v1.cmp(v2),
            //     (ExprTyp::Rational(r1), ExprTyp::Rational(r2)) => r1.cmp(r2),

            //     (ExprTyp::Var(_), ExprTyp::Rational(_)) => GE,
            //     (ExprTyp::Rational(_), ExprTyp::Var(_)) => LE,
            //     _ => unreachable!(),
            // };
        } else if lhs.is_equal_typ(rhs) {
            return cmp_slices(oprnds(lhs), oprnds(rhs));
        }

        match (&lhs.typ, &rhs.typ) {
            (ExprTyp::Undef, _) => LE,
            (_, ExprTyp::Undef) => GE,
            (ExprTyp::Var(v1), ExprTyp::Var(v2)) => v1.cmp(v2),
            (ExprTyp::Rational(r1), ExprTyp::Rational(r2)) => r1.cmp(r2),

            (ExprTyp::Var(_), ExprTyp::Rational(_)) => GE,
            (ExprTyp::Rational(_), ExprTyp::Var(_)) => LE,

            (ExprTyp::NAry(NAryFn::Sum, _), _) => cmp_slices(oprnds(lhs), expr(rhs)),
            (_, ExprTyp::NAry(NAryFn::Sum, _)) => cmp_slices(expr(lhs), oprnds(rhs)),

            (ExprTyp::NAry(NAryFn::Prod, _), _) => cmp_slices(oprnds(lhs), expr(rhs)),
            (_, ExprTyp::NAry(NAryFn::Prod, _)) => cmp_slices(expr(lhs), oprnds(rhs)),

            (_, _) if lhs.sign().is_minus() => cmp_slices(minus(lhs), expr(rhs)),
            (_, _) if rhs.sign().is_minus() => cmp_slices(expr(lhs), minus(rhs)),

            (ExprTyp::Binary(BinaryFn::Pow, _), _) | (_, ExprTyp::Binary(BinaryFn::Pow, _)) => {
                let (b1, e1) = lhs.base_expon_ref();
                let (b2, e2) = rhs.base_expon_ref();

                if b1 != b2 {
                    cmp_slices(expr(b1), expr(b2))
                } else {
                    cmp_slices(expr(e1), expr(e2))
                }
            }

            (ExprTyp::Unary(fn1, _), ExprTyp::Unary(fn2, _)) => {
                if fn1 != fn2 {
                    fn1.cmp(fn2)
                } else {
                    cmp_slices(oprnds(lhs), oprnds(rhs))
                }
            }
            (ExprTyp::Unary(_, _), _) => cmp_slices(oprnds(lhs), expr(rhs)),
            (_, ExprTyp::Unary(_, _)) => cmp_slices(expr(lhs), oprnds(rhs)),
        }
    }

    pub fn inline_trivial_compound(&mut self) -> &mut Expr {
        if self.is_prod() || self.is_sum() {
            if self.n_operands() == 1 {
                let meta = self.meta;
                *self = self.take_nary_operand().pop_front().unwrap();
                self.meta = meta;
            } else if self.n_operands() == 0 {
                if self.is_prod() {
                    *self = Expr::u32(1);
                } else if self.is_sum() {
                    *self = Expr::u32(0);
                }
            }
        }
        self
    }

    pub fn mul_sign_mut(&mut self, s: Sign) -> &mut Expr {
        if s.is_minus() {
            self.meta = Meta::of_neg(self.meta)
        }
        self
    }

    pub fn mul_sign(mut self, s: Sign) -> Expr {
        self.mul_sign_mut(s);
        self
    }

    pub fn split_sign(&mut self) -> (Sign, &mut Expr) {
        if self.sign().is_minus() {
            self.meta = Meta::of_neg(self.meta);
            (Sign::Minus, self)
        } else {
            (Sign::Plus, self)
        }
    }

    fn make_mut_coeff_term<'a>(&'a mut self) -> (&'a mut Expr, &'a mut [Expr]) {
        let meta = self.meta;
        if self.is_prod() {
            let oprnds = self.nary_operand_mut();
            if !oprnds[0].is_rational_const() {
                oprnds.push_front(Expr::u32(1));
            }
        } else {
            *self = Expr {
                typ: ExprTyp::NAry(
                    NAryFn::Prod,
                    Rc::new([Expr::u32(1), self.take_expr()].into()),
                ),
                meta,
            };
        }

        let oprnds = self.nary_operand_mut().as_mut_slice();
        let (coeff, term) = oprnds.split_first_mut().unwrap();
        (coeff, term)
    }

    pub fn make_mut_base_expon(&mut self) -> (&mut Expr, &mut Expr) {
        let meta = self.meta;
        if !self.is_pow() {
            *self = Expr {
                typ: ExprTyp::Binary(BinaryFn::Pow, Rc::new([self.take_expr(), Expr::u32(1)])),
                meta,
            }
        }

        let [base, expon] = self.binary_operand_mut();
        (base, expon)
    }

    pub fn take_expr(&mut self) -> Expr {
        std::mem::replace(self, Expr::placeholder())
    }

    pub fn take_exponent(&mut self) -> Expr {
        std::mem::replace(&mut self.binary_operand_mut()[1], Expr::placeholder())
    }

    pub fn take_binary_operand(&mut self) -> [Expr; 2] {
        std::mem::replace(
            self.binary_operand_mut(),
            [Expr::placeholder(), Expr::placeholder()],
        )
    }

    pub fn take_nary_operand(&mut self) -> FlatDeque<Expr> {
        std::mem::replace(self.nary_operand_mut(), FlatDeque::new())
    }
}

fn merge_operands(
    p: &[Expr],
    q: &[Expr],
    simplify_fn: impl Fn(&[Expr]) -> FlatDeque<Expr>,
) -> FlatDeque<Expr> {
    if p.is_empty() {
        q.into_iter().cloned().collect()
    } else if q.is_empty() {
        p.into_iter().cloned().collect()
    } else {
        let p_0 = &p[0];
        let p_rest = &p[1..];
        let q_0 = &q[1];
        let q_rest = &q[1..];

        let mut h = simplify_fn(&[p_0.clone(), q_0.clone()]);

        if h.is_empty() {
            merge_operands(p_rest, q_rest, simplify_fn)
        } else if h.len() == 1 {
            let mut res = merge_operands(p_rest, q_rest, simplify_fn);
            res.push_front(h.pop_front().unwrap());
            res
        } else if p_0 == &h[0] && q_0 == &h[1] {
            let mut res = merge_operands(p_rest, q, simplify_fn);
            res.push_front(h.pop_front().unwrap());
            res
        } else if q_0 == &h[0] && p_0 == &h[1] {
            let mut res = merge_operands(p, q_rest, simplify_fn);
            res.push_front(h.pop_front().unwrap());
            res
        } else {
            panic!("illegal reduction: {q:?} + {p:?} -> h");
        }
    }
}

fn simplify_prod_operands(args: &[Expr]) -> FlatDeque<Expr> {
    if args.is_empty() {
        [Expr::u32(1)].into()
    } else if args.len() == 1 {
        [args[0].clone()].into()
    } else if args.len() == 2 {
        let lhs = &args[0];
        let rhs = &args[1];

        if lhs.is_prod() && rhs.is_prod() {
            merge_operands(lhs.operands(), rhs.operands(), simplify_prod_operands)
        } else if lhs.is_prod() {
            merge_operands(
                lhs.operands(),
                std::slice::from_ref(rhs),
                simplify_prod_operands,
            )
        } else if rhs.is_prod() {
            merge_operands(
                std::slice::from_ref(lhs),
                rhs.operands(),
                simplify_prod_operands,
            )
        } else if lhs.is_rational_const() && rhs.is_rational_const() {
            let (l, r) = (lhs.as_rational().unwrap(), rhs.as_rational().unwrap());
            let (sign, prod) = mul_signed_ratio(r, l);
            [Expr::signed_rational(sign, prod)].into()
        } else if lhs.is_rational_const() || rhs.is_rational_const() {
            let (mut lhs, mut rhs) = (lhs.clone(), rhs.clone());
            if lhs.simple_order(&rhs).is_ge() {
                std::mem::swap(&mut lhs, &mut rhs);
            }

            [lhs, rhs].into()
        } else if lhs.base_ref() == rhs.base_ref() {
            // let coeff = add_signed_ratio(lhs.exponent_ref(), rhs.rational_coeff());
            let expon = lhs
                .exponent_ref()
                .clone()
                .add_with(rhs.exponent_ref().clone(), EvalMode::basic());
            let base = lhs.base_ref().clone();

            let meta = Meta::of_mul(lhs.meta, rhs.meta);

            let pow = Expr {
                typ: ExprTyp::Binary(BinaryFn::Pow, Rc::new([base, expon])),
                meta,
            };

            [pow].into()
        } else {
            let (mut lhs, mut rhs) = (lhs.clone(), rhs.clone());
            if lhs.simple_order(&rhs).is_ge() {
                std::mem::swap(&mut lhs, &mut rhs);
            }
            [lhs, rhs].into()
        }
    } else {
        let lhs = &args[0];
        let rhs = simplify_prod_operands(&args[1..]);

        if lhs.is_prod() {
            merge_operands(lhs.operands(), rhs.as_slice(), simplify_prod_operands)
        } else {
            merge_operands(
                std::slice::from_ref(&lhs),
                rhs.as_slice(),
                simplify_prod_operands,
            )
        }
    }
}

fn simplify_sum_operands(args: &[Expr]) -> FlatDeque<Expr> {
    if args.is_empty() {
        [Expr::u32(0)].into()
    } else if args.len() == 1 {
        [args[0].clone()].into()
    } else if args.len() == 2 {
        let lhs = &args[0];
        let rhs = &args[1];

        if lhs.is_sum() && rhs.is_sum() {
            merge_operands(lhs.operands(), rhs.operands(), simplify_sum_operands)
        } else if lhs.is_sum() {
            merge_operands(
                lhs.operands(),
                std::slice::from_ref(rhs),
                simplify_sum_operands,
            )
        } else if rhs.is_sum() {
            merge_operands(
                std::slice::from_ref(lhs),
                rhs.operands(),
                simplify_sum_operands,
            )
        } else if lhs.is_rational_const() && rhs.is_rational_const() {
            let (l, r) = (lhs.as_rational().unwrap(), rhs.as_rational().unwrap());
            let (sign, sum) = add_signed_ratio(r, l);
            [Expr::signed_rational(sign, sum)].into()
        } else if lhs.is_rational_const() || rhs.is_rational_const() {
            let (mut lhs, mut rhs) = (lhs.clone(), rhs.clone());
            if lhs.simple_order(&rhs).is_ge() {
                std::mem::swap(&mut lhs, &mut rhs);
            }
            [lhs, rhs].into()
        } else if lhs.term_ref() == rhs.term_ref() {
            let coeff = add_signed_ratio(lhs.rational_coeff(), rhs.rational_coeff());
            let mut ops: FlatDeque<_> = lhs.term_ref().into_iter().cloned().collect();
            ops.push_front(Expr::signed_rational(coeff.0, coeff.1));

            let meta = Meta::of_add(lhs.meta, rhs.meta);

            let prod = Expr {
                typ: ExprTyp::NAry(NAryFn::Prod, Rc::new(ops)),
                meta,
            };

            [prod].into()
        } else {
            let (mut lhs, mut rhs) = (lhs.clone(), rhs.clone());
            if lhs.simple_order(&rhs).is_ge() {
                std::mem::swap(&mut lhs, &mut rhs);
            }
            [lhs, rhs].into()
        }
    } else {
        let lhs = &args[0];
        let rhs = simplify_sum_operands(&args[1..]);

        if lhs.is_sum() {
            merge_operands(lhs.operands(), rhs.as_slice(), simplify_sum_operands)
        } else {
            merge_operands(
                std::slice::from_ref(&lhs),
                rhs.as_slice(),
                simplify_sum_operands,
            )
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let typ_str = match &self.typ {
            ExprTyp::Undef => "\u{2205}".to_string(),
            ExprTyp::Rational(r) => r.to_string(),
            ExprTyp::Var(symbol) => symbol.0.to_string(),
            ExprTyp::Unary(unary_fn, oprnd) => format!("{}({oprnd:?})", unary_fn.name()),
            ExprTyp::Binary(BinaryFn::Pow, _) => {
                let (base, expon) = self.base_expon_ref();
                let mut res = String::new();

                if base.is_atom() {
                    res += &format!("{base:?}^");
                } else {
                    res += &format!("({base:?})^");
                }
                if expon.is_atom() {
                    res += &format!("{expon:?}");
                } else {
                    res += &format!("({expon:?})");
                }
                res
            }
            ExprTyp::NAry(nary_fn, oprnds) => {
                let symbol = match nary_fn {
                    NAryFn::Sum => " + ",
                    NAryFn::Prod => " * ",
                };

                let expr = oprnds.iter().map(|o| format!("{o:?}")).join(symbol);
                expr
            }
        };

        let sign = self.sign();

        // write!(f, "{}{typ_str} [{:?}]", sign.fmt_prefix(), self.meta)
        write!(f, "{}{typ_str}", sign.fmt_prefix())
    }
}

impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(mut self, mut rhs: Self) -> Self::Output {
        rhs *= Expr::i32(-1);
        self += rhs;
        self
    }
}

impl ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Self) -> Self::Output {
        self.add_with(rhs, noctua_global_config().default_eval_mode)
    }
}

impl ops::AddAssign for Expr {
    fn add_assign(&mut self, rhs: Self) {
        self.add_assign_with(rhs, noctua_global_config().default_eval_mode);
    }
}

impl ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Self) -> Self::Output {
        self.mul_with(rhs, noctua_global_config().default_eval_mode)
    }
}

impl ops::MulAssign for Expr {
    fn mul_assign(&mut self, rhs: Self) {
        self.mul_assign_with(rhs, noctua_global_config().default_eval_mode);
    }
}

impl ops::Div for Expr {
    type Output = Expr;
    fn div(mut self, rhs: Self) -> Self::Output {
        self *= rhs.pow(Expr::i32(-1));
        self
    }
}

impl ops::DivAssign for Expr {
    fn div_assign(&mut self, rhs: Self) {
        *self = self.take_expr() / rhs;
    }
}

impl ops::Neg for Expr {
    type Output = Expr;
    fn neg(mut self) -> Self::Output {
        self.mul_sign_mut(Sign::Minus);
        self
    }
}

#[cfg(test)]
mod test {
    use crate::Expr;
    use crate::noctua as n;

    #[test]
    fn pow_with() {
        assert_eq!(n!(0 ^ 0), n!(undef));
        assert_eq!(n!(3 ^ 2), n!(9));
        assert_eq!(n!(3 ^ (-2)), n!(1 / 9));
        // x could be 0
        assert_eq!(n!(x ^ 0), n!(x ^ 0));
    }

    #[test]
    fn simple_order() {
        let order = [
            (n!(1), n!(2)),
            (n!(x), n!(x ^ 2)),
            (n!(a * x ^ 2), n!(x ^ 3)),
            (n!(u), n!(v ^ 1)),
            (n!((1 + x) ^ 2), n!((1 + x) ^ 3)),
            (n!((1 + x) ^ 3), n!((1 + y) ^ 2)),
            (n!(a + b), n!(a + c)),
            (n!(1 + x), n!(y)),
            (n!(a * x ^ 2), n!(x ^ 3)),
        ];

        for (l, r) in order {
            assert!(l.simple_order(&r).is_lt(), "{l:?} vs {r:?}");
        }
    }

    #[test]
    fn reduce() {
        let checks = vec![
            (n!(2 * x), n!(2 * x)),
            (n!(1 + 2), n!(3)),
            (n!(a + undef), n!(undef)),
            (n!(a + (b + c)), n!(a + (b + c))),
            (n!(0 - 2 * b), n!((2 - 4) * b)),
            (n!(a + 0), n!(a)),
            (n!(0 + a), n!(a)),
            (n!(1 + 2), n!(3)),
            (n!(x + 0), n!(x)),
            (n!(0 + x), n!(x)),
            (n!(0 - x), n!((4 - 5) * x)),
            (n!(x - 0), n!(x)),
            (n!(3 - 2), n!(1)),
            (n!(x * 0), n!(0)),
            (n!(0 * x), n!(0)),
            (n!(x * 1), n!(x)),
            (n!(1 * x), n!(x)),
            (n!(0 ^ 0), n!(undef)),
            (n!(0 ^ 1), n!(0)),
            (n!(0 ^ 314), n!(0)),
            (n!(1 ^ 0), n!(1)),
            (n!(314 ^ 0), n!(1)),
            (n!(314 ^ 1), n!(314)),
            (n!(x ^ 1), n!(x)),
            (n!(1 ^ x), n!(1 ^ x)),
            (n!(1 ^ 314), n!(1)),
            (n!(3 ^ 3), n!(27)),
            (n!(a - b), n!(a + ((2 - 3) * b))),
            (n!(a / b), n!(a * b ^ (2 - 3))),
            (n!(((x ^ (1 / 2)) ^ (1 / 2)) ^ 8), n!(x ^ 2)),
            (n!(x + x), n!(2 * x)),
            (n!(2 * x + y + x), n!(3 * x + y)),
            // (n!(sin(0)), n!(0)),
            // (n!(sin(-x)), n!(-1 * sin(x))),
            // (n!(cos(-x)), n!(cos(x))),
            (n!(x * y / (y * x)), n!(1)),
            // (Expr::ln(Expr::n()), n!(1)),
        ];
        for (i, (calc, res)) in checks.iter().enumerate() {
            assert_eq!(calc, res, "{i}: {calc:?} != {res:?}");
        }
    }

    #[test]
    fn sort_args() {
        let checks = vec![
            n!(a + b),
            n!(a + b * c),
            n!(sin(x) * cos(x)),
            n!(3 + a * x ^ 2 + b * x + c),
        ];

        for c in checks {
            let mut args = c.operands().to_vec();
            args.sort_by(Expr::simple_order);
            assert_eq!(&args, c.operands())
        }
    }
}
