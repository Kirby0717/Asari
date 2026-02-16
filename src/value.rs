use super::eval::Error as EvalError;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Option(Option<Box<Value>>),
    Unit,
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}
impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}
impl TryFrom<u64> for Value {
    type Error = EvalError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if let Ok(value) = i64::try_from(value) {
            Ok(Value::Int(value))
        }
        else {
            Err(EvalError::OverFlow)
        }
    }
}
impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Value::Int(value as i64)
    }
}
impl TryFrom<usize> for Value {
    type Error = EvalError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if let Ok(value) = i64::try_from(value) {
            Ok(Value::Int(value))
        }
        else {
            Err(EvalError::OverFlow)
        }
    }
}
impl TryFrom<i128> for Value {
    type Error = EvalError;
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        if let Ok(value) = i64::try_from(value) {
            Ok(Value::Int(value))
        }
        else {
            Err(EvalError::OverFlow)
        }
    }
}
impl TryFrom<u128> for Value {
    type Error = EvalError;
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        if let Ok(value) = i64::try_from(value) {
            Ok(Value::Int(value))
        }
        else {
            Err(EvalError::OverFlow)
        }
    }
}
impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(value as f64)
    }
}
impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        Value::Option(value.map(|i| Box::new(i.into())))
    }
}
impl From<()> for Value {
    fn from(_value: ()) -> Self {
        Value::Unit
    }
}
impl<T: Copy + Into<Value>> From<&T> for Value {
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}
