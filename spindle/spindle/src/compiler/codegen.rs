use crate::compiler::ast::{
    CallExpr, ElseClause, Expr, ForStmt, IfStmt, InfixExpr, LetStmt, LoopStmt, Program,
    ReassignStmt, Stmt, WhileStmt,
};
use crate::compiler::stack::Stack;
use crate::compiler::stack_executor::StackSpawn;
use crate::compiler::token::{IdentToken, Symbol};
use crate::native::NativeFn;
use crate::vec_ext::VecExt;
use crate::vm::{VmBlock, VmFunction, VmFunctionName, VmInstr, VmOperator, VmProgram, VmTerm};
use alloc::collections::TryReserveError;
use alloc::vec::Vec;
use arena::{Arena, ArenaVec};
use core::marker::PhantomData;
use core::mem;

pub struct Codegen<'src, 'par, 'vm> {
    arena: &'vm Arena,
    program: &'par Program<'src, 'par>,
    natives: &'vm [&'vm dyn NativeFn],
}

#[derive(Copy, Clone)]
struct LoopContext {
    break_block: usize,
    continue_block: usize,
    variables: usize,
}

pub struct FunctionCodegen<'src, 'par, 'vm> {
    arena: &'vm Arena,
    natives: &'vm [&'vm dyn NativeFn],
    variables: ArenaVec<'vm, Option<&'src str>>,
    blocks: ArenaVec<'vm, VmBlock<'vm>>,
    break_points: ArenaVec<'vm, LoopContext>,
    phantom: PhantomData<&'par ()>,
}

#[derive(Debug)]
pub enum CompileError<'src> {
    AllocError,
    UnknownVariable(IdentToken<'src>),
    VariableIndexOverflow,
    BadNumberLiteral,
    UnexpectedInfixSymbol,
    Unimplemented,
    UnknownFunction,
    NotInLoop,
}

impl<'src> From<TryReserveError> for CompileError<'src> {
    fn from(_: TryReserveError) -> Self {
        CompileError::AllocError
    }
}

impl<'src> From<core::alloc::AllocError> for CompileError<'src> {
    fn from(_: core::alloc::AllocError) -> Self {
        CompileError::AllocError
    }
}

impl<'src, 'par, 'vm> Codegen<'src, 'par, 'vm> {
    pub fn new(
        arena: &'vm Arena,
        natives: &'vm [&'vm dyn NativeFn],
        program: &'par Program<'src, 'par>,
    ) -> Self {
        Codegen {
            arena,
            program,
            natives,
        }
    }
    pub async fn compile(
        &mut self,
        stack: StackSpawn<'_>,
    ) -> Result<VmProgram<'vm>, CompileError<'src>> {
        let mut functions = ArenaVec::try_with_capacity_in(1, self.arena)?;
        functions.try_push(self.compile_function(stack, &self.program.stmts).await?)?;
        Ok(VmProgram { functions })
    }
    async fn compile_function(
        &mut self,
        stack: StackSpawn<'_>,
        stmts: &'par [Stmt<'src, 'par>],
    ) -> Result<VmFunction<'vm>, CompileError<'src>> {
        Ok(VmFunction {
            blocks: FunctionCodegen {
                arena: self.arena,
                natives: self.natives,
                variables: Vec::new_in(self.arena),
                blocks: Vec::new_in(self.arena),
                break_points: Vec::new_in(self.arena),
                phantom: PhantomData,
            }
            .compile_function(stack, stmts)
            .await?,
        })
    }
}

