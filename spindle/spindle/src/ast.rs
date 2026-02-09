use crate::token::{IdentToken, KeywordToken, NumberToken, SymbolToken};
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
    pub inner: ArenaVec<'par, Stmt<'par>>,
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
    InfixExpr(InfixExpr<'par>),
    Call(CallExpr<'par>),
}

#[derive(Debug)]
pub struct ExprList<'par> {
    pub exprs: ArenaVec<'par, Expr<'par>>,
    pub commas: ArenaVec<'par, SymbolToken>,
}
