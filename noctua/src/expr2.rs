use std::{cmp, fmt, ops, rc::Rc};

use itertools::Itertools;
use crate::log_fn;
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

#[derive(PartialEq)]
pub enum MutView<'a> {
    Atom(&'a mut Atom),
    Expr(&'a mut Expr),
}

impl MutView<'_> {
    fn as_view(&self) -> View<'_> {
        match self {
            MutView::Atom(a) => View::Atom(a),
            MutView::Expr(e) => View::Expr(e),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum View<'a> {
    Atom(&'a Atom),
    Expr(&'a Expr),
}

impl View<'_> {
    /// Order of the expressions in simplified form
    ///
    pub fn simplified_ordering(&self, other: &Self) -> cmp::Ordering {
        use ordering_abbreviations::*;

        if self == other {
            return EQ;
        }

        match (self, other) {
            (View::Atom(a1), View::Atom(a2)) => a1.simplified_ordering(a2),
            (View::Expr(e1), View::Expr(e2)) => e1.simplified_ordering(e2),
            (lhs, rhs) => {
                let (l_ops, r_ops) = (lhs.operands(), rhs.operands());

                if let Some((l, r)) = l_ops.iter().zip(r_ops.iter()).find(|(l, r)| l != r) {
                    l.simplified_ordering(r)
                } else {
                    l_ops.len().cmp(&r_ops.len())
                }
            }
        }
    }

    #[inline]
    pub fn operands(&self) -> &[Atom] {
        match self {
            View::Atom(a) => a.operands(),
            View::Expr(e) => e.operands(),
        }
    }
}

/// Represents an atomic unit: simple values or sub-expressions
///
/// - Inline variants like undef or integers
/// - Compound expressions used as part of an [`Expr`]
#[derive(Clone, PartialEq)]
pub enum Atom {
    Undef,
    U32(u32),
    Rational(Ratio<u32>),
    Var(Rc<str>),

    // NOTE: if the Atom is part of an `Expr` this should only be used if the atom is part
    // of a compound expression. Otherwise promote the `Expr::Atom(Atom::Expr(..))` to `Expr`.
    // In other words when encountering `Expr::Atom(atom)` atom must not be an `Atom::Expr`
    Expr(Rc<Expr>),
}

impl Atom {
    //////////////////////////////////////////////////////
    //////    Constructors
    //////////////////////////////////////////////////////

    #[inline]
    pub fn expr(e: Expr) -> Self {
        match e.typ {
            ExprTyp::Atom(atom) => atom,
            _ => Atom::Expr(e.into()),
        }
    }

    // Note: this function should be used so we can potentially log this call.
    // should probably not be used inside a remove_*** function
    #[inline]
    pub const fn undef() -> Self {
        Atom::Undef
    }

    #[inline]
    pub fn real(r: Real) -> Atom {
        match r.typ {
            crate::real::RealTyp::Zero => Atom::U32(0),
            crate::real::RealTyp::U32(u) => {
                if r.is_positive() {
                    Atom::U32(u)
                } else {
                    Atom::expr(Expr::minus(Expr::u32(u)))
                }
            }
            crate::real::RealTyp::Ratio(r) => Atom::Rational(r),
        }
    }

    //////////////////////////////////////////////////////
    //////    Modifiers
    //////////////////////////////////////////////////////

    #[inline]
    pub fn expand_mut(&mut self) {
        match self {
            Atom::Undef | Atom::U32(_) | Atom::Var(_) | Atom::Rational(_) => (),
            Atom::Expr(expr) => {
                Rc::make_mut(expr).expand_mut();
            }
        }
    }

    /// we want to perform the following modification in-place:
    ///
    /// `Expr::Sum([Atom("x"), ...])` -> `Expr::Sum([Atom(Expr::Pow(Atom("x")), Atom(2)), ...])`
    ///
    /// While variables and integers are represented by [`Atom`], sums and powers must be [`Expr`].
    /// Notice how because the operands of [`Expr::Sum`] are [`Atom`]s we must first wrap
    /// [`Expr::Pow`] in an [`Atom`].
    ///
    /// This function returns a mutable expression reference by wrapping replacing `self`
    /// with itself wrapped in [`Expr::Atom`] and [`Atom::Expr`]
    ///
    /// `*self = Atom::Expr(Expr::Atom(self))`
    ///
    /// we then can return`&mut Expr::Atom(...)`
    ///
    /// The caller must ensure that the returned `&mut Expr` does not remain a [`Expr::Atom`]
    /// otherwise we would have the invalid state: `Expr::Sum([Atom::Expr(Expr::Atom(Atom(1))), ...])`,
    /// instead of `Expr::Sum([Atom(1), ...])`
    #[inline]
    pub(crate) fn as_mut_expr_with<'a, T>(&'a mut self, f: impl Fn(&'a mut Expr) -> T) -> T {
        if let Atom::Expr(rc) = self {
            let mut_expr = Rc::make_mut(rc);
            // assert!(!matches!(mut_expr, Expr::_Atom_(_)));
            return f(mut_expr);
        }
        let orig = std::mem::replace(self, Atom::Undef);
        *self = Atom::Expr(Rc::new(Expr::atom(orig)));

        let Atom::Expr(expr) = self else {
            unreachable!()
        };

        let mut_expr: &mut Expr = Rc::get_mut(expr).expect("The Rc above has not been cloned");
        f(mut_expr)
    }

    #[log_fn]
    #[inline]
    pub(crate) fn as_mut_expr(&mut self) -> &mut Expr {
        self.as_mut_expr_with(|e| e)
    }

    #[inline]
    pub(crate) fn cleanup_indirection(&mut self) -> &mut Atom {
        if let Atom::Expr(e) = self {
            if e.is_atom() {
                let expr = &mut Rc::make_mut(e).operands_mut()[0];
                let atom = std::mem::replace(expr, Atom::Undef);
                *self = atom;
            }
            // if let Expr::atom(atom) = Rc::make_mut(e) {
            //     let atom = std::mem::replace(atom, Atom::Undef);
            //     *self = atom;
            // }
        }
        self
    }

    #[inline]
    pub fn cancle_signs(&mut self) -> Sign {
        match self {
            Atom::Expr(e) => Rc::make_mut(e).cancle_signs(),
            _ => Sign::Plus,
        }
    }

    //////////////////////////////////////////////////////
    //////    Accessors
    //////////////////////////////////////////////////////

    #[inline]
    pub fn view(&self) -> View<'_> {
        match self {
            Atom::Expr(e) => e.view(),
            _ => View::Atom(self),
        }
    }

    #[inline]
    pub fn base_ref(&self) -> View<'_> {
        match self {
            Atom::Expr(e) => e.base_ref(),
            _ => View::Atom(self),
        }
    }
    #[inline]
    pub fn exponent_ref(&self) -> &Atom {
        match self {
            Atom::Expr(e) => e.exponent_ref(),
            _ => &Atom::U32(1),
        }
    }

    #[inline]
    pub fn operands(&self) -> &[Atom] {
        match self {
            Atom::Expr(rc) => rc.operands(),
            _ => std::slice::from_ref(self),
        }
    }

    #[inline]
    pub fn as_real(&self) -> Option<Real> {
        match self {
            Atom::U32(u) => Some(Real::u32(*u)),
            Atom::Rational(r) => Some(Real::rational(*r)),
            _ => None,
        }
    }

    #[inline]
    pub fn meta(&self) -> Meta {
        match self {
            Atom::Expr(e) => e.meta(),
            _ => Meta::SIMPLE_FORM | Meta::EXPAND_FORM,
        }
    }

    #[inline]
    pub fn is_u32_and(&self, f: impl FnOnce(u32) -> bool) -> bool {
        match self {
            Atom::U32(u) => f(*u),
            _ => false,
        }
    }
    #[inline]
    pub fn is_var_and(&self, f: impl FnOnce(&str) -> bool) -> bool {
        match self {
            Atom::Var(v) => f(&*v),
            _ => false,
        }
    }
    #[inline]
    pub fn is_expr_and(&self, f: impl FnOnce(&Expr) -> bool) -> bool {
        match self {
            Atom::Expr(e) => f(&*e),
            _ => false,
        }
    }

    #[inline]
    pub fn is_u32(&self) -> bool {
        self.is_u32_and(|_| true)
    }
    #[inline]
    pub fn is_var(&self) -> bool {
        self.is_var_and(|_| true)
    }
    #[inline]
    pub fn is_expr(&self) -> bool {
        self.is_expr_and(|_| true)
    }

    /// Order of the expressions in simplified form
    ///
    #[log_fn]
    pub fn simplified_ordering(&self, other: &Atom) -> cmp::Ordering {
        use ordering_abbreviations::*;

        if self == other {
            return EQ;
        }

        match (self, other) {
            (Atom::Undef, _) => return GE,
            (_, Atom::Undef) => return LE,

            (Atom::U32(u1), Atom::U32(u2)) => return u1.cmp(u2),
            (Atom::Var(v1), Atom::Var(v2)) => return v1.cmp(v2),

            (Atom::U32(_), Atom::Var(_)) => return LE,
            (Atom::Var(_), Atom::U32(_)) => return GE,

            _ => (),
        }

        let (mut l_iter, mut r_iter) = (self.operands().into_iter(), other.operands().into_iter());

        loop {
            match (l_iter.next(), r_iter.next()) {
                (Some(l), Some(r)) => {
                    if l != r {
                        return l.simplified_ordering(&r);
                    }
                }
                (Some(_), None) => return GE,
                (None, Some(_)) => return LE,
                (None, None) => return EQ,
            }
        }

        // while let (Some(l), Some(r)) = (l_iter.next(), r_iter.next()) {
        //     if l != r {
        //         return l.simplified_ordering(&r);
        //     }
        // }

        // match (l_iter.next(), r_iter.next()) {
        //     (Some(_), None) => GE,
        //     (None, Some(_)) => LE,
        //     _ => EQ,
        // }
    }
}

