pub mod error;
#[cfg(test)]
mod test;
pub mod value;

use crate::ast::Stmt;
use crate::interp::error::InterpError;
use crate::interp::value::Value;
use crate::stack::{Stack, StackBox};
use crate::vm::{VmExpr, VmFunctionName, VmOperator, VmProgram, VmStmt};
use alloc::boxed::Box;
use core::alloc::AllocError;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::pin::Pin;
use heapless::VecView;
use log::info;

pub struct Interp<'vm> {
    program: &'vm VmProgram<'vm>,
    value_stack: &'vm mut VecView<Value>,
}

impl<'vm> Interp<'vm> {
    pub fn new(program: &'vm VmProgram<'vm>, value_stack: &'vm mut VecView<Value>) -> Self {
        Interp {
            program,
            value_stack,
        }
    }
    pub async fn interp(&mut self, mut stack: Stack<'_>) -> Result<(), InterpError<'vm>> {
        let main = self
            .program
            .functions
            .get(0)
            .ok_or(InterpError::MissingMainFunction)?;
        self.interp_stmt(stack, &main.stmt).await?;
        Ok(())
    }
    async fn interp_stmt(
        &mut self,
        mut stack: Stack<'_>,
        stmt: &'vm VmStmt<'vm>,
    ) -> Result<(), InterpError<'vm>> {
        match stmt {
            VmStmt::LetStmt(stmt) => {
                let value = self.interp_expr(stack.reborrow(), &stmt.expr).await?;
                self.push_value(value)?;
                self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
                self.pop_value();
                Ok(())
            }
            VmStmt::ExprStmt(stmt) => {
                self.interp_expr(stack.reborrow(), &stmt.expr).await?;
                self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
                Ok(())
            }
            VmStmt::Noop => Ok(()),
            VmStmt::ForStmt(stmt) => {
                let init = self
                    .interp_expr(stack.reborrow(), &stmt.init)
                    .await?
                    .into_number()?;
                let limit = self
                    .interp_expr(stack.reborrow(), &stmt.limit)
                    .await?
                    .into_number()?;
                for x in init..limit {
                    self.push_value(Value::Number(x))?;
                    self.interp_stmt_rec(stack.reborrow(), &stmt.inner).await?;
                    self.pop_value();
                }
                Ok(())
            }
        }
    }
    async fn interp_stmt_rec(
        &mut self,
        mut stack: Stack<'_>,
        stmt: &'vm VmStmt<'vm>,
    ) -> Result<(), InterpError<'vm>> {
        Ok(stack
            .recurse(async move |stack| self.interp_stmt(stack, stmt).await)
            .await??)
    }

    pub async fn interp_expr(
        &mut self,
        mut stack: Stack<'_>,
        expr: &'vm VmExpr<'vm>,
    ) -> Result<Value, InterpError<'vm>> {
        match expr {
            VmExpr::Call(call) => {
                for arg in &call.args {
                    let value = self.interp_expr_rec(stack.reborrow(), arg).await?;
                    self.push_value(value)?;
                }
                let result = match call.function {
                    VmFunctionName::Print => self.interp_print(call.args.len()).await?,
                };
                for arg in &call.args {
                    self.pop_value();
                }
                Ok(result)
            }
            VmExpr::Operator(op) => {
                let left = self.interp_expr_rec(stack.reborrow(), &op.left).await?;
                let right = self.interp_expr_rec(stack.reborrow(), &op.right).await?;
                match (op.operator, left, right) {
                    (VmOperator::Plus, Value::Number(left), Value::Number(right)) => {
                        Ok(Value::Number(left + right))
                    }
                    (VmOperator::Minus, Value::Number(left), Value::Number(right)) => {
                        Ok(Value::Number(left - right))
                    }
                    (VmOperator::Times, Value::Number(left), Value::Number(right)) => {
                        Ok(Value::Number(left * right))
                    }
                    (VmOperator::Divide, Value::Number(left), Value::Number(right)) => {
                        Ok(Value::Number(left / right))
                    }
                    _ => return Err(InterpError::OperatorError),
                }
            }
            VmExpr::Var(n) => Ok(self.value_stack[self.value_stack.len() - n - 1].clone()),
            VmExpr::Number(n) => Ok(Value::Number(*n)),
        }
    }

    async fn interp_expr_rec(
        &mut self,
        mut stack: Stack<'_>,
        expr: &'vm VmExpr<'vm>,
    ) -> Result<Value, InterpError<'vm>> {
        Ok(stack
            .recurse(async move |stack| self.interp_expr(stack, expr).await)
            .await??)
    }

    fn push_value(&mut self, value: Value) -> Result<(), InterpError<'vm>> {
        Ok(self
            .value_stack
            .push(value)
            .map_err(|_| InterpError::AllocError)?)
    }

    fn pop_value(&mut self) {
        self.value_stack.pop().unwrap();
    }

    async fn interp_print(&mut self, argc: usize) -> Result<Value, InterpError<'vm>> {
        for arg in self.value_stack.iter().rev().take(argc) {
            info!("{}", arg);
        }
        Ok(Value::None)
    }
}
