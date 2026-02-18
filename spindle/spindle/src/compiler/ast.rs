use crate::compiler::token::{IdentToken, Keyword, KeywordToken, NumberToken, SymbolToken, Token};

#[derive(Debug, Copy, Clone)]
pub struct Program<'src, 'par> {
    pub stmts: &'par [Stmt<'src, 'par>],
}

#[derive(Debug, Copy, Clone)]
pub enum Stmt<'src, 'par> {
    Let(LetStmt<'src, 'par>),
    ExprStmt(Expr<'src, 'par>),
    For(ForStmt<'src, 'par>),
    If(IfStmt<'src, 'par>),
    Loop(LoopStmt<'src, 'par>),
    While(WhileStmt<'src, 'par>),
    Reassign(ReassignStmt<'src, 'par>),
    Break,
    Continue,
}

#[derive(Debug, Copy, Clone)]
pub struct LetStmt<'src, 'par> {
    pub let_token: KeywordToken,
    pub ident: IdentToken<'src>,
    pub equals: SymbolToken,
    pub expr: Expr<'src, 'par>,
}

#[derive(Debug, Copy, Clone)]
pub struct ForStmt<'src, 'par> {
    pub for_token: KeywordToken,
    pub ident: IdentToken<'src>,
    pub init_expr: Expr<'src, 'par>,
    pub limit_expr: Expr<'src, 'par>,
    pub open_brace: SymbolToken,
    pub inner: &'par [Stmt<'src, 'par>],
    pub close_brace: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub struct IfStmt<'src, 'par> {
    pub if_token: KeywordToken,
    pub cond_expr: Expr<'src, 'par>,
    pub open_brace: SymbolToken,
    pub then_stmt: &'par [Stmt<'src, 'par>],
    pub close_brace: SymbolToken,
    pub else_clause: Option<ElseClause<'src, 'par>>,
}

#[derive(Debug, Copy, Clone)]
pub struct LoopStmt<'src, 'par> {
    pub loop_token: KeywordToken,
    pub open_brace: SymbolToken,
    pub inner: &'par [Stmt<'src, 'par>],
    pub close_brace: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub struct WhileStmt<'src, 'par> {
    pub while_token: KeywordToken,
    pub cond: Expr<'src, 'par>,
    pub open_brace: SymbolToken,
    pub inner: &'par [Stmt<'src, 'par>],
    pub close_brace: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub enum ElseClause<'src, 'par> {
    Else {
        else_token: KeywordToken,
        open_brace: SymbolToken,
        else_stmt: &'par [Stmt<'src, 'par>],
        close_brace: SymbolToken,
    },
    ElseIf {
        else_token: KeywordToken,
        else_if_stmt: &'par IfStmt<'src, 'par>,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct ReassignStmt<'src, 'par> {
    pub ident: IdentToken<'src>,
    pub equals: SymbolToken,
    pub expr: Expr<'src, 'par>,
}

#[derive(Debug, Copy, Clone)]
pub struct InfixExpr<'src, 'par> {
    pub left: &'par Expr<'src, 'par>,
    pub symbol: SymbolToken,
    pub right: &'par Expr<'src, 'par>,
}

#[derive(Debug, Copy, Clone)]
pub struct CallExpr<'src, 'par> {
    pub callee: &'par Expr<'src, 'par>,
    pub lparen: SymbolToken,
    pub args: ExprList<'src, 'par>,
    pub rparen: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub struct ParensExpr<'src, 'par> {
    pub lparen: SymbolToken,
    pub expr: &'par Expr<'src, 'par>,
    pub rparen: SymbolToken,
}

#[derive(Debug, Copy, Clone)]
pub enum Expr<'src, 'par> {
    Var(IdentToken<'src>),
    Parens(ParensExpr<'src, 'par>),
    Number(NumberToken<'src>),
    False(KeywordToken),
    True(KeywordToken),
    Null(KeywordToken),
    InfixExpr(InfixExpr<'src, 'par>),
    Call(CallExpr<'src, 'par>),
    String(&'par str),
}

#[derive(Debug, Copy, Clone)]
pub struct ExprList<'src, 'par> {
    pub exprs: &'par [Expr<'src, 'par>],
    pub commas: &'par [SymbolToken],
}