#[derive(Clone, PartialEq)]
pub enum ExprTyp {
    Atom(Atom),

    /// Is used to represent negative values
    ///
    /// Will be interpreted as the expression -1 * [`Atom`]
    /// For cohesion negative integers are represented as `Expr::Minus(Atom::U32(0))`
    Minus(Atom),

    Sum(FlatDeque<Atom>),
    Prod(FlatDeque<Atom>),
    Pow([Atom; 2]),
}

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq)]
    pub struct Meta: u32 {
        const NONE          = 0b000;
        const FROZEN_FORM   = 0b001;
        const SIMPLE_FORM   = 0b010;
        const EXPAND_FORM   = 0b100;
    }
}

impl fmt::Debug for Meta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = vec![];
        if self.contains(Meta::NONE) { res.push("_") }
        if self.contains(Meta::FROZEN_FORM) { res.push("F") }
        if self.contains(Meta::SIMPLE_FORM) { res.push("S") }
        if self.contains(Meta::EXPAND_FORM) { res.push("E") }
        write!(f, "{}", res.join("|"))
    }
}

impl Meta {
    fn remove_all(self, flags: impl AsRef<[Meta]>) -> Meta {
        let mut res = self;
        for f in flags.as_ref() {
            res = res.intersection(f.complement())
        }
        res
    }

    fn is_any(self, flags: impl AsRef<[Meta]>) -> bool {
        let mut res = false;
        for f in flags.as_ref() {
            res |= self == *f;
        }
        res
    }
}

/// Expression composed of [`Atom`] units and operations.
///
/// This design allows to store simple expressions with very little overhead.
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

impl Expr {
    //////////////////////////////////////////////////////
    //////    Constructors
    //////////////////////////////////////////////////////

    #[inline]
    pub fn atom(a: Atom) -> Self {
        match a {
            Atom::Expr(e) => Rc::unwrap_or_clone(e),
            a => Expr::const_atom(a),
        }
    }

