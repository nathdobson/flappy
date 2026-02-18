use crate::compiler::codegen::Codegen;
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::compiler::stack::{Stack, StackBox, StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use crate::interp::heap::Heap;
use crate::interp::value::Value;
use crate::native::{NativeError, NativeFn, PrintFn};
use crate::vm::VmProgram;
use crate::{Spindle, SpindleError};
use alloc::string::String;
use alloc::vec::Vec;
use arena::ArenaStorage;
use core::alloc::AllocError;
use core::fmt;
use core::pin::Pin;
use std::cell::RefCell;
//
// pub async fn with_test_compile<T>(
//     code: &str,
//     callback: impl for<'vm> AsyncFnOnce(&'vm VmProgram<'vm>) -> T,
// ) -> T {
//     let capacity: usize = 100000;
//     let mut arena_par_slice = vec![0u8; capacity];
//     let mut arena_par_storage = ArenaStorage::new(&mut arena_par_slice);
//     let arena_par = arena_par_storage.start();
//     let mut stack = new_stack::<65536>();
//     let stack: &mut StackStorage = &mut stack;
//     let mut stack = stack.start();
//     let program = stack_executor(stack.reborrow(), async |spawn| {
//         Parser::new(Lexer::new(code, arena_par), arena_par)
//             .parse_program(spawn)
//             .await
//             .unwrap()
//     })
//     .await
//     .unwrap();
//
//     let mut arena_vm_slice = vec![0u8; capacity];
//     let mut arena_vm_storage = ArenaStorage::new(&mut arena_vm_slice);
//     let arena_vm = arena_vm_storage.start();
//     let program = stack_executor(stack.reborrow(), async |spawn| {
//         Codegen::new(arena_vm, &[&PrintFn], &program)
//             .compile(spawn)
//             .await
//             .unwrap()
//     })
//     .await
//     .unwrap();
//
//     callback(&program).await
// }

pub const fn test_natives() -> &'static [&'static dyn NativeFn] {
    &[&PrintFn]
}

pub type TestSpindle = Spindle<65536, 65536, 128, 1024, 65536>;

struct TestPrintFn;

std::thread_local! {
    static TEST_LOGS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

impl NativeFn for TestPrintFn {
    fn name(&self) -> &'static str {
        "print"
    }

    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<
        Pin<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>>,
        AllocError,
    > {
        Ok(stack
            .push_init(async move {
                let mut string = format!(
                    "{}",
                    fmt::from_fn(|f| {
                        for arg in args {
                            arg.format(heap, f)?;
                        }
                        Ok(())
                    })
                );
                TEST_LOGS.with(|test_logs| test_logs.borrow_mut().push(string));
                Ok(Value::Null)
            })?
            .into_pin())
    }
}

pub async fn interp(code: &str) -> Result<Vec<String>, SpindleError<'_>> {
    TestSpindle::new().run(code, &[&TestPrintFn]).await?;
    Ok(TEST_LOGS.with(|x| x.take()))
}
