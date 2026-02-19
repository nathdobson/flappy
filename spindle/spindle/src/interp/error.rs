use core::alloc::AllocError;
use core::marker::PhantomData;
use crate::native::NativeError;

#[derive(Debug)]
pub enum InterpError {
    MissingMainFunction,
    AllocError,
    OperatorError,
    ForLoopTypeError,
    NativeError,
    StackEmpty,
    IntegerOverflow,
    BadStackIndex,
    NotNumber,
}

impl From<AllocError> for InterpError {
    fn from(_: AllocError) -> Self {
        InterpError::AllocError
    }
}

impl From<NativeError> for InterpError {
    fn from(_: NativeError) -> Self {
        InterpError::NativeError
    }
}

#[derive(Debug)]
pub struct TypeError;
