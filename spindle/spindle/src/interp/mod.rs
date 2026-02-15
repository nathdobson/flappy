pub mod error;
pub mod heap;
#[cfg(test)]
mod heap_test;
mod linked_slab;
mod slab;
#[cfg(test)]
mod test;
pub mod value;
// mod inline_metadata;
mod heap_types;
mod inline_slice;
pub mod stack;
#[cfg(test)]
mod stack_test;

use crate::interp::error::InterpError;
use crate::interp::heap::Heap;
use crate::interp::heap_types::{HeapString, HeapStringInPlace};
use crate::interp::value::Value;
use crate::native::NativeFn;
use crate::vm::{VmCallExpr, VmExpr, VmFunctionName, VmOperator, VmProgram, VmStmt};
use core::fmt::Display;
use heapless::string::StringInPlace;
use heapless::VecView;
use crate::interp::stack::Stack;

pub struct Interp<'vm> {
    program: &'vm VmProgram<'vm>,
    value_stack: &'vm mut VecView<Value>,
    heap: Heap<'vm>,
    natives: &'vm [&'vm dyn NativeFn],
}

impl<'vm> Interp<'vm> {
    pub fn new(
        program: &'vm VmProgram<'vm>,
        value_stack: &'vm mut VecView<Value>,
        heap: Heap<'vm>,
        natives: &'vm [&'vm dyn NativeFn],
    ) -> Self {
        Interp {
            program,
            value_stack,
            heap,
            natives,
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
                self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
                Ok(())
            }
            VmStmt::IfStmt(stmt) => {
                let cond = self.interp_expr(stack.reborrow(), &stmt.cond).await?;
                if cond.into_bool() {
                    self.interp_stmt_rec(stack.reborrow(), &stmt.then_branch)
                        .await?;
                } else {
                    self.interp_stmt_rec(stack.reborrow(), &stmt.else_branch)
                        .await?;
                }
                self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
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
                let result = self.call_fn(stack.reborrow(), call).await?;
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
            VmExpr::Var(n) => {
                Ok(self.value_stack[self.value_stack.len() - n - 1].clone_in(&mut self.heap))
            }
            VmExpr::Number(n) => Ok(Value::Number(*n)),
            VmExpr::Null => Ok(Value::Null),
            VmExpr::Boolean(x) => Ok(Value::Bool(*x)),
            VmExpr::String(s) => {
                let r = self
                    .heap
                    .insert(HeapStringInPlace::new(StringInPlace::new(s.len()))?)
                    .unwrap();
                self.heap
                    .get_typed_mut::<HeapString>(&r)
                    .unwrap()
                    .push_str(s)
                    .unwrap();
                Ok(Value::Ref(r))
            }
        }
    }

    async fn call_fn(
        &mut self,
        mut stack: Stack<'_>,
        call: &'vm VmCallExpr<'vm>,
    ) -> Result<Value, InterpError<'vm>> {
        match call.function {
            // VmFunctionName::Print => self.interp_print(call.args.len()).await?,
            VmFunctionName::Native(x) => Ok(self.natives[x]
                .native_call(
                    &mut stack,
                    &mut self.heap,
                    &self.value_stack[self.value_stack.len() - call.args.len()..],
                )?
                .into_pin()
                .await?),
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
}
