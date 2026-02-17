use crate::compiler::ast::{
    CallExpr, ElseClause, Expr, ForStmt, IfStmt, InfixExpr, LetStmt, LoopStmt, Program, Stmt,
    WhileStmt,
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

pub struct FunctionCodegen<'par, 'vm> {
    arena: &'vm Arena,
    natives: &'vm [&'vm dyn NativeFn],
    variables: ArenaVec<'vm, Option<&'par str>>,
    blocks: ArenaVec<'vm, VmBlock<'vm>>,
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
            }
            .compile_function(stmts)?,
        })
    }

    fn compile_stmt(
        &mut self,
        stmts: &'par [Stmt<'par>],
        blocks: &'vm mut ArenaVec<'vm, VmBlock<'vm>>,
    ) -> Result<(), CompileError<'par, 'vm>> {
        match stmts.split_first() {
            None => {
                todo!();
                // Ok(self.arena.alloc_box(VmStmt::Noop)?)
            }
            Some((stmt, next)) => match stmt {
                Stmt::Let(stmt) => {
                    todo!();
                    // let expr = self.compile_expr(&stmt.expr)?;
                    // self.variables.try_push(Some(stmt.ident.ident))?;
                    // let result = self.arena.alloc_box(VmStmt::LetStmt(VmLetStmt {
                    //     expr,
                    //     next: self.compile_stmt(next)?,
                    // }))?;
                    // self.variables.pop();
                    // Ok(result)
                }
                Stmt::ExprStmt(expr) => {
                    todo!();
                    //     Ok(self.arena.alloc_box(VmStmt::ExprStmt(VmExprStmt {
                    //     expr: self.compile_expr(expr)?,
                    //     next: self.compile_stmt(next)?,
                    // }))?),
                }
                Stmt::For(stmt) => {
                    todo!();
                    // let init = self.compile_expr(&stmt.init_expr)?;
                    // let limit = self.compile_expr(&stmt.limit_expr)?;
                    // self.variables.try_push(Some(stmt.ident.ident))?;
                    // let inner = self.compile_stmt(&stmt.inner)?;
                    // self.variables.pop();
                    // let next = self.compile_stmt(next)?;
                    // let result = self.arena.alloc_box(VmStmt::ForStmt(VmForStmt {
                    //     init,
                    //     limit,
                    //     inner,
                    //     next,
                    // }))?;
                    // Ok(result)
                }
                Stmt::If(stmt) => {
                    todo!();
                    // self.compile_if_stmt(stmt, next)
                }
                Stmt::Loop(stmt) => {
                    todo!();
                    // self.compile_loop_stmt(stmt, next)
                }
                Stmt::While(stmt) => {
                    todo!();
                    // self.compile_while_stmt(stmt, next)
                }
            },
        }
    }
    // fn compile_if_stmt(
    //     &mut self,
    //     stmt: &'par IfStmt<'par>,
    //     next: &'par [Stmt<'par>],
    // ) -> Result<BoxVmStmt<'vm>, CompileError<'par, 'vm>> {
    //     let cond = self.compile_expr(&stmt.cond_expr)?;
    //     let then = self.compile_stmt(&stmt.then_stmt)?;
    //     let else_clause = match &stmt.else_clause {
    //         None => self.arena.alloc_box(VmStmt::Noop)?,
    //         Some(else_clause) => match else_clause {
    //             ElseClause::Else { else_stmt, .. } => self.compile_stmt(else_stmt)?,
    //             ElseClause::ElseIf { else_if_stmt, .. } => {
    //                 self.compile_if_stmt(else_if_stmt, &[])?
    //             }
    //         },
    //     };
    //     let next = self.compile_stmt(next)?;
    //     Ok(self.arena.alloc_box(VmStmt::IfStmt(VmIfStmt {
    //         cond,
    //         then_branch: then,
    //         else_branch: else_clause,
    //         next,
    //     }))?)
    // }
    // fn compile_loop_stmt(
    //     &mut self,
    //     stmt: &'par LoopStmt<'par>,
    //     next: &'par [Stmt<'par>],
    // ) -> Result<BoxVmStmt<'vm>, CompileError<'par, 'vm>> {
    //     let inner = self.compile_stmt(&stmt.inner)?;
    //     let next = self.compile_stmt(next)?;
    //     todo!();
    // }
    // fn compile_while_stmt(
    //     &mut self,
    //     stmt: &'par WhileStmt<'par>,
    //     next: &'par [Stmt<'par>],
    // ) -> Result<BoxVmStmt<'vm>, CompileError<'par, 'vm>> {
    //     todo!();
    // }
    // fn compile_expr(&mut self, expr: &Expr) -> Result<BoxVmExpr<'vm>, CompileError<'par, 'vm>> {
    //     match expr {
    //         Expr::Var(v) => {
    //             let index = self
    //                 .variables
    //                 .iter()
    //                 .rev()
    //                 .position(|x| *x == Some(v.ident))
    //                 .ok_or_else(|| CompileError::UnknownVariable)?;
    //             Ok(self.arena.alloc_box(VmExpr::Var(index))?)
    //         }
    //         Expr::Parens(x) => self.compile_expr(&x.expr),
    //         Expr::Number(x) => Ok(self.arena.alloc_box(VmExpr::Number(
    //             x.number
    //                 .parse()
    //                 .map_err(|_| CompileError::BadNumberLiteral)?,
    //         ))?),
    //         Expr::InfixExpr(expr) => {
    //             let operator = match expr.symbol.symbol {
    //                 Symbol::Plus => VmOperator::Plus,
    //                 Symbol::Minus => VmOperator::Minus,
    //                 Symbol::Times => VmOperator::Times,
    //                 Symbol::Divide => VmOperator::Divide,
    //                 _ => return Err(CompileError::UnexpectedInfixSymbol),
    //             };
    //             Ok(self.arena.alloc_box(VmExpr::Operator(VmOperatorExpr {
    //                 operator,
    //                 left: self.compile_expr(&expr.left)?,
    //                 right: self.compile_expr(&expr.right)?,
    //             }))?)
    //         }
    //         Expr::Call(expr) => self.compile_call_expr(expr),
    //         Expr::Null(_) => Ok(self.arena.alloc_box(VmExpr::Null)?),
    //         Expr::False(_) => Ok(self.arena.alloc_box(VmExpr::Boolean(false))?),
    //         Expr::True(_) => Ok(self.arena.alloc_box(VmExpr::Boolean(true))?),
    //         Expr::String(x) => Ok(self
    //             .arena
    //             .alloc_box(VmExpr::String(self.arena.alloc_str(x)?))?),
    //     }
    // }
    //
    // fn compile_call_expr(
    //     &mut self,
    //     expr: &CallExpr,
    // ) -> Result<BoxVmExpr<'vm>, CompileError<'par, 'vm>> {
    //     let function = match &*expr.callee {
    //         Expr::Var(ident) => {
    //             let function = self
    //                 .natives
    //                 .iter()
    //                 .position(|x| x.name() == ident.ident)
    //                 .ok_or(CompileError::UnknownFunction)?;
    //             VmFunctionName::Native(function)
    //         }
    //         _ => return Err(CompileError::Unimplemented),
    //     };
    //     let mut args = ArenaVec::try_with_capacity_in(expr.args.exprs.len(), &self.arena)?;
    //     for arg in &expr.args.exprs {
    //         args.try_push(self.compile_expr(arg)?)?;
    //         self.variables.push(None);
    //     }
    //     let result = self
    //         .arena
    //         .alloc_box(VmExpr::Call(VmCallExpr { function, args }))?;
    //     for _ in 0..expr.args.exprs.len() {
    //         self.variables.pop();
    //     }
    //     Ok(result)
    // }
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
        while self.variables.len() > orig_vars {
            self.variables.pop();
        }
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
        self.push_instr(cond, VmInstr::Load(counter))?;
        self.push_instr(cond, VmInstr::Load(limit))?;
        self.push_instr(cond, VmInstr::Binop(VmOperator::LessThan))?;
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
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        todo!();
        Ok(block)
    }
    fn compile_while_stmt(
        &mut self,
        stmt: &'par WhileStmt<'par>,
        mut block: usize,
    ) -> Result<usize, CompileError<'par, 'vm>> {
        todo!();
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
        let var = self
            .variables
            .iter()
            .position(|x| *x == Some(var.ident))
            .ok_or(CompileError::UnknownVariable(var))?;
        self.push_instr(block, VmInstr::Load(var))?;
        Ok(block)
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
        for arg in &expr.args.exprs {
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
}
