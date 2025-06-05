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

impl Meta {

    #[inline]
    pub const fn of_u32(u: u32) -> Self {
        let mut meta = Meta::IS_INTEGER;
        if u == 0 {
            return Meta::IS_ZERO
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
    pub const fn of_add(l: Meta, r: Meta) -> Meta {
        use Meta as M;
        l.dbg_check_valid();
        r.dbg_check_valid();

        let mut res = Meta::empty();

        bitop!(res |= M::HAS_UNDEF.if_either(l, r));
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

        bitop!(res |= M::IS_NUMERIC.if_both(l, r));
        bitop!(res |= M::IS_INTEGER.if_both(l, r));
        bitop!(res |= M::IS_REAL.if_both(l, r));
        bitop!(res |= M::IS_ODD.if_both(l, r));
        bitop!(res |= M::IS_NON_ZERO.if_both(l, r));

        bitop!(res |= M::HAS_UNDEF.if_either(l, r));
        bitop!(res |= M::IS_ZERO.if_either(l, r));
        bitop!(res |= M::IS_EVEN.if_either(l, r));

        if M::IS_POSITIVE.in_both(l, r) || M::IS_NEGATIVE.in_both(l, r) {
            bitop!(res |= M::IS_POSITIVE);
        }

        bitop!(res |= M::IS_NEGATIVE.if_both_exclusive(l, r));

        res.dbg_check_valid();
        res
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
            return M::of_u32(1)
        }

        if b.has(M::IS_ZERO) && e.has(M::IS_POSITIVE) {
            return M::of_u32(0)
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
        if l.has(self) {
            self
        } else {
            Meta::empty()
        }
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
    Sin, Cos, Tan,
    ASin, ACos, ATan,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryFn {
    Pow
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NAryFn {
    Sum, Prod,
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
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PowMode {
    Frozen,
    #[default]
    Basic,
    Expand,
}


#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvalMode {
    add: AddMode,
    mul: MulMode,
    pow: PowMode,
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
            typ: ExprTyp::Undef,
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
            Expr { typ: ExprTyp::Unary(_, oprnd), .. } => std::slice::from_mut(Rc::make_mut(oprnd)),
            Expr { typ: ExprTyp::Binary(_, oprnds), .. } => return Rc::make_mut(oprnds).as_mut_slice(),
            Expr { typ: ExprTyp::NAry(_, oprnds), .. } => return Rc::make_mut(oprnds).as_mut_slice(),
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

    const _ONE_EXPONENT_REF: &'static Expr = &Expr::u32(1);

    /// if `self` is [`Expr::Pow`] return the base otherwise return `self`
    #[inline]
    pub fn base_ref(&self) -> &Expr {
        self.base_expon_ref().0
    }

    /// if `self` is [`Expr::Pow`] return the exponent otherwise return 1
    #[inline]
    pub fn exponent_ref(&self) -> &Expr {
        self.base_expon_ref().0
    }

    /// if `self` is [`BinaryFn::Pow`] return (base, exponent) otherwise (`self`, 1)
    #[inline]
    pub fn base_expon_ref(&self) -> (&Expr, &Expr) {
        match &self.typ {
            ExprTyp::Binary(BinaryFn::Pow, base_expon) => {
                let [base, expon] = base_expon.as_ref();
                (base, expon)
            },
            _ => (self, Expr::_ONE_EXPONENT_REF)
        }
    }


    #[inline]
    pub fn is_one(&self) -> bool {
        self.is_int32_const_and(|s, u| s.is_plus() && u == 1)
    }
    
    #[inline]
    pub fn is_int32_const_and(&self, f: impl FnOnce(Sign, u32) -> bool) -> bool {
        self.is_rational_const_and(|s, r| if r.is_integer() {
            f(s, *r.numer())
        } else {
            false
        })
    }

    #[inline]
    pub fn is_rational_const_and(&self, f: impl FnOnce(Sign, Ratio<u32>) -> bool) -> bool {
        let sign = self.sign();
        match &self.typ {
            ExprTyp::Rational(ratio) => f(sign, *ratio),
            _ => false,
        }
    }

    #[inline]
    pub fn is_atom(&self) -> bool {
        match &self.typ {
            ExprTyp::Undef | ExprTyp::Rational(_) | ExprTyp::Var(_) => true,
            _ => false
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
    //////    Modifiers
    //////////////////////////////////////////////////////

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
    
    pub fn make_mut_pow(&mut self) -> (&mut Expr, &mut Expr) {
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

    pub fn take_unary_operand(&mut self) -> Expr {
        self.unary_operand_mut().take_expr()
    }

    pub fn take_binary_operand(&mut self) -> [Expr; 2] {
        std::mem::replace(self.binary_operand_mut(), [Expr::placeholder(), Expr::placeholder()])
    }

    pub fn take_nary_operand(&mut self) -> FlatDeque<Expr> {
        std::mem::replace(self.nary_operand_mut(), FlatDeque::new())
    }
}


#[cfg(test)]
mod test {
    use crate::noctua as n;
    use crate::Expr;

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
            (n!((x ^ (1 / 2) ^ (1 / 2)) ^ 8), n!(x ^ 2)),
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
            n!(b * c + a),
            n!(sin(x) * cos(x)),
            n!(a * x ^ 2 + b * x + c + 3),
        ];

        for c in checks {
            let mut args = c.operands().to_vec();
            args.sort_by(Expr::simple_order);
            assert_eq!(&args, c.operands())
        }
    }
}
