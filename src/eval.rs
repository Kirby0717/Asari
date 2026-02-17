use std::collections::HashMap;

use super::{
    parse::{
        Expr, ExprInfix, ExprPostfix, ExprPrefix, Primary, Spanned, SpecialVar,
    },
    value::*,
};

#[derive(Clone, Debug)]
pub enum Error {
    TypeError,
    OverFlow,
    UnwrapNone,
    UnknownShellVar,
    CastError,
}
pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct Context {
    pub shell_name: String,
    pub shell_vars: HashMap<String, Value>,
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

pub fn eval_expr(expr: &Spanned<Expr>, env: &Context) -> Result<Value> {
    use Expr::*;
    Ok(match &expr.inner {
        Primary(primary) => eval_primary(primary, env)?,
        Prefix(expr, prefix) => eval_prefix(expr, prefix, env)?,
        Infix(expr1, expr2, infix) => eval_infix(expr1, expr2, infix, env)?,
        Postfix(expr, postfix) => eval_postfix(expr, postfix, env)?,
    })
}
macro_rules! match_op {
    {
        $value:expr;
        $($mac:ident: {
            $($base:tt { $( $name:ident $op:tt ),* $(,)? })*
        })*
        @extra:
        $( $extra_pat:pat => $extra_expr:expr ),* $(,)?
    } => {
        Ok(match $value {
            $($($( $mac!( pat $base $name $op ) => $mac!( body $base $name $op ), )*)*)*
            $( $extra_pat => $extra_expr, )*
        })
    };
}
macro_rules! primitive_prefix {
    (pat  [$type:ident($var:ident)] $prefix: ident $op:tt)
        => {($type($var), $prefix)};
    (body [$type:ident($var:ident)] $prefix: ident $op:tt)
        => {($op $var).into()};
}
macro_rules! checked_prefix {
    (pat  [$type:ident($var:ident)] $prefix: ident $op:tt) => {
        ($type($var), $prefix)
    };
    (body [$type:ident($var:ident)] $prefix: ident $op:tt) => {
        ($var.$op().ok_or(Error::OverFlow)?).into()
    };
}
macro_rules! primitive_infix {
    (pat  [$type:ident($var1:ident, $var2:ident)] $infix: ident $op:tt)
        => {($type($var1), $type($var2), $infix)};
    (body [$type:ident($var1:ident, $var2:ident)] $infix: ident $op:tt)
        => {($var1 $op $var2).into()};
}
macro_rules! checked_infix {
    (pat  [$type:ident($var1:ident, $var2:ident)] $infix: ident $op:tt) => {
        ($type($var1), $type($var2), $infix)
    };
    (body [$type:ident($var1:ident, $var2:ident)] $infix: ident $op:tt) => {
        ($var1.$op($var2).ok_or(Error::OverFlow)?).into()
    };
}
macro_rules! _primitive_postfix {
    (pat  [$type:ident($var:ident)] $postfix: ident $op:tt)
        => {($type($var), $postfix)};
    (body [$type:ident($var:ident)] $postfix: ident $op:tt)
        => {($op $var).into()};
}

fn eval_prefix(
    expr: &Spanned<Expr>,
    prefix: &ExprPrefix,
    env: &Context,
) -> Result<Value> {
    let value = eval_expr(expr, env)?;
    use ExprPrefix::*;
    use Value::*;
    match_op! {(value, prefix);
        primitive_prefix: {
            [ Float(a) ] {        Neg -, }
            [ Bool(a)  ] { Not !,        }
        }
        checked_prefix: {
            [ Int(a) ] { Neg checked_neg }
        }
        @extra:
        _ => return Err(Error::TypeError),
    }
}
fn eval_infix(
    expr1: &Spanned<Expr>,
    expr2: &Spanned<Expr>,
    infix: &ExprInfix,
    env: &Context,
) -> Result<Value> {
    let value1 = eval_expr(expr1, env)?;
    let value2 = eval_expr(expr2, env)?;
    use ExprInfix::*;
    use Value::*;
    match_op! { (value1, value2, infix);
        primitive_infix: {
            [ String(s1, s2) ] {
                Equal ==, NotEqual !=,
                Less <, LessEqual <=, Greater >, GreaterEqual >=,
            }
            [ Int(a, b) ] {
                Equal ==, NotEqual !=,
                Less <, LessEqual <=, Greater >, GreaterEqual >=,
            }
            [ Float(a, b) ] {
                Add +, Sub -, Mul *, Div /, Rem %,
                Equal ==, NotEqual !=,
                Less <, LessEqual <=, Greater >, GreaterEqual >=,
            }
            [ Bool(a, b)     ] { Equal ==, NotEqual !=, And &&, Or || }
            [ Array(v1, v2)  ] { Equal ==, NotEqual != }
            [ Option(o1, o2) ] { Equal ==, NotEqual != }
        }
        checked_infix: {
            [ Int(a, b) ] {
                Add checked_add,
                Sub checked_sub,
                Mul checked_mul,
                Div checked_div,
                Rem checked_rem,
            }
        }
        @extra:
        (String(a), String(b), Add) => (a + &b).into(),
        (Option(a), b, UnwrapOr) => *a.unwrap_or(Box::new(b)),
        _ => return Err(Error::TypeError),
    }
}
fn eval_postfix(
    expr: &Spanned<Expr>,
    postfix: &ExprPostfix,
    env: &Context,
) -> Result<Value> {
    let value = eval_expr(expr, env)?;
    use ExprPostfix::*;
    use Value::*;
    Ok(match (value, postfix) {
        (Option(a), Unwrap) => {
            if let Some(a) = a {
                *a
            }
            else {
                return Err(Error::UnwrapNone);
            }
        }
        (Option(a), IsSome) => a.is_some().into(),
        (String(s), Length) => s.chars().count().try_into()?,
        (Array(v), Length) => v.len().try_into()?,
        (Array(v), Index(index)) => {
            let index = eval_expr(index, env)?;
            let Int(index) = index
            else {
                return Err(Error::TypeError);
            };
            let index = if index >= 0 {
                // 正
                usize::try_from(index).ok()
            }
            else {
                // 負
                if let Ok(index) = isize::try_from(index) {
                    v.len().checked_add_signed(index)
                }
                else {
                    None
                }
            };
            index.and_then(|index| v.get(index).cloned()).into()
        }
        (v, Cast(t)) => v.cast(&t.inner)?,
        _ => return Err(Error::TypeError),
    })
}
fn eval_primary(primary: &Spanned<Primary>, env: &Context) -> Result<Value> {
    use Primary::*;
    Ok(match &primary.inner {
        String(str) => str.clone().into(),
        PathString(..) => todo!(),
        SpecialVar(special_var) => eval_special_var(special_var, env)?,
        EnvVar(env_var) => std::env::var(env_var).ok().into(),
        ShellVar(shell_var) => env
            .shell_vars
            .get(shell_var)
            .ok_or(Error::UnknownShellVar)?
            .clone(),
        Paren(expr) => eval_expr(expr, env)?,
        Array(array) => array
            .iter()
            .map(|expr| eval_expr(expr, env))
            .collect::<Result<Vec<_>>>()?
            .into(),
        Bool(bool) => bool.into(),
        Int(int) => int.into(),
        Float(float) => float.into(),
        Option(option) => option
            .as_ref()
            .map(|expr| eval_expr(expr, env))
            .transpose()?
            .into(),
        Unit => ().into(),
    })
}
fn eval_special_var(special_var: &SpecialVar, env: &Context) -> Result<Value> {
    use SpecialVar::*;
    Ok(match special_var {
        ExitStatus => env.last_status.into(),
        Pid => std::process::id().into(),
        BackgroundPid => env.last_pid.into(),
        ShellName => env.shell_name.clone().into(),
    })
}
