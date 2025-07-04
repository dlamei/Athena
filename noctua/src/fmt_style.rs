use std::fmt;

use num_rational::Ratio;

use crate::{
    Expr,
    expr::{BinaryFn, ExprTyp, NAryFn},
};

pub trait ExprStyle<E> {
    fn sym_sub(&mut self) -> Result<(), E>;
    fn sym_add(&mut self) -> Result<(), E>;
    fn sym_mul(&mut self) -> Result<(), E>;
    fn sym_div(&mut self) -> Result<(), E>;
    fn sym_pow(&mut self) -> Result<(), E>;

    fn lparen(&mut self) -> Result<(), E>;
    fn rparen(&mut self) -> Result<(), E>;
    fn space(&mut self) -> Result<(), E>;

    fn undef(&mut self) -> Result<(), E>;
    fn rational(&mut self, r: Ratio<u32>) -> Result<(), E>;
    fn var(&mut self, name: &str) -> Result<(), E>;

    fn style_unry_rational_power() -> bool { true }

    // these hooks let the printer decide when to wrap
    fn use_paren_in_pow(e: &Expr) -> bool {
        !(e.is_atom() || e.is_unary())
            || e.sign().is_minus()
            || e.is_rational_const_and(|_, x| !x.is_integer())
    }
    fn use_paren_in_prod(e: &Expr) -> bool {
        e.is_sum()
    }

    fn expr(&mut self, expr: &Expr) -> Result<(), E> {
        // handle leading minus on sums:
        if expr.sign().is_minus() && expr.is_sum() {
            self.lparen()?;
            self.sym_sub()?;
            self.expr_rec(expr)?;
            return self.rparen();
        }
        if expr.sign().is_minus() {
            self.sym_sub()?;
        }
        self.expr_rec(expr)
    }

    fn atom(&mut self, e: &Expr, use_paren: bool) -> Result<(), E> {
        if use_paren {
            self.lparen()?;
        }
        if e.sign().is_minus() {
            self.sym_sub()?;
        }
        self.expr_rec(e)?;
        if use_paren {
            self.rparen()?;
        }
        Ok(())
    }

    fn pow(&mut self, pow: &Expr) -> Result<(), E> {
        let [b, e] = pow.binary_operands();

        // 1/x  → “1 / x”
        if e.is_minus_one() {
            self.rational(Ratio::ONE)?;
            self.sym_div()?;
            // Self::fmt_atom::<Fmt>(b, b.is_sum())
            self.atom(b, b.is_sum())
        } else {
            // sin^n(x) special case
            if Self::style_unry_rational_power() && b.is_unary() && b.sign().is_plus() && e.is_rational_const() {
                let inner = b.unary_operand();
                self.var(b.get_unry_typ().unwrap().name())?;
                self.sym_pow()?;
                self.atom(e, e.is_sum())?;
                self.lparen()?;
                // inner.fmt_rec::<Fmt>()?;
                self.expr_rec(inner)?;
                return self.rparen();
            }
            self.atom(b, Self::use_paren_in_pow(b))?;
            self.sym_pow()?;
            self.atom(e, Self::use_paren_in_pow(e))
        }
    }

