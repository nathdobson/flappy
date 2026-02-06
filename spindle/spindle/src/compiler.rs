use crate::ast::{Expr, Program, Stmt};
use crate::token::Symbol;
use crate::vec_ext::VecExt;
use crate::vm::{
    BoxVmExpr, BoxVmStmt, VmCallExpr, VmExpr, VmExprStmt, VmFunction, VmFunctionName, VmLetStmt,
    VmOperator, VmOperatorExpr, VmProgram, VmStmt,
};
use alloc::collections::TryReserveError;
use arena::{Arena, ArenaVec};
use core::marker::PhantomData;

pub struct Compiler<'par, 'vm> {
    arena: &'vm Arena,
    program: &'par Program<'par>,
    variables: ArenaVec<'vm, &'par str>,
}

#[derive(Debug)]
pub enum CompileError<'par, 'vm> {
    Unused(!, PhantomData<(&'par (), &'vm ())>),
    AllocError,
    UnknownVariable,
    VariableIndexOverflow,
    BadNumberLiteral,
    UnexpectedInfixSymbol,
    Unimplemented,
    UnknownFunction,
}

impl<'par, 'vm> From<TryReserveError> for CompileError<'par, 'vm> {
    fn from(_: TryReserveError) -> Self {
        CompileError::AllocError
    }
}

impl<'par, 'vm> From<core::alloc::AllocError> for CompileError<'par, 'vm> {
    fn from(_: core::alloc::AllocError) -> Self {
        CompileError::AllocError
    }
}

impl<'par, 'vm> Compiler<'par, 'vm> {
    pub fn new(arena: &'vm Arena, program: &'par Program<'par>) -> Self {
        Compiler {
            arena,
            program,
            variables: ArenaVec::new_in(arena),
        }
    }
    pub fn compile(&mut self) -> Result<VmProgram<'vm>, CompileError<'par, 'vm>> {
        let mut functions = ArenaVec::try_with_capacity_in(1, self.arena)?;
        functions.try_push(self.compile_function(&self.program.stmts)?)?;
        Ok(VmProgram { functions })
    }
    fn compile_function(
        &mut self,
        stmts: &'par [Stmt<'par>],
    ) -> Result<VmFunction<'vm>, CompileError<'par, 'vm>> {
        Ok(VmFunction {
            stmt: self.compile_stmt(stmts)?,
        })
    }

    fn compile_stmt(
        &mut self,
        stmts: &'par [Stmt<'par>],
    ) -> Result<BoxVmStmt<'vm>, CompileError<'par, 'vm>> {
        match stmts.split_first() {
            None => Ok(self.arena.alloc_box(VmStmt::Noop)?),
            Some((stmt, next)) => match stmt {
                Stmt::Let(stmt) => {
                    self.variables.try_push(stmt.ident.ident)?;
                    let result = self.arena.alloc_box(VmStmt::LetStmt(VmLetStmt {
                        expr: self.compile_expr(&stmt.expr)?,
                        next: self.compile_stmt(next)?,
                    }))?;
                    self.variables.pop();
                    Ok(result)
                }
                Stmt::ExprStmt(expr) => Ok(self.arena.alloc_box(VmStmt::ExprStmt(VmExprStmt {
                    expr: self.compile_expr(expr)?,
                    next: self.compile_stmt(next)?,
                }))?),
            },
        }
    }

    fn compile_expr(&self, expr: &Expr) -> Result<BoxVmExpr<'vm>, CompileError<'par, 'vm>> {
        match expr {
            Expr::Var(v) => {
                let index = self
                    .variables
                    .iter()
                    .position(|x| *x == v.ident)
                    .ok_or_else(|| CompileError::UnknownVariable)?;
                Ok(self.arena.alloc_box(VmExpr::Var(index))?)
            }
            Expr::Parens(x) => self.compile_expr(&x.expr),
            Expr::Number(x) => Ok(self.arena.alloc_box(VmExpr::Number(
                x.number
                    .parse()
                    .map_err(|_| CompileError::BadNumberLiteral)?,
            ))?),
            Expr::InfixExpr(expr) => {
                let operator = match expr.symbol.symbol {
                    Symbol::Plus => VmOperator::Plus,
                    Symbol::Minus => VmOperator::Minus,
                    Symbol::Times => VmOperator::Times,
                    Symbol::Divide => VmOperator::Divide,
                    _ => return Err(CompileError::UnexpectedInfixSymbol),
                };
                Ok(self.arena.alloc_box(VmExpr::Operator(VmOperatorExpr {
                    operator,
                    left: self.compile_expr(&expr.left)?,
                    right: self.compile_expr(&expr.right)?,
                }))?)
            }
            Expr::Call(expr) => {
                let function = match &*expr.callee {
                    Expr::Var(ident) => match ident.ident {
                        "print" => VmFunctionName::Print,
                        _ => return Err(CompileError::UnknownFunction),
                    },
                    _ => return Err(CompileError::Unimplemented),
                };
                let mut args = ArenaVec::try_with_capacity_in(expr.args.exprs.len(), &self.arena)?;
                for arg in &expr.args.exprs {
                    args.try_push(self.compile_expr(arg)?)?;
                }
                Ok(self
                    .arena
                    .alloc_box(VmExpr::Call(VmCallExpr { function, args }))?)
            }
        }
    }
}
