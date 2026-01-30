use alloc::collections::TryReserveError;
use crate::ast::{CallExpr, Expr, ExprList, InfixExpr, LetStmt, ParensExpr, Program, Stmt};
use crate::lexer::{Lexer, LexerError};
use crate::token::{
    IdentToken, Keyword, KeywordToken, Location, NumberToken, Symbol, SymbolToken, Token,
};
use arena::{Arena, ArenaVec};
use core::iter::Peekable;

pub struct Parser<'par, I: Iterator<Item = Result<Token<'par>, LexerError<'par>>>> {
    tokens: Peekable<I>,
    arena: &'par Arena,
}
use crate::vec_ext::VecExt;
use alloc::vec::Vec;

#[derive(Debug)]
pub enum ParserError<'par> {
    LexerError(LexerError<'par>),
    UnexpectedEof,
    ExpectedIdent,
    ExpectedSymbol(Symbol),
    ExpectedKeyword(Keyword),
    AllocError,
}

#[derive(Debug)]
pub struct AnnotatedParserError<'par> {
    pub cause: ParserError<'par>,
    pub next_token: Option<Token<'par>>,
}

impl<'par> From<LexerError<'par>> for ParserError<'par> {
    fn from(e: LexerError<'par>) -> Self {
        ParserError::LexerError(e)
    }
}

impl<'par> From<alloc::alloc::AllocError> for ParserError<'par> {
    fn from(_: alloc::alloc::AllocError) -> Self {
        ParserError::AllocError
    }
}

impl<'par> From<TryReserveError> for ParserError<'par> {
    fn from(_: TryReserveError) -> Self {
        ParserError::AllocError
    }
}

