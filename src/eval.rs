use calcu_rs::{
    base::{Base, CalcursType, Symbol},
    rational::Rational,
};

use crate::parser::{AstKind, Token, TokenKind, AST};

pub fn eval_binary(op: &Token, lhs: Base, rhs: Base) -> Base {
    match op.kind {
        TokenKind::Mul => lhs * rhs,
        TokenKind::Div => lhs / rhs,
        TokenKind::Add => lhs + rhs,
        TokenKind::Sub => lhs - rhs,
        TokenKind::Pow => lhs.pow(rhs),
        _ => panic!("undefined binary operator"),
    }
}

pub fn eval_unary(op: &Token, val: Base) -> Base {
    match op.kind {
        TokenKind::Add => val,
        TokenKind::Sub => Rational::minus_one().base() * val,
        _ => panic!("undefined unary operator"),
    }
}

pub fn eval(ast: &AST) -> Base {
    use AstKind as AK;
    match ast.kind.as_ref() {
        AK::Ident(name) => Symbol::new(name.to_string()).base(),
        AK::Integer(val) => Rational::new(*val as i32, 1).base(),
        AK::Binary(op, lhs, rhs) => eval_binary(op, eval(&lhs), eval(&rhs)),
        AK::Unary(op, val) => eval_unary(op, eval(&val)),
        AK::ParenExpr(_, _, expr) => eval(&expr),
        AK::Err(_) => panic!("unhandled parser error"),
    }
}
