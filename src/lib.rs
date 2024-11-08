pub mod athena;
pub mod error;
pub mod eval;
pub mod parser;

pub mod lexer {
    pub use crate::parser::{lex, LexerResult};
}

pub mod wasm_bridge;

pub type Span = std::ops::Range<usize>;

pub fn merge_span(s1: &Span, s2: &Span) -> Span {
    let start = std::cmp::min(s1.start, s2.start);
    let end = std::cmp::max(s1.end, s2.end);
    start..end
}