    #[inline]
    pub const fn const_atom(a: Atom) -> Self {
        match a {
            Atom::Expr(_) => panic!("only simple atoms allowed at compile time"),
            _ => Expr {
                typ: ExprTyp::Atom(a),
                meta: Meta::SIMPLE_FORM.union(Meta::EXPAND_FORM),
            },
        }
    }

    #[inline]
    pub const fn u32(u: u32) -> Self {
        Self::const_atom(Atom::U32(u))
    }

    #[inline]
    pub fn var(v: impl AsRef<str>) -> Self {
        Self::const_atom(Atom::Var(v.as_ref().into()))
    }

    #[inline]
    pub const fn undef() -> Self {
        Expr::const_atom(Atom::undef())
    }

    #[inline]
    pub const fn i32(i: i32) -> Self {
        let u = i.unsigned_abs();
        if i > 0 {
            Expr::u32(u)
        } else {
            Expr {
                typ: ExprTyp::Minus(Atom::U32(u)),
                meta: Meta::SIMPLE_FORM.union(Meta::EXPAND_FORM),
            }
        }
    }

    #[inline]
    pub fn minus(mut e: Expr) -> Self {
        (&mut e).minus_mut();
        e
    }

    #[inline]
    pub fn real(r: Real) -> Self {
        let e = match r.typ {
            crate::real::RealTyp::Zero => Expr::u32(0),
            crate::real::RealTyp::U32(u) => Expr::u32(u),
            crate::real::RealTyp::Ratio(r) => Expr::atom(Atom::Rational(r)),
        };

        if r.is_negative() {
            Expr::minus(e)
        } else {
            e
        }
    }

    //////////////////////////////////////////////////////
    //////    Modifiers
    //////////////////////////////////////////////////////

    #[inline]
    pub fn minus_mut(self: &mut Expr) -> &mut Self {
        self.apply_sign(Sign::Minus)
    }

    #[log_fn]
    pub fn add_with(&mut self, mut rhs: Expr, strat: AddStrategy) -> &mut Expr {
        if matches!(strat, AddStrategy::Frozen) {
            *self = Expr {
                typ: ExprTyp::Sum([Atom::expr(self.remove_expr()), Atom::expr(rhs)].into()),
                meta: Meta::FROZEN_FORM,
            };
            return self;
        }

        self.cleanup_mut();
        rhs.cleanup_mut();

        let (l_meta, r_meta) = (self.meta, rhs.meta);

        const UNDEF: ExprTyp = ExprTyp::Atom(Atom::Undef);
        const ZERO: ExprTyp = ExprTyp::Atom(Atom::U32(0));

        match (self.typ(), rhs.typ()) {
            (&UNDEF, _) | (_, &UNDEF) => {
                *self = Expr::undef();
                return self;
            }
            (&ZERO, _) => {
                *self = rhs;
                return self;
            }
            (_, &ZERO) => {
                return self;
            }
            (_, _) => {
                if let (Some(lhs), Some(rhs)) = (self.as_real(), rhs.as_real()) {
                    *self = Expr::real(lhs + rhs);
                    return self;
                }
            }
        };

        match strat {
            AddStrategy::Simple => match (&mut self.typ, &mut rhs.typ) {
                (ExprTyp::Sum(s1), ExprTyp::Sum(s2)) => {
                    s1.extend(s2.drain(..));
                }
                (ExprTyp::Sum(s), _) => {
                    s.push_back(Atom::expr(rhs));
                }
                (_, ExprTyp::Sum(_)) => {
                    let mut sum = rhs.remove_nary_operands().unwrap();
                    sum.push_front(Atom::expr(self.remove_expr()));

                    *self = Expr {
                        typ: ExprTyp::Sum(sum),
                        meta: l_meta.union(r_meta),
                    };
                }
                _ => {
                    let args = [Atom::expr(self.remove_expr()), Atom::expr(rhs)];
                    *self = Expr {
                        typ: ExprTyp::Sum(args.into()),
                        meta: l_meta.union(r_meta),
                    };
                }
            },
            AddStrategy::Coeff => {
                todo!()
            }
            AddStrategy::Frozen => unreachable!(),
        }
        self.cleanup_mut()
    }

    pub fn apply_sign(&mut self, s: Sign) -> &mut Expr {
        let meta = self.meta;
        match self.cancle_signs() * s {
            Sign::Minus => {
                *self = Expr {
                    typ: ExprTyp::Minus(Atom::expr(self.remove_expr())),
                    meta,
                }
            }
            Sign::Plus => (),
        }
        self
    }

