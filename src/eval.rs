use calcu_rs::{Expr, ExprContext, Node, Rational};

use crate::parser::{AstKind, Token, TokenKind, AST};

pub fn eval_binary<'a>(op: &Token, lhs: Expr<'a>, rhs: Expr<'a>, c: &'a ExprContext) -> Expr<'a> {
    use TokenKind as TK;
    let n = match op.kind {
        TK::Add => Node::Add([lhs.id(), rhs.id()]),
        TK::Sub => {
            let min_one = c.insert(Node::MINUS_ONE);
            let min_rhs = c.insert(Node::Mul([min_one, rhs.id()]));
            Node::Add([lhs.id(), min_rhs])
        }
        TK::Mul => Node::Mul([lhs.id(), rhs.id()]),
        TK::Div => {
            let min_one = c.insert(Node::MINUS_ONE);
            let div_rhs = c.insert(Node::Pow([rhs.id(), min_one]));
            Node::Mul([lhs.id(), div_rhs])
        }
        TK::Pow => Node::Pow([lhs.id(), rhs.id()]),
        _ => panic!("undefined binary operator"),
    };
    c.make_expr(n)
}

pub fn eval_unary<'a>(op: &Token, val: Expr<'a>, c: &'a ExprContext) -> Expr<'a> {
    match op.kind {
        TokenKind::Add => val,
        TokenKind::Sub => {
            let min_one = c.insert(Node::MINUS_ONE);
            c.make_expr(Node::Mul([val.id(), min_one]))
        }
        _ => panic!("undefined unary operator"),
    }
}
//
fn eval_node<'a>(ast: &AST, c: &'a ExprContext) -> Expr<'a> {
    use AstKind as AK;
    match ast.kind.as_ref() {
        AK::Ident(name) => c.make_expr(c.var(name)), 
        AK::Integer(val) => c.make_expr(Node::Rational(Rational::from(*val as i64))),
        AK::Float(_val) => todo!(),
        AK::Binary(op, lhs, rhs) => eval_binary(op, eval_node(&lhs, c), eval_node(&rhs, c), c),
        AK::Unary(op, val) => eval_unary(op, eval_node(&val, c), c),
        AK::ParenExpr(_, _, expr) => eval_node(&expr, c),
        AK::Err(_) => panic!("unhandled parser error"),
    }
}

pub fn eval<'a>(ast: &AST, c: &'a ExprContext) -> Expr<'a> {
    let e = eval_node(ast, c);
    e.apply_rules(calcu_rs::ExprFold, &calcu_rs::scalar_rules())
}
