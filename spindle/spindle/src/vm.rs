use crate::compiler::ast::{ElseClause, Expr};
use alloc::boxed::Box;
use alloc::vec::Vec;
use arena::{ArenaBox, ArenaVec};

#[derive(Debug, Eq, PartialEq)]
pub struct VmLetStmt<'vm> {
    pub expr: BoxVmExpr<'vm>,
    pub next: BoxVmStmt<'vm>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmExprStmt<'vm> {
    pub expr: BoxVmExpr<'vm>,
    pub next: BoxVmStmt<'vm>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmForStmt<'vm> {
    pub init: BoxVmExpr<'vm>,
    pub limit: BoxVmExpr<'vm>,
    pub inner: BoxVmStmt<'vm>,
    pub next: BoxVmStmt<'vm>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmIfStmt<'vm> {
    pub cond: BoxVmExpr<'vm>,
    pub then_branch: BoxVmStmt<'vm>,
    pub else_branch: BoxVmStmt<'vm>,
    pub next: BoxVmStmt<'vm>,
}

pub type BoxVmStmt<'vm> = ArenaBox<'vm, VmStmt<'vm>>;
#[derive(Debug, Eq, PartialEq)]
pub enum VmStmt<'vm> {
    LetStmt(VmLetStmt<'vm>),
    ExprStmt(VmExprStmt<'vm>),
    Noop,
    ForStmt(VmForStmt<'vm>),
    IfStmt(VmIfStmt<'vm>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum VmFunctionName {
    Native(usize),
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmCallExpr<'vm> {
    pub function: VmFunctionName,
    pub args: ArenaVec<'vm, BoxVmExpr<'vm>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VmOperator {
    Plus,
    Times,
    Minus,
    Divide,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmOperatorExpr<'vm> {
    pub operator: VmOperator,
    pub left: ArenaBox<'vm, VmExpr<'vm>>,
    pub right: ArenaBox<'vm, VmExpr<'vm>>,
}

pub type BoxVmExpr<'vm> = ArenaBox<'vm, VmExpr<'vm>>;
#[derive(Debug, Eq, PartialEq)]
pub enum VmExpr<'vm> {
    Call(VmCallExpr<'vm>),
    Operator(VmOperatorExpr<'vm>),
    Var(usize),
    Number(i64),
    Null,
    Boolean(bool),
    String(&'vm str),
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmFunction<'vm> {
    pub stmt: BoxVmStmt<'vm>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmProgram<'vm> {
    pub functions: ArenaVec<'vm, VmFunction<'vm>>,
}
