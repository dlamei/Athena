use std::cmp;
use std::{cell::UnsafeCell, collections::VecDeque, fmt, ops, rc::Rc};

use itertools::Itertools;

use crate::config::{AddStrategy, MulStrategy, PowStrategy, noctua_global_config};
use crate::flat_deque::FlatDeque;
use crate::real::{Real, Sign};



mod seal {
    use super::*;

    /// Prevents the construction of [`Expr::Atom`] and [`Atom::Expr`] outside this module
    #[derive(Debug, Clone, PartialEq)]
    pub struct ExprAtomSeal {
        private: (),
    }

    impl ExprAtomSeal {
        const fn new() -> Self {
            Self { private: () }
        }
    }

    /// Represents an atomic unit: simple values or sub-expressions
    ///
    /// - Inline variants like undef or integers
    /// - Compound expressions used as part of an [`Expr`]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Atom {
        Undef,
        U32(u32),
        Var(Rc<str>),

        // IMPORTANT: if the Atom is part of an `Expr` this should only be used if the atom is part
        // of a compound expression. Otherwise promote the `Expr::Atom(Atom::Expr(..))` to `Expr`.
        // In other words when encountering `Expr::Atom(atom)` atom must not be an `Atom::Expr`
        Expr(Rc<Expr>, ExprAtomSeal),
    }

    impl Atom {
        #[inline]
        pub const fn to_const_expr(self) -> Expr {
            match self {
                Atom::Expr(_, _) => panic!("can't promote Atom::Expr in const fn"),
                _ => Expr::Atom(self, ExprAtomSeal::new()),
            }
        }

        #[inline]
        pub fn to_expr(self) -> Expr {
            match self {
                Atom::Expr(expr, _) => Rc::unwrap_or_clone(expr),
                _ => Expr::Atom(self, ExprAtomSeal::new()),
            }
        }

        /// IMPORTANT: Should only be called on atoms that are part of a compount expression
        ///
        /// If this atom is not already wrapped as an Expr, promote it; then return a `&mut Expr`
        /// for in-place mutation
        #[inline]
        pub(crate) fn promote_to_expr(&mut self) -> &mut Expr {
            if let Atom::Expr(rc, _) = self {
                return Rc::make_mut(rc);
            }
            // take it out:
            let orig = std::mem::replace(self, Atom::Undef);
            *self = Atom::Expr(orig.to_expr().into(), ExprAtomSeal::new());

            match self {
                Atom::Expr(expr, _) => Rc::get_mut(expr).expect("no other ptr should exist"),
                _ => unreachable!(),
            }
        }

        /// IMPORTANT: Should only be called on atoms that are part of a compount expression
        ///
        /// Ensure this atom is an expr, then apply `f`
        #[inline]
        pub(crate) fn with_expr_mut<'a, T, F>(&'a mut self, f: F) -> T
        where
            F: FnOnce(&'a mut Expr) -> T + 'a,
        {
            f(self.promote_to_expr())
        }
    }

    /// Expression composed of [`Atom`] units and operations.
    ///
    /// This design allows to store simple expressions with very little overhead.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Expr {
        Atom(Atom, ExprAtomSeal),

        /// Is used to represent negative values
        ///
        /// Will be interpreted as the expression -1 * [`Atom`]
        /// For cohesion negative integers are represented as `Expr::Minus(Atom::U32(0))`
        Minus(Atom),

        Sum(FlatDeque<Atom>),
        Prod(FlatDeque<Atom>),

        Sub([Atom; 2]),
        Div([Atom; 2]),
        Pow([Atom; 2]),
    }

    impl Expr {
        pub fn to_atom(self) -> Atom {
            match self {
                Expr::Atom(atom, _) => atom,
                _ => Atom::Expr(self.into(), ExprAtomSeal::new()),
            }
        }
    }
}

pub use seal::{Atom, Expr};

