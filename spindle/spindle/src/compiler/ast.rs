use crate::compiler::token::{IdentToken, Keyword, KeywordToken, NumberToken, SymbolToken, Token};

#[derive(Debug, Copy, Clone)]
pub struct Program<'par> {
    pub stmts: &'par [Stmt<'par>],
}

#[derive(Debug, Copy, Clone)]
pub enum Stmt<'par> {
    Let(LetStmt<'par>),
    ExprStmt(Expr<'par>),
    For(ForStmt<'par>),
    If(IfStmt<'par>),
    Loop(LoopStmt<'par>),
    While(WhileStmt<'par>),
    Reassign(ReassignStmt<'par>),
    Break,
}

#[derive(Debug, Copy, Clone)]
pub struct LetStmt<'par> {
    pub let_token: KeywordToken,
    pub ident: IdentToken<'par>,
    pub equals: SymbolToken,
    pub expr: Expr<'par>,
}

#[derive(Debug, Copy, Clone)]
pub struct ForStmt<'par> {
    pub for_token: KeywordToken,
    pub ident: IdentToken<'par>,
    pub init_expr: Expr<'par>,
    pub limit_expr: Expr<'par>,
    pub open_brace: SymbolToken,
    pub inner: &'par [Stmt<'par>],
    pub close_brace: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub struct IfStmt<'par> {
    pub if_token: KeywordToken,
    pub cond_expr: Expr<'par>,
    pub open_brace: SymbolToken,
    pub then_stmt: &'par [Stmt<'par>],
    pub close_brace: SymbolToken,
    pub else_clause: Option<ElseClause<'par>>,
}

#[derive(Debug, Copy, Clone)]
pub struct LoopStmt<'par> {
    pub loop_token: KeywordToken,
    pub open_brace: SymbolToken,
    pub inner: &'par [Stmt<'par>],
    pub close_brace: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub struct WhileStmt<'par> {
    pub while_token: KeywordToken,
    pub cond: Expr<'par>,
    pub open_brace: SymbolToken,
    pub inner: &'par [Stmt<'par>],
    pub close_brace: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub enum ElseClause<'par> {
    Else {
        else_token: KeywordToken,
        open_brace: SymbolToken,
        else_stmt: &'par [Stmt<'par>],
        close_brace: SymbolToken,
    },
    ElseIf {
        else_token: KeywordToken,
        else_if_stmt: &'par IfStmt<'par>,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct ReassignStmt<'par> {
    pub ident: IdentToken<'par>,
    pub equals: SymbolToken,
    pub expr: Expr<'par>,
}

#[derive(Debug, Copy, Clone)]
pub struct InfixExpr<'par> {
    pub left: &'par Expr<'par>,
    pub symbol: SymbolToken,
    pub right: &'par Expr<'par>,
}

#[derive(Debug, Copy, Clone)]
pub struct CallExpr<'par> {
    pub callee: &'par Expr<'par>,
    pub lparen: SymbolToken,
    pub args: ExprList<'par>,
    pub rparen: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub struct ParensExpr<'par> {
    pub lparen: SymbolToken,
    pub expr: &'par Expr<'par>,
    pub rparen: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub enum Expr<'par> {
    Var(IdentToken<'par>),
    Parens(ParensExpr<'par>),
    Number(NumberToken<'par>),
    False(KeywordToken),
    True(KeywordToken),
    Null(KeywordToken),
    InfixExpr(InfixExpr<'par>),
    Call(CallExpr<'par>),
    String(&'par str),
}

#[derive(Debug, Copy, Clone)]
pub struct ExprList<'par> {
    pub exprs: &'par [Expr<'par>],
    pub commas: &'par [SymbolToken],
}
