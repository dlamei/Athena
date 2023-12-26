use athena::{lexer, parse_expr, AstFile};
use codespan_reporting::files::SimpleFile;
//TODO: file index

fn main() {
    let code = "(x + 2)^2";
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
    let mut ast_file = AstFile::from_tokens(tokens, token_len);
    let ast = parse_expr(&mut ast_file);

    println!("{}", ast);
}
