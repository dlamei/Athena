use codespan_reporting::files::SimpleFile;
//TODO: file index

fn main() {
    let file = SimpleFile::new("<STDIN>", "(1 / (3 * x)) $ * x * 3;");

    let lex = athena::lexer::lex(file.source());

    if lex.has_err() {
        lex.into_errors()
            .into_iter()
            .for_each(|err| err.emit(&file));
        return;
    }

    println!(
        "{:?}",
        lex.tokens().iter().map(|(tok, _)| tok).collect::<Vec<_>>()
    );
}
