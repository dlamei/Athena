use calcu_rs::ExprContext;
use athena_lib::{eval, lexer, parser};
use codespan_reporting::files::SimpleFile;
//TODO: file index

fn main() {
    let code = "x * x";
    let file = SimpleFile::new("<STDIN>", code);

    let lex = lexer::lex(file.source());

    if lex.has_err() {
        lex.into_errors()
            .into_iter()
            .for_each(|err| err.emit(&file));
        return;
    }

    //println!("{:?}", lex.tokens());
    println!();
    for tok in lex.tokens() {
        print!("{} ", tok);
    }
    println!();

    let token_len = lex.tokens().len();
    let tokens = lex.into_tokens().into_boxed_slice();
    let mut ast_file = parser::AstFile::from_tokens(tokens, token_len);
    let ast = parser::parse_expr(&mut ast_file);

    if !ast_file.errors.is_empty() {
        for err in ast_file.errors {
            err.emit(&file);
        }
        return;
    }

    println!("{}", ast);
    let c = ExprContext::new();
    println!("{}", eval::eval(&ast, &c).fmt_ast());
}
