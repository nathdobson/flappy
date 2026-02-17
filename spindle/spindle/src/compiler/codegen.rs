use crate::compiler::ast::{
    CallExpr, ElseClause, Expr, ForStmt, IfStmt, InfixExpr, LetStmt, LoopStmt, Program,
    ReassignStmt, Stmt, WhileStmt,
};
use crate::compiler::token::{IdentToken, Symbol};
use crate::native::NativeFn;
use crate::vec_ext::VecExt;
use crate::vm::{VmBlock, VmFunction, VmFunctionName, VmInstr, VmOperator, VmProgram, VmTerm};
use alloc::collections::TryReserveError;
use alloc::vec::Vec;
use arena::{Arena, ArenaVec};
use core::marker::PhantomData;
use core::mem;

pub struct Codegen<'par, 'vm> {
    arena: &'vm Arena,
    program: &'par Program<'par>,
    natives: &'vm [&'vm dyn NativeFn],
}

#[derive(Copy, Clone)]
struct BreakPoint {
    block: usize,
    variables: usize,
}

pub struct FunctionCodegen<'par, 'vm> {
    arena: &'vm Arena,
    natives: &'vm [&'vm dyn NativeFn],
    variables: ArenaVec<'vm, Option<&'par str>>,
    blocks: ArenaVec<'vm, VmBlock<'vm>>,
    break_points: ArenaVec<'vm, BreakPoint>,
}

#[derive(Debug)]
pub enum CompileError<'par, 'vm> {
    Unused(!, PhantomData<(&'par (), &'vm ())>),
    AllocError,
    UnknownVariable(&'par IdentToken<'par>),
    VariableIndexOverflow,
    BadNumberLiteral,
    UnexpectedInfixSymbol,
    Unimplemented,
    UnknownFunction,
    NotInLoop,
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

impl<'par, 'vm> Codegen<'par, 'vm> {
    pub fn new(
        arena: &'vm Arena,
        natives: &'vm [&'vm dyn NativeFn],
        program: &'par Program<'par>,
    ) -> Self {
        Codegen {
            arena,
            program,
            natives,
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
            blocks: FunctionCodegen {
                arena: self.arena,
                natives: self.natives,
                variables: Vec::new_in(self.arena),
                blocks: Vec::new_in(self.arena),
                break_points: Vec::new_in(self.arena),
            }
            .compile_function(stmts)?,
        })
    }
}

