use core::fmt;
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
    pub fn clone_in(&self, heap: &mut Heap<'_>) -> Self {
        match self {
            Value::Null => Value::Null,
            Value::Bool(x) => Value::Bool(*x),
            Value::Number(x) => Value::Number(*x),
            Value::Ref(x) => Value::Ref(heap.clone_ref(x)),
        }
    }
    pub fn format(&self, heap: &Heap, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null")?,
            Value::Bool(arg) => write!(f, "{}", arg)?,
            Value::Number(arg) => write!(f, "{}", arg)?,
            Value::Ref(arg) => write!(f, "{}", heap.get(arg))?,
        }
        Ok(())
    }
}
