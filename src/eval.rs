use std::fmt;

use calcu_rs::{Expr, SymbolicExpr};

use crate::{
    athena,
    parser::{AstKind, Token, TokenKind, AST},
};

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
    let builtin = match athena::get_builtin(name) {
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
    eval_node(ast)
}