impl<'par, I> Parser<'par, I>
where
    I: Iterator<Item = Result<Token<'par>, LexerError<'par>>>,
{
    pub fn new(tokens: I, arena: &'par Arena) -> Self {
        Self {
            tokens: tokens.peekable(),
            arena,
        }
    }
    fn try_peek_token(&mut self) -> Result<Option<&Token<'par>>, ParserError<'par>> {
        if let Some(token) = self.tokens.peek() {
            match token {
                Ok(x) => Ok(Some(x)),
                Err(e) => Err((*e).into()),
            }
        } else {
            Ok(None)
        }
    }
    fn peek_token(&mut self) -> Result<&Token<'par>, ParserError<'par>> {
        Ok(self.try_peek_token()?.ok_or(ParserError::UnexpectedEof)?)
    }
    fn try_next_token(&mut self) -> Result<Option<Token<'par>>, ParserError<'par>> {
        if let Some(token) = self.tokens.next() {
            Ok(Some(token?))
        } else {
            Ok(None)
        }
    }
    fn next_token(&mut self) -> Result<Token<'par>, ParserError<'par>> {
        Ok(self.try_next_token()?.ok_or(ParserError::UnexpectedEof)?)
    }
    pub fn parse_program(&mut self) -> Result<Program<'par>, AnnotatedParserError<'par>> {
        match self.parse_program_ast() {
            Ok(program) => Ok(program),
            Err(e) => Err(AnnotatedParserError {
                cause: e,
                next_token: match self.tokens.next() {
                    Some(Ok(x)) => Some(x),
                    _ => None,
                },
            }),
        }
    }
    fn parse_program_ast(&mut self) -> Result<Program<'par>, ParserError<'par>> {
        let mut stmts = Vec::new_in(self.arena);
        loop {
            if let Some(_eof) = self.try_parse_eof()? {
                break;
            }
            stmts.try_push(self.try_parse_statement()?)?;
        }
        Ok(Program { stmts })
    }
    fn try_parse_eof(&mut self) -> Result<Option<()>, ParserError<'par>> {
        if let Some(_) = self.try_peek_token()? {
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
    fn try_parse_statement(&mut self) -> Result<Stmt<'par>, ParserError<'par>> {
        let result = if let Some(let_stmt) = self.try_parse_let_statement()? {
            Stmt::Let(let_stmt)
        } else {
            Stmt::ExprStmt(self.parse_expr()?)
        };
        self.parse_symbol(Symbol::Semi)?;
        Ok(result)
    }
    fn try_parse_let_statement(&mut self) -> Result<Option<LetStmt<'par>>, ParserError<'par>> {
        let Some(let_token) = self.try_parse_keyword(Keyword::Let)? else {
            return Ok(None);
        };
        let ident = self.parse_ident()?;
        let equals = self.parse_symbol(Symbol::Equals)?;
        let expr = self.parse_expr()?;
        Ok(Some(LetStmt {
            let_token,
            ident,
            equals,
            expr,
        }))
    }
    fn parse_expr(&mut self) -> Result<Expr<'par>, ParserError<'par>> {
        self.parse_expr2()
    }
    fn parse_expr2(&mut self) -> Result<Expr<'par>, ParserError<'par>> {
        let mut expr = self.parse_expr1()?;
        loop {
            let symbol = if let Some(plus) = self.try_parse_symbol(Symbol::Plus)? {
                plus
            } else if let Some(minus) = self.try_parse_symbol(Symbol::Minus)? {
                minus
            } else {
                break;
            };
            let expr2 = self.parse_expr1()?;
            expr = Expr::InfixExpr(InfixExpr {
                left: self.arena.alloc_box(expr)?,
                symbol,
                right: self.arena.alloc_box(expr2)?,
            });
        }
        Ok(expr)
    }
    fn parse_expr1(&mut self) -> Result<Expr<'par>, ParserError<'par>> {
        let mut expr = self.parse_expr0()?;
        loop {
            if let Some(lparen) = self.try_parse_symbol(Symbol::LParen)? {
                let args = self.parse_expr_list()?;
                let rparen = self.parse_symbol(Symbol::RParen)?;
                expr = Expr::Call(CallExpr {
                    callee: self.arena.alloc_box(expr)?,
                    lparen,
                    args,
                    rparen,
                });
            } else {
                break;
            }
        }
        Ok(expr)
    }
    fn parse_expr0(&mut self) -> Result<Expr<'par>, ParserError<'par>> {
        if let Some(ident) = self.try_parse_ident()? {
            Ok(Expr::Var(ident))
        } else if let Some(number) = self.try_parse_number()? {
            Ok(Expr::Number(number))
        } else if let Some(lparen) = self.try_parse_symbol(Symbol::LParen)? {
            let expr = self.parse_expr()?;
            let rparen = self.parse_symbol(Symbol::RParen)?;
            Ok(Expr::Parens(ParensExpr {
                lparen,
                expr: self.arena.alloc_box(expr)?,
                rparen,
            }))
        } else {
            todo!();
        }
    }
    fn parse_expr_list(&mut self) -> Result<ExprList<'par>, ParserError<'par>> {
        let mut exprs = Vec::new_in(self.arena);
        let mut commas = Vec::new_in(self.arena);
        exprs.try_push(self.parse_expr()?)?;
        loop {
            if let Some(comma) = self.try_parse_symbol(Symbol::Comma)? {
                commas.try_push(comma)?;
                exprs.try_push(self.parse_expr()?)?;
            } else {
                break;
            }
        }
        Ok(ExprList { exprs, commas })
    }
    fn parse_ident(&mut self) -> Result<IdentToken<'par>, ParserError<'par>> {
        Ok(self.try_parse_ident()?.ok_or(ParserError::ExpectedIdent)?)
    }
    fn parse_symbol(&mut self, symbol: Symbol) -> Result<SymbolToken, ParserError<'par>> {
        Ok(self
            .try_parse_symbol(symbol)?
            .ok_or(ParserError::ExpectedSymbol(symbol))?)
    }
    fn parse_keyword(&mut self, keyword: Keyword) -> Result<KeywordToken, ParserError<'par>> {
        Ok(self
            .try_parse_keyword(keyword)?
            .ok_or(ParserError::ExpectedKeyword(keyword))?)
    }
    fn try_parse_keyword(&mut self, k: Keyword) -> Result<Option<KeywordToken>, ParserError<'par>> {
        match self.try_peek_token()? {
            Some(Token::Keyword(KeywordToken { keyword, .. })) if *keyword == k => {
                match self.next_token()? {
                    Token::Keyword(token) => Ok(Some(token)),
                    _ => unreachable!(),
                }
            }
            _ => Ok(None),
        }
    }
    fn try_parse_symbol(&mut self, s: Symbol) -> Result<Option<SymbolToken>, ParserError<'par>> {
        match self.try_peek_token()? {
            Some(Token::Symbol(SymbolToken { symbol, .. })) if *symbol == s => {
                match self.next_token()? {
                    Token::Symbol(token) => Ok(Some(token)),
                    _ => unreachable!(),
                }
            }
            _ => Ok(None),
        }
    }
    fn try_parse_ident(&mut self) -> Result<Option<IdentToken<'par>>, ParserError<'par>> {
        match self.try_peek_token()? {
            Some(Token::Ident(_)) => match self.next_token()? {
                Token::Ident(token) => Ok(Some(token)),
                _ => unreachable!(),
            },
            _ => Ok(None),
        }
    }
    fn try_parse_number(&mut self) -> Result<Option<NumberToken<'par>>, ParserError<'par>> {
        match self.try_peek_token()? {
            Some(Token::Number(_)) => match self.next_token()? {
                Token::Number(token) => Ok(Some(token)),
                _ => unreachable!(),
            },
            _ => Ok(None),
        }
    }
}
