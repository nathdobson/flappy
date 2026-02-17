use core::alloc::AllocError;
use core::marker::PhantomData;
use crate::native::NativeError;

#[derive(Debug)]
pub enum InterpError<'vm> {
    Unused(!, PhantomData<&'vm ()>),
    MissingMainFunction,
    AllocError,
    OperatorError,
    ForLoopTypeError,
    NativeError,
    StackEmpty,
    IntegerOverflow,
    BadStackIndex,
}

impl From<AllocError> for InterpError<'_> {
    fn from(_: AllocError) -> Self {
        InterpError::AllocError
    }
}

impl From<NativeError> for InterpError<'_> {
    fn from(_: NativeError) -> Self {
        InterpError::NativeError
    }
}

#[derive(Debug)]
pub struct TypeError;
