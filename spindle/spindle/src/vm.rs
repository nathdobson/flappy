use crate::compiler::ast::{ElseClause, Expr, Stmt};
use crate::interp::value::Value;
use alloc::boxed::Box;
use alloc::vec::Vec;
use arena::{ArenaBox, ArenaVec};

#[derive(Debug, Eq, PartialEq)]
pub enum VmFunctionName {
    Native(usize),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VmOperator {
    Plus,
    Times,
    Minus,
    Divide,

    Less,
    LessEquals,
    Greater,
    GreaterEquals,
    EqualsEquals,
    Remainder,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VmUnaryOperator {
    Not,
    Negate,
}

#[derive(Debug, Eq, PartialEq)]
pub enum VmInstr<'vm> {
    Unused(&'vm !),

    Integer(i64),
    Bool(bool),

    Unop(VmUnaryOperator),
    Binop(VmOperator),
    Call(VmFunctionName, usize),

    Pop,
    Load(usize),
    Store(usize),
    Dup,
    String(&'vm str),
    Null,
}

#[derive(Debug, Eq, PartialEq)]
pub enum VmTerm<'vm> {
    Unused(&'vm !),
    Jump(usize),
    Uninit,
    Return,
    CondJump { yes: usize, no: usize },
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmBlock<'vm> {
    pub instrs: ArenaVec<'vm, VmInstr<'vm>>,
    pub term: VmTerm<'vm>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmFunction<'vm> {
    pub blocks: ArenaVec<'vm, VmBlock<'vm>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VmProgram<'vm> {
    pub functions: ArenaVec<'vm, VmFunction<'vm>>,
}
