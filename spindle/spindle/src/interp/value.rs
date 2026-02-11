use crate::interp::error::InterpError;
use crate::interp::heap::{Heap, HeapRef};
use core::fmt::{Display, Formatter, write};

#[derive(Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Number(i64),
    Ref(HeapRef),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(x) => write!(f, "{}", x),
            Value::Number(x) => write!(f, "{}", x),
            Value::Ref(x) => write!(f, "{:?}", x),
        }
    }
}

impl Value {
    pub fn into_number(self) -> Result<i64, InterpError<'static>> {
        match self {
            Value::Number(x) => Ok(x),
            _ => Err(InterpError::ForLoopTypeError),
        }
    }
    pub fn into_bool(self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(value) => value,
            Value::Number(value) => value != 0,
            Value::Ref(_) => true,
        }
    }
    pub fn clone_in(&self, heap: &mut Heap<'_>) -> Self {
        match self {
            Value::Null => Value::Null,
            Value::Bool(x) => Value::Bool(*x),
            Value::Number(x) => Value::Number(*x),
            Value::Ref(x) => Value::Ref(heap.clone_ref(x)),
        }
    }
}
