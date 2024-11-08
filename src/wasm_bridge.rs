use std::{
    fmt,
    io::{self, Write},
};

use calcu_rs::{
    atom::Irrational,
    sym_fmt::{FmtAtom, SymbolicFormatter},
};
use codespan_reporting::{
    files::SimpleFile,
    term::termcolor::{self, WriteColor},
};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    athena, eval, lexer,
    parser::{self, AstFile},
};

#[allow(non_snake_case)]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: String);

    type Error;

    #[wasm_bindgen(constructor)]
    fn new() -> Error;

    #[wasm_bindgen(structural, method, getter)]
    fn stack(error: &Error) -> String;
}

fn panic_hook(info: &std::panic::PanicInfo) {
    let mut msg = info.to_string();
    msg.push_str("\n\nStack:\n\n");
    let e = Error::new();
    let stack = e.stack();
    msg.push_str(&stack);

    msg.push_str("\n\n");
    error(msg);
}

fn color_to_css(color: termcolor::Color) -> String {
    use termcolor::Color as C;
    match color {
        C::Black => "var(--black)".into(),
        C::Blue => "var(--blue)".into(),
        C::Green => "var(--green)".into(),
        C::Red => "var(--red)".into(),
        C::Cyan => "var(--cyan)".into(),
        C::Magenta => "var(--magenta)".into(),
        C::Yellow => "var(--yellow)".into(),
        C::White => "var(--white)".into(),
        C::Ansi256(_) => "inherit".into(),
        C::Rgb(r, g, b) => format!("rgb({r}, {g}, {b})"),
        _ => todo!(),
    }
}

fn col_spec_to_span(spec: &termcolor::ColorSpec) -> String {
    let mut span = String::from("<span ");

    if spec.fg().is_some() || spec.bg().is_some() {
        span += "style=\"";
    }
    if let Some(fg) = spec.fg() {
        span += "color:";
        span += &color_to_css(*fg);
        span += ";";
    }
    if let Some(bg) = spec.bg() {
        span += "background-color:";
        span += &color_to_css(*bg);
        span += ";";
    }

    if spec.bold() {
        span += "font-weight:bold;";
    }
    if spec.italic() {
        span += "font-style:italic;";
    }

    span += "\">";
    span
}

#[derive(Debug, Default)]
struct CSSWriter {
    buffer: String,
    inside_span: bool,
}

impl io::Write for CSSWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use io::{Error, ErrorKind};
        let str = match String::from_utf8(buf.to_vec()) {
            Ok(str) => str,
            Err(err) => return Err(Error::new(ErrorKind::Other, format!("{}", err))),
        };

        self.buffer.push_str(&str);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl WriteColor for CSSWriter {
    fn supports_color(&self) -> bool {
        true
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        if self.inside_span {
            self.buffer += "</span>";
        }
        self.buffer += &col_spec_to_span(spec);
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        if self.inside_span {
            self.buffer += "</span>";
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct MathJaxFmt<'a>(&'a FmtAtom);

impl fmt::Display for MathJaxFmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$")?;
        Self::atom(&self.0, f)?;
        write!(f, "$")
    }
}

impl SymbolicFormatter for MathJaxFmt<'_> {
    #[inline]
    fn symbl_sub(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "-")
    }
    #[inline]
    fn symbl_add(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+")
    }
    #[inline]
    fn symbl_mul(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r" \cdot ")
    }
    #[inline]
    fn symbl_div(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/")
    }
    #[inline]
    fn symbl_pow(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "^")
    }
    #[inline]
    fn space(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, " ")
    }
    #[inline]
    fn comma(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ",")
    }
    #[inline]
    fn lparen(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r" {{\left( ")
    }
    #[inline]
    fn rparen(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r" \right)}} ")
    }
    #[inline]
    fn undef(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r" \emptyset ")
    }
    #[inline]
    fn var(v: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if [
            "sin", "arcsin", "cos", "arccos", "tan", "arctan", "sec", "exp", "ln", "log",
        ]
        .contains(&v)
        {
            write!(f, r"\{v}")
        } else {
            write!(f, "{v}")
        }
    }

    fn rational(r: &calcu_rs::prelude::Rational, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match r.is_int() {
            true => write!(f, "{}", r.numer()),
            false => write!(f, "\\frac{{{}}}{{{}}}", r.numer(), r.denom()),
        }
    }

    fn irrational(i: &Irrational, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match i {
                Irrational::E => r"e",
                Irrational::PI => r"\pi",
            }
        )
    }
}

