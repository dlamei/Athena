use std::{cell::UnsafeCell, collections::VecDeque, fmt, ops, rc::Rc};

use flat_deque::FlatDeque;
use itertools::Itertools;

pub mod flat_deque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoctuaConfig {
    pub simplify_during_construction: bool,
}

impl Default for NoctuaConfig {
    fn default() -> Self {
        Self {
            simplify_during_construction: true,
        }
    }
}

static NOCTUA_CONFIG: once_cell::sync::Lazy<std::sync::RwLock<NoctuaConfig>> =
    once_cell::sync::Lazy::new(|| std::sync::RwLock::new(NoctuaConfig::default()));

pub struct ConfigGuard {
    old: NoctuaConfig,
}

impl ConfigGuard {
    pub fn install(new_cfg: NoctuaConfig) -> Self {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        let old = guard.clone();
        *guard = new_cfg;
        Self { old }
    }
}

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        *guard = self.old.clone();
    }
}

pub fn get_config() -> NoctuaConfig {
    NOCTUA_CONFIG.read().unwrap().clone()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtomView<'a> {
    Undef,
    U32(u32),
    Var(&'a str),
    Neg(&'a Atom),
    Sum(&'a [Atom]),
    Prod(&'a [Atom]),
    Sub(&'a [Atom; 2]),
    Div(&'a [Atom; 2]),
    Pow(&'a [Atom; 2]),
}

impl AtomView<'_> {
    pub fn is_i32(&self, val: i32) -> bool {
        match self {
            AtomView::U32(v) if val >= 0 => *v == val.unsigned_abs(),
            AtomView::Neg(Atom::U32(v)) if val < 0 => *v == val.unsigned_abs(),
            _ => false,
        }
    }

    // pub fn into_expr(&self) -> Expr {
    //     match self {
    //         AtomView::Undef => Expr::undef(),
    //         AtomView::U32(val) => Expr::Atom(Atom::U32(*val)),
    //         AtomView::Var(var) => Expr::Atom(Atom::Var((*var).into())),
    //         AtomView::Neg(val) => Expr::Neg((**val).clone()),
    //         AtomView::Sum(oprnds) => Expr::sum(oprnds.into_iter().cloned()),
    //         AtomView::Prod(oprnds) => Expr::prod(oprnds.into_iter().cloned()),
    //         AtomView::Sub(oprnds) => Expr::Sub((*oprnds).to_owned()),
    //         AtomView::Div(oprnds) => Expr::Div((*oprnds).to_owned()),
    //         AtomView::Pow(oprnds) => Expr::Pow((*oprnds).to_owned()),
    //     }
    // }

    // pub fn into_atom(&self) -> Atom {
    //     self.into_expr().as_atom()
    // }
}

mod seal {
    use super::*;

    /// Prevents the construction of [`Expr::Atom`] and [`Atom::Expr`] outside this module
    #[derive(Debug, Clone, PartialEq)]
    pub struct ExprAtomSeal(());

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
                _ => Expr::Atom(self, ExprAtomSeal(())),
            }
        }

        #[inline]
        pub fn to_expr(self) -> Expr {
            match self {
                Atom::Expr(expr, _) => Rc::unwrap_or_clone(expr),
                _ => Expr::Atom(self, ExprAtomSeal(())),
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
            *self = Atom::Expr(orig.to_expr().into(), ExprAtomSeal(()));

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

        // pub(crate) fn wrap_atom_in_expr_with<'a, T, F>(&'a mut self, f: F) -> T
        //     where F: FnOnce(&'a mut Expr) -> T + 'a
        // {
        //     if let Atom::Expr(expr, _) = self {
        //         return f(Rc::make_mut(expr))
        //     }

        //     let mut tmp = Atom::Undef;
        //     std::mem::swap(&mut tmp, self);
        //     *self = Atom::Expr(tmp.to_expr().into(), ExprAtomSeal(()));
        //     match self {
        //         Atom::Expr(expr, _) => f(Rc::get_mut(expr).expect("no other ptr should exist")),
        //         _ => unreachable!(),
        //     }
        // }
    }

    /// Expression composed of [`Atom`] units and operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Expr {
        Atom(Atom, ExprAtomSeal),

        /// Is used to represent negative values
        /// 
        /// Will be interpreted as the expression -1 * [`Atom`]
        /// For cohesion negative integers are represented as `Expr::Neg(Atom::U32(0))`
        Neg(Atom),

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
                _ => Atom::Expr(self.into(), ExprAtomSeal(())),
            }
        }
    }
}

pub use seal::{Atom, Expr};

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
    pub fn view(&self) -> AtomView<'_> {
        match self {
            Atom::Undef => AtomView::Undef,
            Atom::U32(val) => AtomView::U32(*val),
            Atom::Var(var) => AtomView::Var(var.as_ref()),
            Atom::Expr(expr, _) => panic!("we never call view on operands"),
        }
    }

    pub fn expand(&mut self) {
        match self {
            Atom::Undef | Atom::U32(_) | Atom::Var(_) => (),
            Atom::Expr(expr, _) => Rc::make_mut(expr).expand(),
        }
    }

    pub fn base_view(&self) -> AtomView<'_> {
        match self {
            Atom::Expr(expr, _) => expr.base_view(),
            _ => self.view(),
        }
    }

    pub fn exponent(&self) -> AtomView<'_> {
        match self {
            Atom::Expr(expr, _) => expr.exponent_view(),
            _ => AtomView::U32(1),
        }
    }

    pub fn is_expr(&self) -> bool {
        matches!(self, Atom::Expr(_, _))
    }

    /// IMPORTANT: Should only be called on atoms that are part of a compount expression
    ///
    /// Wrap the atom in [`Expr::Pow`] with the atom as base, returning a mutable reference to the
    /// exponent atom
    fn atom_exponent_mut(&mut self) -> &mut Atom {
        self.with_expr_mut(|a| a.exponent_atom_mut())
    }

    //     /// IMPORTANT: Should only be called on atoms that are part of a compount expression
    //     ///
    //     /// Wrap the atom in [`Expr::Atom`] and then [`Atom::Expr`], returning a mutable reference to
    //     /// the inner [`Expr`]
    //     fn wrap_atom_in_expr(&mut self) -> &mut Expr {
    //         self.expr_with(|e| e)
    //     }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Atom(atom, _) => write!(f, "{atom}"),
            Expr::Neg(Atom::Expr(expr, _)) => write!(f, "-({expr})"),
            Expr::Neg(atom) => write!(f, "-{atom}"),
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

