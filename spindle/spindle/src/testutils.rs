use crate::compiler::codegen::Codegen;
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::compiler::stack::{Stack, StackBox, StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use crate::interp::heap::Heap;
use crate::interp::value::Value;
use crate::native::{FormatFn, NativeError, NativeFn, PrintFn};
use crate::vm::VmProgram;
use crate::{Spindle, SpindleError, SpindleOptions};
use alloc::string::String;
use alloc::vec::Vec;
use core::alloc::AllocError;
use core::fmt;
use core::pin::Pin;
use std::cell::RefCell;

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

pub static TEST_SPINDLE_OPTIONS: SpindleOptions = SpindleOptions {
    compaction_ratio: 1.0,
};

pub async fn interp(code: &str) -> Result<Vec<String>, SpindleError<'_>> {
    TestSpindle::new()
        .run(TEST_SPINDLE_OPTIONS.clone(), code, &[&TestPrintFn, &FormatFn])
        .await?;
    Ok(TEST_LOGS.with(|x| x.take()))
}
