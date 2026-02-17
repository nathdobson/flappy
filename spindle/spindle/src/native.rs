use crate::interp::error::InterpError;
use crate::interp::heap::Heap;
use crate::compiler::stack::{Stack, StackBox};
use crate::interp::value::Value;
use core::alloc::AllocError;
use core::fmt;
use core::pin::Pin;
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
                            match arg {
                                Value::Null => write!(f, "null")?,
                                Value::Bool(arg) => write!(f, "{}", arg)?,
                                Value::Number(arg) => write!(f, "{}", arg)?,
                                Value::Ref(arg) => write!(f, "{}", heap.get(arg))?,
                            }
                        }
                        Ok(())
                    })
                );
                Ok(Value::Null)
            })?
            .into_pin())
    }
}
