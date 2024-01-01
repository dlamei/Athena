use std::io;

use codespan_reporting::{term::termcolor::{self, WriteColor}, files::SimpleFile};
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use crate::{lexer, parser::{self, AstFile}, eval};

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

#[wasm_bindgen]
pub struct DivWriter {
    document: web_sys::Document,
    target: web_sys::Element,
    buffer: String,
}

#[wasm_bindgen]
impl DivWriter {

    #[wasm_bindgen(constructor)]
    pub fn new(query: &str) -> Result<DivWriter, JsValue> {

        let window = web_sys::window().expect("no global window found");
        let document = window.document().expect("no document found");
        let target = document.query_selector(query)?.unwrap();

        Ok(Self { target, document, buffer: Default::default() })
    }

    fn js_flush(&mut self) -> Result<(), JsValue> {
        let p = self.document.create_element("p")?;
        p.set_text_content(Some(&self.buffer));
        self.target.append_child(&p)?;
        Ok(())
    }
}

impl io::Write for DivWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use io::{Error, ErrorKind};
        self.buffer += match String::from_utf8(buf.to_vec()) {
            Ok(ref str) => str,
            Err(err) => return Err(Error::new(ErrorKind::Other, format!("{}", err))),
        };

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
        false
    }

    fn set_color(&mut self, _spec: &termcolor::ColorSpec) -> io::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub struct AthenaContext {
    writer: Box<dyn WriteColor>,
}

trait WasmAbiWriteColor: WriteColor + wasm_bindgen::convert::FromWasmAbi {}
impl <T: WriteColor + wasm_bindgen::convert::FromWasmAbi> WasmAbiWriteColor for T {}

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


//#[wasm_bindgen]
//pub fn parse(code: &str) -> String {
//    use io::Write;
//    let mut output_buffer = String::new();
//
//    let mut writer = HTMLWriter {
//        global_buffer: &mut output_buffer,
//        buffer: Default::default(),
//    };
//
//
//    let file = SimpleFile::new("<STDIN>", code);
//
//    let lex = lexer::lex(file.source());
//
//    if lex.has_err() {
//        lex.into_errors()
//            .into_iter()
//            .for_each(|err| err.emit_to_writer(&file, &mut writer));
//        writer.flush().unwrap();
//        return output_buffer;
//    }
//
//    let token_len = lex.tokens().len();
//    let tokens = lex.into_tokens().into_boxed_slice();
//    let mut ast_file = AstFile::from_tokens(tokens, token_len);
//    let ast = parser::parse_expr(&mut ast_file);
//    
//    if ast.has_err {
//        for e in ast_file.errors {
//            e.emit_to_writer(&file, &mut writer);
//        }
//        writer.flush().unwrap();
//        return output_buffer;
//    }
//
//    let res = eval::eval(&ast);
//    write!(writer, "{}", res).unwrap();
//    writer.flush().unwrap();
//
//    output_buffer
//}
