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

use crate::compiler::stack::Stack;
use crate::interp::error::InterpError;
use crate::interp::heap::Heap;
use crate::interp::heap_types::{HeapString, HeapStringInPlace};
use crate::interp::value::Value;
use crate::native::NativeFn;
use crate::vm::{
    VmFunction, VmFunctionName, VmInstr, VmOperator, VmProgram, VmTerm, VmUnaryOperator,
};
use core::fmt::Display;
use heapless::VecView;
use heapless::string::StringInPlace;

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
    pub async fn interp(&mut self, mut stack: Stack<'_>) -> Result<(), InterpError> {
        let main: &VmFunction = self
            .program
            .functions
            .get(0)
            .ok_or(InterpError::MissingMainFunction)?;
        self.interp_func(main, stack).await?;
        assert!(
            self.value_stack.is_empty(),
            "Not empty: {:?}",
            self.value_stack
        );
        Ok(())
    }
    pub async fn interp_func(
        &mut self,
        fun: &'vm VmFunction<'vm>,
        mut stack: Stack<'_>,
    ) -> Result<(), InterpError> {
        let mut block = 0;
        loop {
            for instr in &fun.blocks[block].instrs {
                self.interp_instr(instr, stack.reborrow()).await?;
            }
            if let Some(b) = self
                .interp_term(&fun.blocks[block].term, stack.reborrow())
                .await?
            {
                block = b;
            } else {
                break;
            }
        }
        Ok(())
    }
    pub async fn interp_instr(
        &mut self,
        instr: &'vm VmInstr<'vm>,
        mut stack: Stack<'_>,
    ) -> Result<(), InterpError> {
        match instr {
            VmInstr::Unused(x) => match **x {},
            VmInstr::Integer(x) => self.push_value(Value::Number(*x)),
            VmInstr::Bool(x) => self.push_value(Value::Bool(*x)),
            VmInstr::String(s) => {
                let r = self
                    .heap
                    .insert(HeapStringInPlace::new(StringInPlace::new(s.len()))?)
                    .unwrap();
                self.heap
                    .get_typed_mut::<HeapString>(&r)
                    .unwrap()
                    .push_str(s)
                    .unwrap();
                self.push_value(Value::Ref(r))?;
                Ok(())
            }
            VmInstr::Binop(x) => self.interp_binop(*x),
            VmInstr::Call(fun, args) => self.interp_call(fun, *args, stack).await,
            VmInstr::Pop => {
                self.pop_value()?;
                Ok(())
            }
            VmInstr::Load(x) => {
                let copied = self
                    .value_stack
                    .get(*x)
                    .ok_or(InterpError::BadStackIndex)?
                    .clone_in(&mut self.heap);
                self.push_value(copied)?;
                Ok(())
            }
            VmInstr::Store(x) => {
                *self
                    .value_stack
                    .get_mut(*x)
                    .ok_or(InterpError::BadStackIndex)? = self.pop_value()?;
                Ok(())
            }
            VmInstr::Unop(x) => self.interp_unop(*x),
            VmInstr::Null => {
                self.push_value(Value::Null)?;
                Ok(())
            }
        }
    }
    pub fn interp_binop(&mut self, op: VmOperator) -> Result<(), InterpError> {
        let b = self.pop_value()?;
        let a = self.pop_value()?;
        match op {
            VmOperator::Plus | VmOperator::Times | VmOperator::Minus | VmOperator::Divide => {
                let a = a.into_number()?;
                let b = b.into_number()?;
                let c = match op {
                    VmOperator::Plus => a.checked_add(b).ok_or(InterpError::IntegerOverflow)?,
                    VmOperator::Times => a.checked_mul(b).ok_or(InterpError::IntegerOverflow)?,
                    VmOperator::Minus => a.checked_sub(b).ok_or(InterpError::IntegerOverflow)?,
                    VmOperator::Divide => a.checked_div(b).ok_or(InterpError::IntegerOverflow)?,
                    _ => unreachable!(),
                };
                self.push_value(Value::Number(c))?;
            }
            VmOperator::Less
            | VmOperator::LessEquals
            | VmOperator::Greater
            | VmOperator::GreaterEquals
            | VmOperator::EqualsEquals => {
                let a = a.into_number()?;
                let b = b.into_number()?;
                let c = match op {
                    VmOperator::Less => a < b,
                    VmOperator::LessEquals => a <= b,
                    VmOperator::Greater => a > b,
                    VmOperator::GreaterEquals => a >= b,
                    VmOperator::EqualsEquals => a == b,
                    _ => unreachable!(),
                };
                self.push_value(Value::Bool(c))?;
            }
        }
        Ok(())
    }
    pub fn interp_unop(&mut self, op: VmUnaryOperator) -> Result<(), InterpError> {
        let a = self.pop_value()?;
        let result = match op {
            VmUnaryOperator::Negate => {
                let a = a.into_number()?;
                Value::Number(-a)
            }
            VmUnaryOperator::Not => {
                let a = a.into_bool();
                Value::Bool(!a)
            }
        };
        self.push_value(result)?;
        Ok(())
    }
    pub async fn interp_call(
        &mut self,
        fun: &'vm VmFunctionName,
        args: usize,
        mut stack: Stack<'_>,
    ) -> Result<(), InterpError> {
        match fun {
            VmFunctionName::Native(native) => {
                let result = self.natives[*native]
                    .native_call(
                        &mut stack,
                        &mut self.heap,
                        &self.value_stack[self.value_stack.len() - args..],
                    )?
                    .await?;
                for x in 0..args {
                    self.pop_value()?;
                }
                self.push_value(result)?;
            }
        }
        Ok(())
    }
    pub async fn interp_term(
        &mut self,
        term: &'vm VmTerm<'vm>,
        mut stack: Stack<'_>,
    ) -> Result<Option<usize>, InterpError> {
        match term {
            VmTerm::Unused(x) => match **x {},
            VmTerm::Jump(x) => Ok(Some(*x)),
            VmTerm::Uninit => todo!(),
            VmTerm::Return => Ok(None),
            VmTerm::CondJump { yes, no } => {
                if self.pop_value()?.into_bool() {
                    Ok(Some(*yes))
                } else {
                    Ok(Some(*no))
                }
            }
        }
    }
    // async fn interp_stmt(
    //     &mut self,
    //     mut stack: Stack<'_>,
    //     stmt: &'vm VmStmt<'vm>,
    // ) -> Result<(), InterpError> {
    //     match stmt {
    //         VmStmt::LetStmt(stmt) => {
    //             let value = self.interp_expr(stack.reborrow(), &stmt.expr).await?;
    //             self.push_value(value)?;
    //             self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
    //             self.pop_value();
    //             Ok(())
    //         }
    //         VmStmt::ExprStmt(stmt) => {
    //             self.interp_expr(stack.reborrow(), &stmt.expr).await?;
    //             self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
    //             Ok(())
    //         }
    //         VmStmt::Noop => Ok(()),
    //         VmStmt::ForStmt(stmt) => {
    //             let init = self
    //                 .interp_expr(stack.reborrow(), &stmt.init)
    //                 .await?
    //                 .into_number()?;
    //             let limit = self
    //                 .interp_expr(stack.reborrow(), &stmt.limit)
    //                 .await?
    //                 .into_number()?;
    //             for x in init..limit {
    //                 self.push_value(Value::Number(x))?;
    //                 self.interp_stmt_rec(stack.reborrow(), &stmt.inner).await?;
    //                 self.pop_value();
    //             }
    //             self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
    //             Ok(())
    //         }
    //         VmStmt::IfStmt(stmt) => {
    //             let cond = self.interp_expr(stack.reborrow(), &stmt.cond).await?;
    //             if cond.into_bool() {
    //                 self.interp_stmt_rec(stack.reborrow(), &stmt.then_branch)
    //                     .await?;
    //             } else {
    //                 self.interp_stmt_rec(stack.reborrow(), &stmt.else_branch)
    //                     .await?;
    //             }
    //             self.interp_stmt_rec(stack.reborrow(), &stmt.next).await?;
    //             Ok(())
    //         }
    //     }
    // }
    // async fn interp_stmt_rec(
    //     &mut self,
    //     mut stack: Stack<'_>,
    //     stmt: &'vm VmStmt<'vm>,
    // ) -> Result<(), InterpError> {
    //     Ok(stack
    //         .recurse(async move |stack| self.interp_stmt(stack, stmt).await)
    //         .await??)
    // }
    //
    // pub async fn interp_expr(
    //     &mut self,
    //     mut stack: Stack<'_>,
    //     expr: &'vm VmExpr<'vm>,
    // ) -> Result<Value, InterpError> {
    //     match expr {
    //         VmExpr::Call(call) => {
    //             for arg in &call.args {
    //                 let value = self.interp_expr_rec(stack.reborrow(), arg).await?;
    //                 self.push_value(value)?;
    //             }
    //             let result = self.call_fn(stack.reborrow(), call).await?;
    //             for arg in &call.args {
    //                 self.pop_value();
    //             }
    //             Ok(result)
    //         }
    //         VmExpr::Operator(op) => {
    //             let left = self.interp_expr_rec(stack.reborrow(), &op.left).await?;
    //             let right = self.interp_expr_rec(stack.reborrow(), &op.right).await?;
    //             match (op.operator, left, right) {
    //                 (VmOperator::Plus, Value::Number(left), Value::Number(right)) => {
    //                     Ok(Value::Number(left + right))
    //                 }
    //                 (VmOperator::Minus, Value::Number(left), Value::Number(right)) => {
    //                     Ok(Value::Number(left - right))
    //                 }
    //                 (VmOperator::Times, Value::Number(left), Value::Number(right)) => {
    //                     Ok(Value::Number(left * right))
    //                 }
    //                 (VmOperator::Divide, Value::Number(left), Value::Number(right)) => {
    //                     Ok(Value::Number(left / right))
    //                 }
    //                 _ => return Err(InterpError::OperatorError),
    //             }
    //         }
    //         VmExpr::Var(n) => {
    //             Ok(self.value_stack[self.value_stack.len() - n - 1].clone_in(&mut self.heap))
    //         }
    //         VmExpr::Number(n) => Ok(Value::Number(*n)),
    //         VmExpr::Null => Ok(Value::Null),
    //         VmExpr::Boolean(x) => Ok(Value::Bool(*x)),
    //         VmExpr::String(s) => {
    //             let r = self
    //                 .heap
    //                 .insert(HeapStringInPlace::new(StringInPlace::new(s.len()))?)
    //                 .unwrap();
    //             self.heap
    //                 .get_typed_mut::<HeapString>(&r)
    //                 .unwrap()
    //                 .push_str(s)
    //                 .unwrap();
    //             Ok(Value::Ref(r))
    //         }
    //     }
    // }
    //
    // async fn call_fn(
    //     &mut self,
    //     mut stack: Stack<'_>,
    //     call: &'vm VmCallExpr<'vm>,
    // ) -> Result<Value, InterpError> {
    //     match call.function {
    //         // VmFunctionName::Print => self.interp_print(call.args.len()).await?,
    //         VmFunctionName::Native(x) => Ok(self.natives[x]
    //             .native_call(
    //                 &mut stack,
    //                 &mut self.heap,
    //                 &self.value_stack[self.value_stack.len() - call.args.len()..],
    //             )?
    //             .into_pin()
    //             .await?),
    //     }
    // }
    //
    // async fn interp_expr_rec(
    //     &mut self,
    //     mut stack: Stack<'_>,
    //     expr: &'vm VmExpr<'vm>,
    // ) -> Result<Value, InterpError> {
    //     Ok(stack
    //         .recurse(async move |stack| self.interp_expr(stack, expr).await)
    //         .await??)
    // }

    fn push_value(&mut self, value: Value) -> Result<(), InterpError> {
        self.value_stack
            .push(value)
            .map_err(|_| InterpError::AllocError)?;
        Ok(())
    }

    fn pop_value(&mut self) -> Result<Value, InterpError> {
        self.value_stack.pop().ok_or(InterpError::StackEmpty)
    }
}
