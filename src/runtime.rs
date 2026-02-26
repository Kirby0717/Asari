pub mod eval;
pub mod exec;
pub mod shell_command;
pub mod subst;

use eval::Error as EvalError;
use exec::{Error as ExecError, RedirectError};
use shell_command::Error as ShellCommandError;
use subst::Error as SubstError;

use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct Shell {
    context: Context,
}
impl Shell {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn execute(
        &mut self,
        shell_command: &crate::parse::CommandLine,
    ) -> Result<()> {
        exec::execute_command_line(shell_command, &mut self.context)
    }
}

type Result<T> = ::std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Exit(i32),
    Eval(EvalError),
    Exec(ExecError),
    ShellCommand(ShellCommandError),
    Subst(SubstError),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Exit(code) => write!(f, "コード{code}で終了します"),
            Eval(e) => e.fmt(f),
            Exec(e) => e.fmt(f),
            ShellCommand(e) => e.fmt(f),
            Subst(e) => e.fmt(f),
        }
    }
}
impl From<EvalError> for Error {
    fn from(value: EvalError) -> Self {
        Error::Eval(value)
    }
}
impl From<ExecError> for Error {
    fn from(value: ExecError) -> Self {
        Error::Exec(value)
    }
}
impl From<RedirectError> for Error {
    fn from(value: RedirectError) -> Self {
        Error::Exec(ExecError::Redirect(value))
    }
}
impl From<ShellCommandError> for Error {
    fn from(value: ShellCommandError) -> Self {
        Error::ShellCommand(value)
    }
}
impl From<SubstError> for Error {
    fn from(value: SubstError) -> Self {
        Error::Subst(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub shell_name: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub shell_vars: HashMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_pid: Option<u32>,
    pub last_status: i32,
}
impl Default for Context {
    fn default() -> Self {
        Context {
            shell_name: "asari".to_string(),
            shell_vars: Default::default(),
            last_pid: None,
            last_status: 0,
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Type {
    String,
    Int,
    Float,
    Bool,
    Array(Box<Type>),
    Option(Box<Type>),
    Unit,
    #[allow(unused)]
    Unknown,
}

#[derive(Debug)]
pub struct CastError;
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
    (try $type_name:ident $rust_type:ident : $($t:ty),*) => {
        $(impl TryFrom<$t> for Value {
            type Error = CastError;
            fn try_from(v: $t) -> Result<Self, Self::Error> {
                $rust_type::try_from(v).map(Value::$type_name).map_err(|_| CastError)
            }
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
        Value::Option(value.map(|i| Box::new(i.into())))
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