    #[log_fn]
    pub fn mul_with(&mut self, mut rhs: Expr, strat: MulStrategy) -> &mut Expr {
        if matches!(strat, MulStrategy::Frozen) {
            *self = Expr {
                typ: ExprTyp::Prod([Atom::expr(self.remove_expr()), Atom::expr(rhs)].into()),
                meta: Meta::FROZEN_FORM,
            };
            return self;
        }

        self.cleanup_mut();
        rhs.cleanup_mut();

        let (l_meta, r_meta) = (self.meta(), rhs.meta());

        const UNDEF: ExprTyp = ExprTyp::Atom(Atom::Undef);
        const ZERO: ExprTyp = ExprTyp::Atom(Atom::U32(0));
        const ONE: ExprTyp = ExprTyp::Atom(Atom::U32(1));

        // remove potential signs and wrap result with the resulting sign
        let sign = self.cancle_signs() * rhs.cancle_signs();

        match (self.typ(), rhs.typ()) {
            (&UNDEF, _) | (_, &UNDEF) => {
                *self = Expr::undef();
                return self;
            }
            (&ZERO, _) | (_, &ZERO) => {
                *self = Expr::u32(0);
                return self;
            }
            (&ONE, _) => {
                *self = rhs;
                return self.apply_sign(sign);
            }
            (_, &ONE) => {
                return self.apply_sign(sign);
            }
            // (&ONE, _) | (_, &ONE) => {
            //     *self = rhs;
            //     return self.apply_sign(sign);
            // }
            (ExprTyp::Prod(_), _) => {
                let ExprTyp::Prod(p) = &mut self.typ else {
                   unreachable!() 
                };

                if let Some(rhs) = rhs.as_real() {
                    if let Some(cnst) = p.front().map(|a| a.as_real()).flatten() {
                        *p.front_mut().unwrap() = Atom::real(cnst * rhs);
                    } else {
                        p.push_front(Atom::real(rhs))
                    }
                    return self.apply_sign(sign);
                }
            }
            (_, ExprTyp::Prod(_)) => {
                let ExprTyp::Prod(p) = &mut rhs.typ else {
                   unreachable!() 
                };

                if let Some(lhs) = self.as_real() {
                    if let Some(cnst) = p.front().map(|a| a.as_real()).flatten() {
                        *p.front_mut().unwrap() = Atom::real(cnst * lhs);
                    } else {
                        p.push_front(Atom::real(lhs))
                    }
                    return self.apply_sign(sign);
                }
            }
            (_, _) => {
                if let (Some(lhs), Some(rhs)) = (self.as_real(), rhs.as_real()) {
                    *self = Expr::real(lhs * rhs);
                    return self.apply_sign(sign);
                }
            }
        };

        match strat {
            MulStrategy::Simple => match (&mut self.typ, &mut rhs.typ) {
                (ExprTyp::Prod(s1), ExprTyp::Prod(s2)) => {
                    s1.extend(s2.drain(..));
                }
                (ExprTyp::Prod(s), _) => {
                    s.push_back(Atom::expr(rhs));
                }
                (_, ExprTyp::Prod(s)) => {
                    let lhs = self.remove_expr();
                    s.push_front(Atom::expr(lhs));
                }
                _ => *self = Expr {
                    typ: ExprTyp::Prod([Atom::expr(self.remove_expr()), Atom::expr(rhs.remove_expr())].into()),
                    meta: Meta::SIMPLE_FORM,
                },
            },
            MulStrategy::Base => {
                if rhs.is_prod() {
                    for oprnd in rhs.remove_nary_operands().unwrap() {
                        self.mul_with(Expr::atom(oprnd), MulStrategy::Base);
                    }
                } else if self.is_prod() {
                    let ExprTyp::Prod(p) = &mut self.typ else {
                        unreachable!()
                    };

                    if let Some(oprnd) = p.iter_mut().find(|a| a.base_ref() == rhs.base_ref()) {
                        let (_, exp) = oprnd.as_mut_expr().as_pow_mut();
                        exp.as_mut_expr().add_with(
                            Expr::atom(rhs.remove_exponent().unwrap()),
                            AddStrategy::Simple,
                        );
                        // *exp.as_mut_expr() += Expr::atom(rhs.remove_exponent().unwrap());
                        exp.cleanup_indirection();
                    } else {
                        p.push_back(Atom::expr(rhs));
                    }
                } else if self.base_ref() == rhs.base_ref() {
                    let _ = rhs.as_pow_mut();
                    let r_exp = rhs.remove_exponent().unwrap();
                    let (_, exp) = self.as_pow_mut();

                    let expon_expr = exp.as_mut_expr();
                    expon_expr.add_with(Expr::atom(r_exp), AddStrategy::Simple);
                    exp.cleanup_indirection();
                } else {
                    let lhs = Atom::expr(self.remove_expr());
                    *self = Expr {
                        typ: ExprTyp::Prod([lhs, Atom::expr(rhs)].into()),
                        meta: Meta::SIMPLE_FORM ^ (l_meta & r_meta),
                    }
                }
                self.cleanup_mut();
            }
            MulStrategy::Expand => match (self.typ(), rhs.typ()) {
                (ExprTyp::Sum(_), _) => {
                    let mut sum = Expr::u32(0);

                    for term in self.remove_nary_operands().unwrap() {
                        let mut prod = Expr::atom(term);
                        prod.mul_with(rhs.clone(), MulStrategy::Expand);
                        sum.add_with(prod, AddStrategy::Simple);
                    }

                    *self = sum;
                }
                (_, ExprTyp::Sum(_)) => {
                    let mut sum = Expr::u32(0);
                    for term in rhs.remove_nary_operands().unwrap() {
                        let mut prod = self.clone();
                        prod.mul_with(Expr::atom(term), MulStrategy::Expand);
                        sum.add_with(prod, AddStrategy::Simple);
                    }

                    *self = sum;
                }
                (_, _) => {
                    self.mul_with(rhs, MulStrategy::Base);
                }
            },
            MulStrategy::Frozen => unreachable!(),
        }
        self.apply_sign(sign)
    }

