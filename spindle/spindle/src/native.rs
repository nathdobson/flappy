use crate::compiler::stack::{Stack, StackBox};
use crate::interp::error::InterpError;
use crate::interp::heap::Heap;
use crate::interp::value::Value;
use core::alloc::AllocError;
use core::fmt;
use core::pin::Pin;
use heapless::String;
use log::info;

pub struct NativeError;

pub trait NativeFn: 'static {
    fn name(&self) -> &'static str;
    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<
        Pin<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>>,
        AllocError,
    >;
}

pub struct PrintFn;

impl NativeFn for PrintFn {
    fn name(&self) -> &'static str {
        "print"
    }

    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        mut stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<
        Pin<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>>,
        AllocError,
    > {
        Ok(stack
            .push_init(async move {
                info!(
                    "{}",
                    fmt::from_fn(|f| {
                        for arg in args {
                            arg.format(heap, f)?;
                        }
                        Ok(())
                    })
                );
                Ok(Value::Null)
            })?
            .into_pin())
    }
}

pub struct FormatFn;

impl NativeFn for FormatFn {
    fn name(&self) -> &'static str {
        "format"
    }

    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        mut stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<
        Pin<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>>,
        AllocError,
    > {
        Ok(stack
            .push_init(async move {
                let mut result = String::<128>::new();
                for arg in args {
                    use core::fmt::Write;
                    write!(&mut result, "{}", fmt::from_fn(|f| arg.format(heap, f)))
                        .map_err(|_| NativeError)?;
                }
                let result = heap.insert_str(&result).map_err(|_| NativeError)?;
                Ok(Value::Ref(result))
            })?
            .into_pin())
    }
}
