// use crate::ast::Stmt;
// use crate::stack::{StackBox, StackFrame};
// use crate::vm::{VmExpr, VmProgram, VmStmt};
// use alloc::boxed::Box;
// use core::alloc::AllocError;
// use core::marker::PhantomData;
// use core::mem::MaybeUninit;
// use core::pin::Pin;
//
// pub struct Interp<'vm> {
//     program: &'vm VmProgram<'vm>,
// }
//
// pub enum Value {
//     Number(i64),
// }
//
// pub enum InterpError<'vm> {
//     Unused(!, PhantomData<&'vm ()>),
//     MissingMainFunction,
//     AllocError,
// }
//
// impl From<AllocError> for InterpError<'_> {
//     fn from(_: AllocError) -> Self {
//         InterpError::AllocError
//     }
// }
//
// type InterpStmt<'vm: 'a, 'a> = impl 'a + Future<Output = Result<(), InterpError<'vm>>>;
//
// impl<'vm> Interp<'vm> {
//     pub fn new(program: &'vm VmProgram<'vm>) -> Self {
//         Interp { program }
//     }
//     pub async fn interp(&mut self, mut stack: StackFrame<'_>) -> Result<(), InterpError<'vm>> {
//         let main = self
//             .program
//             .functions
//             .get(0)
//             .ok_or(InterpError::MissingMainFunction)?;
//         self.interp_stmt(stack.reborrow(), &main.stmt).await?;
//         Ok(())
//     }
//     async fn interp_stmt<'a>(
//         &'a mut self,
//         mut stack: StackFrame<'a>,
//         stmt: &'vm VmStmt<'vm>,
//     ) -> Result<(), InterpError<'vm>> {
//         match stmt {
//             VmStmt::LetStmt(stmt) => {
//                 self.interp_expr(stack.reborrow(), &stmt.expr).await?;
//                 self.interp_stmt_rec(stack.reborrow(), &stmt.next)?.await?;
//                 Ok(())
//             }
//             VmStmt::ExprStmt(stmt) => todo!(),
//             VmStmt::Noop => todo!(),
//         }
//     }
//     #[define_opaque(InterpStmt)]
//     fn interp_stmt_fut<'a>(
//         &'a mut self,
//         mut stack: StackFrame<'a>,
//         stmt: &'vm VmStmt<'vm>,
//     ) -> InterpStmt<'vm, 'a>
//     where
//         'vm: 'a,
//     {
//         self.interp_stmt(stack, stmt)
//     }
//     fn interp_stmt_rec<'a>(
//         &'a mut self,
//         mut stack: StackFrame<'a>,
//         stmt: &'vm VmStmt<'vm>,
//     ) -> Result<Pin<StackBox<'a, InterpStmt<'vm, 'a>>>, InterpError<'vm>> {
//         let mut result: StackBox<'a, MaybeUninit<InterpStmt<'vm, 'a>>> = stack.alloc_uninit()?;
//         let result: StackBox<'a, InterpStmt<'vm, 'a>> =
//             Box::write(result, self.interp_stmt_fut(stack, stmt));
//         let result: Pin<StackBox<'a, InterpStmt<'vm, 'a>>> = result.into();
//         Ok(result)
//     }
//     pub async fn interp_expr<'sf>(
//         &mut self,
//         mut stack: StackFrame<'sf>,
//         expr: &'vm VmExpr<'vm>,
//     ) -> Result<(), InterpError<'vm>> {
//         match expr {
//             VmExpr::Call(_) => todo!(),
//             VmExpr::Operator(_) => todo!(),
//             VmExpr::Var(_) => todo!(),
//             VmExpr::Number(_) => todo!(),
//         }
//     }
// }