    #[log_fn]
    pub fn pow_with(&mut self, mut expon: Expr, strat: PowStrategy) -> &mut Expr {
        if matches!(strat, PowStrategy::Frozen) {
            *self = Expr {
                typ: ExprTyp::Pow([Atom::expr(self.remove_expr()), Atom::expr(expon)].into()),
                meta: Meta::FROZEN_FORM,
            };
            return self;
        }

        const UNDEF: ExprTyp = ExprTyp::Atom(Atom::Undef);
        const ZERO: ExprTyp = ExprTyp::Atom(Atom::U32(0));
        const ONE: ExprTyp = ExprTyp::Atom(Atom::U32(1));

        self.cleanup_mut();
        expon.cleanup_mut();

        match (self.typ(), expon.typ()) {
            (&ZERO, &ZERO) | (&UNDEF, _) | (_, &UNDEF) => {
                *self = Expr::undef();
                return self;
            }
            (&ZERO, _) if expon.is_real_and(Real::is_negative) => {
                *self = Expr::undef();
                return self;
            }
            (&ZERO, _) if expon.is_real_and(Real::is_positive) => {
                *self = Expr::u32(0);
                return self;
            }
            (_, &ONE) => return self,
            _ => {
                if let (Some(base), Some(expon)) = (self.as_real(), expon.as_real()) {
                    let (pow, rem) = base.pow_simplify(expon);

                    let mut res = Expr::real(pow);

                    if let Some(rem) = rem {
                        res *= Expr {
                            typ: ExprTyp::Pow([Atom::real(base), Atom::real(rem)]),
                            meta: Meta::SIMPLE_FORM.union(Meta::EXPAND_FORM),
                        };
                    }
                    *self = res;
                    return self;
                }
            }
        };

        match strat {
            PowStrategy::Simple => {
                *self = Expr {
                    typ: ExprTyp::Pow([Atom::expr(self.remove_expr()), Atom::expr(expon)].into()),
                    meta: Meta::SIMPLE_FORM,
                };
            }
            PowStrategy::Expand => {
                match (self.typ(), expon.typ()) {
                    (ExprTyp::Pow(_), _) => {
                        let [b, e] = self.remove_binary_operands().unwrap();
                        let mut e = Expr::atom(e);
                        e.mul_with(expon, MulStrategy::Expand);
                        let mut pow = Expr::atom(b);
                        pow.pow_with(e, PowStrategy::Expand);
                        *self = pow;
                    }
                    (ExprTyp::Prod(_), _) => {
                        let mut prod = Expr::u32(1);
                        for op in self.remove_nary_operands().unwrap() {
                            let mut pow = Expr::atom(op);
                            pow.pow_with(expon.clone(), PowStrategy::Expand);
                            prod.mul_with(pow, MulStrategy::Expand);
                        }
                    }
                    (ExprTyp::Sum(_), ExprTyp::Atom(Atom::U32(n))) if *n > 1 => {
                        let n = *n;
                        let mut sum = Expr::u32(0);

                        // oprnds = term + rest
                        let mut oprnds = self.remove_nary_operands().unwrap();

                        let term = oprnds
                            .pop_front()
                            .map(|o| Expr::atom(o))
                            .expect("called clean_expr before");

                        let mut rest = Expr {
                            typ: ExprTyp::Sum(oprnds),
                            meta: Meta::EXPAND_FORM,
                        };

                        rest.inline_trivial_compound();

                        for k in 0..=n {
                            if k == 0 {
                                // term^n * 1
                                let mut a = term.clone();
                                a.pow_with(expon.clone(), PowStrategy::Expand);
                                sum.add_with(a, AddStrategy::Simple);
                            } else if k == n {
                                // 1 * rest^n
                                let mut b = rest.clone();
                                b.pow_with(expon.clone(), PowStrategy::Expand);
                                sum.add_with(b, AddStrategy::Simple);
                            } else {
                                // binom(n, k) * term^k * rest^(n - k)
                                let c = num::integer::binomial(n, k);
                                let mut a = term.clone();
                                let mut b = rest.clone();

                                a.pow_with(Expr::u32(k), PowStrategy::Expand);
                                b.pow_with(Expr::u32(n - k), PowStrategy::Expand);

                                a.mul_with(Expr::u32(c), MulStrategy::Expand)
                                    .mul_with(b, MulStrategy::Expand);
                                sum.add_with(a, AddStrategy::Simple);
                            }
                        }

                        *self = sum;
                    }
                    _ => {
                        *self = Expr {
                            typ: ExprTyp::Pow(
                                [Atom::expr(self.remove_expr()), Atom::expr(expon)].into(),
                            ),
                            meta: Meta::SIMPLE_FORM,
                        };
                    }
                }
            }
            PowStrategy::Frozen => unreachable!(),
        }

        self
    }

    #[inline]
    pub fn pow(mut self, exp: Expr) -> Expr {
        self.pow_with(exp, noctua_global_config().default_pow_strategy);
        self
    }

    #[inline]
    pub fn pow_mut(&mut self, exp: Expr) -> &mut Expr {
        self.pow_with(exp, noctua_global_config().default_pow_strategy);
        self
    }

    #[inline]
    pub fn as_pow_mut(&mut self) -> (&mut Atom, &mut Atom) {
        if self.is_pow() {
            if let ExprTyp::Pow([base, expon]) = &mut self.typ {
                (base, expon)
            } else {
                unreachable!()
            }
        } else {
            *self = Expr {
                typ: ExprTyp::Pow([Atom::expr(self.remove_expr()), Atom::U32(1)]),
                meta: Meta::FROZEN_FORM.union(Meta::EXPAND_FORM),
            };
            if let ExprTyp::Pow([base, expon]) = &mut self.typ {
                (base, expon)
            } else {
                unreachable!()
            }
        }
    }

    #[inline]
    pub fn expand_root_mut(&mut self) -> &mut Expr {
        self.meta |= Meta::EXPAND_FORM;
        match &mut self.cleanup_mut().typ {
            ExprTyp::Atom(_) | ExprTyp::Sum(_) => (),

            ExprTyp::Minus(_) => {
                self.operands_mut().iter_mut().for_each(|op| {
                    op.as_mut_expr().minus_mut();
                    op.cleanup_indirection();
                });
            }

            ExprTyp::Prod(_) => {
                let mut prod = Expr::u32(1);
                for op in self.remove_nary_operands().unwrap() {
                    prod.mul_with(Expr::atom(op), MulStrategy::Expand);
                }
                *self = prod;
            }

            ExprTyp::Pow(_) => {
                let [base, expon] = self.remove_binary_operands().unwrap();
                let mut pow = Expr::atom(base);
                pow.pow_with(Expr::atom(expon), PowStrategy::Expand);
                *self = pow;
            }
        }

        self
    }

    #[log_fn]
    pub fn expand_mut(&mut self) -> &mut Expr {
        self.operands_mut().iter_mut().for_each(Atom::expand_mut);
        self.cleanup_mut().expand_root_mut()
    }

    pub fn expand(mut self) -> Expr {
        self.expand_mut();
        self
    }

    #[inline]
    pub fn replace(&mut self, e: Expr) -> Expr {
        std::mem::replace(self, e)
    }