    fn prod(&mut self, prod: &Expr) -> Result<(), E> {
        let ops = prod.nary_operands();
        if ops.is_empty() {
            self.rational(Ratio::ONE)?;
            return Ok(());
        }
        for (i, curr) in ops.iter().enumerate() {
            if i > 0 {
                let prev = &ops[i - 1];

                // division for a^(–1):
                if curr.is_pow() && curr.exponent_ref().is_minus_one() {
                    self.sym_div()?;
                    self.atom(curr.base_ref(), curr.base_ref().is_sum())?;
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
                            self.space()?;
                            self.sym_mul()?;
                            self.space()?;
                        } else {
                            self.sym_mul()?;
                    }
                }
            }
            self.atom(curr, curr.is_sum())?;
        }
        Ok(())
    }

    fn sum(&mut self, prod: &Expr) -> Result<(), E> {
        let ops = prod.nary_operands();
        if ops.is_empty() {
            self.rational(Ratio::ZERO)?;
            return Ok(());
        }
        for (i, e) in ops.iter().enumerate() {
            if i > 0 {
                if e.sign().is_minus() {
                    self.space()?;
                    self.sym_sub()?;
                    self.space()?;
                } else {
                    self.space()?;
                    self.sym_add()?;
                    self.space()?;
                }
            }
            self.expr_rec(e)?;
            // e.fmt_rec::<Fmt>(f)?;
        }
        Ok(())
    }

    fn unary(&mut self, e: &Expr) -> Result<(), E> {
        let op = e.get_unry_typ().unwrap();
        let arg = e.unary_operand();
        self.var(op.name())?;
        self.lparen()?;
        self.expr_rec(arg)?;
        self.rparen()
    }

    fn expr_rec(&mut self, e: &Expr) -> Result<(), E> {
        match &e.typ {
            ExprTyp::Undef => self.undef(),
            ExprTyp::Rational(r) => self.rational(*r),
            ExprTyp::Var(sym) => self.var(&sym.0),

            ExprTyp::Unary(_, _) => {
                self.unary(e)
            }

            ExprTyp::Binary(BinaryFn::Pow, _) => {
                self.pow(e)
            }

            ExprTyp::NAry(NAryFn::Prod, _) => {
                self.prod(e)
            }

            ExprTyp::NAry(NAryFn::Sum, _) => {
                self.sum(e)
            }
        }
    }

}


impl Expr {
    pub fn unicode_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        UnicodeStyle { f }.expr(self)
    }

    pub fn ascii_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ASCIIStyle { f }.expr(self)
    }
}

pub struct UnicodeStyle<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
}

impl ExprStyle<fmt::Error> for UnicodeStyle<'_, '_> {
    fn sym_sub(&mut self) -> fmt::Result {
        write!(self.f, "−")
    }
    fn sym_add(&mut self) -> fmt::Result {
        write!(self.f, "+")
    }
    fn sym_mul(&mut self) -> fmt::Result {
        write!(self.f, "·")
    }
    fn sym_div(&mut self) -> fmt::Result {
        write!(self.f, "/")
    }
    fn sym_pow(&mut self) -> fmt::Result {
        write!(self.f, "^")
    }

    fn lparen(&mut self) -> fmt::Result {
        write!(self.f, "(")
    }
    fn rparen(&mut self) -> fmt::Result {
        write!(self.f, ")")
    }
    fn space(&mut self) -> fmt::Result {
        write!(self.f, " ")
    }

    fn undef(&mut self) -> fmt::Result {
        write!(self.f, "∅")
    }
    fn rational(&mut self, r: Ratio<u32>) -> fmt::Result {
        write!(self.f, "{r}")
    }
    fn var(&mut self, name: &str) -> fmt::Result {
        write!(self.f, "{name}")
    }
}

pub struct ASCIIStyle<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
}

impl ExprStyle<fmt::Error> for ASCIIStyle<'_, '_> {
    fn sym_sub(&mut self) -> fmt::Result {
        write!(self.f, "-")
    }
    fn sym_add(&mut self) -> fmt::Result {
        write!(self.f, "+")
    }
    fn sym_mul(&mut self) -> fmt::Result {
        write!(self.f, "*")
    }
    fn sym_div(&mut self) -> fmt::Result {
        write!(self.f, "/")
    }
    fn sym_pow(&mut self) -> fmt::Result {
        write!(self.f, "^")
    }

    fn lparen(&mut self) -> fmt::Result {
        write!(self.f, "[")
    }
    fn rparen(&mut self) -> fmt::Result {
        write!(self.f, "]")
    }
    fn space(&mut self) -> fmt::Result {
        write!(self.f, " ")
    }

    fn undef(&mut self) -> fmt::Result {
        write!(self.f, "undef")
    }
    fn rational(&mut self, r: Ratio<u32>) -> fmt::Result {
        write!(self.f, "{r}")
    }
    fn var(&mut self, name: &str) -> fmt::Result {
        write!(self.f, "{name}")
    }

    fn style_unry_rational_power() -> bool { false }
}

