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
use core::time::Duration;
use log::info;
use std::cell::RefCell;
use std::time::SystemTime;

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

struct TestNowUsFn;

impl NativeFn for TestNowUsFn {
    fn name(&self) -> &'static str {
        "now_us"
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
                Ok(Value::Number(
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_micros() as i64,
                ))
            })?
            .into_pin())
    }
}

struct TestSleepUsFn;

impl NativeFn for TestSleepUsFn {
    fn name(&self) -> &'static str {
        "sleep_us"
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
                if let Some(arg) = args.get(0) {
                    match arg {
                        Value::Number(number) => {
                            println!("Sleeping {}", number);
                            tokio::time::sleep(Duration::from_micros(*number as u64)).await;
                        }
                        _ => return Err(NativeError),
                    }
                } else {
                    return Err(NativeError);
                }
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
        .run(
            TEST_SPINDLE_OPTIONS.clone(),
            code,
            &[&TestPrintFn, &FormatFn, &TestNowUsFn, &TestSleepUsFn],
        )
        .await?;
    Ok(TEST_LOGS.with(|x| x.take()))
}