#[wasm_bindgen]
pub struct AthenaContext;

impl AthenaContext {
    const HEADER: &'static str = "
  ▄█████████▄ ┏████▄   ┏████▄
 ┏███━━━━┓███ ┗━┓███   ┗━┓███
 ┃███    ┃███   ┃███     ┃███ ▄▄▄▄    ▄████████  ┏███▄ ▄▄▄▄▄    ▄███████▄
 ┃███    ┃███ ┏███████   ┃█████████  ┏███━━━┓███ ┗━┓█████████  ┏██━━━━┓██
 ┃███████████ ┗━┓███┛    ┃███━━┓███  ┃██████████   ┃███━━┓███  ┗━┛▄█████▌
 ┃███━━━━┓███   ┃███     ┃███  ┃███  ┃███━━━━━┛    ┃███  ┃███  ┏██━━━━┓██
 ┃███    ┃███   ┃███ ▄▄  ┃███  ┃███  ┃███▄   ███   ┃███  ┃███  ┃██    ┃██▄
┏█████  ┏█████  ┗┓█████ ┏████▌┏█████ ┗┓████████   ┏████▌┏█████ ┗┓██████┓███
┗━━━┛   ┗━━━┛    ┗━━━┛  ┗━━┛  ┗━━━┛   ┗━━━━━━┛    ┗━━┛  ┗━━━┛   ┗━━━━┛ ┗━┛
";

    const HELP_COMMANDS: &'static [(&'static str, &'static str)] = &[
        ("help", "print out this message"),
        ("functions", "list all available functions"),
        ("clear", "clear the screen (Ctrl-l)"),
        ("dark", "dark mode"),
        ("light", "light mode"),
    ];
}

#[allow(non_snake_case)]
#[wasm_bindgen]
impl AthenaContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(panic_hook));
        Self
    }

    #[wasm_bindgen]
    pub fn list_builtins(&self) -> String {
        let mut buf = String::new();

        for func in athena::BUILTINS {
            buf += &format!("{func}\n");
        }

        buf
    }

    #[wasm_bindgen]
    pub fn startup(&self) -> String {
        let out = Self::HEADER.to_string();
        out + "\n" + &self.help()
    }

    #[wasm_bindgen]
    pub fn help(&self) -> String {
        let indent = Self::HELP_COMMANDS
            .iter()
            .map(|(cmd, _)| cmd.len())
            .max()
            .unwrap_or(0);

        Self::HELP_COMMANDS
            .iter()
            .map(|(cmd, desc)| format!("{cmd:<indent$} - {desc}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[wasm_bindgen]
    pub fn eval(&mut self, code: String) -> String {
        let mut writer = CSSWriter::default();

        let file = SimpleFile::new("<STDIN>", code);
        let lex = lexer::lex(file.source());

        if lex.has_err() {
            lex.into_errors()
                .into_iter()
                .for_each(|err| err.emit_to_writer(&file, &mut writer));
            return writer.buffer;
        }

        let tokens = lex.into_tokens().into_boxed_slice();
        let mut ast_file = AstFile::from_tokens(tokens);
        let ast = parser::parse_expr(&mut ast_file);

        if !ast_file.errors.is_empty() {
            for e in ast_file.errors {
                e.emit_to_writer(&file, &mut writer);
            }
            return writer.buffer;
        }

        let res = eval::eval(&ast);

        write!(writer, "{}", MathJaxFmt(&res.fmt_ast())).unwrap();
        writer.buffer
    }
}
