use athena::AstFile;
use codespan_reporting::files::SimpleFile;
//TODO: file index

fn main() {
    let file = SimpleFile::new("<STDIN>", "1 * (2 + 3) == 6");

    let lex = athena::lexer::lex(file.source());

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
    let ast = athena::parse_expr(&mut ast_file);

    println!("{}", ast);
}