    /// Perform basic simplifications on the outermost expression
    #[inline]
    pub fn cleanup_mut(&mut self) -> &mut Expr {
        self.inline_trivial_compound();
        self
    }

    /// Simplifies trivial compound expressions
    ///
    /// Inline n-ary operations as long as equivalency is maintained
    pub fn inline_trivial_compound(&mut self) -> &mut Expr {
        match &mut self.typ {
            ExprTyp::Minus(Atom::U32(0)) => {
                *self = Expr::u32(0);
            }
            ExprTyp::Minus(Atom::Expr(e)) if e.is_minus() => {
                match &mut Rc::make_mut(e).typ {
                    ExprTyp::Minus(a) => {
                        *self = Expr::atom(std::mem::replace(a, Atom::Undef));
                    },
                    _ => ()
                }
            }
            ExprTyp::Sum(oprnds) if oprnds.is_empty() => {
                *self = Expr::u32(0);
            }
            ExprTyp::Prod(oprnds) if oprnds.is_empty() => {
                *self = Expr::u32(1);
            }
            ExprTyp::Sum(oprnds) | ExprTyp::Prod(oprnds) if oprnds.len() == 1 => {
                *self = Expr::atom(self.remove_nary_operands().unwrap().pop_front().unwrap());
            }
            _ => (),
        }
        self
    }

    //////////////////////////////////////////////////////
    //////    Accessors
    //////////////////////////////////////////////////////