impl<'par, 'vm> FunctionCodegen<'par, 'vm> {
    fn compile_function(
        mut self,
        stmt: &'par [Stmt<'par>],
    ) -> Result<ArenaVec<'vm, VmBlock<'vm>>, CompileError<'par, 'vm>> {
        let mut block = self.add_block()?;
        let block = self.compile_stmts(stmt, block)?;
        self.terminate(block, VmTerm::Return)?;
        Ok(self.blocks)
    }
    fn add_block(&mut self) -> Result<usize, CompileError<'par, 'vm>> {
        let index = self.blocks.len();
        self.blocks.try_push(VmBlock {
            instrs: Vec::new_in(self.arena),
            term: VmTerm::Uninit,
        })?;
        Ok(index)
    }
    fn compile_stmts(
        &mut self,
        stmt: &'par [Stmt<'par>],
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let orig_vars = self.variables.len();
        for stmt in stmt {
            block = self.compile_stmt(stmt, block)?;
        }
        block = self.pop_until(orig_vars, block)?;
        Ok(block)
    }
    fn compile_stmt(
        &mut self,
        stmt: &'par Stmt<'par>,
        block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        match stmt {
            Stmt::Let(stmt) => self.compile_let_stmt(stmt, block),
            Stmt::ExprStmt(expr) => self.compile_expr_stmt(expr, block),
            Stmt::For(stmt) => self.compile_for_stmt(stmt, block),
            Stmt::If(stmt) => self.compile_if_stmt(stmt, block),
            Stmt::Loop(stmt) => self.compile_loop_stmt(stmt, block),
            Stmt::While(stmt) => self.compile_while_stmt(stmt, block),
            Stmt::Break => self.compile_break_stmt(block),
            Stmt::Reassign(stmt) => self.compile_reassign_stmt(stmt, block),
        }
    }

    fn compile_let_stmt(
        &mut self,
        stmt: &'par LetStmt,
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        block = self.compile_expr(&stmt.expr, block)?;
        self.variables.try_push(Some(stmt.ident.ident))?;
        Ok(block)
    }
    fn compile_expr_stmt(
        &mut self,
        expr: &'par Expr,
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        block = self.compile_expr(expr, block)?;
        self.push_instr(block, VmInstr::Pop)?;
        Ok(block)
    }
    fn compile_for_stmt(
        &mut self,
        stmt: &'par ForStmt<'par>,
        mut start: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let counter = self.variables.len();
        let limit = counter + 1;
        start = self.compile_expr(&stmt.init_expr, start)?;
        start = self.compile_expr(&stmt.limit_expr, start)?;
        self.variables.try_push(Some(stmt.ident.ident))?;
        self.variables.try_push(None)?;

        let cond = self.add_block()?;
        let mut inner = self.add_block()?;
        let join = self.add_block()?;
        self.break_points.try_push(BreakPoint {
            block: join,
            variables: counter,
        })?;
        self.push_instr(cond, VmInstr::Load(counter))?;
        self.push_instr(cond, VmInstr::Load(limit))?;
        self.push_instr(cond, VmInstr::Binop(VmOperator::Less))?;
        self.terminate(start, VmTerm::Jump(cond))?;
        self.terminate(
            cond,
            VmTerm::CondJump {
                yes: inner,
                no: join,
            },
        )?;
        inner = self.compile_stmts(&stmt.inner, inner)?;
        self.push_instr(inner, VmInstr::Load(counter))?;
        self.push_instr(inner, VmInstr::Integer(1))?;
        self.push_instr(inner, VmInstr::Binop(VmOperator::Plus))?;
        self.push_instr(inner, VmInstr::Store(counter))?;
        self.terminate(inner, VmTerm::Jump(cond))?;

        self.variables.pop();
        self.variables.pop();
        self.break_points.pop();
        Ok(join)
    }
    fn compile_if_stmt(
        &mut self,
        stmt: &'par IfStmt<'par>,
        init: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let init = self.compile_expr(&stmt.cond_expr, init)?;
        let mut yes = self.add_block()?;
        let mut no = self.add_block()?;
        self.terminate(init, VmTerm::CondJump { yes, no })?;
        yes = self.compile_stmts(&stmt.then_stmt, yes)?;
        if let Some(else_clause) = &stmt.else_clause {
            match else_clause {
                ElseClause::Else { else_stmt, .. } => {
                    no = self.compile_stmts(else_stmt, no)?;
                }
                ElseClause::ElseIf { else_if_stmt, .. } => {
                    no = self.compile_if_stmt(else_if_stmt, no)?;
                }
            }
        }
        let join = self.add_block()?;
        self.terminate(yes, VmTerm::Jump(join))?;
        self.terminate(no, VmTerm::Jump(join))?;
        Ok(join)
    }

    fn compile_loop_stmt(
        &mut self,
        stmt: &'par LoopStmt<'par>,
        mut init: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let enter = self.add_block()?;
        let join = self.add_block()?;
        self.break_points.push(BreakPoint {
            block: join,
            variables: self.variables.len(),
        });
        self.terminate(init, VmTerm::Jump(enter))?;
        let exit = self.compile_stmts(&stmt.inner, enter)?;
        self.terminate(exit, VmTerm::Jump(enter))?;
        self.break_points.pop();
        Ok(join)
    }
    fn compile_while_stmt(
        &mut self,
        stmt: &'par WhileStmt<'par>,
        init: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let cond_enter = self.add_block()?;
        let inner_enter = self.add_block()?;
        let join = self.add_block()?;
        self.terminate(init, VmTerm::Jump(cond_enter))?;
        let cond_exit = self.compile_expr(&stmt.cond, cond_enter)?;
        self.terminate(
            cond_exit,
            VmTerm::CondJump {
                yes: inner_enter,
                no: join,
            },
        )?;
        let inner_exit = self.compile_stmts(&stmt.inner, inner_enter)?;
        self.terminate(inner_exit, VmTerm::Jump(cond_enter))?;
        Ok(join)
    }
    fn compile_break_stmt(&mut self, mut block: usize) -> Result<usize, CompileError<'par, 'vm>> {
        let point = *self.break_points.last().ok_or(CompileError::NotInLoop)?;
        self.pop_until(point.variables, block)?;
        self.terminate(block, VmTerm::Jump(point.block))?;
        block = self.add_block()?;
        Ok(block)
    }
    fn compile_reassign_stmt(
        &mut self,
        stmt: &'par ReassignStmt<'par>,
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let var = self.find_var(&stmt.ident)?;
        block = self.compile_expr(&stmt.expr, block)?;
        self.push_instr(block, VmInstr::Store(var))?;
        Ok(block)
    }
    fn compile_expr(
        &mut self,
        expr: &'par Expr<'par>,
        block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        match expr {
            Expr::Var(x) => Ok(self.compile_var(x, block)?),
            Expr::Parens(_) => todo!(),
            Expr::Number(n) => self.compile_number_literal(n.number, block),
            Expr::False(x) => self.compile_bool_literal(false, block),
            Expr::True(x) => self.compile_bool_literal(true, block),
            Expr::Null(_) => todo!(),
            Expr::InfixExpr(expr) => self.compile_infix_expr(expr, block),
            Expr::Call(expr) => self.compile_call_expr(expr, block),
            Expr::String(x) => self.compile_string_literal(x, block),
        }
    }
    fn compile_var(
        &mut self,
        var: &'par IdentToken,
        block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        let var = self.find_var(var)?;
        self.push_instr(block, VmInstr::Load(var))?;
        Ok(block)
    }
    fn find_var(&self, var: &'par IdentToken) -> Result<usize, CompileError<'par, 'vm>> {
        self.variables
            .iter()
            .position(|x| *x == Some(var.ident))
            .ok_or(CompileError::UnknownVariable(var))
    }
    fn compile_number_literal(
        &mut self,
        number: &'par str,
        block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        self.push_instr(
            block,
            VmInstr::Integer(number.parse().ok().ok_or(CompileError::BadNumberLiteral)?),
        )?;
        Ok(block)
    }
    fn compile_bool_literal(
        &mut self,
        value: bool,
        block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        self.push_instr(block, VmInstr::Bool(value))?;
        Ok(block)
    }
    fn compile_string_literal(
        &mut self,
        value: &'par str,
        block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        self.push_instr(block, VmInstr::String(self.arena.alloc_str(value)?))?;
        Ok(block)
    }
    fn compile_infix_expr(
        &mut self,
        expr: &'par InfixExpr<'par>,
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        block = self.compile_expr(&expr.left, block)?;
        block = self.compile_expr(&expr.right, block)?;
        self.push_instr(
            block,
            VmInstr::Binop(match expr.symbol.symbol {
                Symbol::Plus => VmOperator::Plus,
                Symbol::Minus => VmOperator::Minus,
                Symbol::Times => VmOperator::Times,
                Symbol::Divide => VmOperator::Divide,
                Symbol::Less => VmOperator::Less,
                Symbol::LessEquals => VmOperator::LessEquals,
                Symbol::Greater => VmOperator::Greater,
                Symbol::GreaterEquals => VmOperator::GreaterEquals,
                Symbol::EqualsEquals => VmOperator::EqualsEquals,
                _ => todo!("{:?}", expr.symbol),
            }),
        )?;
        Ok(block)
    }
    fn compile_call_expr(
        &mut self,
        expr: &'par CallExpr<'par>,
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        for arg in expr.args.exprs {
            block = self.compile_expr(arg, block)?;
        }
        let name = match &*expr.callee {
            Expr::Var(x) => VmFunctionName::Native(
                self.natives
                    .iter()
                    .position(|f| f.name() == x.ident)
                    .ok_or(CompileError::UnknownFunction)?,
            ),
            _ => return Err(CompileError::UnknownFunction),
        };
        self.push_instr(block, VmInstr::Call(name, expr.args.exprs.len()))?;
        Ok(block)
    }
    fn push_instr(
        &mut self,
        block: usize,
        instr: VmInstr<'vm>,
    ) -> Result<(), CompileError<'par, 'vm>> {
        Ok(self.blocks[block].instrs.try_push(instr)?)
    }
    fn terminate(
        &mut self,
        block: usize,
        term: VmTerm<'vm>,
    ) -> Result<(), CompileError<'par, 'vm>> {
        match mem::replace(&mut self.blocks[block].term, term) {
            VmTerm::Uninit => {}
            _ => panic!("double termination of block"),
        }
        Ok(())
    }
    fn pop_until(&mut self, count: usize, block: usize) -> Result<usize, CompileError<'par, 'vm>> {
        while self.variables.len() > count {
            self.push_instr(block, VmInstr::Pop)?;
            self.variables.pop();
        }
        Ok(block)
    }
}
