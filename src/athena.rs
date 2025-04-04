use athena_lib::{self as athena, parser::AstKind};
use calcu_rs::Expr;

fn ast_to_bytecode(ast: &athena::parser::AST) {
    match ast.kind.as_ref() {
        AstKind::Ident(rc) => todo!(),
        AstKind::Integer(_) => todo!(),
        AstKind::Binary(token, ast, ast1) => todo!(),
        AstKind::Unary(token, ast) => todo!(),
        AstKind::ParenExpr(token, token1, ast) => todo!(),
        AstKind::Func(rc, vec) => todo!(),
        AstKind::Err(ast_error_kind) => todo!(),
    }
}

fn parse(code: &str) {

    let tokens = athena::parser::lex(code);
    let mut ast_file = athena::parser::AstFile::from_tokens(tokens.tokens().clone().into_boxed_slice());
    let ast = athena::parser::parse_expr(&mut ast_file);

    match ast {
        _ => todo!(),
    }

}
