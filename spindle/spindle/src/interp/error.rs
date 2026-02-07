use core::alloc::AllocError;
use core::marker::PhantomData;

#[derive(Debug)]
pub enum InterpError<'vm> {
    Unused(!, PhantomData<&'vm ()>),
    MissingMainFunction,
    AllocError,
    OperatorError,
}

impl From<AllocError> for InterpError<'_> {
    fn from(_: AllocError) -> Self {
        InterpError::AllocError
    }
}
