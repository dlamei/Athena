use std::cmp;
use std::{fmt, ops, rc::Rc};

use itertools::Itertools;
use crate::log_fn;

use crate::config::{AddStrategy, MulStrategy, PowStrategy, noctua_global_config};
use crate::flat_deque::FlatDeque;
use crate::real::{Real, Sign};

mod ordering_abbreviations {
    pub const GE: std::cmp::Ordering = std::cmp::Ordering::Greater;
    pub const LE: std::cmp::Ordering = std::cmp::Ordering::Less;
    pub const EQ: std::cmp::Ordering = std::cmp::Ordering::Equal;
}

/// Represents an atomic unit: simple values or sub-expressions
///
/// - Inline variants like undef or integers
/// - Compound expressions used as part of an [`Expr`]
#[derive(Clone, PartialEq)]
pub enum Atom {
    _Undef_,
    _U32_(u32),
    _Var_(Rc<str>),

    // IMPORTANT: if the Atom is part of an `Expr` this should only be used if the atom is part
    // of a compound expression. Otherwise promote the `Expr::Atom(Atom::Expr(..))` to `Expr`.
    // In other words when encountering `Expr::Atom(atom)` atom must not be an `Atom::Expr`
    _Expr_(Rc<Expr>),
}

/// # Constructors
impl Atom {
    #[inline]
    #[allow(non_snake_case)]
    pub const fn Undef() -> Atom {
        Atom::_Undef_
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn U32(u: u32) -> Atom {
        Atom::_U32_(u)
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Var(v: impl AsRef<str>) -> Atom {
        Atom::_Var_(v.as_ref().into())
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Expr(e: Expr) -> Atom {
        match e {
            Expr::_Atom_(atom) => atom,
            e => Atom::_Expr_(e.into()),
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Real(r: Real) -> Atom {
        match r {
            Real::Zero => Atom::U32(0),
            Real::U32(Sign::Plus, u) => Atom::U32(u),
            Real::U32(Sign::Minus, _) => Atom::Expr(Expr::Real(r)),
        }
    }

    /// Should be used in functions like [`Expr::replace`]
    #[inline]
    #[allow(non_snake_case)]
    pub const fn None() -> Atom {
        Atom::_Undef_
    }
}

/// # Accessors
impl Atom {
    pub fn view(&self) -> View<'_> {
        match self {
            Atom::_Expr_(e) => e.view(),
            _ => View::Atom(self),
        }
    }

    #[inline]
    pub fn as_real(&self) -> Option<Real> {
        match self {
            Atom::_U32_(u) => Some(Real::u32(*u)),
            _ => None,
        }
    }

    pub fn view_base(&self) -> View<'_> {
        match self {
            Atom::_Expr_(expr) => expr.view_base(),
            _ => View::Atom(self),
        }
    }

    pub fn view_exponent(&self) -> View<'_> {
        match self {
            Atom::_Expr_(expr) => expr.view_exponent(),
            _ => View::Atom(self),
        }
    }


    pub fn operands(&self) -> &[Atom] {
        match self {
            Atom::_Expr_(expr) => expr.operands(),
            _ => std::slice::from_ref(self),
        }
    }
}

/// # Modifiers
impl Atom {
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
        if let Atom::_Expr_(rc) = self {
            let mut_expr = Rc::make_mut(rc);
            // assert!(!matches!(mut_expr, Expr::_Atom_(_)));
            return f(mut_expr);
        }
        let orig = std::mem::replace(self, Atom::None());
        *self = Atom::_Expr_(Rc::new(Expr::Atom(orig)));

        let Atom::_Expr_(expr) = self else {
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
        if let Atom::_Expr_(e) = self {
            if let Expr::_Atom_(atom) = Rc::make_mut(e) {
                let atom = std::mem::replace(atom, Atom::None());
                *self = atom;
            }
        }
        self
    }

    #[inline]
    pub fn expand_mut(&mut self) {
        match self {
            Atom::_Undef_ | Atom::_U32_(_) | Atom::_Var_(_) => (),
            Atom::_Expr_(expr) => {
                Rc::make_mut(expr).expand_mut();
            }
        }
    }
}

/// # Misc.
impl Atom {
    /// Order of the expressions in simplified form
    ///
    #[log_fn]
    pub fn simplified_ordering(&self, other: &Atom) -> cmp::Ordering {
        use ordering_abbreviations::*;

        if self == other {
            return EQ;
        }

        match (self, other) {
            (Atom::_Undef_, _) => return GE,
            (_, Atom::_Undef_) => return LE,

            (Atom::_U32_(u1), Atom::_U32_(u2)) => return u1.cmp(u2),
            (Atom::_Var_(v1), Atom::_Var_(v2)) => return v1.cmp(v2),

            (Atom::_U32_(_), Atom::_Var_(_)) => return LE,
            (Atom::_Var_(_), Atom::_U32_(_)) => return GE,

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

/// Expression composed of [`Atom`] units and operations.
///
/// This design allows to store simple expressions with very little overhead.
#[derive(Clone, PartialEq)]
pub enum Expr {
    _Atom_(Atom),

    /// Is used to represent negative values
    ///
    /// Will be interpreted as the expression -1 * [`Atom`]
    /// For cohesion negative integers are represented as `Expr::Minus(Atom::U32(0))`
    _Minus_(Atom),

    _Sum_(FlatDeque<Atom>),
    _Prod_(FlatDeque<Atom>),
    _Pow_([Atom; 2]),
}

/// # Constructors
impl Expr {
    #[inline]
    #[allow(non_snake_case)]
    pub const fn ConstAtom(atom: Atom) -> Expr {
        match atom {
            Atom::_Expr_(_) => panic!(),
            _ => Expr::_Atom_(atom),
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn Undef() -> Expr {
        Expr::ConstAtom(Atom::Undef())
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn U32(u: u32) -> Expr {
        Expr::ConstAtom(Atom::U32(u))
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn I32(i: i32) -> Expr {
        let atom = Atom::U32(i.unsigned_abs());
        if i < 0 {
            Expr::Minus(atom)
        } else {
            Expr::ConstAtom(atom)
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn Real(r: Real) -> Expr {
        match r {
            Real::Zero => Expr::U32(0),
            Real::U32(Sign::Minus, u) => Expr::Minus(Atom::U32(u)),
            Real::U32(Sign::Plus, u) => Expr::U32(u),
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Var(v: impl AsRef<str>) -> Expr {
        Expr::ConstAtom(Atom::Var(v))
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Atom(atom: Atom) -> Expr {
        // Self::Atom_(atom)
        match atom {
            Atom::_Expr_(expr) => Rc::unwrap_or_clone(expr),
            _ => Expr::_Atom_(atom),
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn Minus(atom: Atom) -> Expr {
        Self::_Minus_(atom)
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Sum(oprnds: impl Into<FlatDeque<Atom>>) -> Expr {
        Self::_Sum_(oprnds.into())
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn Prod(oprnds: impl Into<FlatDeque<Atom>>) -> Expr {
        Self::_Prod_(oprnds.into())
    }

    #[inline]
    #[allow(non_snake_case)]
    pub const fn Pow(base: Atom, exponent: Atom) -> Expr {
        Self::_Pow_([base, exponent])
    }
}

/// # Accessors
impl Expr {

    #[inline]
    pub fn view(&self) -> View<'_> {
        match self {
            Expr::_Atom_(atom) => atom.view(),
            expr => View::Expr(expr),
        }
    }


    /// Similar to [`Expr::remove_sign_mut`], but will not modify the expression,
    /// returning a view of the unsigned expressions instead.
    #[inline]
    pub fn unsigned_view(&self) -> (Sign, View<'_>) {
        match self {
            Expr::_Minus_(atom) => (Sign::Minus, atom.view()),
            _ => (Sign::Plus, self.view()),
        }
    }

    /// Return the outermost sign
    ///
    /// returns [`Sign::Minus`] if `self` is [`Expr::Minus`], [`Sign::Plus`] otherwise
    #[inline]
    pub fn root_sign(&self) -> Sign {
        self.unsigned_view().0
    }

    /// Will try to represent the current expression with [`Real`]
    #[inline]
    pub fn as_real(&self) -> Option<Real> {
        let (sign, view) = self.unsigned_view();
        match view {
            View::Atom(&Atom::_U32_(v)) => Some(Real::u32_with_sign(sign, v)),
            _ => None,
        }
    }

    #[inline]
    pub fn operands(&self) -> &[Atom] {
        match self {
            Expr::_Atom_(atom) | Expr::_Minus_(atom) => std::slice::from_ref(atom),
            Expr::_Sum_(vec) | Expr::_Prod_(vec) => vec.as_slice(),
            // Expr::Sub(oprnds) | Expr::Div(oprnds) |
            Expr::_Pow_(oprnds) => oprnds,
        }
    }

    #[inline]
    pub fn operands_view(&self) -> impl Iterator<Item = View<'_>> {
        self.operands().iter().map(|o| o.view())
    }

    #[inline]
    pub fn n_operands(&self) -> usize {
        self.operands().len()
    }

    #[inline]
    pub fn operands_mut(&mut self) -> &mut [Atom] {
        match self {
            Expr::_Atom_(atom) | Expr::_Minus_(atom) => std::slice::from_mut(atom),
            Expr::_Sum_(vec) | Expr::_Prod_(vec) => vec.as_mut_slice(),
            // Expr::Sub(oprnds) | Expr::Div(oprnds) |
            Expr::_Pow_(oprnds) => oprnds,
        }
    }

    #[inline]
    pub fn view_base(&self) -> View<'_> {
        match self {
            Expr::_Atom_(_) | Expr::_Minus_(_) | Expr::_Sum_(_) | Expr::_Prod_(_) => self.view(),
            // | Expr::Sub(_)
            // | Expr::Div(_)
            Expr::_Pow_([base, _]) => View::Atom(base),
        }
    }

    #[inline]
    pub fn view_exponent(&self) -> View<'_> {
        View::Atom(match self {
            Expr::_Atom_(_) | Expr::_Minus_(_) | Expr::_Sum_(_) | Expr::_Prod_(_) => {
                &Atom::_U32_(1)
            }
            // | Expr::Sub(_)
            // | Expr::Div(_) => &Atom::U32(1),
            Expr::_Pow_([_, expon]) => expon,
        })
    }



    /// if `self` is [`Expr::Pow`] return (base, exponent) otherwise (`self`, 1)
    #[inline]
    fn get_base_exponent(self) -> (Atom, Atom) {
        match self {
            Expr::_Pow_([base, expon]) => (base, expon),
            _ => (Atom::Expr(self), Atom::U32(1)),
        }
    }

    /// if `self` is [`Expr::Pow`] return the exponent otherwise return 1
    #[inline]
    fn get_exponent(self) -> Atom {
        self.get_base_exponent().1
    }


    #[inline]
    pub fn is_unsigned_atom(&self) -> bool {
        match self {
            Expr::_Atom_(Atom::_Expr_(_)) => false,
            Expr::_Atom_(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_signed_atom(&self) -> bool {
        match self {
            Expr::_Atom_(Atom::_Expr_(_)) | Expr::_Minus_(Atom::_Expr_(_)) => false,
            Expr::_Atom_(_) | Expr::_Minus_(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_real_and(&self, f: impl Fn(Real) -> bool) -> bool {
        self.as_real().is_some_and(f)
    }


    pub fn has_negative_sign(&self) -> bool {
        matches!(self, Expr::_Minus_(_))
    }


    #[inline]
    pub fn as_mut_pow(&mut self) -> (&mut Atom, &mut Atom) {
        match self {
            Expr::_Pow_([base, exp]) => (base, exp),
            _ => {
                let orig = self.replace(Expr::Pow(Atom::None(), Atom::U32(1)));
                self.operands_mut()[0] = Atom::Expr(orig);
                self.as_mut_pow()
            }
        }
    }
}

/// # Modifiers
impl Expr {

    #[log_fn]
    pub fn add_with(&mut self, mut rhs: Expr, strat: AddStrategy) -> &mut Expr {
        if matches!(strat, AddStrategy::Frozen) {
            *self = Expr::Sum([Atom::Expr(self.remove_expr()), Atom::Expr(rhs)]);
            return self;
        }

        self.cleanup_mut();
        rhs.cleanup_mut();

        const UNDEF: Expr = Expr::Undef();
        const ZERO: Expr = Expr::U32(0);

        match (&*self, &rhs) {
            (&UNDEF, _) | (_, &UNDEF) => {
                *self = UNDEF;
                return self;
            }
            (&ZERO, _) => {
                *self = rhs;
                return self;
            }
            (_, &ZERO) => {
                return self;
            }
            (lhs, rhs) => {
                if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
                    *self = Expr::Real(lhs + rhs);
                    return self;
                }
            }
        };

        match strat {
            AddStrategy::Simple => {
                *self = match (self.remove_expr(), rhs) {
                    (Expr::_Sum_(mut p1), Expr::_Sum_(p2)) => {
                        p1.extend(p2);
                        Expr::Sum(p1)
                    }
                    (Expr::_Sum_(mut p), rhs) => {
                        p.push_back(Atom::Expr(rhs));
                        Expr::Sum(p)
                    }
                    (lhs, Expr::_Sum_(mut p)) => {
                        p.push_front(Atom::Expr(lhs));
                        Expr::Sum(p)
                    }
                    (lhs, rhs) => Expr::Sum([Atom::Expr(lhs), Atom::Expr(rhs)]),
                };
            }
            AddStrategy::Coeff => {
                todo!()
            }
            AddStrategy::Frozen => unreachable!(),
        }
        self.cleanup_mut()
    }

    #[log_fn]
    pub fn mul_with(&mut self, mut rhs: Expr, strat: MulStrategy) -> &mut Expr {
        if matches!(strat, MulStrategy::Frozen) {
            *self = Expr::Prod([Atom::Expr(self.remove_expr()), Atom::Expr(rhs)]);
            return self;
        }

        self.cleanup_mut();
        rhs.cleanup_mut();

        const UNDEF: Expr = Expr::Undef();
        const ZERO: Expr = Expr::U32(0);
        const ONE: Expr = Expr::U32(1);

        // remove potential signs and wrap result with the resulting sign
        let sign = self.remove_sign_mut() * rhs.remove_sign_mut();

        match (&*self, &rhs) {
            (&UNDEF, _) | (_, &UNDEF) => {
                *self = UNDEF;
                return self;
            }
            (&ZERO, _) | (_, &ZERO) => {
                *self = ZERO;
                return self;
            }
            (&ONE, _) | (_, &ONE) => {
                *self = rhs;
                return self.wrap_in_sign(sign);
            }
            (Expr::_Prod_(_), rhs) => {
                let Expr::_Prod_(p) = self else {
                    unreachable!();
                };

                if let Some(rhs) = rhs.as_real() {
                    if let Some(cnst) = p.front().map(|a| a.as_real()).flatten() {
                        *p.front_mut().unwrap() = Atom::Real(cnst * rhs);
                    } else {
                        p.push_front(Atom::Real(rhs))
                    }
                    return self.wrap_in_sign(sign)
                }
            }
            (lhs, Expr::_Prod_(_)) => {
                let Expr::_Prod_(p) = &mut rhs else {
                    unreachable!();
                };

                if let Some(lhs) = lhs.as_real() {
                    if let Some(cnst) = p.front().map(|a| a.as_real()).flatten() {
                        *p.front_mut().unwrap() = Atom::Real(cnst * lhs);
                    } else {
                        p.push_front(Atom::Real(lhs))
                    }
                    return self.wrap_in_sign(sign)
                }
            }
            (lhs, rhs) => {
                if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
                    *self = Expr::Real(lhs * rhs);
                    return self.wrap_in_sign(sign);
                }
            }
        };

        

        match strat {
            MulStrategy::Simple => {
                *self = match (self.remove_expr(), rhs) {
                    (Expr::_Prod_(mut p1), Expr::_Prod_(p2)) => {
                        p1.extend(p2);
                        Expr::Prod(p1)
                    }
                    (Expr::_Prod_(mut p), rhs) => {
                        p.push_back(Atom::Expr(rhs));
                        Expr::Prod(p)
                    }
                    (lhs, Expr::_Prod_(mut p)) => {
                        p.push_front(Atom::Expr(lhs));
                        Expr::Prod(p)
                    }
                    (lhs, rhs) => Expr::Prod([Atom::Expr(lhs), Atom::Expr(rhs)]),
                };
            }
            MulStrategy::Base => {
                if let Expr::_Prod_(p) = rhs {
                    if p.is_empty() {
                        *self = ZERO;
                        return self.wrap_in_sign(sign);
                    }
                    for oprnd in p {
                        self.mul_with(Expr::Atom(oprnd), MulStrategy::Base);
                    }
                } else if let Expr::_Prod_(p) = self {
                    if let Some(oprnd) = p.iter_mut().find(|a| a.view_base() == rhs.view_base()) {
                        let (_, exp) = oprnd.as_mut_expr().as_mut_pow();
                        *exp.as_mut_expr() += Expr::Atom(rhs.get_exponent());
                        exp.cleanup_indirection();
                    } else {
                        p.push_back(Atom::Expr(rhs));
                    }
                } else if self.view_base() == rhs.view_base() {
                    let (_, exp) = self.as_mut_pow();
                    *exp.as_mut_expr() += Expr::Atom(rhs.get_exponent());
                    exp.cleanup_indirection();
                } else {
                    let lhs = Atom::Expr(self.remove_expr());
                    *self = Expr::Prod([lhs, Atom::Expr(rhs)]);
                }
                self.cleanup_mut();
            }
            MulStrategy::Expand => match (&*self, rhs) {
                (Expr::_Sum_(_), rhs) => {
                    let mut sum = Expr::Sum([]);

                    for term in self.drain_operands(..).unwrap() {
                        let mut prod = Expr::Atom(term);
                        prod.mul_with(rhs.clone(), MulStrategy::Expand);
                        sum += prod;
                    }

                    *self = sum;
                }
                (_, Expr::_Sum_(oprnds)) => {
                    let mut sum = Expr::Sum([]);
                    for term in oprnds {
                        let mut prod = self.clone();
                        prod.mul_with(Expr::Atom(term), MulStrategy::Expand);
                        sum += prod;
                    }

                    *self = sum;
                }
                (_, rhs) => {
                    self.mul_with(rhs, MulStrategy::Base);
                }
            },
            MulStrategy::Frozen => unreachable!(),
        }
        self.wrap_in_sign(sign)
    }

    #[log_fn]
    pub fn pow_with(&mut self, mut expon: Expr, strat: PowStrategy) -> &mut Expr {
        if matches!(strat, PowStrategy::None) {
            *self = Expr::Pow(Atom::Expr(self.remove_expr()), Atom::Expr(expon));
            return self;
        }

        const UNDEF: Expr = Expr::Undef();
        const ONE: Expr = Expr::U32(1);
        const ZERO: Expr = Expr::U32(0);

        self.cleanup_mut();
        expon.cleanup_mut();

        match (&*self, &expon) {
            (&ZERO, &ZERO) | (&UNDEF, _) | (_, &UNDEF) => {
                *self = Expr::Undef();
                return self;
            }
            (&ZERO, e) if e.is_real_and(Real::is_negative) => {
                *self = UNDEF;
                return self;
            }
            (&ZERO, e) if e.is_real_and(Real::is_positive) => {
                *self = ZERO;
                return self;
            }
            (_, &ONE) => return self,
            (lhs, rhs) => {
                if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
                    *self = Expr::Real(lhs.pow(rhs));
                    return self;
                }
            }
        };

        match strat {
            PowStrategy::Simple => {
                *self = Expr::Pow(Atom::Expr(self.remove_expr()), Atom::Expr(expon))
            }
            PowStrategy::Expand => {
                match (&*self, &expon) {
                    (Expr::_Pow_(_), _) => {
                        let (b, e) = self.remove_expr().get_base_exponent();
                        let mut e = Expr::Atom(e);
                        e.mul_with(expon, MulStrategy::Expand);
                        let mut pow = Expr::Atom(b);
                        pow.pow_with(e, PowStrategy::Expand);
                        *self = pow;
                    }
                    (Expr::_Prod_(_), _) => {
                        let mut prod = Expr::U32(1);
                        for op in self.drain_operands(..).unwrap() {
                            let mut pow = Expr::Atom(op);
                            pow.pow_with(expon.clone(), PowStrategy::Expand);
                            prod.mul_with(pow, MulStrategy::Expand);
                        }
                    }
                    (Expr::_Sum_(_), Expr::_Atom_(Atom::_U32_(n))) if *n > 1 => {
                        let n = *n;
                        let mut sum = Expr::U32(0);

                        // oprnds = term + rest
                        let mut oprnds = self.remove_nary_operands().unwrap();

                        let term = oprnds
                            .pop_front()
                            .map(|o| Expr::Atom(o))
                            .expect("called clean_expr before");

                        let mut rest = Expr::Sum(oprnds);
                        rest.inline_trivial_compound();

                        for k in 0..=n {
                            if k == 0 {
                                // term^n * 1
                                let mut a = term.clone();
                                a.pow_with(expon.clone(), PowStrategy::Expand);
                                sum += a
                            } else if k == n {
                                // 1 * rest^n
                                let mut b = rest.clone();
                                b.pow_with(expon.clone(), PowStrategy::Expand);
                                sum += b;
                            } else {
                                // binom(n, k) * term^k * rest^(n - k)
                                let c = num::integer::binomial(n, k);
                                let mut a = term.clone();
                                let mut b = rest.clone();

                                a.pow_with(Expr::U32(k), PowStrategy::Expand);
                                b.pow_with(Expr::U32(n - k), PowStrategy::Expand);

                                a.mul_with(Expr::U32(c), MulStrategy::Expand)
                                    .mul_with(b, MulStrategy::Expand);
                                sum += a;
                            }
                        }

                        *self = sum;
                    }
                    _ => {
                        *self = Expr::Pow(Atom::Expr(self.remove_expr()), Atom::Expr(expon));
                    }
                }
            }
            PowStrategy::None => unreachable!(),
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

    /// Performs simple sign simplifications: -0 -> 0, -undef -> undef, --x -> x
    #[inline]
    pub fn clean_sign(&mut self) -> &mut Self {
        match self {
            Expr::_Minus_(Atom::_Undef_) => *self = Expr::Undef(),
            Expr::_Minus_(Atom::_U32_(0)) => *self = Expr::Undef(),
            Expr::_Minus_(Atom::_Expr_(expr)) if expr.has_negative_sign() => {
                let mut inner = Rc::make_mut(expr).remove_expr();
                inner.flip_sign();
                *self = inner;
            }
            _ => (),
        }
        self
    }

    #[inline]
    pub fn flip_sign(&mut self) -> &mut Self {
        match self {
            Expr::_Atom_(Atom::_U32_(0)) => (),
            Expr::_Minus_(atom) => {
                let tmp = std::mem::replace(atom, Atom::None());
                *self = Expr::Atom(tmp);
            }
            _ => {
                *self = Expr::Minus(Atom::Expr(self.remove_expr()));
            }
        }

        self
    }

    #[inline]
    fn wrap_in_sign(&mut self, s: Sign) -> &mut Self {
        let sign = self.remove_sign_mut() * s;
        match sign {
            Sign::Minus => *self = Expr::Minus(Atom::Expr(self.remove_expr())),
            Sign::Plus => (),
        }
        self
    }

    #[inline]
    pub fn replace(&mut self, e: Expr) -> Expr {
        std::mem::replace(self, e)
    }

    #[inline]
    pub fn expand_root_mut(&mut self) -> &mut Expr {
        match self.cleanup_mut() {
            Expr::_Atom_(_) | Expr::_Sum_(_) => (),

            Expr::_Minus_(_) => {
                self.operands_mut().iter_mut().for_each(|op| {
                    op.as_mut_expr().flip_sign();
                    op.cleanup_indirection();
                });
            }

            Expr::_Prod_(oprnds) => {
                let mut prod = Expr::U32(1);
                for op in oprnds.drain(..) {
                    prod.mul_with(Expr::Atom(op), MulStrategy::Expand);
                }
                *self = prod;
            }

            Expr::_Pow_(_) => {
                let (base, expon) = self.remove_expr().get_base_exponent();
                let mut pow = Expr::Atom(base);
                pow.pow_with(Expr::Atom(expon), PowStrategy::Expand);
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

    /// Removes the sign of the current expression and returns it
    ///
    /// If `self` is [`Expr::Minus`] replace it with the inner expression and
    /// return [`Sign::Negative`] otherwise return [`Sign::Positive`]
    #[inline]
    pub fn remove_sign_mut(&mut self) -> Sign {
        match self {
            Expr::_Minus_(atom) => {
                let a = std::mem::replace(atom, Atom::None());
                *self = Expr::Atom(a);
                Sign::Minus
            }
            _ => Sign::Plus,
        }
    }

    #[inline]
    fn remove_expr(&mut self) -> Expr {
        std::mem::replace(self, Expr::Atom(Atom::None()))
    }

    #[inline]
    fn remove_bin_operands(&mut self) -> Option<[Atom; 2]> {
        match self {
            // Expr::Sub(oprnds) | Expr::Div(oprnds) |
            Expr::_Pow_(oprnds) => Some(std::mem::replace(oprnds, [Atom::None(), Atom::None()])),
            _ => None,
        }
    }

    #[inline]
    fn remove_nary_operands(&mut self) -> Option<FlatDeque<Atom>> {
        match &*self {
            Expr::_Sum_(_) | Expr::_Prod_(_) => (),
            _ => return None,
        }
        let tmp = self.remove_expr();
        match tmp {
            Expr::_Sum_(oprnds) | Expr::_Prod_(oprnds) => Some(oprnds),
            _ => unreachable!(),
        }
    }

    /// Use [`Expr::take_exponent`] if possible
    ///
    /// if `self` is `Expr::Pow(base, expon)` replace `expon` with [`Atom::None`] and return it
    #[inline]
    fn remove_exponent(&mut self) -> Option<Atom> {
        match self {
            Expr::_Pow_([_, expon]) => Some(std::mem::replace(expon, Atom::None())),
            _ => None,
        }
    }


    fn drain_operands<R>(&mut self, range: R) -> Option<crate::flat_deque::Drain<Atom>>
    where
        R: ops::RangeBounds<usize>,
    {
        match self {
            Expr::_Sum_(oprnds) | Expr::_Prod_(oprnds) => Some(oprnds.drain(range)),
            _ => None,
        }
    }

    /// Simplifies trivial compound expressions
    ///
    /// Inline n-ary operations as long as equivalency is maintained
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use noctua::{Expr, Atom};
    ///
    /// let mut x = Expr::Var("x");
    /// let mut s = Expr::Sum([Atom::Expr(x.clone())]);
    /// s.inline_trivial_compound();
    /// assert_eq!(s, x);
    ///
    /// let mut s = Expr::Sum([]);
    /// s.inline_trivial_compound();
    /// assert_eq!(s, Expr::U32(0));
    ///
    /// let mut p = Expr::Prod([]);
    /// p.inline_trivial_compound();
    /// assert_eq!(p, Expr::U32(1));
    /// ```
    pub fn inline_trivial_compound(&mut self) -> &mut Expr {
        match self {
            Expr::_Sum_(oprnds) if oprnds.is_empty() => {
                *self = Expr::U32(0);
            }
            Expr::_Prod_(oprnds) if oprnds.is_empty() => {
                *self = Expr::U32(1);
            }
            Expr::_Sum_(oprnds) | Expr::_Prod_(oprnds) if oprnds.len() == 1 => {
                *self = Expr::Atom(oprnds.pop_front().unwrap());
            }
            _ => (),
        }
        self
    }

    /// Perform basic simplifications on the outermost expression
    #[inline]
    pub fn cleanup_mut(&mut self) -> &mut Expr {
        self.inline_trivial_compound().clean_sign()
        // .rewrite_sub()
        // .rewrite_div()
    }

    /// Perform basic simplifications on the outermost expression
    #[inline]
    pub fn cleanup(mut self) -> Expr {
        self.cleanup_mut();
        self
    }

}


/// # Misc
impl Expr {
    pub fn cmp_expression_type(&self, other: &Expr) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
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

        const MINUS_ONE: &Expr = &Expr::_Minus_(Atom::U32(1));

        fn expr_view(e: &Expr) -> impl Iterator<Item = View<'_>> {
            [e.view()].into_iter()
        }

        fn minus_view(e: &Expr) -> impl Iterator<Item = View<'_>> {
            [MINUS_ONE.view()].into_iter().chain(e.operands_view())
        }

        if lhs == rhs {
            return EQ;
        } else if lhs.cmp_expression_type(rhs) {
            return cmp_views(lhs.operands_view(), rhs.operands_view());
        } else if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
            return lhs.cmp(&rhs);
        }

        const UNDEF: Expr = Expr::Undef();

        match (lhs, rhs) {
            (&UNDEF, _) => GE,
            (_, &UNDEF) => LE,

            // treat non-sum element as if it were a product with a single operand and compare
            (Expr::_Sum_(_), _) => cmp_views(lhs.operands_view(), expr_view(rhs)),
            (_, Expr::_Sum_(_)) => cmp_views(expr_view(lhs), rhs.operands_view()),

            // treat non-product element as if it were a product with a single operand and compare
            (_, Expr::_Prod_(_)) => cmp_views(expr_view(lhs), rhs.operands_view()),
            (Expr::_Prod_(_), _) => cmp_views(lhs.operands_view(), expr_view(rhs)),

            // treat minus as -1 * ... and commpare like the product
            (Expr::_Minus_(_), _) => cmp_views(expr_view(lhs), minus_view(rhs)),
            (_, Expr::_Minus_(_)) => cmp_views(minus_view(lhs), expr_view(rhs)),

            // treat non-power expressions as if they had an exponent of 1
            (Expr::_Pow_(_), _) | (_, Expr::_Pow_(_)) => {
                let (b1, e1) = (lhs.view_base(), lhs.view_exponent());
                let (b2, e2) = (rhs.view_base(), rhs.view_exponent());

                if b1 != b2 {
                    cmp_views([b1].into_iter(), [b2].into_iter())
                } else {
                    cmp_views([e1].into_iter(), [e2].into_iter())
                }
            }

            (Expr::_Atom_(a1), Expr::_Atom_(a2)) => a1.simplified_ordering(a2),
            // (_, Expr::Atom(_)) => LE,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum View<'a> {
    Atom(&'a Atom),
    Expr(&'a Expr),
}

impl View<'_> {
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


impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(mut self, mut rhs: Self) -> Self::Output {
        rhs *= Expr::I32(-1);
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
        self *= Expr::Pow(Atom::Expr(rhs), Atom::Expr(Expr::I32(-1)));
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
    fn neg(self) -> Self::Output {
        match self {
            Expr::_Minus_(atom) => Expr::Atom(atom),
            expr => Expr::Minus(Atom::Expr(expr)),
        }
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
        // match self {
        //     Atom::_Undef_ => write!(f, "\u{2205}"),
        //     Atom::_U32_(u) => write!(f, "{u}u32"),
        //     Atom::_Var_(v) => write!(f, "{v}"),
        //     Atom::_Expr_(expr) => write!(f, "A[{expr:?}]"),
        // }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::_Undef_ => write!(f, "\u{2205}"),
            Atom::_U32_(val) => write!(f, "{val}"),
            Atom::_Var_(var) => write!(f, "{var}"),
            Atom::_Expr_(expr) => write!(f, "{expr}"),
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
        // match self {
        //     Expr::_Atom_(a) => write!(f, "{a:?}"),
        //     Expr::_Minus_(a) => write!(f, "MINUS[{a:?}]"),
        //     Expr::_Sum_(oprnds) => write!(f, "SUM[{:?}]", oprnds.iter().format(" + ")),
        //     Expr::_Prod_(oprnds) => write!(f, "PROD[{:?}]", oprnds.iter().format(" * ")),
        //     Expr::_Pow_([base, exp]) => write!(f, "POW[{base:?} ^ {exp:?}]"),
        // }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::_Atom_(atom) => write!(f, "{atom}"),
            Expr::_Minus_(Atom::_Expr_(expr)) => write!(f, "-({expr})"),
            Expr::_Minus_(atom) => write!(f, "-{atom}"),
            Expr::_Sum_(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[+]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[+{}]", oprnds[0]);
                }
                write!(f, "[{}]", oprnds.iter().format(" + "))
            }
            Expr::_Prod_(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[*]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[*{}]", oprnds[0]);
                }
                write!(f, "[{}]", oprnds.iter().format(" * "))
            }
            Expr::_Pow_([lhs, rhs]) => {
                if matches!(lhs, Atom::_Expr_(_)) {
                    write!(f, "({lhs})^")?;
                } else {
                    write!(f, "{lhs}^")?;
                }
                if matches!(rhs, Atom::_Expr_(_)) {
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
            View::Atom(a) => write!(f, "{a:?}"),
            View::Expr(e) => write!(f, "{e:?}"),
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
    fn sort_args() {
        let checks = vec![
            n!(a + b),
            n!(b * c + a),
            n!(sin(x) * cos(x)),
            n!(a * x ^ 2 + b * x + c + 3),
        ];

        for c in checks {

            assert_eq!(c.sort_args(), c)
        }
    }
}
