use athena_lib::{eval, lexer, parser};
use codespan_reporting::files::SimpleFile;

#[cfg(not(target_arch = "wasm32"))]
use rustyline::{error::ReadlineError, DefaultEditor};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> rustyline::Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;

                let file = SimpleFile::new("<STDIN>", line.clone());
                let lex = lexer::lex(file.source());

                if lex.has_err() {
                    lex.into_errors()
                        .into_iter()
                        .for_each(|err| err.emit(&file));
                    continue;
                }

                //println!();
                //for tok in lex.tokens() {
                //    print!("{} ", tok);
                //}
                //println!();

                let tokens = lex.into_tokens().into_boxed_slice();
                let mut ast_file = parser::AstFile::from_tokens(tokens);
                let ast = parser::parse_expr(&mut ast_file);

                if !ast_file.errors.is_empty() {
                    for err in ast_file.errors {
                        err.emit(&file);
                    }
                    continue;
                }

                println!("{}", ast);
                println!("{}", eval::eval(&ast));
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
