use std::fmt;

use calcu_rs::{Expr, SymbolicExpr};

use crate::parser::{self, AstKind, Token, TokenKind, AST};

pub fn eval_binary(op: &Token, lhs: Expr, rhs: Expr) -> Expr {
    use TokenKind as TK;
    match op.kind {
        TK::Add => lhs + rhs,
        TK::Sub => lhs - rhs,
        TK::Mul => lhs * rhs,
        TK::Div => lhs / rhs,
        TK::Pow => Expr::pow(lhs, rhs),
        _ => panic!("undefined binary operator"),
    }
}

pub fn eval_unary(op: &Token, val: Expr) -> Expr {
    match op.kind {
        TokenKind::Add => val,
        TokenKind::Sub => Expr::min_one() * val,
        _ => panic!("undefined unary operator"),
    }
}

fn call_builtin(name: &str, args: &[Expr]) -> Expr {
    let builtin = match parser::get_builtin(name) {
        Some(builtin) => builtin,
        None => panic!("builtin function {name} not defined"),
    };

    assert!(
        args.len() == builtin.n_params(),
        "builtin function {name} has {} arguments, not {}",
        builtin.n_params(),
        args.len()
    );

    builtin.call(args)
}

/*
fn call_rust_func(name: &str, args: &Vec<Expr>) -> Expr {
    match name {
        "sin" => call!(Expr::sin, args, 1),
        "arcsin" => call!(Expr::arc_sin, args, 1),
        "cos" => call!(Expr::cos, args, 1),
        "arccos" => call!(Expr::arc_cos, args, 1),
        "tan" => call!(Expr::tan, args, 1),
        "arctan" => call!(Expr::arc_tan, args, 1),
        "sec" => call!(Expr::sec, args, 1),
        "ln" => call!(Expr::ln, args, 1),
        "log10" => call!(Expr::log10, args, 1),
        "exp" => call!(Expr::exp, args, 1),
        "sqrt" => call!(Expr::sqrt, args, 1),

        "numer" => call!(Expr::numerator, args, 1),
        "denom" => call!(Expr::denominator, args, 1),
        "pow_base" => call!(Expr::base, args, 1),
        "pow_exp" => call!(Expr::exponent, args, 1),

        "free_of" => Expr::from(call!(Expr::free_of, args, 2) as u32),

        "reduce" => call!(Expr::reduce, args, 1),
        "expand" => call!(Expr::expand, args, 1),
        "expand_main_op" => call!(Expr::expand_main_op, args, 1),
        "cancel" => call!(Expr::cancel, args, 1),
        "rationalize" => call!(Expr::rationalize, args, 1),
        "factor_out" => call!(Expr::factor_out, args, 1),
        "common_factors" => call!(Expr::common_factors, args, 2),
        "deriv" => call!(Expr::derivative, args, 2),

        _ => Expr::undef(),
    }
}
*/

fn eval_func(name: &str, args: &Vec<AST>) -> Expr {
    let args: Vec<_> = args.iter().map(|ast| eval_node(ast)).collect();
    call_builtin(name, &args)
}

fn eval_var(var: &str) -> Expr {
    match var {
        "undef" => Expr::undef(),
        "pi" => Expr::pi(),
        _ => Expr::var(var),
    }
}

fn eval_node(ast: &AST) -> Expr {
    use AstKind as AK;

    match ast.kind.as_ref() {
        AK::Ident(var) => eval_var(var),
        AK::Integer(i) => Expr::from(*i),
        AK::Binary(op, lhs, rhs) => eval_binary(op, eval_node(&lhs), eval_node(&rhs)),
        AK::Unary(op, val) => eval_unary(op, eval_node(&val)),
        AK::ParenExpr(_, _, expr) => eval_node(&expr),
        AK::Err(_) => panic!("unhandled parser error"),
        AK::Func(rc, vec) => eval_func(rc, vec),
    }
}

pub fn eval(ast: &AST) -> Expr {
    eval_node(ast).reduce()
}