impl From<Expr> for Atom {
    fn from(value: Expr) -> Self {
        value.to_atom()
    }
}

impl From<Atom> for Expr {
    fn from(value: Atom) -> Self {
        value.to_expr()
    }
}

enum MulStrategy {
    None,
    Simple,
    Base,
    Expand,
}

impl Expr {
    #[inline]
    pub const fn u32(val: u32) -> Expr {
        Atom::U32(val).to_const_expr()
    }

    pub const fn i32(val: i32) -> Expr {
        let atom = Atom::U32(val.unsigned_abs());
        if val < 0 {
            Expr::Neg(atom)
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

    pub fn binary(op: fn([Atom; 2]) -> Expr, a: impl Into<Atom>, b: impl Into<Atom>) -> Expr {
        op([a.into(), b.into()])
    }

    /// Creates an n-ary expression
    ///
    /// Takes as arguments a constructor function and an iterator over items that
    /// implement [`Into<Atom>`]
    ///
    /// Usefull for e.g. Sum, Prod
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use noctua::Expr;
    ///
    /// Expr::n_ary(Expr::Sum, [Expr::from(1), Expr::from(2)]);
    /// Expr::n_ary(Expr::Prod, [Expr::from(2), Expr::from(3)]);
    /// ```
    #[inline]
    pub fn n_ary<A: Into<Atom>, I: IntoIterator<Item = A>>(
        op: fn(FlatDeque<Atom>) -> Expr,
        oprnds: I,
    ) -> Expr {
        op(oprnds.into_iter().map(|a| a.into()).collect())
    }

    #[inline]
    pub fn prod<A: Into<Atom>, I: IntoIterator<Item = A>>(oprnds: I) -> Expr {
        Expr::n_ary(Expr::Prod, oprnds)
    }

    #[inline]
    pub fn sum<A: Into<Atom>, I: IntoIterator<Item = A>>(oprnds: I) -> Expr {
        Expr::n_ary(Expr::Sum, oprnds)
    }

    pub fn add_atom(self, a: impl Into<Atom>) -> Expr {
        let atom: Atom = a.into();
        if self == Self::undef() || atom == Atom::Undef {
            return Self::undef();
        }

        todo!()
    }

    pub fn pow(&mut self, exp: impl Into<Atom>) {
        let mut tmp = Self::undef();
        std::mem::swap(self, &mut tmp);
        *self = Expr::binary(Expr::Pow, tmp, exp)
    }

    pub fn view(&self) -> AtomView<'_> {
        match self {
            Expr::Atom(atom, _) => atom.view(),
            Expr::Neg(atom) => AtomView::Neg(atom),
            Expr::Sum(oprnds) => AtomView::Sum(oprnds.as_slice()),
            Expr::Prod(oprnds) => AtomView::Prod(oprnds.as_slice()),
            Expr::Sub(oprnds) => AtomView::Sub(oprnds),
            Expr::Div(oprnds) => AtomView::Div(oprnds),
            Expr::Pow(oprnds) => AtomView::Pow(oprnds),
        }
    }

    pub fn operands(&self) -> &[Atom] {
        match self {
            Expr::Atom(atom, _) | Expr::Neg(atom) => std::slice::from_ref(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }

    pub fn operands_mut(&mut self) -> &mut [Atom] {
        match self {
            Expr::Atom(atom, _) | Expr::Neg(atom) => std::slice::from_mut(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_mut_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }

    fn drain_operands<R>(&mut self, range: R) -> Option<flat_deque::Drain<Atom>> 
    where
        R: ops::RangeBounds<usize>,
    {
        match self {
            Expr::Sum(oprnds)
            | Expr::Prod(oprnds) => Some(oprnds.drain(range)),
            _ => None
        }
    }

    pub fn base_view(&self) -> AtomView<'_> {
        match self {
            Expr::Atom(_, _)
            | Expr::Neg(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => self.view(),
            Expr::Pow([base, _]) => base.view(),
        }
    }

    pub fn exponent_view(&self) -> AtomView<'_> {
        match self {
            Expr::Atom(_, _)
            | Expr::Neg(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => AtomView::U32(1),
            Expr::Pow([_, expon]) => expon.view(),
        }
    }

    pub fn exponent(&self) -> Expr {
        match self {
            Expr::Atom(_, _)
            | Expr::Neg(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => Expr::u32(1),
            Expr::Pow([_, expon]) => expon.clone().to_expr(),
        }
    }

    /// returns a mutable reference to the exponent [`Atom`]
    ///
    /// Will wrap self in [`Expr::Pow`] with exponent = 1, if needed
    pub fn exponent_atom_mut(&mut self) -> &mut Atom {
        match self {
            Expr::Atom(_, _)
            | Expr::Neg(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => {
                self.pow(Atom::U32(1));
                &mut self.operands_mut()[1]
            }
            Expr::Pow([base, _]) => base,
        }
    }

    pub fn collect_numer_denom(&self) -> Expr {
        let numer = Expr::from(1);
        let denom = Expr::from(1);
        todo!()
    }

    /// Simplifies trivial compound expressions
    ///
    /// Inline n-ary operations as long as equivalency is maintained
    ///
    /// # Examples
    /// ```rust
    /// # use noctua::Expr;
    ///
    /// let x = Expr::from("x");
    /// let mut s = Expr::Sum([x.clone().into()].into());
    /// s.inline_trivial_compound();
    /// assert_eq!(s, x);
    ///
    /// let mut p = Expr::Prod([].into());
    /// p.inline_trivial_compound();
    /// assert_eq!(p, Expr::u32(1));
    /// ```
    pub fn inline_trivial_compound(&mut self) {
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
    }

    pub fn mul_with(&mut self, rhs: Expr, strat: MulStrategy) {
        if matches!(strat, MulStrategy::None) {
            *self = Expr::prod([self.clone(), rhs]);
            return;
        }

        const UNDEF: Expr = Expr::undef();
        const ZERO: Expr = Expr::u32(0);
        const ONE: Expr = Expr::u32(1);

        match (&*self, &rhs) {
            (&UNDEF, _) | (_, &UNDEF) => {
                *self = UNDEF;
                return;
            }
            (&ZERO, _) | (_, &ZERO) => {
                *self = ZERO;
                return;
            }
            (&ONE, other) | (other, &ONE) => {
                *self = other.clone();
                return;
            }
            _ => (),
        };

        match strat {
            MulStrategy::Simple => {
                *self = match (self.clone(), rhs) {
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
                    (lhs, rhs) => {
                        Expr::prod([lhs, rhs])
                    }
                };
            }
            MulStrategy::Base => {
                if let Expr::Prod(p) = rhs {
                    if p.is_empty() {
                        *self = ZERO;
                        return;
                    }
                    for oprnd in p {
                        self.mul_with(oprnd.into(), MulStrategy::Base);
                        return;
                    }
                } else if let Expr::Prod(p) = self {
                    if let Some(oprnd) = p.iter_mut().find(|a| a.base_view() == rhs.base_view()) {
                        *oprnd
                            // swap oprnd with Pow with the oprnd as base and Atom(1) as exponent,
                            // returning a mutable reference to the exponent atom
                            .atom_exponent_mut()
                            // wrap the exponent, Atom(1), in Atom::Expr(Expr::Atom(exponent))
                            // returning a mutable reference to the inner expression(&mut Expr::Atom(exponent))
                            .promote_to_expr() += rhs.exponent();
                    } else {
                        p.push_back(rhs.to_atom());
                    }
                } else if self.base_view() == rhs.base_view() {
                    *self.exponent_atom_mut().promote_to_expr() += rhs.exponent();
                } else {
                    *self = Expr::prod([self.clone(), rhs]);
                }
                self.inline_trivial_compound();
            }
            MulStrategy::Expand => {
                match (self, rhs) {
                    (lhs @ Expr::Sum(_), rhs) => {
                        let mut sum = Expr::Sum([].into());

                        for term in lhs.drain_operands(..).unwrap() {
                            let mut prod = term.to_expr();
                            prod.mul_with(rhs.clone(), MulStrategy::Expand);
                            sum += prod;
                        }

                        *lhs = sum
                    }
                    (lhs, Expr::Sum(oprnds)) => {
                        let mut sum = Expr::Sum([].into());
                        for term in oprnds {
                            let mut prod = lhs.clone();
                            prod.mul_with(term.to_expr(), MulStrategy::Expand);
                            sum += prod;
                        }

                        *lhs = sum
                    }
                    (lhs, rhs) => {
                        lhs.mul_with(rhs, MulStrategy::Base);
                    }
                };
            },
            MulStrategy::None => unreachable!(),
        }
    }

    pub fn expand(&mut self) {
        self.operands_mut().iter_mut().for_each(|a| a.expand());
        self.inline_trivial_compound();
        match self {
            Expr::Atom(_, _) | Expr::Neg(_) | Expr::Sum(_) => (),
            Expr::Prod(oprnds) => {
                let mut prod = Expr::u32(0);     
                for op in oprnds.drain(..) {
                    prod.mul_with(op.to_expr(), MulStrategy::Expand);
                }
            }
            Expr::Sub([lhs, rhs]) => {
                rhs.promote_to_expr().mul_with()
                // lhs += rhs.mul
            },
            Expr::Div(_) => todo!(),
            Expr::Pow(_) => todo!(),
        }
    }
}

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
        Expr::u32(value)
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Expr::i32(value)
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Atom::Var(value.into()).to_expr()
    }
}

impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::binary(Self::Sub, self, rhs)
    }
}

impl ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
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
            (lhs, rhs) => Expr::sum([lhs, rhs]),
        }
    }
}

impl ops::AddAssign for Expr {
    fn add_assign(&mut self, rhs: Self) {
        let mut tmp = Expr::undef();
        std::mem::swap(&mut tmp, self);
        *self = tmp + rhs;
    }
}

impl ops::Mul for Expr {
    type Output = Expr;
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.mul_with(rhs, MulStrategy::Simple);
        self
    }
}

impl ops::MulAssign for Expr {
    fn mul_assign(&mut self, rhs: Self) {
        let mut tmp = Expr::undef();
        std::mem::swap(&mut tmp, self);
        *self = tmp * rhs;
    }
}

impl ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Self) -> Self::Output {
        Expr::binary(Expr::Div, self, rhs)
    }
}

impl ops::DivAssign for Expr {
    fn div_assign(&mut self, rhs: Self) {
        let mut tmp = Expr::undef();
        std::mem::swap(&mut tmp, self);
        *self = tmp / rhs;
    }
}

pub fn run() {
    let mut a = Expr::from("x") + Expr::from("y");
    let b = Expr::from("a") + Expr::from("x");
    a.mul_with(b, MulStrategy::Expand);
    println!("{a}");
}
