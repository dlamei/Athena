use derive_more::Display;
use itertools::{Either, Itertools};
use logos::Logos;

use crate::{
    error::{ErrCode, Error},
    Span,
};

use calcu_rs::prelude as calc;

pub fn lex_number<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Option<calc::Numeric> {
    let val = lex.slice().parse::<i32>().ok()?;
    Some(calc::Rational::from(val).num())
}

#[derive(Logos, Debug, PartialEq, Clone, Display)]
#[logos(skip r"([ \t\f]+|//.*)")]
#[logos(subpattern unicode_ident = r"\p{XID_Start}\p{XID_Continue}*")]
#[logos(subpattern ascii_ident = r"[_a-zA-Z][_0-9a-zA-Z]*")]
pub enum Token<'a> {
    #[display(fmt = "*")]
    #[token("*")]
    Mul,
    #[display(fmt = "/")]
    #[token("/")]
    Div,
    #[display(fmt = "+")]
    #[token("+")]
    Add,
    #[display(fmt = "-")]
    #[token("-")]
    Sub,
    #[display(fmt = "^")]
    #[token("^")]
    Pow,
    #[display(fmt = "=")]
    #[token("=")]
    Assign,
    #[display(fmt = "(")]
    #[token("(")]
    LParen,
    #[display(fmt = ")")]
    #[token(")")]
    RParen,
    #[display(fmt = "{{")]
    #[token("{")]
    LCurly,
    #[display(fmt = "}}")]
    #[token("}")]
    RCurly,
    #[display(fmt = ":")]
    #[token(":")]
    Colon,

    #[display(fmt = "{}", val.0)]
    #[regex("(?&unicode_ident)", |lex| lex.slice())]
    Ident(&'a str),

    // #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    #[display(fmt = "{}", val.0)]
    #[regex("[0-9]+", lex_number)]
    Num(calc::Numeric),

    #[display(fmt = "\n")]
    #[token(";")]
    #[token("\n")]
    NL,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LexerResult<'a> {
    tokens: Vec<(Token<'a>, Span)>,
    errors: Vec<Error>,
}

impl<'a> LexerResult<'a> {
    pub fn has_err(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_errors(self) -> Vec<Error> {
        self.errors
    }

    pub fn tokens(&self) -> &Vec<(Token<'a>, Span)> {
        &self.tokens
    }
}

pub fn lex<'a>(code: &'a str) -> LexerResult<'a> {
    let (tokens, errors): (Vec<_>, Vec<_>) =
        Token::lexer(code)
            .spanned()
            .partition_map(|(tok, span)| match tok {
                Ok(tok) => Either::Left((tok, span)),
                Err(_) => {
                    let err = ErrCode::Lexer.to_err(span, "unknown character");
                    Either::Right(err)
                }
            });

    LexerResult { tokens, errors }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Parser<'a> {
    tokens: Vec<(Token<'a>, Span)>,
}

impl<'a> From<Vec<(Token<'a>, Span)>> for Parser<'a> {
    fn from(tokens: Vec<(Token<'a>, Span)>) -> Self {
        Self { tokens }
    }
}

impl<'a> Parser<'a> {}

pub enum AstKind {
    Item(calc::Base),
    Add(calc::Add),
    Pow(Box<AstKind>, Box<AstKind>),
}
