use derive_more::Display;
use itertools::{Either, Itertools};
use logos::Logos;
use std::fmt;
use std::fmt::Formatter;
use std::rc::Rc;

use crate::{
    error::{ErrCode, Error},
    Span,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    kind: TokenKind,
    span: Span,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[derive(Logos, Debug, PartialEq, Clone, Display)]
#[logos(skip r"([ \t\f]+|//.*)")]
#[logos(subpattern unicode_ident = r"\p{XID_Start}\p{XID_Continue}*")]
#[logos(subpattern ascii_ident = r"[_a-zA-Z][_0-9a-zA-Z]*")]
pub enum TokenKind {
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
    #[display(fmt = "==")]
    #[token("==")]
    CmpEq,
    #[display(fmt = ">")]
    #[token(">")]
    CmpGt,
    #[display(fmt = "<")]
    #[token("<")]
    CmpLt,
    #[display(fmt = ">=")]
    #[token(">=")]
    CmpGtEq,
    #[display(fmt = "<=")]
    #[token("<=")]
    CmpLtEq,

    #[display(fmt = "(")]
    #[token("(")]
    OpenParen,
    #[display(fmt = ")")]
    #[token(")")]
    CloseParen,
    #[display(fmt = "{{")]
    #[token("{")]
    OpenCurly,
    #[display(fmt = "}}")]
    #[token("}")]
    CloseCurly,
    #[display(fmt = ":")]
    #[token(":")]
    Colon,
    #[display(fmt = "=")]
    #[token("=")]
    Eq,

    #[display(fmt = "{}", val.0)]
    #[regex("(?&unicode_ident)", |lex| Rc::from(lex.slice()))]
    Ident(Rc<str>),

    // #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    #[display(fmt = "{}", val.0)]
    #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    Integer(u32),

    #[display(fmt = "\n")]
    #[token(";")]
    #[token("\n")]
    NL,

    #[display(fmt = "EOF")]
    EOF,
}

impl TokenKind {
    pub const fn precedence(&self) -> u32 {
        use TokenKind as TK;
        match self {
            TK::CmpEq | TK::CmpGt | TK::CmpLt | TK::CmpGtEq | TK::CmpLtEq => 1,
            TK::Add | TK::Sub => 2,
            TK::Mul | TK::Div => 3,
            TK::Pow => 4,
            _ => 0,
        }
    }

    #[inline]
    pub const fn is_binary_op(&self) -> bool {
        use TokenKind as TK;
        match self {
            TK::Add
            | TK::Sub
            | TK::Mul
            | TK::Div
            | TK::Pow
            | TK::CmpEq
            | TK::CmpGt
            | TK::CmpLt
            | TK::CmpGtEq
            | TK::CmpLtEq => true,
            _ => false,
        }
    }

    #[inline]
    pub const fn is_unary_op(&self) -> bool {
        use TokenKind as TK;
        match self {
            TK::Add | TK::Sub => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LexerResult {
    tokens: Vec<Token>,
    errors: Vec<Error>,
}

impl LexerResult {
    pub fn has_err(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }
    pub fn into_errors(self) -> Vec<Error> {
        self.errors
    }
    pub fn into_tokens(self) -> Vec<Token> {
        self.tokens
    }
}

pub fn lex(code: &str) -> LexerResult {
    let (mut tokens, errors): (Vec<_>, Vec<_>) =
        TokenKind::lexer(code)
            .spanned()
            .partition_map(|(tok, span)| match tok {
                Ok(tok) => Either::Left(Token { kind: tok, span }),
                Err(_) => {
                    let err = ErrCode::Lexer.to_err(span, "unknown character");
                    Either::Right(err)
                }
            });

    if let Some(last) = tokens.last() {
        tokens.push(Token {
            kind: TokenKind::EOF,
            span: last.span.clone(),
        })
    }

    LexerResult { tokens, errors }
}

// inspiration: https://github.dev/odin-lang/Odin/blob/master/src/parser.cpp

#[derive(Debug, PartialEq, Clone)]
pub struct AstFile {
    tokens: Box<[Token]>,
    token_count: usize,
    curr_token_index: usize,
    prev_token_index: usize,
    curr_token: Token,
    prev_token: Token,
}

impl AstFile {
    pub fn from_tokens(tokens: Box<[Token]>, token_count: usize) -> Self {
        assert_ne!(token_count, 0);
        let first = tokens[0].clone();
        Self {
            tokens,
            token_count,
            curr_token_index: 0,
            prev_token_index: 0,
            curr_token: first.clone(),
            prev_token: first,
        }
    }

    //fn _next_token(&mut self) -> bool {
    //    if self.curr_token_index + 1 < self.token_count {
    //        self.curr_token_index += 1;
    //        self.curr_token = self
    //            .tokens
    //            .get(self.curr_token_index)
    //            .expect("curr_token_index should never be out of bounds")
    //            .clone();
    //        true
    //    } else {
    //        false
    //    }
    //}

    /// advances to the next token \
    /// returns the token before advancement
    fn advance_token(&mut self) -> &Token {
        self.prev_token_index = self.curr_token_index;
        self.prev_token = self.curr_token.clone();

        if self.curr_token_index + 1 < self.token_count {
            self.curr_token_index += 1;
            self.curr_token = self
                .tokens
                .get(self.curr_token_index)
                .expect("curr_token_index should never be out of bounds")
                .clone();
        }

        &self.prev_token
    }

    #[inline]
    fn expect_token(&mut self, kind: TokenKind) -> &Token {
        if self.curr_token.kind != kind {
            todo!("expect_token: {}, found: {}", kind, self.curr_token.kind);
        } else {
            self.advance_token();
            &self.prev_token
        }
    }

    #[inline]
    fn current(&self) -> &Token {
        &self.curr_token
    }

    #[inline]
    fn previous(&self) -> &Token {
        &self.prev_token
    }
}

fn parse_operand(f: &mut AstFile) -> AST {
    use AstKind as AK;
    use TokenKind as TK;
    let span = f.current().span.clone();
    match f.current().kind.clone() {
        TK::Ident(name) => {
            f.advance_token();
            AST::new(AK::Ident(name), span)
        }

        TK::Integer(val) => {
            f.advance_token();
            AST::new(AK::Integer(val), span)
        }

        TK::OpenParen => {
            let open = f.expect_token(TK::OpenParen).clone();
            let operand = parse_expr(f);
            let close = f.expect_token(TK::CloseParen).clone();
            let span = open.span.start..close.span.end;
            AST::new(AK::ParenExpr(open, close, operand), span)
        }
        _ => todo!("unexpected EOF"),
    }
}

fn parse_unary_expr(f: &mut AstFile) -> AST {
    match &f.current().kind {
        unary_op if unary_op.is_unary_op() => {
            let op = f.current().clone();
            let operand = parse_operand(f);
            AST::unary(op, operand)
        }
        _ => parse_operand(f),
    }
}

fn parse_binary_expr(f: &mut AstFile, prec_in: u32) -> AST {
    let mut expr = parse_unary_expr(f);

    loop {
        let op = f.current().clone();
        let op_prec = op.kind.precedence();
        if op_prec < prec_in {
            break;
        }

        if !op.kind.is_binary_op() {
            todo!("syntax error: not a binary op");
        }

        f.advance_token();
        let rhs = parse_binary_expr(f, op_prec + 1);
        expr = AST::binary(op, expr, rhs);
    }

    expr
}

pub fn parse_expr(f: &mut AstFile) -> AST {
    parse_binary_expr(f, 0 + 1)
}

// TODO: arena allocator
#[derive(Debug, PartialEq, Clone)]
pub struct AST {
    kind: Box<AstKind>,
    span: Span,
}

impl fmt::Display for AST {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl AST {
    pub fn new(kind: AstKind, span: Span) -> Self {
        Self {
            kind: kind.into(),
            span,
        }
    }

    pub fn binary(tok: Token, lhs: AST, rhs: AST) -> Self {
        assert!(tok.kind.is_binary_op());
        let span = lhs.span.start..rhs.span.end;
        Self::new(AstKind::Binary(tok, lhs, rhs), span)
    }

    pub fn unary(tok: Token, expr: AST) -> Self {
        assert!(tok.kind.is_unary_op());
        let span = tok.span.start..expr.span.end;
        Self::new(AstKind::Unary(tok, expr), span)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstKind {
    Ident(Rc<str>),
    Integer(u32),
    Binary(Token, AST, AST),      // e.g op, expr, expr
    Unary(Token, AST),            // e.g op, expr
    ParenExpr(Token, Token, AST), // open, close, expr
}

impl fmt::Display for AstKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use AstKind as AK;
        match self {
            AK::Ident(name) => write!(f, "{}", name),
            AK::Integer(val) => write!(f, "{}", val),
            AK::Binary(op, lhs, rhs) => write!(f, "({} {} {})", lhs, op, rhs),
            AK::Unary(op, expr) => write!(f, "({} {})", op, expr),
            AK::ParenExpr(open, close, expr) => write!(f, "{} {} {}", open, expr, close),
        }
    }
}
