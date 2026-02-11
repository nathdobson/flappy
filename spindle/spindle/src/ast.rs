use crate::token::{IdentToken, Keyword, KeywordToken, NumberToken, SymbolToken};
use arena::{ArenaBox, ArenaVec};

#[derive(Debug)]
pub struct Program<'par> {
    pub stmts: ArenaVec<'par, Stmt<'par>>,
}

#[derive(Debug)]
pub enum Stmt<'par> {
    Let(LetStmt<'par>),
    ExprStmt(Expr<'par>),
    For(ForStmt<'par>),
    If(IfStmt<'par>),
}

#[derive(Debug)]
pub struct LetStmt<'par> {
    pub let_token: KeywordToken,
    pub ident: IdentToken<'par>,
    pub equals: SymbolToken,
    pub expr: Expr<'par>,
}

#[derive(Debug)]
pub struct ForStmt<'par> {
    pub for_token: KeywordToken,
    pub ident: IdentToken<'par>,
    pub init_expr: Expr<'par>,
    pub limit_expr: Expr<'par>,
    pub open_brace: SymbolToken,
    pub inner: ArenaVec<'par, Stmt<'par>>,
    pub close_brace: SymbolToken,
}

#[derive(Debug)]
pub struct IfStmt<'par> {
    pub if_token: KeywordToken,
    pub cond_expr: Expr<'par>,
    pub open_brace: SymbolToken,
    pub then_stmt: ArenaVec<'par, Stmt<'par>>,
    pub close_brace: SymbolToken,
    pub else_clause: Option<ElseClause<'par>>,
}

#[derive(Debug)]
pub enum ElseClause<'par> {
    Else {
        else_token: KeywordToken,
        open_brace: SymbolToken,
        else_stmt: ArenaVec<'par, Stmt<'par>>,
        close_brace: SymbolToken,
    },
    ElseIf {
        else_token: KeywordToken,
        else_if_stmt: ArenaBox<'par, IfStmt<'par>>,
    },
}

pub type BoxExpr<'par> = ArenaBox<'par, Expr<'par>>;
pub type BoxStmt<'par> = ArenaBox<'par, Stmt<'par>>;

#[derive(Debug)]
pub struct InfixExpr<'par> {
    pub left: BoxExpr<'par>,
    pub symbol: SymbolToken,
    pub right: BoxExpr<'par>,
}

#[derive(Debug)]
pub struct CallExpr<'par> {
    pub callee: BoxExpr<'par>,
    pub lparen: SymbolToken,
    pub args: ExprList<'par>,
    pub rparen: SymbolToken,
}

#[derive(Debug)]
pub struct ParensExpr<'par> {
    pub lparen: SymbolToken,
    pub expr: BoxExpr<'par>,
    pub rparen: SymbolToken,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ExprList<'par> {
    pub exprs: ArenaVec<'par, Expr<'par>>,
    pub commas: ArenaVec<'par, SymbolToken>,
}
