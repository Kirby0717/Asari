pub mod eval;
pub mod exec;
pub mod shell_command;
pub mod subst;

use crate::parse::Span;
use eval::Error as EvalError;
use exec::{Error as ExecError, RedirectError};
use shell_command::Error as ShellCommandError;
use subst::Error as SubstError;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct Shell {
    context: Context,
}
impl Shell {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_input(&mut self, input: &str) {
        self.context.current_input = input.to_string();
    }
    pub fn execute(&mut self, shell_command: &crate::parse::CommandLine) {
        exec::execute_command_line(shell_command, &mut self.context)
    }
}

#[derive(Clone, Debug)]
pub struct SpannedError<E> {
    pub kind: E,
    pub span: Span,
}
impl<E> SpannedError<E> {
    pub fn display(&self, input: &str) -> String
    where
        E: std::fmt::Display,
    {
        let mut display = String::new();
        display += &format!("{}\n", input.replace(['\n', '\r'], " "));
        display += &format!(
            "{}{} {}\n",
            " ".repeat(self.span.start),
            "^".repeat(self.span.len()),
            self.kind
        );
        display
    }
}
pub trait WithSpan<T, E> {
    fn with_span(self, span: &Span) -> Result<T, SpannedError<E>>;
}
impl<T, E> WithSpan<T, E> for Result<T, E> {
    fn with_span(self, span: &Span) -> Result<T, SpannedError<E>> {
        self.map_err(|kind| SpannedError {
            kind,
            span: span.clone(),
        })
    }
}
macro_rules! impl_spanned_from {
    ($from:ident => $to:ident :: $variant:ident) => {
        impl From<SpannedError<$from>> for SpannedError<$to> {
            fn from(e: SpannedError<$from>) -> Self {
                SpannedError {
                    kind: $to::$variant(e.kind),
                    span: e.span,
                }
            }
        }
    };
}
impl_spanned_from!(EvalError         => ExecError::Eval);
impl_spanned_from!(SubstError        => EvalError::Subst);
impl_spanned_from!(RedirectError     => ExecError::Redirect);
impl_spanned_from!(ShellCommandError => ExecError::ShellCommand);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub shell_name: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub shell_vars: HashMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_pid: Option<u32>,
    pub last_status: i32,
    pub current_input: String,
}
impl Default for Context {
    fn default() -> Self {
        Context {
            shell_name: "asari".to_string(),
            shell_vars: Default::default(),
            last_pid: None,
            last_status: 0,
            current_input: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Option(Option<Box<Value>>),
    Unit,
}
impl Value {
    pub fn get_type(&self) -> Type {
        use Value::*;
        match self {
            String(_) => Type::String,
            Int(_) => Type::Int,
            Float(_) => Type::Float,
            Bool(_) => Type::Bool,
            Array(v) => Type::Array(Box::new(
                v.first().map_or(Type::Unknown, Value::get_type),
            )),
            Option(o) => Type::Option(Box::new(
                o.as_deref().map_or(Type::Unknown, Value::get_type),
            )),
            Unit => Type::Unit,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Type {
    String,
    Int,
    Float,
    Bool,
    Array(Box<Type>),
    Option(Box<Type>),
    Unit,
    Unknown,
}
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Type::*;
        match self {
            String => write!(f, "string"),
            Int => write!(f, "int"),
            Float => write!(f, "float"),
            Bool => write!(f, "bool"),
            Array(t) => write!(f, "array<{t}>"),
            Option(t) => write!(f, "option<{t}>"),
            Unit => write!(f, "unit"),
            Unknown => write!(f, "_"),
        }
    }
}

macro_rules! impl_from_value {
    ($type_name:ident $rust_type:ident : $($t:ty),*) => {
        $(impl From<$t> for Value {
            fn from(v: $t) -> Self { Value::$type_name(v as $rust_type) }
        })*
    };
    (direct : $($type_name:ident $rust_type:ident),*) => {
        $(impl From<$rust_type> for Value {
            fn from(v: $rust_type) -> Self { Value::$type_name(v) }
        })*
    };
}
impl_from_value!(Int i64: i8, i16, i32, i64, isize);
impl_from_value!(Int i64: u8, u16, u32);
impl_from_value!(Float f64: f32, f64);
impl_from_value!(direct : String String, Bool bool);
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        Value::Option(value.map(|v| Box::new(v.into())))
    }
}
impl From<()> for Value {
    fn from(_value: ()) -> Self {
        Value::Unit
    }
}

#[cfg(not(unix))]
pub fn status_into_i32(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}
#[cfg(unix)]
pub fn status_into_i32(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or_else(|| {
        use std::os::unix::process::ExitStatusExt;
        status.signal().map(|sig| 128 + sig).unwrap_or(1)
    })
}