    #[inline]
    pub fn view(&self) -> View<'_> {
        match self.typ() {
            ExprTyp::Atom(atom) => {
                self.dbg_assert_valid();
                View::Atom(atom)
            }
            _ => View::Expr(self),
        }
    }

    #[inline]
    pub const fn meta(&self) -> Meta {
        self.meta
    }

    #[inline]
    pub const fn typ(&self) -> &ExprTyp {
        &self.typ
    }

    pub fn operands(&self) -> &[Atom] {
        match self.typ() {
            ExprTyp::Atom(atom) | ExprTyp::Minus(atom) => std::slice::from_ref(atom),
            ExprTyp::Sum(oprnds) | ExprTyp::Prod(oprnds) => oprnds.as_slice(),
            ExprTyp::Pow(oprnds) => oprnds,
        }
    }

    pub fn operands_mut(&mut self) -> &mut [Atom] {
        self.dbg_assert_valid();

        match &mut self.typ {
            ExprTyp::Atom(atom) | ExprTyp::Minus(atom) => std::slice::from_mut(atom),
            ExprTyp::Sum(oprnds) | ExprTyp::Prod(oprnds) => oprnds.as_mut_slice(),
            ExprTyp::Pow(oprnds) => oprnds,
        }
    }

    /// if `self` is [`Expr::Pow`] return (base, exponent) otherwise (`self`, 1)
    #[inline]
    pub fn base_expon_ref(&self) -> (View<'_>, &Atom) {
        match self.typ() {
            ExprTyp::Pow([base, expon]) => {
                (base.view(), expon)
            }
            _ => (self.view(), &Atom::U32(1)),
        }
    }

    /// if `self` is [`Expr::Pow`] return the base otherwise return `self`
    #[inline]
    pub fn base_ref(&self) -> View<'_> {
        self.base_expon_ref().0
    }

    /// if `self` is [`Expr::Pow`] return the exponent otherwise return 1
    #[inline]
    pub fn exponent_ref(&self) -> &Atom {
        self.base_expon_ref().1
    }

    #[log_fn]
    #[inline]
    pub fn cancle_signs(&mut self) -> Sign {
        match &self.typ {
            ExprTyp::Atom(_) => self.operands_mut()[0].cancle_signs(),
            ExprTyp::Minus(_) => {
                let s = self.operands_mut()[0].cancle_signs();
                let inner = self.remove_unary_operands().unwrap();
                *self = Expr::atom(inner);
                s * Sign::Minus
            }
            ExprTyp::Prod(_) => {
                let mut sign = Sign::Plus;
                for a in self.operands_mut() {
                    sign *= a.cancle_signs();
                }
                sign
            }
            _ => Sign::Plus,
        }
    }

    /// Similar to [`Expr::cancle_signs`], but will not modify the expression,
    /// returning a view of the unsigned expressions instead.
    #[inline]
    pub fn cancle_sign_ref(&self) -> (Sign, View<'_>) {
        match self.typ() {
            ExprTyp::Minus(atom) => (Sign::Minus, atom.view()),
            _ => (Sign::Plus, self.view()),
        }
    }

    #[inline]
    pub fn is_sum_and(&self, f: impl FnOnce(&FlatDeque<Atom>) -> bool) -> bool {
        self.dbg_assert_valid();
        match &self.typ {
            ExprTyp::Sum(oprnds) => f(oprnds),
            _ => false,
        }
    }
    #[inline]
    pub fn is_prod_and(&self, f: impl FnOnce(&FlatDeque<Atom>) -> bool) -> bool {
        self.dbg_assert_valid();
        match &self.typ {
            ExprTyp::Prod(oprnds) => f(oprnds),
            _ => false,
        }
    }
    #[inline]
    pub fn is_pow_and(&self, f: impl FnOnce(&[Atom; 2]) -> bool) -> bool {
        self.dbg_assert_valid();
        match &self.typ {
            ExprTyp::Pow(oprnds) => f(oprnds),
            _ => false,
        }
    }
    #[inline]
    pub fn is_atom_and(&self, f: impl FnOnce(&Atom) -> bool) -> bool {
        self.dbg_assert_valid();
        match &self.typ {
            ExprTyp::Atom(a) => f(a),
            _ => false,
        }
    }
    #[inline]
    pub fn is_minus_and(&self, f: impl FnOnce(&Atom) -> bool) -> bool {
        self.dbg_assert_valid();
        match &self.typ {
            ExprTyp::Minus(a) => f(a),
            _ => false,
        }
    }
    #[inline]
    pub fn is_real_and(&self, f: impl FnOnce(Real) -> bool) -> bool {
        self.dbg_assert_valid();
        self.as_real().is_some_and(|r| f(r))
    }
    #[inline]
    pub fn is_prod(&self) -> bool {
        self.is_prod_and(|_| true)
    }
    #[inline]
    pub fn is_pow(&self) -> bool {
        self.is_pow_and(|_| true)
    }
    #[inline]
    pub fn is_atom(&self) -> bool {
        self.is_atom_and(|_| true)
    }
    #[inline]
    pub fn is_minus(&self) -> bool {
        self.is_minus_and(|_| true)
    }

    #[inline]
    fn remove_expr(&mut self) -> Expr {
        std::mem::replace(self, Expr::const_atom(Atom::Undef))
    }

    #[inline]
    fn remove_unary_operands(&mut self) -> Option<Atom> {
        match &mut self.typ {
            // Expr::Sub(oprnds) | Expr::Div(oprnds) |
            ExprTyp::Minus(a) => Some(std::mem::replace(a, Atom::Undef)),
            _ => None,
        }
    }

    #[inline]
    fn remove_binary_operands(&mut self) -> Option<[Atom; 2]> {
        match &mut self.typ {
            // Expr::Sub(oprnds) | Expr::Div(oprnds) |
            ExprTyp::Pow(oprnds) => Some(std::mem::replace(oprnds, [Atom::Undef, Atom::Undef])),
            _ => None,
        }
    }

    #[inline]
    fn remove_nary_operands(&mut self) -> Option<FlatDeque<Atom>> {
        match &self.typ {
            ExprTyp::Sum(_) | ExprTyp::Prod(_) => (),
            _ => return None,
        }
        let tmp = self.remove_expr();
        match tmp.typ {
            ExprTyp::Sum(oprnds) | ExprTyp::Prod(oprnds) => Some(oprnds),
            _ => unreachable!(),
        }
    }

    #[inline]
    fn remove_exponent(&mut self) -> Option<Atom> {
        self.remove_binary_operands().map(|[_, expon]| expon)
    }

    #[inline]
    fn remove_base(&mut self) -> Option<Atom> {
        self.remove_binary_operands().map(|[base, _]| base)
    }

    /// Will try to represent the current expression with [`Real`]
    #[inline]
    pub fn as_real(&self) -> Option<Real> {
        let (sign, view) = self.cancle_sign_ref();
        self.dbg_assert_valid();
        match view {
            View::Atom(&Atom::U32(v)) => Some(Real::signed_u32(sign, v)),
            View::Atom(&Atom::Rational(r)) => Some(Real::signed_rational(sign, r)),
            _ => None,
        }
    }

    pub fn eq_typ(&self, other: &Expr) -> bool {
        std::mem::discriminant(&self.typ) == std::mem::discriminant(&other.typ)
    }

    /// Order of the expressions in simplified form
    ///
    #[log_fn]
    pub fn simplified_ordering(&self, other: &Expr) -> cmp::Ordering {
        use ordering_abbreviations::*;

        let (lhs, rhs) = (self, other);

        fn cmp_views<'a>(
            lhs: impl Iterator<Item = View<'a>>,
            rhs: impl Iterator<Item = View<'a>>,
        ) -> cmp::Ordering {
            let (mut l_iter, mut r_iter) = (lhs.into_iter(), rhs.into_iter());

            loop {
                match (l_iter.next(), r_iter.next()) {
                    (Some(l), Some(r)) => {
                        if l != r {
                            return l.simplified_ordering(&r);
                        }
                    }
                    (Some(_), None) => return GE,
                    (None, Some(_)) => return LE,
                    (None, None) => return EQ,
                }
            }

            // while let (Some(l), Some(r)) = (l_iter.next(), r_iter.next()) {
            //     if l != r {
            //         return l.simplified_ordering(&r);
            //     }
            // }

            // match (l_iter.next(), r_iter.next()) {
            //     (Some(_), None) => cmp::Ordering::Greater,
            //     (None, Some(_)) => cmp::Ordering::Less,
            //     _ => cmp::Ordering::Equal
            // }
        }

        const MINUS_ONE: Expr = Expr {
            typ: ExprTyp::Minus(Atom::U32(1)),
            meta: Meta::SIMPLE_FORM.union(Meta::EXPAND_FORM),
        };

        #[inline]
        pub fn ops_view<'a>(e: &'a Expr) -> impl Iterator<Item = View<'a>> {
            e.operands().iter().map(|o| o.view())
        }

        fn expr_view(e: &Expr) -> impl Iterator<Item = View<'_>> {
            [e.view()].into_iter()
        }

        fn minus_view(e: &Expr) -> impl Iterator<Item = View<'_>> {
            [MINUS_ONE.view()].into_iter().chain(ops_view(e))
        }

        if lhs == rhs {
            return EQ;
        } else if lhs.eq_typ(rhs) {
            return cmp_views(ops_view(lhs), ops_view(rhs));
        } else if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
            return lhs.cmp(&rhs);
        }

        const UNDEF: ExprTyp = ExprTyp::Atom(Atom::Undef);

        match (lhs.typ(), rhs.typ()) {
            (&UNDEF, _) => GE,
            (_, &UNDEF) => LE,

            // treat non-sum element as if it were a product with a single operand and compare
            (ExprTyp::Sum(_), _) => cmp_views(ops_view(lhs), expr_view(rhs)),
            (_, ExprTyp::Sum(_)) => cmp_views(expr_view(lhs), ops_view(rhs)),

            // treat non-product element as if it were a product with a single operand and compare
            (_, ExprTyp::Prod(_)) => cmp_views(expr_view(lhs), ops_view(rhs)),
            (ExprTyp::Prod(_), _) => cmp_views(ops_view(lhs), expr_view(rhs)),

            // treat minus as -1 * ... and commpare like the product
            (ExprTyp::Minus(_), _) => cmp_views(expr_view(lhs), minus_view(rhs)),
            (_, ExprTyp::Minus(_)) => cmp_views(minus_view(lhs), expr_view(rhs)),

            // treat non-power expressions as if they had an exponent of 1
            (ExprTyp::Pow(_), _) | (_, ExprTyp::Pow(_)) => {
                let (b1, e1) = (lhs.base_ref(), lhs.exponent_ref().view());
                let (b2, e2) = (rhs.base_ref(), rhs.exponent_ref().view());

                if b1 != b2 {
                    cmp_views([b1].into_iter(), [b2].into_iter())
                } else {
                    cmp_views([e1].into_iter(), [e2].into_iter())
                }
            }

            (ExprTyp::Atom(a1), ExprTyp::Atom(a2)) => a1.simplified_ordering(a2),
            // (_, Expr::Atom(_)) => LE,
        }
    }

    #[inline]
    fn dbg_assert_valid(&self) {
        match &self.typ {
            ExprTyp::Atom(atom) => debug_assert!(!atom.is_expr()),
            ExprTyp::Sum(oprnds) | ExprTyp::Prod(oprnds) => {
                debug_assert!(oprnds.len() > 1)
            }
            ExprTyp::Minus(_) | ExprTyp::Pow(_) => (),
        }
        // self.
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
    fn add(mut self, rhs: Self) -> Self::Output {
        self.add_with(rhs, noctua_global_config().default_add_strategy);
        self
    }
}

