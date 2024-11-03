use calcu_rs::{Expr, SymbolicExpr};

use crate::parser::{AstKind, Token, TokenKind, AST};

pub fn eval_binary<'a>(op: &Token, lhs: Expr, rhs: Expr) -> Expr {
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

pub fn eval_unary<'a>(op: &Token, val: Expr) -> Expr {
    match op.kind {
        TokenKind::Add => val,
        TokenKind::Sub => Expr::min_one() * val,
        _ => panic!("undefined unary operator"),
    }
}
//
fn eval_node<'a>(ast: &AST) -> Expr {
    use AstKind as AK;
    match ast.kind.as_ref() {
        AK::Ident(name) => Expr::var(name),
        AK::Integer(val) => Expr::from(*val),
        AK::Float(_val) => todo!(),
        AK::Binary(op, lhs, rhs) => eval_binary(op, eval_node(&lhs), eval_node(&rhs)),
        AK::Unary(op, val) => eval_unary(op, eval_node(&val)),
        AK::ParenExpr(_, _, expr) => eval_node(&expr),
        AK::Err(_) => panic!("unhandled parser error"),
    }
}

pub fn eval(ast: &AST) -> Expr {
    eval_node(ast).reduce()
}
