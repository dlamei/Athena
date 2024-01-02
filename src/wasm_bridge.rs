use std::io;

use codespan_reporting::{
    files::SimpleFile,
    term::termcolor::{self, WriteColor},
};
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use crate::{
    eval, lexer,
    parser::{self, AstFile},
};

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

struct ConsoleWriter {
    buffer: String,
}

impl io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use io::{Error, ErrorKind};
        self.buffer += match String::from_utf8(buf.to_vec()) {
            Ok(ref str) => str,
            Err(err) => return Err(Error::new(ErrorKind::Other, format!("{}", err))),
        };
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        log(&self.buffer);
        self.buffer.clear();
        Ok(())
    }
}

impl WriteColor for ConsoleWriter {
    fn supports_color(&self) -> bool {
        false
    }

    fn set_color(&mut self, _spec: &termcolor::ColorSpec) -> io::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn color_to_css(color: termcolor::Color) -> String {
    use termcolor::Color as C;
    match color {
        C::Black        => "var(--black)".into(),
        C::Blue         => "var(--blue)".into(),
        C::Green        => "var(--green)".into(),
        C::Red          => "var(--red)".into(),
        C::Cyan         => "var(--cyan)".into(),
        C::Magenta      => "var(--magenta)".into(),
        C::Yellow       => "var(--yellow)".into(),
        C::White        => "var(--white)".into(),
        C::Ansi256(_)   => "inherit".into(),
        C::Rgb(r, g, b) => format!("rgb({r}, {g}, {b})"),
        _ => todo!(),
    }
}

#[wasm_bindgen]
pub struct DivWriter {
    buffer: web_sys::Element,
    curr_span: web_sys::Element,
    document: web_sys::Document,
    target: web_sys::Element,
}

    fn spec_to_css(spec: &termcolor::ColorSpec, e: &web_sys::Element) -> Result<(), JsValue> {
        let mut style = String::new();

        if let Some(fg) = spec.fg() {
            style += "color:";
            style += &color_to_css(*fg);
            style += ";";
        }
        if let Some(bg) = spec.bg() {
            style += "background-color:";
            style += &color_to_css(*bg);
            style += ";";
        }
        if spec.bold() {
            style += "font-weight:bold;";
        }
        if spec.italic() {
            e.set_attribute("font-style", "italic")?;
            style += "font-style:italic;";
        }

        e.set_attribute("style", &style)?;

        Ok(())
    }

#[wasm_bindgen]
impl DivWriter {
    #[wasm_bindgen(constructor)]
    pub fn new(query: &str) -> Result<DivWriter, JsValue> {
        let window = web_sys::window().expect("no global window found");
        let document = window.document().expect("no document found");
        let target = document.query_selector(query)?.unwrap();
        let buffer = document.create_element("p")?;
        let curr_span = document.create_element("span")?;

        Ok(Self {
            target,
            document,
            buffer,
            curr_span,
        })
    }

    fn push_span(&mut self) -> Result<(), JsValue> {
        self.buffer.append_child(&self.curr_span)?;
        self.curr_span = self.document.create_element("span")?;
        Ok(())
    }

    fn js_flush(&mut self) -> Result<(), JsValue> {
        self.buffer.append_child(&self.curr_span)?;
        self.target.append_child(&self.buffer)?;
        self.buffer = self.document.create_element("p")?;
        self.curr_span = self.document.create_element("span")?;
        Ok(())
    }
}

impl io::Write for DivWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use io::{Error, ErrorKind};
        let str = match String::from_utf8(buf.to_vec()) {
            Ok(str) => str,
            Err(err) => return Err(Error::new(ErrorKind::Other, format!("{}", err))),
        };
        // self.buffer += &str;
        self.curr_span.insert_adjacent_text("beforeend", &str).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        use io::{Error, ErrorKind};
        if let Err(js_err) = self.js_flush() {
            return Err(Error::new(ErrorKind::Other, format!("{:?}", js_err)));
        }

        Ok(())
    }
}

impl WriteColor for DivWriter {
    fn supports_color(&self) -> bool {
        true
    }

    //TODO: error
    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        self.push_span().unwrap();
        spec_to_css(spec, &self.curr_span).unwrap();
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        self.push_span().unwrap();
        Ok(())
    }
}

#[wasm_bindgen]
pub struct AthenaContext {
    writer: Box<dyn WriteColor>,
}

#[wasm_bindgen]
impl AthenaContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(panic_hook));
        let writer = ConsoleWriter {
            buffer: Default::default(),
        };

        Self {
            writer: Box::new(writer),
        }
    }

    #[wasm_bindgen]
    pub fn div_writer(&mut self, query: &str) {
        let writer = DivWriter::new(query).unwrap();
        self.writer = Box::new(writer);
    }

    #[wasm_bindgen]
    pub fn append(&mut self, code: String) {
        use io::Write;

        let file = SimpleFile::new("<STDIN>", code);

        let lex = lexer::lex(file.source());

        if lex.has_err() {
            lex.into_errors()
                .into_iter()
                .for_each(|err| err.emit_to_writer(&file, &mut self.writer));
            self.writer.flush().unwrap();
            return;
        }

        let token_len = lex.tokens().len();
        let tokens = lex.into_tokens().into_boxed_slice();
        let mut ast_file = AstFile::from_tokens(tokens, token_len);
        let ast = parser::parse_expr(&mut ast_file);

        if !ast_file.errors.is_empty() {
            for e in ast_file.errors {
                e.emit_to_writer(&file, &mut self.writer);
            }
            self.writer.flush().unwrap();
            return;
        }

        let res = eval::eval(&ast);
        write!(self.writer, "{}", res).unwrap();
        self.writer.flush().unwrap();
    }
}
