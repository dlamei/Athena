use std::fmt;

use num_rational::Ratio;

use crate::{
    Expr,
    expr::{BinaryFn, ExprTyp, NAryFn},
};

/// 1) The user‐supplied style hook:
pub trait ExprStyle {
    fn sym_sub(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn sym_add(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn sym_mul(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn sym_div(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn sym_pow(f: &mut fmt::Formatter<'_>) -> fmt::Result;

    fn lparen(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn rparen(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn comma(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn space(f: &mut fmt::Formatter<'_>) -> fmt::Result;

    fn undef(f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn rational(r: &Ratio<u32>, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn var(name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    fn needs_paren_in_pow(e: &Expr) -> bool;
    fn needs_paren_in_prod(e: &Expr) -> bool;
}


/// 2) The generic pretty‐printer, parameterized by Fmt:
impl Expr {
    pub fn fmt_with<Fmt: ExprStyle>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // handle leading minus on sums:
        if self.sign().is_minus() && self.is_sum() {
            Fmt::lparen(f)?;
            Fmt::sym_sub(f)?;
            self.fmt_rec::<Fmt>(f)?;
            return Fmt::rparen(f);
        }
        if self.sign().is_minus() {
            Fmt::sym_sub(f)?;
        }
        self.fmt_rec::<Fmt>(f)
    }

    fn fmt_atom<Fmt: ExprStyle>(e: &Expr, use_paren: bool, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if use_paren {
            Fmt::lparen(f)?;
        }
        if e.sign().is_minus() {
            Fmt::sym_sub(f)?;
        }
        e.fmt_rec::<Fmt>(f)?;
        if use_paren {
            Fmt::rparen(f)?;
        }
        Ok(())
    }

    fn fmt_pow<Fmt: ExprStyle>(pow: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [b, e] = pow.binary_operands();

        // 1/x  → “1 / x”
        if e.is_minus_one() {
            Fmt::var("1", f)?;
            Fmt::sym_div(f)?;
            Self::fmt_atom::<Fmt>(b, b.is_sum(), f)
        } else {
            // sin^n(x) special case
            if b.is_unary() && b.sign().is_plus() && e.is_rational_const() {
                let inner = b.unary_operand();
                Fmt::var(b.get_unry_typ().unwrap().name(), f)?;
                Fmt::sym_pow(f)?;
                Self::fmt_atom::<Fmt>(e, e.is_sum(), f)?;
                Fmt::lparen(f)?;
                inner.fmt_rec::<Fmt>(f)?;
                return Fmt::rparen(f);
            }
            Self::fmt_atom::<Fmt>(b, Fmt::needs_paren_in_pow(b), f)?;
            Fmt::sym_pow(f)?;
            Self::fmt_atom::<Fmt>(e, Fmt::needs_paren_in_pow(e), f)
        }
    }

    fn fmt_prod<Fmt: ExprStyle>(prod: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ops = prod.nary_operands();
        if ops.is_empty() {
            Fmt::var("1", f)?;
            return Ok(());
        }
        for (i, curr) in ops.iter().enumerate() {
            if i > 0 {
                let prev = &ops[i - 1];

                // division for a^(–1):
                if curr.is_pow() && curr.exponent_ref().is_minus_one() {
                    Fmt::sym_div(f)?;
                    Self::fmt_atom::<Fmt>(curr.base_ref(), curr.base_ref().is_sum(), f)?;
                    continue;
                }

                // implicit concat: number→atom or sum→sum
                let atomish = curr.is_var() || curr.is_pow() || curr.is_sum();
                if prev.is_rational_const() && atomish
                    || prev.is_sum() && curr.is_sum()
                {
                    // skip mul entirely
                } else {
                    // **special case**: if prev was a power, emit “ * ” for readability
                    if prev.is_pow() && 
                        !(prev.base_ref().is_unary() && prev.exponent_ref().is_rational_const()) {
                            Fmt::space(f)?;
                            Fmt::sym_mul(f)?;
                            Fmt::space(f)?;
                        } else {
                            Fmt::sym_mul(f)?;
                    }
                }
            }
            Self::fmt_atom::<Fmt>(curr, curr.is_sum(), f)?;
        }
        Ok(())
    }

    fn fmt_sum<Fmt: ExprStyle>(prod: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ops = prod.nary_operands();
        if ops.is_empty() {
            Fmt::var("0", f)?;
            return Ok(());
        }
        for (i, e) in ops.iter().enumerate() {
            if i > 0 {
                if e.sign().is_minus() {
                    Fmt::space(f)?;
                    Fmt::sym_sub(f)?;
                    Fmt::space(f)?;
                } else {
                    Fmt::space(f)?;
                    Fmt::sym_add(f)?;
                    Fmt::space(f)?;
                }
            }
            e.fmt_rec::<Fmt>(f)?;
        }
        Ok(())
    }

    fn fmt_rec<Fmt: ExprStyle>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.typ {
            ExprTyp::Undef => Fmt::undef(f),

            ExprTyp::Rational(r) => Fmt::rational(r, f),

            ExprTyp::Var(sym) => Fmt::var(&sym.0, f),

            ExprTyp::Unary(op, arg) => {
                Fmt::var(op.name(), f)?;
                Fmt::lparen(f)?;
                arg.fmt_rec::<Fmt>(f)?;
                Fmt::rparen(f)
            }

            ExprTyp::Binary(BinaryFn::Pow, _) => {
                Self::fmt_pow::<Fmt>(self, f)
            }

            ExprTyp::NAry(NAryFn::Prod, ops) => {
                Self::fmt_prod::<Fmt>(self, f)
            }

            ExprTyp::NAry(NAryFn::Sum, ops) => {
                Self::fmt_sum::<Fmt>(self, f)
            }
        }
    }
}

pub struct UnicodeStyle;

impl ExprStyle for UnicodeStyle {
    fn sym_sub(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "−")
    }
    fn sym_add(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+")
    }
    fn sym_mul(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "·")
    }
    fn sym_div(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/")
    }
    fn sym_pow(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "^")
    }

    fn lparen(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")
    }
    fn rparen(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ")")
    }
    fn comma(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ",")
    }
    fn space(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, " ")
    }

    fn undef(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "∅")
    }
    fn rational(r: &Ratio<u32>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{r}")
    }
    fn var(name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{name}")
    }

    // these hooks let the printer decide when to wrap
    fn needs_paren_in_pow(e: &Expr) -> bool {
        !(e.is_atom() || e.is_unary())
            || e.sign().is_minus()
            || e.is_rational_const_and(|_, x| !x.is_integer())
    }
    fn needs_paren_in_prod(e: &Expr) -> bool {
        e.is_sum()
    }
}
