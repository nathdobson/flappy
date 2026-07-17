use crate::compiler::ast::{
    CallExpr, ElseClause, Expr, ExprList, ForStmt, IfStmt, InfixExpr, LetStmt, LoopStmt,
    MutateStmt, ParensExpr, PrefixExpr, Program, Stmt, WhileStmt,
};
use crate::compiler::lexer::{Lexer, LexerError};
use crate::compiler::token::{
    IdentToken, Keyword, KeywordToken, Location, NumberToken, Symbol, SymbolToken, Token,
};
use alloc::collections::TryReserveError;
use arena::{Arena, ArenaVec, IntoRef};
use core::iter::Peekable;

pub struct Parser<'src: 'par, 'par, I: Iterator<Item = Result<Token<'src, 'par>, LexerError<'src>>>>
{
    tokens: Lookahead<2, I>,
    arena: &'par Arena,
}
use crate::compiler::lookahead::Lookahead;
use crate::compiler::stack::Stack;
use crate::compiler::stack_executor::StackSpawn;
use crate::vec_ext::VecExt;
use crate::vm::VmOperator;
use alloc::vec::Vec;

#[derive(Debug)]
pub enum ParserError<'src> {
    LexerError(LexerError<'src>),
    UnexpectedEof,
    ExpectedIdent,
    ExpectedSymbol(Symbol),
    ExpectedKeyword(Keyword),
    AllocError,
    ExpectedExpr,
    ExpectedIfOrBrace,
}

#[derive(Debug)]
pub struct AnnotatedParserError<'src> {
    pub cause: ParserError<'src>,
    pub next_token: Option<Token<'src, 'static>>,
}

impl<'src> From<LexerError<'src>> for ParserError<'src> {
    fn from(e: LexerError<'src>) -> Self {
        ParserError::LexerError(e)
    }
}

impl<'src> From<alloc::alloc::AllocError> for ParserError<'src> {
    fn from(_: alloc::alloc::AllocError) -> Self {
        ParserError::AllocError
    }
}

impl<'src> From<TryReserveError> for ParserError<'src> {
    fn from(_: TryReserveError) -> Self {
        ParserError::AllocError
    }
}

