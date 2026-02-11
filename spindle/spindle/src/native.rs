use crate::interp::error::InterpError;
use crate::interp::heap::Heap;
use crate::interp::value::Value;
use crate::stack::{Stack, StackBox};
use core::alloc::AllocError;
use core::fmt;
use log::info;

pub struct NativeError;

pub trait NativeFn: 'static + Sync + Send {
    fn name(&self) -> &'static str;
    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>, AllocError>;
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
    ) -> Result<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>, AllocError>
    {
        Ok(stack.push_init(async move {
            for arg in args {
                info!(
                    "{}",
                    fmt::from_fn(|f| match arg {
                        Value::Null => write!(f, "null"),
                        Value::Bool(arg) => write!(f, "{}", arg),
                        Value::Number(arg) => write!(f, "{}", arg),
                        Value::Ref(arg) => write!(f, "{}", heap.get(arg)),
                    })
                );
            }
            Ok(Value::Null)
        })? as StackBox<_>)
    }
}
