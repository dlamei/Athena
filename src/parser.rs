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
    pub kind: TokenKind,
    pub span: Span,
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
//#[logos(subpattern float = r"[+-]?[0-9]*[.][0-9]+")]
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
    #[display(fmt = ",")]
    #[token(",")]
    Comma,
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
                    let err = ErrCode::Lexer.to_err(&span, "unknown character");
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

const MAX_N_ERRORS: usize = 1;

#[derive(Debug, PartialEq, Clone)]
pub struct AstFile {
    tokens: Box<[Token]>,
    token_count: usize,
    curr_token_index: usize,
    prev_token_index: usize,
    curr_token: Token,
    prev_token: Token,

    pub errors: Vec<Error>,
}

impl AstFile {
    pub fn from_tokens(tokens: Box<[Token]>) -> Self {
        //assert_ne!(token_count, 0);
        let token_count = tokens.len();
        let first = tokens.get(0).cloned().unwrap_or(Token {
            kind: TokenKind::EOF,
            span: 0..0,
        });
        Self {
            tokens,
            token_count,
            curr_token_index: 0,
            prev_token_index: 0,
            curr_token: first.clone(),
            prev_token: first,
            errors: vec![],
        }
    }

    fn jump_to_end(&mut self) {
        self.curr_token_index = self.tokens.len() - 1;
        self.prev_token_index = self.curr_token_index;
        self.curr_token = self
            .tokens
            .get(self.curr_token_index)
            .expect("curr_token_index should never be out of bounds")
            .clone();
        self.prev_token = self.curr_token.clone();
    }

    fn syntax_err<MSG: Into<String>>(&mut self, span: &Span, msg: MSG) {
        if self.errors.len() >= MAX_N_ERRORS {
            self.jump_to_end();
            return;
        }

        self.errors.push(ErrCode::Syntax.to_err(span, msg));
    }

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
            let curr = self.curr_token.clone();
            self.syntax_err(
                &curr.span,
                format!("expected token: '{}', found: {}", kind, curr.kind),
            );
        }

        self.advance_token();
        &self.prev_token
    }

    #[inline]
    fn current(&self) -> &Token {
        &self.curr_token
    }

    // #[inline]
    // fn previous(&self) -> &Token {
    //     &self.prev_token
    // }
}

fn parse_func_args(name: Rc<str>, start_span: usize, f: &mut AstFile) -> Result<AST, AstError> {
    use TokenKind as TK;
    let _ = f.expect_token(TK::OpenParen);
    let mut args = vec![];
    while f.current().kind != TK::CloseParen {
        args.push(parse_expr(f));
        if f.current().kind != TK::Comma {
            break;
        }
        f.advance_token();
    }
    let close = f.expect_token(TK::CloseParen);
    Ok(AST::new(
        AstKind::Func(name, args),
        start_span..close.span.end,
    ))
}

fn parse_operand(f: &mut AstFile) -> Result<AST, AstError> {
    use AstKind as AK;
    use TokenKind as TK;
    let span = f.current().span.clone();
    match f.current().kind.clone() {
        TK::Ident(name) => {
            f.advance_token();
            if f.current().kind == TK::OpenParen {
                parse_func_args(name, span.start, f)
            } else {
                Ok(AST::new(AK::Ident(name), span))
            }
        }

        TK::Integer(val) => {
            f.advance_token();
            Ok(AST::new(AK::Integer(val), span))
        }

        TK::OpenParen => {
            let open = f.expect_token(TK::OpenParen).clone();
            //TODO: expr-level?
            let operand = parse_expr(f);
            let close = f.expect_token(TK::CloseParen).clone();
            let span = open.span.start..close.span.end;
            Ok(AST::new(AK::ParenExpr(open, close, operand), span))
        }
        tok => {
            f.advance_token();
            Err(AstError::new(AstErrorKind::BadExpr(tok), span))
        }
    }
}

