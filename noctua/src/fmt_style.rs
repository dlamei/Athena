use std::fmt::{self, Formatter};

use num_rational::Ratio;

use crate::{
    Expr,
    expr::{BinaryFn, ExprTyp, NAryFn},
};


pub trait ExprStyle {
    fn sym_add() -> &'static str { "+" }
    fn sym_sub() -> &'static str { "-" }
    fn sym_mul() -> &'static str { "*" }
    fn sym_div() -> &'static str { "/" }
    fn sym_pow() -> &'static str { "^" }

    fn lparen() -> &'static str { "(" }
    fn rparen() -> &'static str { ")" }
    fn space() -> &'static str { " " }

    fn undef() -> &'static str { "undef" }

    fn rational(r: Ratio<u32>, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{r}") }
    fn var(name: &str, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{name}") }

    fn style_unry_rational_power() -> bool { true }
    fn style_implicit_mul() -> bool { true }

    // these hooks let the printer decide when to wrap
    fn use_paren_in_pow(e: &Expr) -> bool {
        !(e.is_atom() || e.is_unary())
            || e.sign().is_minus()
            || e.is_rational_const_and(|_, x| !x.is_integer())
    }
    fn use_paren_in_prod(e: &Expr) -> bool {
        e.is_sum()
    }

    fn expr(expr: &Expr, f: &mut Formatter<'_>) -> fmt::Result {
        // handle leading minus on sums:
        if expr.sign().is_minus() && expr.is_sum() {
            write!(f, "{}", Self::lparen())?;
            write!(f, "{}", Self::sym_sub())?;
            Self::expr_rec(expr, f)?;
            return write!(f, "{}", Self::rparen());
        }
        if expr.sign().is_minus() {
            write!(f, "{}", Self::sym_sub())?;
        }
        Self::expr_rec(expr, f)
    }

    fn atom(e: &Expr, use_paren: bool, f: &mut Formatter<'_>) -> fmt::Result {
        if use_paren {
            write!(f, "{}", Self::lparen())?;
        }
        if e.sign().is_minus() {
            write!(f, "{}", Self::sym_sub())?;
        }
        Self::expr_rec(e, f)?;
        if use_paren {
            write!(f, "{}", Self::rparen())?;
        }
        Ok(())
    }

    fn pow(pow: &Expr, f: &mut Formatter<'_>) -> fmt::Result {
        let [b, e] = pow.binary_operands();

        // 1/x  → “1 / x”
        if e.is_minus_one() {
            Self::rational(Ratio::ONE, f)?;
            write!(f, "{}", Self::sym_div())?;
            // Self::fmt_atom::<Fmt>(b, b.is_sum())
            Self::atom(b, b.is_sum(), f)
        } else {
            // sin^n(x) special case
            if Self::style_unry_rational_power() && b.is_unary() && b.sign().is_plus() && e.is_rational_const() {
                let inner = b.unary_operand();
                Self::var(b.get_unry_typ().unwrap().name(), f)?;
                write!(f, "{}", Self::sym_pow())?;
                Self::atom(e, e.is_sum(), f)?;
                write!(f, "{}", Self::lparen())?;
                // inner.fmt_rec::<Fmt>()?;
                Self::expr_rec(inner, f)?;
                return write!(f, "{}", Self::rparen());
            }
            Self::atom(b, Self::use_paren_in_pow(b), f)?;
            write!(f, "{}", Self::sym_pow())?;
            Self::atom(e, Self::use_paren_in_pow(e), f)
        }
    }

    fn prod(prod: &Expr, f: &mut Formatter<'_>) -> fmt::Result {
        let ops = prod.nary_operands();
        if ops.is_empty() {
            Self::rational(Ratio::ONE, f)?;
            return Ok(());
        }
        for (i, curr) in ops.iter().enumerate() {
            if i > 0 {
                let prev = &ops[i - 1];

                // division for a^(–1):
                if curr.is_pow() && curr.exponent_ref().is_minus_one() {
                    write!(f, "{}", Self::sym_div())?;
                    Self::atom(curr.base_ref(), curr.base_ref().is_sum(), f)?;
                    continue;
                }

                // implicit concat: number→atom or sum→sum
                let atomish = curr.is_var() || curr.is_pow() || curr.is_sum();
                if Self::style_implicit_mul() && prev.is_rational_const() && atomish
                    || prev.is_sum() && curr.is_sum()
                {
                    // skip mul entirely
                } else {
                    // **special case**: if prev was a power, emit “ * ” for readability
                    if prev.is_pow() && 
                        !(prev.base_ref().is_unary() && prev.exponent_ref().is_rational_const()) {
                            write!(f, "{}", Self::space())?;
                            write!(f, "{}", Self::sym_mul())?;
                            write!(f, "{}", Self::space())?;
                        } else {
                            write!(f, "{}", Self::sym_mul())?;
                    }
                }
            }
            Self::atom(curr, curr.is_sum(), f)?;
        }
        Ok(())
    }

    fn sum(prod: &Expr, f: &mut Formatter<'_>) -> fmt::Result {
        let ops = prod.nary_operands();
        if ops.is_empty() {
            Self::rational(Ratio::ZERO, f)?;
            return Ok(());
        }
        for (i, e) in ops.iter().enumerate() {
            if i > 0 {
                if e.sign().is_minus() {
                    write!(f, "{}", Self::space())?;
                    write!(f, "{}", Self::sym_sub())?;
                    write!(f, "{}", Self::space())?;
                } else {
                    write!(f, "{}", Self::space())?;
                    write!(f, "{}", Self::sym_add())?;
                    write!(f, "{}", Self::space())?;
                }
            }
            Self::expr_rec(e, f)?;
            // e.fmt_rec::<Fmt>(f)?;
        }
        Ok(())
    }

    fn unary(e: &Expr, f: &mut Formatter<'_>) -> fmt::Result {
        let op = e.get_unry_typ().unwrap();
        let arg = e.unary_operand();
        Self::var(op.name(), f)?;
        write!(f, "{}", Self::lparen())?;
        Self::expr_rec(arg, f)?;
        write!(f, "{}", Self::rparen())
    }

    fn expr_rec(e: &Expr, f: &mut Formatter<'_>) -> fmt::Result {
        match &e.typ {
            ExprTyp::Undef => write!(f, "{}", Self::undef()),
            ExprTyp::Rational(r) => Self::rational(*r, f),
            ExprTyp::Var(sym) => Self::var(&sym.0, f),

            ExprTyp::Unary(_, _) => {
                Self::unary(e, f)
            }

            ExprTyp::Binary(BinaryFn::Pow, _) => {
                Self::pow(e, f)
            }

            ExprTyp::NAry(NAryFn::Prod, _) => {
                Self::prod(e, f)
            }

            ExprTyp::NAry(NAryFn::Sum, _) => {
                Self::sum(e, f)
            }
        }
    }

}


impl Expr {
    pub fn fmt_with_style<Style: ExprStyle>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Style::expr(self, f)
    }
}

pub struct UnicodeStyle;

impl ExprStyle for UnicodeStyle {
    fn sym_sub() -> &'static str {
        "−"
    }
    fn sym_mul() -> &'static str {
        "·"
    }
    fn undef() -> &'static str {
        "∅"
    }
}

pub struct ASCIIStyle;

impl ExprStyle for ASCIIStyle {

    fn lparen() -> &'static str {
        "["
    }
    fn rparen() -> &'static str {
        "]"
    }

    fn style_unry_rational_power() -> bool { false }
    fn style_implicit_mul() -> bool { false } 
}