mod ordering_abbreviations {
    pub const GE: std::cmp::Ordering = std::cmp::Ordering::Greater;
    pub const LE: std::cmp::Ordering = std::cmp::Ordering::Less;
    pub const EQ: std::cmp::Ordering = std::cmp::Ordering::Equal;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View<'a> {
    Atom(&'a Atom),
    Expr(&'a Expr),
}


impl View<'_> {
    pub fn simplified_ordering(&self, other: &Self) -> cmp::Ordering {
        use ordering_abbreviations::*;
        debug_assert!(!matches!(self, Self::Expr(Expr::Atom(_, _))));
        debug_assert!(!matches!(other, Self::Expr(Expr::Atom(_, _))));

        if self == other {
            return EQ;
        }

        match (self, other) {
            (View::Atom(a1), View::Atom(a2)) => a1.simplified_ordering(a2),
            (View::Atom(_), _) => GE,
            (_, View::Atom(_)) => LE,

            (View::Expr(e1), View::Expr(e2)) => e1.simplified_ordering(e2),
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Undef => write!(f, "\u{2205}"),
            Atom::U32(val) => write!(f, "{val}"),
            Atom::Var(var) => write!(f, "{}", *var),
            Atom::Expr(expr, _) => write!(f, "{expr}"),
        }
    }
}

impl Atom {
    pub fn view(&self) -> View<'_> {
        match self {
            Atom::Expr(e, _) => e.view(),
            _ => View::Atom(self)
        }
    }

    pub fn expand(&mut self) {
        match self {
            Atom::Undef | Atom::U32(_) | Atom::Var(_) => (),
            Atom::Expr(expr, _) => {
                Rc::make_mut(expr).expand_mut();
            }
        }
    }

    pub fn base_view(&self) -> View<'_> {
        match self {
            Atom::Expr(expr, _) => expr.base_view(),
            _ => View::Atom(self),
        }
    }

    pub fn exponent_view(&self) -> View<'_> {
        match self {
            Atom::Expr(expr, _) => expr.exponent_view(),
            _ => View::Atom(self),
        }
    }

    pub fn base_exponent_view(&self) -> (View<'_>, View<'_>) {
        (self.base_view(), self.exponent_view())
    }

    pub fn is_expr(&self) -> bool {
        matches!(self, Atom::Expr(_, _))
    }

    /// IMPORTANT: Should only be called on atoms that are part of a compount expression.
    ///
    /// Wrap the atom in [`Expr::Pow`] with the atom as base, returning a mutable reference to the
    /// exponent atom
    fn atom_exponent_mut(&mut self) -> &mut Atom {
        self.with_expr_mut(|a| a.exponent_atom_mut())
    }

    /// Order of the expressions in simplified form
    /// 
    /// we use simple lexicological ordering when comparing equal variants.
    /// The variants themselves are ordered: [`Atom::Undef`], [`Atom::U32`], [`Atom::Var`], [`Atom::Expr`]
    pub fn simplified_ordering(&self, other: &Atom) -> cmp::Ordering {
        use ordering_abbreviations::*;

        if self == other {
            return EQ
        }
        match (self, other) {
            (Atom::Undef, _) => GE,
            (_, Atom::Undef) => LE,

            (Atom::U32(u1), Atom::U32(u2)) => u1.cmp(u2),
            (Atom::U32(u1), _) => GE,
            (_, Atom::U32(u1)) => LE,

            (Atom::Var(v1), Atom::Var(v2)) => v1.cmp(v2),
            (Atom::Var(_), _) => GE,
            (_, Atom::Var(_)) => LE,

            (Atom::Expr(lhs, _), Atom::Expr(rhs, _)) => lhs.simplified_ordering(rhs),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Atom(atom, _) => write!(f, "{atom}"),
            Expr::Minus(Atom::Expr(expr, _)) => write!(f, "-({expr})"),
            Expr::Minus(atom) => write!(f, "-{atom}"),
            Expr::Sum(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[+]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[+{}]", oprnds[0]);
                }
                write!(f, "[{}]", oprnds.iter().format(" + "))
            }
            Expr::Prod(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[*]");
                } else if oprnds.len() == 1 {
                    return write!(f, "[*{}]", oprnds[0]);
                }
                write!(f, "[{}]", oprnds.iter().format(" * "))
            }
            Expr::Sub([lhs, rhs]) => {
                write!(f, "{lhs} - ")?;
                if matches!(rhs, Atom::Expr(_, _)) {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            }
            Expr::Div([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_, _)) {
                    write!(f, "({lhs})/")?;
                } else {
                    write!(f, "{lhs}/")?;
                }
                if matches!(rhs, Atom::Expr(_, _)) {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            }
            Expr::Pow([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_, _)) {
                    write!(f, "({lhs})^")?;
                } else {
                    write!(f, "{lhs}^")?;
                }
                if matches!(rhs, Atom::Expr(_, _)) {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            }
        }
    }
}