fn parse_unary_expr(f: &mut AstFile) -> Result<AST, AstError> {
    match &f.current().kind {
        unary_op if unary_op.is_unary_op() => {
            let op = f.advance_token().clone();
            let operand = parse_operand(f).unwrap_or_else(|bad_expr| {
                bad_expr.syntax_err(f, format!("bad operand for unary '{}'", op.kind))
            });

            Ok(AST::unary(op, operand))
        }
        _ => parse_operand(f),
    }
}

fn parse_binary_expr(f: &mut AstFile, prec_in: u32) -> Result<AST, AstError> {
    let mut expr = parse_unary_expr(f)?;

    loop {
        let op = f.current().clone();
        let op_prec = op.kind.precedence();
        if op_prec < prec_in {
            break;
        }

        if !op.kind.is_binary_op() {
            //TODO: is that possible?
            panic!("syntax error: not a binary op");
        }

        f.advance_token();
        let rhs = parse_binary_expr(f, op_prec + 1).unwrap_or_else(|bad_rhs| {
            bad_rhs.syntax_err(f, format!("bad rhs for binary '{}'", op.kind))
        });

        expr = AST::binary(op, expr, rhs);
    }

    Ok(expr)
}

pub fn parse_expr(f: &mut AstFile) -> AST {
    if f.token_count == 0 {
        return AST::new(AstKind::Integer(0), 0..0);
    }
    parse_binary_expr(f, 0 + 1)
        .unwrap_or_else(|bad_expr| bad_expr.syntax_err(f, "could not parse expression"))
}

// TODO: arena allocator
#[derive(Debug, PartialEq, Clone)]
pub struct AST {
    pub kind: Box<AstKind>,
    pub span: Span,
    pub has_err: bool,
}

impl fmt::Display for AST {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl AST {
    pub fn new(kind: AstKind, span: Span) -> Self {
        Self {
            has_err: matches![kind, AstKind::Err(_)],
            span,
            kind: kind.into(),
        }
    }

    pub fn binary(tok: Token, lhs: AST, rhs: AST) -> Self {
        assert!(tok.kind.is_binary_op());
        let span = lhs.span.start..rhs.span.end;
        let has_err = lhs.has_err || rhs.has_err;
        Self {
            kind: AstKind::Binary(tok, lhs, rhs).into(),
            span,
            has_err,
        }
    }

    pub fn unary(tok: Token, expr: AST) -> Self {
        assert!(tok.kind.is_unary_op());
        let span = tok.span.start..expr.span.end;
        let has_err = expr.has_err;
        Self {
            kind: AstKind::Unary(tok, expr).into(),
            span,
            has_err,
        }
    }

    pub fn err(err: AstError) -> Self {
        Self::new(AstKind::Err(err.kind), err.span)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstKind {
    Ident(Rc<str>),
    Integer(u32),
    Binary(Token, AST, AST),      // e.g op, expr, expr
    Unary(Token, AST),            // e.g op, expr
    ParenExpr(Token, Token, AST), // open, close, expr
    Func(Rc<str>, Vec<AST>),

    Err(AstErrorKind),
}

#[derive(Debug, PartialEq, Clone)]
pub struct AstError {
    kind: AstErrorKind,
    span: Span,
}

impl AstError {
    pub fn new(kind: AstErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn syntax_err<MSG: std::fmt::Display>(self, f: &mut AstFile, m: MSG) -> AST {
        f.syntax_err(&self.span, format!("{}: {}", m, self.kind));
        AST::err(self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstErrorKind {
    BadExpr(TokenKind),
}

impl fmt::Display for AstErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use AstErrorKind as AE;
        match self {
            AE::BadExpr(tok) => write!(f, "{}", tok),
        }
    }
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
            AK::Err(err) => write!(f, "{:?}", err),
            AK::Func(name, args) => {
                if args.is_empty() {
                    return write!(f, "{name}()");
                }
                write!(f, "{name}")?;
                write!(f, "(")?;
                let mut args = args.iter();
                if let Some(a) = args.next() {
                    write!(f, "{a}")?;
                }

                for a in args {
                    write!(f, ", {a}")?;
                }
                write!(f, ")")
            }
        }
    }
}