impl<'src: 'par, 'par, I> Parser<'src, 'par, I>
where
    I: Iterator<Item = Result<Token<'src, 'par>, LexerError<'src>>>,
{
    pub fn new(tokens: I, arena: &'par Arena) -> Self {
        Self {
            tokens: Lookahead::new(tokens),
            arena,
        }
    }
    fn try_peek_token(
        &mut self,
        n: usize,
    ) -> Result<Option<&Token<'src, 'par>>, ParserError<'src>> {
        if let Some(token) = self.tokens.peek(n) {
            match token {
                Ok(x) => Ok(Some(x)),
                Err(e) => Err((*e).into()),
            }
        } else {
            Ok(None)
        }
    }

    fn peek_token(&mut self, n: usize) -> Result<&Token<'src, 'par>, ParserError<'src>> {
        Ok(self.try_peek_token(n)?.ok_or(ParserError::UnexpectedEof)?)
    }
    fn try_next_token(&mut self) -> Result<Option<Token<'src, 'par>>, ParserError<'src>> {
        if let Some(token) = self.tokens.next() {
            Ok(Some(token?))
        } else {
            Ok(None)
        }
    }
    fn next_token(&mut self) -> Result<Token<'src, 'par>, ParserError<'src>> {
        Ok(self.try_next_token()?.ok_or(ParserError::UnexpectedEof)?)
    }
    pub async fn parse_program(
        &mut self,
        stack: StackSpawn<'_>,
    ) -> Result<&'par Program<'src, 'par>, AnnotatedParserError<'src>> {
        match self.parse_program_ast(stack).await {
            Ok(program) => Ok(program),
            Err(e) => Err(AnnotatedParserError {
                cause: e,
                next_token: match self.tokens.next() {
                    Some(Ok(x)) => Some(x.erased()),
                    _ => None,
                },
            }),
        }
    }
    async fn parse_program_ast(
        &mut self,
        stack: StackSpawn<'_>,
    ) -> Result<&'par Program<'src, 'par>, ParserError<'src>> {
        Ok(self.arena.alloc_ref(Program {
            stmts: self.parse_statement_list(stack).await?,
        })?)
    }
    async fn parse_statement_list(
        &mut self,
        mut stack: StackSpawn<'_>,
    ) -> Result<&'par [Stmt<'src, 'par>], ParserError<'src>> {
        Ok(stack
            .recurse(async |mut stack| {
                let mut stmts = Vec::new_in(self.arena);
                while let Some(stmt) = self.try_parse_statement(stack.reborrow()).await? {
                    stmts.try_push(stmt)?;
                }
                Ok::<_, ParserError<'src>>(stmts.into_ref())
            })
            .await??)
    }
    fn try_parse_eof(&mut self) -> Result<Option<()>, ParserError<'src>> {
        if let Some(_) = self.try_peek_token(0)? {
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
    async fn try_parse_statement(
        &mut self,
        mut stack: StackSpawn<'_>,
    ) -> Result<Option<Stmt<'src, 'par>>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            None => return Ok(None),
            Some(token) => match token {
                Token::Symbol(symbol) => match symbol.symbol {
                    Symbol::RBrace => return Ok(None),
                    _ => {}
                },
                _ => {}
            },
        }
        if let Some(if_stmt) = self.try_parse_if_statement(stack.reborrow()).await? {
            Ok(Some(Stmt::If(if_stmt)))
        } else if let Some(let_stmt) = self.try_parse_let_statement()? {
            Ok(Some(Stmt::Let(let_stmt)))
        } else if let Some(for_stmt) = self.try_parse_for_statement(stack.reborrow()).await? {
            Ok(Some(Stmt::For(for_stmt)))
        } else if let Some(loop_stmt) = self.try_parse_loop_statement(stack.reborrow()).await? {
            Ok(Some(Stmt::Loop(loop_stmt)))
        } else if let Some(while_stmt) = self.try_parse_while_statement(stack.reborrow()).await? {
            Ok(Some(Stmt::While(while_stmt)))
        } else if let Some(break_stmt) = self.try_parse_break_statement()? {
            Ok(Some(Stmt::Break))
        } else if let Some(break_stmt) = self.try_parse_continue_statement()? {
            Ok(Some(Stmt::Continue))
        } else if let Some(reassign_stmt) = self.try_parse_reassign_statement()? {
            Ok(Some(Stmt::Mutate(reassign_stmt)))
        } else {
            let result = Stmt::ExprStmt(self.parse_expr()?);
            self.parse_symbol(Symbol::Semi)?;
            Ok(Some(result))
        }
    }
    fn try_parse_let_statement(
        &mut self,
    ) -> Result<Option<LetStmt<'src, 'par>>, ParserError<'src>> {
        let Some(let_token) = self.try_parse_keyword(Keyword::Let)? else {
            return Ok(None);
        };
        let ident = self.parse_ident()?;
        let equals = self.parse_symbol(Symbol::Equals)?;
        let expr = self.parse_expr()?;
        self.parse_symbol(Symbol::Semi)?;
        Ok(Some(LetStmt {
            let_token,
            ident,
            equals,
            expr,
        }))
    }
    async fn try_parse_for_statement(
        &mut self,
        stack: StackSpawn<'_>,
    ) -> Result<Option<ForStmt<'src, 'par>>, ParserError<'src>> {
        let Some(for_token) = self.try_parse_keyword(Keyword::For)? else {
            return Ok(None);
        };
        let ident = self.parse_ident()?;
        self.parse_keyword(Keyword::In)?;
        let init_expr = self.parse_expr()?;
        self.parse_symbol(Symbol::DotDot)?;
        let limit_expr = self.parse_expr()?;
        let open_brace = self.parse_symbol(Symbol::LBrace)?;
        let inner = self.parse_statement_list(stack).await?;
        let close_brace = self.parse_symbol(Symbol::RBrace)?;
        Ok(Some(ForStmt {
            for_token,
            ident,
            init_expr,
            limit_expr,
            open_brace,
            inner,
            close_brace,
        }))
    }
    async fn try_parse_if_statement(
        &mut self,
        mut stack: StackSpawn<'_>,
    ) -> Result<Option<IfStmt<'src, 'par>>, ParserError<'src>> {
        Ok(stack
            .recurse(async |mut stack| -> Result<_, ParserError<'src>> {
                let Some(if_token) = self.try_parse_keyword(Keyword::If)? else {
                    return Ok(None);
                };
                let cond = self.parse_expr()?;
                let open_brace = self.parse_symbol(Symbol::LBrace)?;
                let then = self.parse_statement_list(stack.reborrow()).await?;
                let close_brace = self.parse_symbol(Symbol::RBrace)?;
                let else_clause = self.try_parse_else_clause(stack.reborrow()).await?;
                Ok(Some(IfStmt {
                    if_token,
                    cond_expr: cond,
                    open_brace,
                    then_stmt: then,
                    close_brace,
                    else_clause,
                }))
            })
            .await??)
    }
    async fn try_parse_else_clause(
        &mut self,
        mut stack: StackSpawn<'_>,
    ) -> Result<Option<ElseClause<'src, 'par>>, ParserError<'src>> {
        if let Some(else_token) = self.try_parse_keyword(Keyword::Else)? {
            if let Some(open_brace) = self.try_parse_symbol(Symbol::LBrace)? {
                let else_stmt = self.parse_statement_list(stack).await?;
                let close_brace = self.parse_symbol(Symbol::RBrace)?;
                Ok(Some(ElseClause::Else {
                    else_token,
                    open_brace,
                    else_stmt,
                    close_brace,
                }))
            } else {
                Ok(Some(ElseClause::ElseIf {
                    else_token,
                    else_if_stmt: self.arena.alloc_ref(
                        self.try_parse_if_statement(stack.reborrow())
                            .await?
                            .ok_or(ParserError::ExpectedIfOrBrace)?,
                    )?,
                }))
            }
        } else {
            Ok(None)
        }
    }
    async fn try_parse_loop_statement(
        &mut self,
        stack: StackSpawn<'_>,
    ) -> Result<Option<LoopStmt<'src, 'par>>, ParserError<'src>> {
        let Some(loop_token) = self.try_parse_keyword(Keyword::Loop)? else {
            return Ok(None);
        };
        let open_brace = self.parse_symbol(Symbol::LBrace)?;
        let inner = self.parse_statement_list(stack).await?;
        let close_brace = self.parse_symbol(Symbol::RBrace)?;
        Ok(Some(LoopStmt {
            loop_token,
            open_brace,
            inner,
            close_brace,
        }))
    }
    async fn try_parse_while_statement(
        &mut self,
        stack: StackSpawn<'_>,
    ) -> Result<Option<WhileStmt<'src, 'par>>, ParserError<'src>> {
        let Some(while_token) = self.try_parse_keyword(Keyword::While)? else {
            return Ok(None);
        };
        let cond = self.parse_expr()?;
        let open_brace = self.parse_symbol(Symbol::LBrace)?;
        let inner = self.parse_statement_list(stack).await?;
        let close_brace = self.parse_symbol(Symbol::RBrace)?;
        Ok(Some(WhileStmt {
            while_token,
            cond,
            open_brace,
            inner,
            close_brace,
        }))
    }
    fn try_parse_break_statement(&mut self) -> Result<Option<()>, ParserError<'src>> {
        let Some(break_token) = self.try_parse_keyword(Keyword::Break)? else {
            return Ok(None);
        };
        self.parse_symbol(Symbol::Semi)?;
        Ok(Some(()))
    }
    fn try_parse_continue_statement(&mut self) -> Result<Option<()>, ParserError<'src>> {
        let Some(break_token) = self.try_parse_keyword(Keyword::Continue)? else {
            return Ok(None);
        };
        self.parse_symbol(Symbol::Semi)?;
        Ok(Some(()))
    }
    fn try_parse_reassign_statement(
        &mut self,
    ) -> Result<Option<MutateStmt<'src, 'par>>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            Some(Token::Ident(ident)) => {
                let ident = *ident;
                match self.try_peek_token(1)? {
                    Some(Token::Symbol(oper)) => {
                        let oper = *oper;
                        match oper.symbol {
                            Symbol::Equals | Symbol::PlusEquals | Symbol::MinusEquals => {
                                self.next_token()?;
                                self.next_token()?;
                                let expr = self.parse_expr()?;
                                self.parse_symbol(Symbol::Semi)?;
                                Ok(Some(MutateStmt {
                                    ident,
                                    oper,
                                    expr: Some(expr),
                                }))
                            }
                            Symbol::PlusPlus | Symbol::MinusMinus => {
                                self.next_token()?;
                                self.next_token()?;
                                self.parse_symbol(Symbol::Semi)?;
                                Ok(Some(MutateStmt {
                                    ident,
                                    oper,
                                    expr: None,
                                }))
                            }
                            _ => Ok(None),
                        }
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
    fn parse_expr(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        self.parse_expr5()
    }
    fn parse_expr5(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        let mut expr = self.parse_expr4()?;
        loop {
            let symbol = if let Some(and_and) = self.try_parse_symbol(Symbol::AndAnd)? {
                and_and
            } else if let Some(or_or) = self.try_parse_symbol(Symbol::OrOr)? {
                or_or
            } else {
                break;
            };
            let expr2 = self.parse_expr4()?;
            expr = Expr::InfixExpr(InfixExpr {
                left: self.arena.alloc_ref(expr)?,
                symbol,
                right: self.arena.alloc_ref(expr2)?,
            });
        }
        Ok(expr)
    }
    fn parse_expr4(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        let mut expr = self.parse_expr3()?;
        loop {
            let symbol = if let Some(less) = self.try_parse_symbol(Symbol::Less)? {
                less
            } else if let Some(less_equals) = self.try_parse_symbol(Symbol::LessEquals)? {
                less_equals
            } else if let Some(greater) = self.try_parse_symbol(Symbol::Greater)? {
                greater
            } else if let Some(greater_equals) = self.try_parse_symbol(Symbol::GreaterEquals)? {
                greater_equals
            } else if let Some(equals_equals) = self.try_parse_symbol(Symbol::EqualsEquals)? {
                equals_equals
            } else {
                break;
            };
            let expr2 = self.parse_expr3()?;
            expr = Expr::InfixExpr(InfixExpr {
                left: self.arena.alloc_ref(expr)?,
                symbol,
                right: self.arena.alloc_ref(expr2)?,
            });
        }
        Ok(expr)
    }
    fn parse_expr3(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        let mut expr = self.parse_expr2()?;
        loop {
            let symbol = if let Some(plus) = self.try_parse_symbol(Symbol::Plus)? {
                plus
            } else if let Some(minus) = self.try_parse_symbol(Symbol::Minus)? {
                minus
            } else {
                break;
            };
            let expr2 = self.parse_expr2()?;
            expr = Expr::InfixExpr(InfixExpr {
                left: self.arena.alloc_ref(expr)?,
                symbol,
                right: self.arena.alloc_ref(expr2)?,
            });
        }
        Ok(expr)
    }
    fn parse_expr2(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        let mut expr = self.parse_expr1()?;
        loop {
            let symbol = if let Some(times) = self.try_parse_symbol(Symbol::Times)? {
                times
            } else if let Some(divide) = self.try_parse_symbol(Symbol::Divide)? {
                divide
            } else if let Some(modulo) = self.try_parse_symbol(Symbol::Remainder)? {
                modulo
            } else {
                break;
            };
            let expr2 = self.parse_expr1()?;
            expr = Expr::InfixExpr(InfixExpr {
                left: self.arena.alloc_ref(expr)?,
                symbol,
                right: self.arena.alloc_ref(expr2)?,
            });
        }
        Ok(expr)
    }

    fn parse_expr1(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        let mut expr = self.parse_expr0()?;
        loop {
            if let Some(lparen) = self.try_parse_symbol(Symbol::LParen)? {
                let args = self.parse_expr_list()?;
                let rparen = self.parse_symbol(Symbol::RParen)?;
                expr = Expr::Call(CallExpr {
                    callee: self.arena.alloc_ref(expr)?,
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
    fn parse_expr0(&mut self) -> Result<Expr<'src, 'par>, ParserError<'src>> {
        if let Some(str) = self.try_parse_string_literal()? {
            Ok(Expr::String(str))
        } else if let Some(prefix) = self.try_parse_prefix_expr()? {
            Ok(Expr::PrefixExpr(prefix))
        } else if let Some(false_token) = self.try_parse_keyword(Keyword::False)? {
            Ok(Expr::False(false_token))
        } else if let Some(true_token) = self.try_parse_keyword(Keyword::True)? {
            Ok(Expr::True(true_token))
        } else if let Some(null_token) = self.try_parse_keyword(Keyword::Null)? {
            Ok(Expr::Null(null_token))
        } else if let Some(ident) = self.try_parse_ident()? {
            Ok(Expr::Var(ident))
        } else if let Some(number) = self.try_parse_number()? {
            Ok(Expr::Number(number))
        } else if let Some(lparen) = self.try_parse_symbol(Symbol::LParen)? {
            let expr = self.parse_expr()?;
            let rparen = self.parse_symbol(Symbol::RParen)?;
            Ok(Expr::Parens(ParensExpr {
                lparen,
                expr: self.arena.alloc_ref(expr)?,
                rparen,
            }))
        } else {
            return Err(ParserError::ExpectedExpr);
        }
    }
    fn try_parse_prefix_expr(
        &mut self,
    ) -> Result<Option<PrefixExpr<'src, 'par>>, ParserError<'src>> {
        let symbol = if let Some(symbol) = self.try_parse_symbol(Symbol::Not)? {
            symbol
        } else if let Some(symbol) = self.try_parse_symbol(Symbol::Minus)? {
            symbol
        } else {
            return Ok(None);
        };
        let inner = self.parse_expr()?;
        Ok(Some(PrefixExpr {
            symbol,
            inner: self.arena.alloc_ref(inner)?,
        }))
    }
    fn parse_expr_list(&mut self) -> Result<ExprList<'src, 'par>, ParserError<'src>> {
        match self.peek_token(0)? {
            Token::Symbol(SymbolToken {
                symbol: Symbol::RParen,
                ..
            }) => {
                return Ok(ExprList {
                    exprs: &[],
                    commas: &[],
                });
            }
            _ => {}
        }
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
        Ok(ExprList {
            exprs: exprs.into_ref(),
            commas: commas.into_ref(),
        })
    }
    fn parse_ident(&mut self) -> Result<IdentToken<'src>, ParserError<'src>> {
        Ok(self.try_parse_ident()?.ok_or(ParserError::ExpectedIdent)?)
    }
    fn parse_symbol(&mut self, symbol: Symbol) -> Result<SymbolToken, ParserError<'src>> {
        Ok(self
            .try_parse_symbol(symbol)?
            .ok_or(ParserError::ExpectedSymbol(symbol))?)
    }
    fn parse_keyword(&mut self, keyword: Keyword) -> Result<KeywordToken, ParserError<'src>> {
        Ok(self
            .try_parse_keyword(keyword)?
            .ok_or(ParserError::ExpectedKeyword(keyword))?)
    }
    fn try_parse_keyword(&mut self, k: Keyword) -> Result<Option<KeywordToken>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            Some(Token::Keyword(KeywordToken { keyword, .. })) if *keyword == k => {
                match self.next_token()? {
                    Token::Keyword(token) => Ok(Some(token)),
                    _ => unreachable!(),
                }
            }
            _ => Ok(None),
        }
    }
    fn try_parse_string_literal(&mut self) -> Result<Option<&'par str>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            Some(Token::String(_)) => match self.next_token()? {
                Token::String(token) => Ok(Some(token.value.unwrap())),
                _ => unreachable!(),
            },
            _ => Ok(None),
        }
    }
    fn try_parse_symbol(&mut self, s: Symbol) -> Result<Option<SymbolToken>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            Some(Token::Symbol(SymbolToken { symbol, .. })) if *symbol == s => {
                match self.next_token()? {
                    Token::Symbol(token) => Ok(Some(token)),
                    _ => unreachable!(),
                }
            }
            _ => Ok(None),
        }
    }
    fn try_parse_ident(&mut self) -> Result<Option<IdentToken<'src>>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            Some(Token::Ident(_)) => match self.next_token()? {
                Token::Ident(token) => Ok(Some(token)),
                _ => unreachable!(),
            },
            _ => Ok(None),
        }
    }
    fn try_parse_number(&mut self) -> Result<Option<NumberToken<'src>>, ParserError<'src>> {
        match self.try_peek_token(0)? {
            Some(Token::Number(_)) => match self.next_token()? {
                Token::Number(token) => Ok(Some(token)),
                _ => unreachable!(),
            },
            _ => Ok(None),
        }
    }
}
