mod parser;

pub use parser::{parse_expr, AstFile, AST};

pub mod lexer {
    pub use crate::parser::{lex, LexerResult};
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