impl Expr {
    #[inline]
    pub const fn u32(val: u32) -> Expr {
        Atom::U32(val).to_const_expr()
    }

    pub const fn i32(val: i32) -> Expr {
        let atom = Atom::U32(val.unsigned_abs());
        if val < 0 {
            Expr::Minus(atom)
        } else {
            atom.to_const_expr()
        }
    }

    pub const fn undef() -> Expr {
        Atom::Undef.to_const_expr()
    }

    pub fn var(var: impl AsRef<str>) -> Expr {
        Atom::Var(var.as_ref().into()).to_const_expr()
    }

    pub fn pow(mut self, exp: Expr) -> Expr {
        self.pow_with(exp, noctua_global_config().default_pow_strategy);
        self
    }

    pub fn view(&self) -> View<'_> {
        match self {
            Expr::Atom(atom, _) => atom.view(),
            expr => View::Expr(expr),
        }
    }


    /// Removes the sign of the current expression and returns it
    ///
    /// If `self` is [`Expr::Minus`] replace it with the inner expression and 
    /// return [`Sign::Negative`] otherwise return [`Sign::Positive`]
    #[inline]
    pub fn remove_sign_mut(&mut self) -> Sign {
        match self {
            Expr::Minus(atom) => {
                let a = std::mem::replace(atom, Atom::Undef);
                *self = a.to_expr();
                Sign::Minus
            }
            _ => Sign::Plus,
        }
    }

    /// Similar to [`Expr::remove_sign_mut`], but will not modify the expression,
    /// returning a view of the unsigned expressions instead.
    #[inline]
    pub fn unsigned_view(&self) -> (Sign, View<'_>) {
        match self {
            Expr::Minus(atom) => (Sign::Minus, atom.view()),
            _ => (Sign::Plus, self.view())
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
            View::Atom(&Atom::U32(v)) => Some(Real::u32_with_sign(sign, v)),
            _ => None,
        }
    }

    #[inline]
    pub fn operands(&self) -> &[Atom] {
        match self {
            Expr::Atom(atom, _) | Expr::Minus(atom) => std::slice::from_ref(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }

    pub fn operands_view<'a>(&'a self) -> impl Iterator<Item = View<'a>> {
        self.operands().iter().map(|o| o.view())
    }

    pub fn n_operands(&self) -> usize {
        self.operands().len()
    }

    #[inline]
    pub fn operands_mut(&mut self) -> &mut [Atom] {
        match self {
            Expr::Atom(atom, _) | Expr::Minus(atom) => std::slice::from_mut(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_mut_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }

    #[inline]
    fn swap_with_undef(&mut self) -> Expr {
        std::mem::replace(self, Expr::undef())
    }

    #[inline]
    fn swap_bin_operands_with_undef(&mut self) -> Option<[Atom; 2]> {
        match self {
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => {
                Some(std::mem::replace(oprnds, [Atom::Undef, Atom::Undef]))
            }
            _ => None,
        }
    }

    #[inline]
    fn swap_nary_operands_with_undef(&mut self) -> Option<FlatDeque<Atom>> {
        match &*self {
            Expr::Sum(_) | Expr::Prod(_) => (),
            _ => return None,
        }
        let tmp = self.swap_with_undef();
        match tmp {
            Expr::Sum(oprnds) | Expr::Prod(oprnds) => Some(oprnds),
            _ => unreachable!(),
        }
    }

    /// Use [`Expr::take_exponent`] if possible
    ///
    /// if `self` is `Expr::Pow([base, expon])` replace `expon` with [`Atom::Undef`] and return it
    fn swap_exponent_with_undef(&mut self) -> Option<Atom> {
        match self {
            Expr::Pow([_, expon]) => Some(std::mem::replace(expon, Atom::Undef)),
            _ => None,
        }
    }

    /// if `self` is [`Expr::Pow`] return (base, exponent) otherwise (`self`, 1)
    #[inline]
    fn take_base_exponent(self) -> (Atom, Atom) {
        match self {
            Expr::Pow([base, expon]) => (base, expon),
            _ => (self.to_atom(), Atom::U32(1)),
        }
    }

    /// if `self` is [`Expr::Pow`] return the exponent otherwise return 1
    #[inline]
    fn take_exponent(self) -> Atom {
        self.take_base_exponent().1
    }

    fn drain_operands<R>(&mut self, range: R) -> Option<crate::flat_deque::Drain<Atom>>
    where
        R: ops::RangeBounds<usize>,
    {
        match self {
            Expr::Sum(oprnds) | Expr::Prod(oprnds) => Some(oprnds.drain(range)),
            _ => None,
        }
    }

    pub fn base_view(&self) -> View<'_> {
        match self {
            Expr::Atom(_, _)
            | Expr::Minus(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => self.view(),
            Expr::Pow([base, _]) => View::Atom(base),
        }
    }

    pub fn exponent_view(&self) -> View<'_> {
        View::Atom(
            match self {
            Expr::Atom(_, _)
            | Expr::Minus(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => &Atom::U32(1),
            Expr::Pow([_, expon]) => expon,
        })
    }

    pub fn base_exponent_view(&self) -> (View<'_>, View<'_>) {
        (self.base_view(), self.exponent_view())
    }

    /// returns a mutable reference to the exponent [`Atom`]
    ///
    /// Will wrap self in [`Expr::Pow`] with exponent = 1, if needed
    pub fn exponent_atom_mut(&mut self) -> &mut Atom {
        match self {
            Expr::Atom(_, _)
            | Expr::Minus(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => {
                // self.pow(Atom::U32(1));
                *self = Expr::Pow([self.swap_with_undef().to_atom(), Atom::U32(1)]);
                &mut self.operands_mut()[1]
            }
            Expr::Pow([base, _]) => base,
        }
    }

    pub fn cmp_expression_type(&self, other: &Expr) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }


    /// Order of the expressions in simplified form
    /// 
    pub fn simplified_ordering(&self, other: &Expr) -> cmp::Ordering {
        use ordering_abbreviations::*;

        let (lhs,  rhs) = (self, other);

        fn cmp_views<'a>(lhs: impl Iterator<Item = View<'a>>, rhs: impl Iterator<Item = View<'a>>) -> cmp::Ordering {
            let (mut l_iter, mut r_iter) = (lhs.into_iter(), rhs.into_iter());

            while let (Some(l), Some(r)) = (l_iter.next(), r_iter.next()) {
                if l != r {
                    return l.simplified_ordering(&r);
                }
            }

            match (l_iter.next(), r_iter.next()) {
                (Some(_), None) => cmp::Ordering::Greater,
                (None, Some(_)) => cmp::Ordering::Less,
                _ => cmp::Ordering::Equal,
            }
        }

        fn expr_view<'a>(e: &'a Expr) -> impl Iterator<Item = View<'a>> {
            [e.view()].into_iter()
        }


        const MINUS_ONE: Expr = Expr::i32(-1);

        if lhs == rhs {
            return EQ
        }  else if lhs.cmp_expression_type(rhs) {
            return cmp_views(lhs.operands_view(), rhs.operands_view())
        } else if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
            return lhs.cmp(&rhs)
        } else if let Some(lhs) = lhs.as_real() {
            return LE
        } else if let Some(rhs) = rhs.as_real() {
            return GE
        }

        const UNDEF: Expr = Expr::undef();

        match (lhs, rhs) {
            (&UNDEF, _) => GE,
            (_, &UNDEF) => LE,

            (Expr::Minus(_), _) => {
                cmp_views([MINUS_ONE.view()].into_iter().chain(lhs.operands_view()), expr_view(rhs))
            },

            (Expr::Sum(_), _) => cmp_views(lhs.operands_view(), expr_view(rhs)),
            (_, Expr::Sum(_)) => cmp_views(expr_view(lhs), rhs.operands_view()),

            (Expr::Prod(_), _) => cmp_views(lhs.operands_view(), expr_view(rhs)),
            (_, Expr::Prod(_)) => cmp_views(expr_view(lhs), rhs.operands_view()),

            (Expr::Pow(_), _) | (_, Expr::Pow(_)) => {
                let (b1, e1) = lhs.base_exponent_view();
                let (b2, e2) = rhs.base_exponent_view();

                if b1 != b2 {
                    b1.simplified_ordering(&b2)
                } else {
                    e1.simplified_ordering(&e2)
                }
            },


            (_, Expr::Minus(_)) if lhs.root_sign().is_plus() => GE,
            (Expr::Minus(_), _) if rhs.root_sign().is_plus() => LE,
            (Expr::Minus(_), Expr::Minus(_)) => panic!("should have been handled"),

            (Expr::Atom(_, _), _) => GE,
            (_, Expr::Atom(_, _)) => LE,


            (Expr::Sub(_), _) => GE,
            (_, Expr::Sub(_)) => LE,

            (Expr::Prod(_), _) => GE,
            (_, Expr::Prod(_)) => LE,

            (Expr::Div(_), _) => GE,
            (_, Expr::Div(_)) => LE,
        }
    }

    /// Simplifies trivial compound expressions
    ///
    /// Inline n-ary operations as long as equivalency is maintained
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use noctua::Expr;
    ///
    /// let x = Expr::var("x");
    /// let mut s = Expr::Sum([x.clone().to_atom()].into());
    /// s.inline_trivial_compound();
    /// assert_eq!(s, x);
    ///
    /// let mut s = Expr::Sum([].into());
    /// s.inline_trivial_compound();
    /// assert_eq!(s, Expr::u32(0));
    ///
    /// let mut p = Expr::Prod([].into());
    /// p.inline_trivial_compound();
    /// assert_eq!(p, Expr::u32(1));
    /// ```
    pub fn inline_trivial_compound(&mut self) -> &mut Expr {
        match self {
            Expr::Sum(oprnds) if oprnds.is_empty() => {
                *self = Expr::u32(0);
            }
            Expr::Prod(oprnds) if oprnds.is_empty() => {
                *self = Expr::u32(1);
            }
            Expr::Sum(oprnds) | Expr::Prod(oprnds) if oprnds.len() == 1 => {
                *self = oprnds.pop_front().unwrap().to_expr();
            }
            _ => (),
        }
        self
    }

    /// Perform basic simplifications on the outermost expression
    #[inline]
    pub fn cleanup_mut(&mut self) -> &mut Expr {
        self.inline_trivial_compound()
            .clean_sign()
            .rewrite_sub()
            .rewrite_div()
    }

    /// Perform basic simplifications on the outermost expression
    #[inline]
    pub fn cleanup(mut self) -> Expr {
        self.cleanup_mut();
        self
    }

    #[inline]
    pub fn is_unsigned_atom(&self) -> bool {
        match self {
            Expr::Atom(Atom::Expr(_, _), _) => false,
            Expr::Atom(_, _) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_signed_atom(&self) -> bool {
        match self {
            Expr::Atom(Atom::Expr(_, _), _) | Expr::Minus(Atom::Expr(_, _)) => false,
            Expr::Atom(_, _) | Expr::Minus(_) => true,
            _ => false,
        }
    }

    pub fn is_negative_val(&self) -> bool {
        self.as_real().is_some_and(|r| r.is_negative())
    }

    pub fn is_positive_val(&self) -> bool {
        self.as_real().is_some_and(|r| r.is_positive())
    }

    pub fn pow_with(&mut self, mut expon: Expr, strat: PowStrategy) -> &mut Expr {
        if matches!(strat, PowStrategy::None) {
            *self = Expr::Pow([self.swap_with_undef().to_atom(), expon.to_atom()]);
            return self;
        }

        const UNDEF: Expr = Expr::undef();
        const ONE: Expr = Expr::u32(1);
        const ZERO: Expr = Expr::u32(0);

        self.cleanup_mut();
        expon.cleanup_mut();

        match (&*self, &expon) {
            (&ZERO, &ZERO) | (&UNDEF, _) | (_, &UNDEF) => {
                *self = Expr::undef();
                return self;
            }
            (&ZERO, e) if e.is_negative_val() => {
                *self = UNDEF;
                return self;
            }
            (&ZERO, e) if e.is_positive_val() => {
                *self = ZERO;
                return self;
            }
            (_, &ONE) => {
                return self
            }
            (lhs, rhs) => {
                if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
                    *self = lhs.pow(rhs).to_expr();
                    return self
                }
            }
        };


        match strat {
            PowStrategy::Simple => {
                *self = Expr::Pow([self.swap_with_undef().to_atom(), expon.to_atom()])
            }
            PowStrategy::Expand => {
                match (&*self, &expon) {
                    (Expr::Pow(_), _) => {
                        let (b, e) = self.swap_with_undef().take_base_exponent();
                        let mut e = e.to_expr();
                        e.mul_with(expon, MulStrategy::Expand);
                        let mut pow = b.to_expr();
                        pow.pow_with(e, PowStrategy::Expand);
                        *self = pow;
                        // self.exponent_atom_mut().promote_to_expr().mul_with(expon, MulStrategy::Expand);
                    }
                    (Expr::Prod(_), _) => {
                        let mut prod = Expr::u32(1);
                        for op in self.drain_operands(..).unwrap() {
                            let mut pow = op.to_expr();
                            pow.pow_with(expon.clone(), PowStrategy::Expand);
                            prod.mul_with(pow, MulStrategy::Expand);
                        }
                    }
                    (Expr::Sum(_), Expr::Atom(Atom::U32(n), _)) if *n > 1 => {
                        let n = *n;
                        let mut sum = Expr::u32(0);

                        // oprnds = term + rest
                        let mut oprnds = self.swap_nary_operands_with_undef().unwrap();

                        let term = oprnds
                            .pop_front()
                            .expect("called clean_expr before")
                            .to_expr();
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

                                a.pow_with(Expr::u32(k), PowStrategy::Expand);
                                b.pow_with(Expr::u32(n - k), PowStrategy::Expand);

                                a.mul_with(Expr::u32(c), MulStrategy::Expand)
                                    .mul_with(b, MulStrategy::Expand);
                                sum += a;
                            }
                        }

                        *self = sum;
                    }
                    _ => {
                        *self = Expr::Pow([self.swap_with_undef().to_atom(), expon.to_atom()]);
                    }
                }
            }
            PowStrategy::None => unreachable!(),
        }

        self
    }

    pub fn has_negative_sign(&self) -> bool {
        matches!(self, Expr::Minus(_))
    }

    /// Performs simple rewrites: -0 -> 0, -undef -> undef, --x -> x
    #[inline]
    pub fn clean_sign(&mut self) -> &mut Self {
        match self {
            Expr::Minus(Atom::Undef) => *self = Expr::undef(),
            Expr::Minus(Atom::U32(0)) => *self = Expr::undef(),
            Expr::Minus(Atom::Expr(expr, _)) if expr.has_negative_sign() => {
                let mut inner = Rc::make_mut(expr).swap_with_undef();
                inner.flip_sign();
                *self = inner;
            }
            _ => (),
        }
        self
    }

    pub fn flip_sign(&mut self) -> &mut Self {
        match self {
            Expr::Atom(Atom::U32(0), _) => (),
            Expr::Minus(atom) => {
                let tmp = std::mem::replace(atom, Atom::Undef);
                *self = tmp.to_expr();
            }
            _ => {
                *self = Expr::Minus(self.swap_with_undef().to_atom());
            }
        }

        self
    }

    fn wrap_in_sign(&mut self, s: Sign) -> &mut Self {
        let sign = self.remove_sign_mut() * s;
        match sign {
            Sign::Minus => *self = Expr::Minus(self.swap_with_undef().to_atom()),
            Sign::Plus => (),
        }
        self
    }

    pub fn mul_with<'a>(&'a mut self, mut rhs: Expr, strat: MulStrategy) -> &'a mut Expr {
        if matches!(strat, MulStrategy::None) {
            *self = Expr::Prod([self.swap_with_undef().to_atom(), rhs.to_atom()].into());
            return self;
        }

        self.cleanup_mut();
        rhs.cleanup_mut();

        const UNDEF: Expr = Expr::undef();
        const ZERO: Expr = Expr::u32(0);
        const ONE: Expr = Expr::u32(1);

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
            (lhs, rhs) => {
                if let (Some(lhs), Some(rhs)) = (lhs.as_real(), rhs.as_real()) {
                    *self = (lhs * rhs).to_expr();
                    return self.wrap_in_sign(sign);
                }
            }
        };

        match strat {
            MulStrategy::Simple => {
                *self = match (self.swap_with_undef(), rhs) {
                    (Self::Prod(mut p1), Self::Prod(p2)) => {
                        p1.extend(p2);
                        Self::Prod(p1)
                    }
                    (Self::Prod(mut p), rhs) => {
                        p.push_back(rhs.to_atom());
                        Self::Prod(p)
                    }
                    (lhs, Self::Prod(mut p)) => {
                        p.push_front(lhs.to_atom());
                        Self::Prod(p)
                    }
                    (lhs, rhs) => Expr::Prod([lhs.to_atom(), rhs.to_atom()].into()),
                };
            }
            MulStrategy::Base => {
                if let Expr::Prod(p) = rhs {
                    if p.is_empty() {
                        *self = ZERO;
                        return self.wrap_in_sign(sign);
                    }
                    for oprnd in p {
                        self.mul_with(oprnd.to_expr(), MulStrategy::Base);
                    }
                } else if let Expr::Prod(p) = self {
                    if let Some(oprnd) = p.iter_mut().find(|a| a.base_view() == rhs.base_view()) {
                        *oprnd
                            // swap oprnd with Pow with the oprnd as base and Atom(1) as exponent,
                            // returning a mutable reference to the exponent atom
                            .atom_exponent_mut()
                            // wrap the exponent, Atom(1), in Atom::Expr(Expr::Atom(exponent))
                            // returning a mutable reference to the inner expression(&mut Expr::Atom(exponent))
                            .promote_to_expr() += rhs.take_exponent().to_expr();
                    } else {
                        p.push_back(rhs.to_atom());
                    }
                } else if self.base_view() == rhs.base_view() {
                    *self.exponent_atom_mut().promote_to_expr() += rhs.take_exponent().to_expr()
                } else {
                    let lhs = self.swap_with_undef().to_atom();
                    *self = Expr::Prod([lhs, rhs.to_atom()].into());
                }
                self.cleanup_mut();
            }
            MulStrategy::Expand => match (&*self, rhs) {
                (Expr::Sum(_), rhs) => {
                    let mut sum = Expr::Sum([].into());

                    for term in self.drain_operands(..).unwrap() {
                        let mut prod = term.to_expr();
                        prod.mul_with(rhs.clone(), MulStrategy::Expand);
                        sum += prod;
                    }

                    *self = sum;
                }
                (_, Expr::Sum(oprnds)) => {
                    let mut sum = Expr::Sum([].into());
                    for term in oprnds {
                        let mut prod = self.clone();
                        prod.mul_with(term.to_expr(), MulStrategy::Expand);
                        sum += prod;
                    }

                    *self = sum;
                }
                (_, rhs) => {
                    self.mul_with(rhs, MulStrategy::Base);
                }
            },
            MulStrategy::None => unreachable!(),
        }
        self.wrap_in_sign(sign)
    }

    pub fn add_with<'a>(&'a mut self, mut rhs: Expr, strat: AddStrategy) -> &'a mut Expr {
        if matches!(strat, AddStrategy::None) {
            *self = Expr::Sum([self.swap_with_undef().to_atom(), rhs.to_atom()].into());
            return self;
        }

        self.cleanup_mut();
        rhs.cleanup_mut();

        const UNDEF: Expr = Expr::undef();
        const ZERO: Expr = Expr::u32(0);

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
                    *self = (lhs + rhs).to_expr();
                    return self
                }
            }
        };

        match strat {
            AddStrategy::Simple => {
                *self = match (self.swap_with_undef(), rhs) {
                    (Self::Sum(mut p1), Self::Sum(p2)) => {
                        p1.extend(p2);
                        Self::Sum(p1)
                    }
                    (Self::Sum(mut p), rhs) => {
                        p.push_back(rhs.to_atom());
                        Self::Sum(p)
                    }
                    (lhs, Self::Sum(mut p)) => {
                        p.push_front(lhs.to_atom());
                        Self::Sum(p)
                    }
                    (lhs, rhs) => Expr::Sum([lhs.to_atom(), rhs.to_atom()].into()),
                };
            }
            AddStrategy::Coeff => {
                todo!()
            }
            AddStrategy::None => unreachable!(),
        }
        self.cleanup_mut()
    }

    /// replace `a - b` with `a + (-1 * b)`
    #[inline]
    pub fn rewrite_sub(&mut self) -> &mut Expr {
        match self {
            Expr::Sub(_) => {
                let [lhs, rhs] = self.swap_bin_operands_with_undef().unwrap();
                *self = lhs.to_expr() + Expr::Minus(rhs)
            }
            _ => (),
        }
        self
    }

    /// replace `a / b` with `a * b^(-1)`
    #[inline]
    pub fn rewrite_div(&mut self) -> &mut Expr {
        match self {
            Expr::Div(_) => {
                let [numer, denom] = self.swap_bin_operands_with_undef().unwrap();
                *self = numer.to_expr() * Expr::Pow([denom, Expr::i32(-1).to_atom()]);
            }
            _ => (),
        }
        self
    }

    pub fn expand_mut(&mut self) -> &mut Expr {
        self.operands_mut().iter_mut().for_each(|a| a.expand());
        match self.cleanup_mut() {
            Expr::Atom(_, _) | Expr::Minus(_) | Expr::Sum(_) => (),
            Expr::Prod(oprnds) => {
                let mut prod = Expr::u32(1);
                for op in oprnds.drain(..) {
                    prod.mul_with(op.to_expr(), MulStrategy::Expand);
                }
                *self = prod;
            }
            Expr::Pow(_) => {
                let (base, expon) = self.swap_with_undef().take_base_exponent();
                let mut pow = base.to_expr();
                pow.pow_with(expon.to_expr(), PowStrategy::Expand);
                *self = pow;
            }
            Expr::Sub(_) | Expr::Div(_) => unreachable!("was rewritten to Sum or Prod"),
        }
        self
    }

    pub fn expand(mut self) -> Expr {
        self.expand_mut();
        self
    }
}

impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Sub([self.to_atom(), rhs.to_atom()])
        // Self::binary(Self::Sub, self, rhs)
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
        *self = self.swap_with_undef() + rhs;
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
        *self = self.swap_with_undef() * rhs;
    }
}

impl ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Self) -> Self::Output {
        Self::Div([self.to_atom(), rhs.to_atom()])
    }
}

impl ops::DivAssign for Expr {
    fn div_assign(&mut self, rhs: Self) {
        *self = self.swap_with_undef() / rhs;
    }
}

impl ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Self::Output {
        match self {
            Expr::Minus(atom) => atom.to_expr(),
            expr => Expr::Minus(expr.to_atom()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::noctua;

    #[test]
    fn simplified_ordering() {
    }
}