impl<'src: 'par, 'par, 'vm> FunctionCodegen<'src, 'par, 'vm> {
    async fn compile_function(
        mut self,
        stack: StackSpawn<'_>,
        stmt: &'par [Stmt<'src, 'par>],
    ) -> Result<ArenaVec<'vm, VmBlock<'vm>>, CompileError<'src>> {
        let mut block = self.add_block()?;
        let block = self.compile_stmts(stack, block, stmt).await?;
        self.terminate(block, VmTerm::Return)?;
        assert!(self.break_points.is_empty());
        assert!(self.variables.is_empty());
        Ok(self.blocks)
    }
    fn add_block(&mut self) -> Result<usize, CompileError<'src>> {
        let index = self.blocks.len();
        self.blocks.try_push(VmBlock {
            instrs: Vec::new_in(self.arena),
            term: VmTerm::Uninit,
        })?;
        Ok(index)
    }
    async fn compile_stmts(
        &mut self,
        mut stack: StackSpawn<'_>,
        mut block: usize,
        stmt: &'par [Stmt<'src, 'par>],
    ) -> Result<usize, CompileError<'src>> {
        Ok(stack
            .recurse(async |mut stack| -> Result<usize, CompileError<'src>> {
                let orig_vars = self.variables.len();
                for stmt in stmt {
                    block = self.compile_stmt(stack.reborrow(), block, stmt).await?;
                }
                block = self.pop_until(block, orig_vars)?;
                Ok(block)
            })
            .await??)
    }
    async fn compile_stmt(
        &mut self,
        stack: StackSpawn<'_>,
        block: usize,
        stmt: &'par Stmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        match stmt {
            Stmt::Let(stmt) => self.compile_let_stmt(block, stmt),
            Stmt::ExprStmt(expr) => self.compile_expr_stmt(block, expr),
            Stmt::For(stmt) => self.compile_for_stmt(stack, block, stmt).await,
            Stmt::If(stmt) => self.compile_if_stmt(stack, block, stmt).await,
            Stmt::Loop(stmt) => self.compile_loop_stmt(stack, block, stmt).await,
            Stmt::While(stmt) => self.compile_while_stmt(stack, block, stmt).await,
            Stmt::Break => self.compile_break_stmt(block),
            Stmt::Continue => self.compile_continue_stmt(block),
            Stmt::Reassign(stmt) => self.compile_reassign_stmt(block, stmt),
        }
    }

    fn compile_let_stmt(
        &mut self,
        mut block: usize,
        stmt: &'par LetStmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        block = self.compile_expr(block, &stmt.expr)?;
        self.variables.try_push(Some(stmt.ident.ident))?;
        Ok(block)
    }
    fn compile_expr_stmt(
        &mut self,
        mut block: usize,
        expr: &'par Expr<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        block = self.compile_expr(block, expr)?;
        self.push_instr(block, VmInstr::Pop)?;
        Ok(block)
    }
    async fn compile_for_stmt(
        &mut self,
        mut stack: StackSpawn<'_>,
        mut start: usize,
        stmt: &'par ForStmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        let counter = self.variables.len();
        let limit = counter + 1;
        start = self.compile_expr(start, &stmt.init_expr)?;
        start = self.compile_expr(start, &stmt.limit_expr)?;
        self.variables.try_push(Some(stmt.ident.ident))?;
        self.variables.try_push(None)?;

        let cond = self.add_block()?;
        let mut inner = self.add_block()?;
        let join = self.add_block()?;
        self.break_points.try_push(LoopContext {
            break_block: join,
            continue_block: cond,
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
        inner = self
            .compile_stmts(stack.reborrow(), inner, &stmt.inner)
            .await?;
        self.push_instr(inner, VmInstr::Load(counter))?;
        self.push_instr(inner, VmInstr::Integer(1))?;
        self.push_instr(inner, VmInstr::Binop(VmOperator::Plus))?;
        self.push_instr(inner, VmInstr::Store(counter))?;
        self.terminate(inner, VmTerm::Jump(cond))?;

        self.push_instr(join, VmInstr::Pop)?;
        self.push_instr(join, VmInstr::Pop)?;
        self.variables.pop();
        self.variables.pop();
        self.break_points.pop();
        Ok(join)
    }
    async fn compile_if_stmt(
        &mut self,
        mut stack: StackSpawn<'_>,
        init: usize,
        stmt: &'par IfStmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        Ok(stack
            .recurse(async |mut stack| -> Result<usize, CompileError<'src>> {
                let init = self.compile_expr(init, &stmt.cond_expr)?;
                let mut yes = self.add_block()?;
                let mut no = self.add_block()?;
                self.terminate(init, VmTerm::CondJump { yes, no })?;
                yes = self
                    .compile_stmts(stack.reborrow(), yes, &stmt.then_stmt)
                    .await?;
                if let Some(else_clause) = &stmt.else_clause {
                    match else_clause {
                        ElseClause::Else { else_stmt, .. } => {
                            no = self.compile_stmts(stack.reborrow(), no, else_stmt).await?;
                        }
                        ElseClause::ElseIf { else_if_stmt, .. } => {
                            no = self
                                .compile_if_stmt(stack.reborrow(), no, else_if_stmt)
                                .await?;
                        }
                    }
                }
                let join = self.add_block()?;
                self.terminate(yes, VmTerm::Jump(join))?;
                self.terminate(no, VmTerm::Jump(join))?;
                Ok(join)
            })
            .await??)
    }

    async fn compile_loop_stmt(
        &mut self,
        stack: StackSpawn<'_>,
        mut init: usize,
        stmt: &'par LoopStmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        let enter = self.add_block()?;
        let join = self.add_block()?;
        self.break_points.push(LoopContext {
            break_block: join,
            continue_block: enter,
            variables: self.variables.len(),
        });
        self.terminate(init, VmTerm::Jump(enter))?;
        let exit = self.compile_stmts(stack, enter, &stmt.inner).await?;
        self.terminate(exit, VmTerm::Jump(enter))?;
        self.break_points.pop();
        Ok(join)
    }
    async fn compile_while_stmt(
        &mut self,
        stack: StackSpawn<'_>,
        init: usize,
        stmt: &'par WhileStmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        let cond_enter = self.add_block()?;
        let inner_enter = self.add_block()?;
        let join = self.add_block()?;
        self.terminate(init, VmTerm::Jump(cond_enter))?;
        let cond_exit = self.compile_expr(cond_enter, &stmt.cond)?;
        self.terminate(
            cond_exit,
            VmTerm::CondJump {
                yes: inner_enter,
                no: join,
            },
        )?;
        let inner_exit = self.compile_stmts(stack, inner_enter, &stmt.inner).await?;
        self.terminate(inner_exit, VmTerm::Jump(cond_enter))?;
        Ok(join)
    }
    fn compile_break_stmt(&mut self, mut block: usize) -> Result<usize, CompileError<'src>> {
        let point = *self.break_points.last().ok_or(CompileError::NotInLoop)?;
        self.pop_until(block, point.variables)?;
        self.terminate(block, VmTerm::Jump(point.break_block))?;
        block = self.add_block()?;
        Ok(block)
    }
    fn compile_continue_stmt(&mut self, mut block: usize) -> Result<usize, CompileError<'src>> {
        let point = *self.break_points.last().ok_or(CompileError::NotInLoop)?;
        self.pop_until(block, point.variables)?;
        self.terminate(block, VmTerm::Jump(point.continue_block))?;
        block = self.add_block()?;
        Ok(block)
    }
    fn compile_reassign_stmt(
        &mut self,
        mut block: usize,
        stmt: &'par ReassignStmt<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        let var = self.find_var(&stmt.ident)?;
        block = self.compile_expr(block, &stmt.expr)?;
        self.push_instr(block, VmInstr::Store(var))?;
        Ok(block)
    }
    fn compile_expr(
        &mut self,
        block: usize,
        expr: &'par Expr<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        match expr {
            Expr::Var(x) => Ok(self.compile_var(block, x)?),
            Expr::Parens(_) => todo!(),
            Expr::Number(n) => self.compile_number_literal(block, n.number),
            Expr::False(x) => self.compile_bool_literal(block, false),
            Expr::True(x) => self.compile_bool_literal(block, true),
            Expr::Null(_) => todo!(),
            Expr::InfixExpr(expr) => self.compile_infix_expr(block, expr),
            Expr::Call(expr) => self.compile_call_expr(block, expr),
            Expr::String(x) => self.compile_string_literal(block, x),
        }
    }
    fn compile_var(
        &mut self,
        block: usize,
        var: &'par IdentToken<'src>,
    ) -> Result<usize, CompileError<'src>> {
        let var = self.find_var(var)?;
        self.push_instr(block, VmInstr::Load(var))?;
        Ok(block)
    }
    fn find_var(&self, var: &'par IdentToken<'src>) -> Result<usize, CompileError<'src>> {
        self.variables
            .iter()
            .position(|x| *x == Some(var.ident))
            .ok_or(CompileError::UnknownVariable(*var))
    }
    fn compile_number_literal(
        &mut self,
        block: usize,
        number: &'par str,
    ) -> Result<usize, CompileError<'src>> {
        self.push_instr(
            block,
            VmInstr::Integer(number.parse().ok().ok_or(CompileError::BadNumberLiteral)?),
        )?;
        Ok(block)
    }
    fn compile_bool_literal(
        &mut self,
        block: usize,
        value: bool,
    ) -> Result<usize, CompileError<'src>> {
        self.push_instr(block, VmInstr::Bool(value))?;
        Ok(block)
    }
    fn compile_string_literal(
        &mut self,
        block: usize,
        value: &'par str,
    ) -> Result<usize, CompileError<'src>> {
        self.push_instr(block, VmInstr::String(self.arena.alloc_str(value)?))?;
        Ok(block)
    }
    fn compile_infix_expr(
        &mut self,
        mut block: usize,
        expr: &'par InfixExpr<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        block = self.compile_expr(block, &expr.left)?;
        block = self.compile_expr(block, &expr.right)?;
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
        mut block: usize,
        expr: &'par CallExpr<'src, 'par>,
    ) -> Result<usize, CompileError<'src>> {
        for arg in expr.args.exprs {
            block = self.compile_expr(block, arg)?;
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
    fn push_instr(&mut self, block: usize, instr: VmInstr<'vm>) -> Result<(), CompileError<'src>> {
        Ok(self.blocks[block].instrs.try_push(instr)?)
    }
    fn terminate(&mut self, block: usize, term: VmTerm<'vm>) -> Result<(), CompileError<'src>> {
        match mem::replace(&mut self.blocks[block].term, term) {
            VmTerm::Uninit => {}
            _ => panic!("double termination of block"),
        }
        Ok(())
    }
    fn pop_until(&mut self, block: usize, count: usize) -> Result<usize, CompileError<'src>> {
        while self.variables.len() > count {
            self.push_instr(block, VmInstr::Pop)?;
            self.variables.pop();
        }
        Ok(block)
    }
}
