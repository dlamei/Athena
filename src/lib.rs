use codespan_reporting::{files::SimpleFile, term};
pub use parser::{parse_expr, AstFile, AST};
use wasm_bindgen::prelude::*;

mod parser;
pub mod lexer {
    pub use crate::parser::{lex, LexerResult};
}

struct HTMLWriter {}

impl std::io::Write for HTMLWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

#[wasm_bindgen]
pub fn parse(code: &str) -> String {
    let file = SimpleFile::new("<STDIN>", code);

    let lex = lexer::lex(file.source());

    if lex.has_err() {
        lex.into_errors()
            .into_iter()
            .for_each(|err| err.emit(&file));
        return "".into();
    }

    //println!("{:?}", lex.tokens());
    //println!();
    //for tok in lex.tokens() {
    //    print!("{} ", tok);
    //}
    //println!();

    let token_len = lex.tokens().len();
    let tokens = lex.into_tokens().into_boxed_slice();
    let mut ast_file = AstFile::from_tokens(tokens, token_len);
    let ast = parse_expr(&mut ast_file);

    format!("{}", ast)
}

pub type Span = std::ops::Range<usize>;

pub mod error {
    use codespan_reporting::{
        diagnostic::{Diagnostic, Label},
        files::SimpleFile,
        term::{
            self,
            termcolor::{ColorChoice, StandardStream},
        },
    };

    use crate::Span;

    #[derive(Debug, PartialEq, Clone, Hash, Default)]
    pub enum ErrCode {
        Lexer,
        #[default]
        None,
    }

    impl std::fmt::Display for ErrCode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let ErrCode::None = self {
                write!(f, "...")
            } else {
                write!(f, "{:?}", self)
            }
        }
    }

    impl ErrCode {
        pub fn to_err<MSG: Into<String>>(self, pos: Span, msg: MSG) -> Error {
            Error {
                pos,
                msg: msg.into(),
                code: self,
            }
        }
    }

    //TODO: error collector

    #[derive(Debug, PartialEq, Clone, Hash, Default)]
    pub struct Error {
        pos: Span,
        msg: String,
        code: ErrCode,
    }

    impl Error {
        pub fn new<MSG: Into<String>>(pos: Span, msg: MSG) -> Self {
            Self {
                pos,
                msg: msg.into(),
                code: Default::default(),
            }
        }
        fn to_diagnostics(self) -> Diagnostic<()> {
            let label = Label::primary((), self.pos).with_message(self.msg);

            Diagnostic::error()
                .with_code(format!("{}", self.code))
                .with_labels(vec![label])
        }

        pub fn emit<Name, Source>(self, file: &SimpleFile<Name, Source>)
        where
            Name: Clone + std::fmt::Display,
            Source: AsRef<str>,
        {
            let writer = StandardStream::stderr(ColorChoice::Always);
            let config = codespan_reporting::term::Config::default();

            let diag = self.to_diagnostics();
            term::emit(&mut writer.lock(), &config, file, &diag).expect("I/O: ERROR");
        }
    }
}
