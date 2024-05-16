use calcu_rs::{
    expression::{CalcursType, Construct, Expr, Symbol},
    rational::Rational,
};

use crate::parser::{AstKind, Token, TokenKind, AST};

pub fn eval_binary(op: &Token, lhs: Expr, rhs: Expr) -> Expr {
    match op.kind {
        TokenKind::Mul => lhs * rhs,
        TokenKind::Div => lhs / rhs,
        TokenKind::Add => lhs + rhs,
        TokenKind::Sub => lhs - rhs,
        TokenKind::Pow => lhs.pow(rhs),
        _ => panic!("undefined binary operator"),
    }
}

pub fn eval_unary(op: &Token, val: Expr) -> Expr {
    match op.kind {
        TokenKind::Add => val,
        TokenKind::Sub => Rational::MINUS_ONE * val,
        _ => panic!("undefined unary operator"),
    }
}

fn eval_node(ast: &AST) -> Expr {
    use AstKind as AK;
    match ast.kind.as_ref() {
        AK::Ident(name) => Symbol::new(name.to_string()).into(),
        AK::Integer(val) => Rational::from(*val as i64).into(),
        AK::Float(_val) => todo!(),
        AK::Binary(op, lhs, rhs) => eval_binary(op, eval_node(&lhs), eval_node(&rhs)),
        AK::Unary(op, val) => eval_unary(op, eval_node(&val)),
        AK::ParenExpr(_, _, expr) => eval_node(&expr),
        AK::Err(_) => panic!("unhandled parser error"),
    }
}

pub fn eval(ast: &AST) -> Expr {
    let e = eval_node(ast);
    e.simplify()
}
