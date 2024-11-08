use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term::{
        self,
        termcolor::{Color, ColorChoice, ColorSpec, StandardStream},
    },
};

use crate::Span;

#[derive(Debug, PartialEq, Clone, Hash, Default)]
pub enum ErrCode {
    Lexer,
    Syntax,
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
    pub fn to_err<MSG: Into<String>>(self, pos: &Span, msg: MSG) -> Error {
        Error {
            pos: pos.clone(),
            msg: msg.into(),
            code: self,
        }
    }
}

fn diag_style() -> term::Styles {
        let header = ColorSpec::new().set_bold(true).set_intense(true).clone();

        let source = Color::Rgb(247, 240, 220);
        let red = Color::Rgb(216, 62, 86);
        let blue = Color::Rgb(74, 121, 159);


        term::Styles {
            header_bug: header.clone().set_fg(Some(red)).clone(),
            header_error: header.clone().set_fg(Some(red)).clone(),
            header_warning: header.clone().set_fg(Some(Color::Yellow)).clone(),
            header_note: header.clone().set_fg(Some(Color::Green)).clone(),
            header_help: header.clone().set_fg(Some(Color::Cyan)).clone(),
            header_message: header,

            primary_label_bug: ColorSpec::new().set_fg(Some(red)).clone(),
            primary_label_error: ColorSpec::new().set_fg(Some(red)).clone(),
            primary_label_warning: ColorSpec::new().set_fg(Some(Color::Yellow)).clone(),
            primary_label_note: ColorSpec::new().set_fg(Some(Color::Green)).clone(),
            primary_label_help: ColorSpec::new().set_fg(Some(Color::Cyan)).clone(),
            secondary_label: ColorSpec::new().set_fg(Some(source)).clone(),

            line_number: ColorSpec::new().set_fg(Some(source)).clone(),
            source_border: ColorSpec::new().set_fg(Some(source)).clone(),
            note_bullet: ColorSpec::new().set_fg(Some(source)).clone(),
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
        let label = Label::primary((), self.pos); //.with_message(self.msg);

        Diagnostic::error()
            .with_code(format!("{}", self.code))
            .with_notes(vec![self.msg])
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

    pub fn emit_to_writer<Name, Source, Writer>(
        self,
        file: &SimpleFile<Name, Source>,
        writer: &mut Writer,
    ) where
        Name: Clone + std::fmt::Display,
        Source: AsRef<str>,
        Writer: codespan_reporting::term::termcolor::WriteColor,
    {
        let mut config = codespan_reporting::term::Config::default();
        //config.styles = term::Styles::with_blue(term::termcolor::Color::Rgb(247, 240, 220));
        config.styles = diag_style();
        let diag = self.to_diagnostics();
        term::emit(writer, &config, file, &diag).expect("I/O: ERROR");
    }
}
