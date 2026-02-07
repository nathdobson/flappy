use core::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum Value {
    None,
    Bool(bool),
    Number(i64),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Value::None => write!(f, "None"),
            Value::Bool(x) => write!(f, "{}", x),
            Value::Number(x) => write!(f, "{}", x),
        }
    }
}
