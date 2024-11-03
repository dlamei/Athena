use std::rc::Rc;

use calcu_rs::{Expr, SymbolicExpr};

use crate::parser::{AstKind, Token, TokenKind, AST};

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

macro_rules! call {
    ($fn:path, $a:expr, 1) => {{
        if $a.len() != 1 {
            return Expr::undef()
        }
        $fn(&$a[0])
    }};
    ($fn:path, $a:expr, 2) => {{
        if $a.len() != 2 {
            return Expr::undef()
        }
        $fn(&$a[0], &$a[1])
    }};
    ($fn:path, $a:expr, 3) => {{
        if $a.len() != 3 {
            return Expr::undef()
        }
        $fn(&$a[0], &$a[1], &$a[2])
    }};
}

fn call_rust_func(name: &str, args: &Vec<Expr>) -> Expr {
    match name {
        "reduce" => call!(Expr::reduce, args, 1),
        "expand" => call!(Expr::expand, args, 1),
        "expand_main_op" => call!(Expr::expand_main_op, args, 1),
        "cancel" => call!(Expr::cancel, args, 1),
        "rationalize" => call!(Expr::rationalize, args, 1),
        "factor_out" => call!(Expr::factor_out, args, 1),
        "common_factors" => call!(Expr::common_factors, args, 2),
        "derivative" => call!(Expr::derivative, args, 2),

        _ => Expr::undef(),
    }
}

fn eval_func(name: &str, args: &Vec<AST>) -> Expr {
    let args: Vec<_> = args.iter().map(|ast| eval_node(ast)).collect();
    call_rust_func(name, &args)
}
//
fn eval_node(ast: &AST) -> Expr {
    use AstKind as AK;

    match ast.kind.as_ref() {
        AK::Ident(name) => Expr::var(name),
        AK::Integer(val) => Expr::from(*val),
        AK::Float(_val) => todo!(),
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