impl ops::AddAssign for Expr {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.remove_expr() + rhs;
    }
}

impl ops::Mul for Expr {
    type Output = Expr;
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.mul_with(rhs, noctua_global_config().default_mul_strategy);
        self
    }
}

impl ops::MulAssign for Expr {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.remove_expr() * rhs;
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
        *self = self.remove_expr() / rhs;
    }
}

impl ops::Neg for Expr {
    type Output = Expr;
    fn neg(mut self) -> Self::Output {
        self.minus_mut();
        self
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Undef => write!(f, "\u{2205}"),
            Atom::U32(val) => write!(f, "{val}"),
            Atom::Var(var) => write!(f, "{var}"),
            Atom::Expr(expr) => write!(f, "{expr:?}"),
            Atom::Rational(r) => write!(f, "{r}"),
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Undef => write!(f, "\u{2205}"),
            Atom::U32(val) => write!(f, "{val}"),
            Atom::Var(var) => write!(f, "{var}"),
            Atom::Expr(expr) => write!(f, "{expr}"),
            Atom::Rational(r) => write!(f, "{r}"),
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.typ() {
            ExprTyp::Atom(atom) => write!(f, "{atom:?}"),
            ExprTyp::Minus(Atom::Expr(expr)) => write!(f, "-({expr:?})"),
            ExprTyp::Minus(atom) => write!(f, "-{atom:?}"),
            ExprTyp::Sum(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[+]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[+{:?}]", oprnds[0]);
                }
                write!(f, "[{:?}]", oprnds.iter().format(" + "))
            }
            ExprTyp::Prod(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[*]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[*{:?}]", oprnds[0]);
                }
                write!(f, "[{:?}]", oprnds.iter().format(" * "))
            }
            ExprTyp::Pow([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_)) {
                    write!(f, "({lhs:?})^")?;
                } else {
                    write!(f, "{lhs:?}^")?;
                }
                if matches!(rhs, Atom::Expr(_)) {
                    write!(f, "({rhs:?})")
                } else {
                    write!(f, "{rhs:?}")
                }
            }
        }?;
        write!(f, " ({:?})", self.meta)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.typ() {
            ExprTyp::Atom(atom) => write!(f, "{atom}"),
            ExprTyp::Minus(Atom::Expr(expr)) => write!(f, "-({expr})"),
            ExprTyp::Minus(atom) => write!(f, "-{atom}"),
            ExprTyp::Sum(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[+]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[+{}]", oprnds[0]);
                }
                write!(f, "[{}]", oprnds.iter().format(" + "))
            }
            ExprTyp::Prod(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[*]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[*{}]", oprnds[0]);
                }
                write!(f, "[{}]", oprnds.iter().format(" * "))
            }
            ExprTyp::Pow([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_)) {
                    write!(f, "({lhs})^")?;
                } else {
                    write!(f, "{lhs}^")?;
                }
                if matches!(rhs, Atom::Expr(_)) {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            }
        }
    }
}

impl fmt::Debug for View<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            View::Atom(a) => write!(f, "VA[{a:?}]"),
            View::Expr(e) => write!(f, "VE[{e:?}]"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::noctua as n;

    #[test]
    fn pow_with() {
        assert_eq!(n!(0^0), n!(undef));
        assert_eq!(n!(3^2), n!(9));
        assert_eq!(n!(3^(-2)), n!(1/9));
        // x could be 0
        assert_eq!(n!(x^0), n!(x^0));
    }

    #[test]
    fn simplified_ordering() {
        
        let order = [
            (n!(1), n!(2)),
            (n!(x), n!(x^2)),
            (n!(a * x^2), n!(x^3)),
            (n!(u), n!(v^1)),
            (n!((1+x)^2), n!((1+x)^3)),
            (n!((1+x)^3), n!((1+y)^2)),
            (n!(a+b), n!(a+c)),
            (n!(1+x), n!(y)),
            (n!(a*x^2), n!(x^3)),
        ];

        for (l, r) in order {
            assert!(l.simplified_ordering(&r).is_lt(), "{l:?} vs {r:?}");
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
            (n!(1 ^ x), n!(1)),
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
            assert_eq!(calc, res, "{i}: {calc} != {res}");
        }
    }


    #[test]
    fn sort_args() {
        let checks = vec![
            n!(a + b),
            n!(b * c + a),
            // n!(sin(x) * cos(x)),
            n!(a * x ^ 2 + b * x + c + 3),
        ];

        for c in checks {

            // assert_eq!(c.sort_args(), c)
        }
    }

}
