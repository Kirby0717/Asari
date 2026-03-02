use super::subst::Error as SubstError;
use super::{Context, SpannedError, Type, Value};
use crate::parse::{
    AstType, CommandPart, Expr, ExprInfix, ExprPostfix, ExprPrefix, Primary,
    Span, Spanned, SpecialVar,
};
use crate::runtime::WithSpan;

use std::fmt::Display;
use std::path::{Component, Path, PathBuf};
use std::string::String;

type Result<T> = ::std::result::Result<T, Error>;
type SpannedResult<T> = ::std::result::Result<T, SpannedError<Error>>;
#[derive(Debug)]
pub enum Error {
    Subst(SubstError),
    Apply(ApplyError),
    OverFlow,
    UnwrapNone,
    UnknownShellVar(String),
    UnknownType(String),
    FailCast(Type, Type),
    NoHomeDir,
    InvalidUtf8Path,
    InvalidGlobPattern,
}
impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Subst(e) => e.fmt(f),
            Apply(e) => write!(f, "無効な演算 {e} です"),
            OverFlow => write!(f, "整数がオーバーフローしました"),
            UnwrapNone => write!(f, "noneをunwrapしました"),
            UnknownShellVar(s) => write!(f, "不明なシェル変数@{s}です"),
            UnknownType(name) => write!(f, "不明な型名{name}です"),
            FailCast(t1, t2) => {
                write!(f, "{t1}から{t2}への型変換に失敗しました")
            }
            NoHomeDir => write!(f, "ホームディレクトリの取得に失敗しました"),
            InvalidUtf8Path => write!(f, "パスがUTF-8ではありません"),
            InvalidGlobPattern => write!(f, "不正なglobパターンです"),
        }
    }
}
impl From<SubstError> for Error {
    fn from(value: SubstError) -> Self {
        Error::Subst(value)
    }
}
impl From<ApplyError> for Error {
    fn from(value: ApplyError) -> Self {
        Error::Apply(value)
    }
}
#[derive(Debug)]
pub enum ApplyError {
    Prefix(ExprPrefix, Type),
    Infix(Type, ExprInfix, Type),
    Postfix(Type, ExprPostfix),
    Index(Type, Type),
    Cast(Type, Type),
}
impl std::error::Error for ApplyError {}
impl Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ApplyError::*;
        match self {
            Prefix(prefix, t) => write!(f, "{prefix} {t}"),
            Infix(t1, infix, t2) => write!(f, "{t1} {infix} {t2}"),
            Postfix(t, postfix) => write!(f, "{t} {postfix}"),
            Index(v, i) => write!(f, "{v}[{i}]"),
            Cast(a, b) => write!(f, "{a} as {b}"),
        }
    }
}

pub fn eval_command_part(
    Spanned {
        span,
        inner: command_part,
    }: &Spanned<CommandPart>,
    env: &mut Context,
) -> SpannedResult<Value> {
    Ok(match command_part {
        CommandPart::Unquoted(string) => {
            tilde_expand(string).with_span(span)?.into()
        }
        CommandPart::SimpleExpr(expr) => eval_expr(expr, env)?,
    })
}
pub fn eval_expr(expr: &Expr, env: &mut Context) -> SpannedResult<Value> {
    use Expr::*;
    Ok(match expr {
        Primary(primary) => eval_primary(primary, env)?,
        Prefix(
            Spanned {
                span,
                inner: prefix,
            },
            expr,
        ) => {
            let value = eval_expr(expr, env)?;
            apply_prefix(value, prefix).with_span(span)?
        }
        Infix(expr1, Spanned { span, inner: infix }, expr2) => {
            use ExprInfix::*;
            if matches!(infix, And | Or | UnwrapOr) {
                eval_infix_lazy(span, expr1, expr2, infix, env)?
            }
            else {
                let value1 = eval_expr(expr1, env)?;
                let value2 = eval_expr(expr2, env)?;
                apply_infix(value1, value2, infix).with_span(span)?
            }
        }
        Postfix(
            expr,
            Spanned {
                span,
                inner: postfix,
            },
        ) => {
            let value = eval_expr(expr, env)?;
            apply_postfix(value, postfix).with_span(span)?
        }
        Index(expr_v, expr_i) => {
            let Spanned {
                span,
                inner: expr_i,
            } = expr_i.as_ref();
            let value = eval_expr(expr_v, env)?;
            let index = eval_expr(expr_i, env)?;
            apply_index(value, index).with_span(span)?
        }
        Cast(expr, span, ast_type) => {
            let value = eval_expr(expr, env)?;
            let r#type = eval_ast_type(ast_type)?;
            apply_cast(value, r#type).with_span(span)?
        }
    })
}
fn eval_primary(
    Spanned {
        span,
        inner: primary,
    }: &Spanned<Primary>,
    env: &mut Context,
) -> SpannedResult<Value> {
    use Primary::*;
    Ok(match primary {
        String(str) => str.clone().into(),
        PathString(str) => resolve_path_string(str, env).with_span(span)?,
        SpecialVar(special_var) => {
            resolve_special_var(special_var, env).with_span(span)?
        }
        EnvVar(env_var) => std::env::var(env_var).ok().into(),
        ShellVar(shell_var) => env
            .shell_vars
            .get(shell_var)
            .ok_or(Error::UnknownShellVar(shell_var.to_string()))
            .with_span(span)?
            .clone(),
        Paren(expr) => eval_expr(expr, env)?,
        Array(array) => array
            .iter()
            .map(|expr| eval_expr(expr, env))
            .collect::<SpannedResult<Vec<_>>>()?
            .into(),
        Bool(bool) => (*bool).into(),
        Int(int) => (*int).into(),
        Float(float) => (*float).into(),
        Option(option) => option
            .as_ref()
            .map(|expr| eval_expr(expr, env))
            .transpose()?
            .into(),
        Unit => ().into(),
        CommandSubst(shell_command) => {
            use super::subst::{SubstPayload, execute_substitution};

            let payload = SubstPayload {
                command: shell_command.clone().into_inner(),
                context: env.clone(),
            };
            execute_substitution(&payload, env).with_span(span)?.into()
        }
    })
}
fn eval_infix_lazy(
    span: &Span,
    expr1: &Expr,
    expr2: &Expr,
    infix: &ExprInfix,
    env: &mut Context,
) -> SpannedResult<Value> {
    let value1 = eval_expr(expr1, env)?;
    use ExprInfix::*;
    use Value::*;
    Ok(match (value1, infix) {
        (Bool(false), And) => false.into(),
        (Bool(true), And) => eval_expr(expr2, env)?,
        (Bool(true), Or) => true.into(),
        (Bool(false), Or) => eval_expr(expr2, env)?,
        (Option(Some(v)), UnwrapOr) => *v,
        (Option(None), UnwrapOr) => eval_expr(expr2, env)?,
        (value1, infix) => {
            let value2 = eval_expr(expr2, env)?;
            return Err(Error::Apply(ApplyError::Infix(
                value1.get_type(),
                infix.clone(),
                value2.get_type(),
            )))
            .with_span(span);
        }
    })
}
fn eval_ast_type(
    Spanned {
        span,
        inner: ast_type,
    }: &Spanned<AstType>,
) -> SpannedResult<Type> {
    use AstType::*;
    Ok(match ast_type {
        Unknown => Type::Unknown,
        Normal(name) => match name.as_str() {
            "string" => Type::String,
            "int" => Type::Int,
            "float" => Type::Float,
            "bool" => Type::Bool,
            "unit" => Type::Unit,
            _ => return Err(Error::UnknownType(name.clone())).with_span(span),
        },
        Generics(name, t) => match name.as_str() {
            "array" => Type::Array(Box::new(eval_ast_type(t)?)),
            "option" => Type::Option(Box::new(eval_ast_type(t)?)),
            _ => return Err(Error::UnknownType(name.clone())).with_span(span),
        },
    })
}

/*

match_op { 値;
    マクロ1: {
        基本 { 派生1, 派生2, ... }
        基本 { 派生1, 派生2, ... }
    }
    マクロ2: {
        基本 { 派生1, 派生2, ... }
        基本 { 派生1, 派生2, ... }
    }
    ...
    extre:
    以降普通のmatchアーム
}

派生は 名前 要素 の2つ

各マクロと基本と派生の組合せに対して
マクロ(pat 基本 名前 要素) => マクロ(body 基本 名前 要素)
というアームが追加される。

*/
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

fn apply_prefix(value: Value, prefix: &ExprPrefix) -> Result<Value> {
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
        (value, prefix) => {
            return Err(ApplyError::Prefix(
                prefix.clone(),
                value.get_type(),
            )
            .into());
        }
    }
}
fn apply_infix(
    value1: Value,
    value2: Value,
    infix: &ExprInfix,
) -> Result<Value> {
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
            [ Bool(a, b)     ] { Equal ==, NotEqual != }
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
        (value1, value2, infix) => {
            return Err(ApplyError::Infix(
                value1.get_type(),
                infix.clone(),
                value2.get_type(),
            )
            .into());
        }
    }
}
fn apply_postfix(value: Value, postfix: &ExprPostfix) -> Result<Value> {
    use ExprPostfix::*;
    use Value::*;
    fn usize_into_value(i: usize) -> Result<Value> {
        Ok(Value::Int(i64::try_from(i).map_err(|_| Error::OverFlow)?))
    }
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
        (String(s), Length) => usize_into_value(s.chars().count())?,
        (Array(v), Length) => usize_into_value(v.len())?,
        (value, postfix) => {
            return Err(
                ApplyError::Postfix(value.get_type(), postfix.clone()).into()
            );
        }
    })
}

fn resolve_special_var(
    special_var: &SpecialVar,
    env: &mut Context,
) -> Result<Value> {
    use SpecialVar::*;
    Ok(match special_var {
        ExitStatus => env.last_status.into(),
        Pid => std::process::id().into(),
        BackgroundPid => env.last_pid.into(),
        ShellName => env.shell_name.clone().into(),
    })
}
fn apply_index(value: Value, index: Value) -> Result<Value> {
    use Value::*;
    if let Array(v) = &value
        && let Int(index) = index
    {
        let index = if index >= 0 {
            // 正
            usize::try_from(index).ok()
        }
        else {
            // 負
            if let Ok(index) = isize::try_from(index) {
                v.len().checked_sub_signed(index)
            }
            else {
                None
            }
        };
        Ok(index.and_then(|index| v.get(index).cloned()).into())
    }
    else {
        Err(ApplyError::Index(value.get_type(), index.get_type()).into())
    }
}
fn apply_cast(value: Value, r#type: Type) -> Result<Value> {
    use Error::FailCast;
    use Value::*;
    Ok(match (value, r#type) {
        (String(s), Type::String) => s.clone().into(),
        (String(s), Type::Int) => s
            .parse::<i64>()
            .map_err(|_| FailCast(Type::String, Type::Int))?
            .into(),
        (String(s), Type::Float) => s
            .parse::<f64>()
            .map_err(|_| FailCast(Type::String, Type::Float))?
            .into(),
        (String(s), Type::Bool) => match s.as_str() {
            "true" => true,
            "false" => false,
            _ => return Err(FailCast(Type::String, Type::Bool)),
        }
        .into(),
        (Int(a), Type::String) => a.to_string().into(),
        (Int(a), Type::Int) => a.into(),
        (Int(a), Type::Float) => (a as f64).into(),
        (Float(a), Type::String) => a.to_string().into(),
        (Float(a), Type::Int) => {
            if a.is_finite() {
                (a as i64).into()
            }
            else {
                return Err(FailCast(Type::Float, Type::Int));
            }
        }
        (Float(a), Type::Float) => a.into(),
        (Bool(a), Type::String) => a.to_string().into(),
        (Bool(a), Type::Bool) => a.into(),
        (Array(v), Type::Array(t)) => v
            .into_iter()
            .map(|v| apply_cast(v, *t.clone()))
            .collect::<Result<Vec<_>>>()?
            .into(),
        (Option(o), Type::Option(t)) => {
            o.map(|v| apply_cast(*v, *t)).transpose()?.into()
        }
        (Unit, Type::Unit) => ().into(),
        (a, b) => return Err(ApplyError::Cast(a.get_type(), b).into()),
    })
}
fn resolve_path_string(path_string: &str, _env: &mut Context) -> Result<Value> {
    if path_string.is_empty() {
        return Ok(Vec::<String>::new().into());
    }
    let path = tilde_expand(path_string)?;
    let path = PathBuf::from(path);
    let components = path.components().collect::<Vec<_>>();
    if path.is_relative() {
        let mut r = glob_expand(".", &components)?;
        for path in &mut r {
            if let Some(true_path) = path.strip_prefix(
                (String::from(".") + std::path::MAIN_SEPARATOR_STR).as_str(),
            ) {
                *path = true_path.to_string();
            }
        }
        Ok(r.into())
    }
    else {
        Ok(glob_expand(".", &components)?.into())
    }
}

fn tilde_expand(path: &str) -> Result<String> {
    // チルダを確認
    if let Some(path) = path.strip_prefix('~') {
        // ユーザー名部分を切り出す
        let (user_name, path) = path.split_at(
            path.find(std::path::MAIN_SEPARATOR).unwrap_or(path.len()),
        );

        // ユーザー指定なし
        if user_name.is_empty() {
            // ホームディレクトリの取得
            let Some(home_dir) = dirs::home_dir()
            else {
                return Err(Error::NoHomeDir);
            };
            let Ok(home_dir) = home_dir.into_os_string().into_string()
            else {
                return Err(Error::InvalidUtf8Path);
            };
            return Ok(home_dir + path);
        }
        // ユーザー指定あり
        else {
            // 未実装
            // チルダを戻して返す
            return Ok(String::from("~") + user_name + path);
        }
    }
    Ok(path.to_string())
}

fn glob_expand<P: AsRef<Path>>(
    base: P,
    components: &[Component],
) -> Result<Vec<String>> {
    let base = base.as_ref();
    // 次のコンポーネントを取得
    let Some(component) = components.first()
    else {
        // 無いならマッチしたとして返却
        let match_path =
            base.to_str().ok_or(Error::InvalidUtf8Path)?.to_string();
        return Ok(vec![match_path]);
    };

    // もしNormal以外ならBaseに追加して再帰
    let Component::Normal(pattern) = component
    else {
        return glob_expand(base.join(component), &components[1..]);
    };
    let Some(pattern) = pattern.to_str()
    else {
        return Err(Error::InvalidUtf8Path);
    };

    // ファイル or フォルダの列挙
    let Ok(dirs) = base.read_dir()
    else {
        return Ok(Vec::new());
    };
    let mut r = vec![];
    let match_dot_file = pattern.starts_with('.');
    let pattern = pattern.chars().collect::<Vec<_>>();
    for entry in dirs {
        let Ok(entry) = entry
        else {
            continue;
        };
        let file_name = entry.file_name();
        let Some(target) = file_name.to_str()
        else {
            continue;
        };
        if target.starts_with('.') && !match_dot_file {
            continue;
        }
        let target = target.chars().collect::<Vec<_>>();
        if glob_match(&pattern, &target)? {
            r.append(&mut glob_expand(entry.path(), &components[1..])?);
        }
    }
    r.sort();
    Ok(r)
}
fn glob_match(pattern: &[char], target: &[char]) -> Result<bool> {
    Ok(if let Some(pattern_first) = pattern.first() {
        match pattern_first {
            '?' => {
                !target.is_empty() && glob_match(&pattern[1..], &target[1..])?
            }
            '*' => {
                // 次も * なら計算省略のため枝切り
                if pattern.get(1).is_some_and(|c| *c == '*') {
                    glob_match(&pattern[1..], target)?
                }
                else {
                    for skip_len in 0..=target.len() {
                        if glob_match(&pattern[1..], &target[skip_len..])? {
                            return Ok(true);
                        }
                    }
                    false
                }
            }
            '[' => {
                if let Some(pos) = pattern.iter().position(|c| *c == ']') {
                    let set = &pattern[1..pos];
                    if let Some(tc) = target.first() {
                        glob_match_class(set, *tc)?
                            && glob_match(&pattern[pos + 1..], &target[1..])?
                    }
                    else {
                        false
                    }
                }
                else {
                    // 閉じられていないならエラー
                    return Err(Error::InvalidGlobPattern);
                }
            }
            pc => {
                target.first().is_some_and(|tc| tc == pc)
                    && glob_match(&pattern[1..], &target[1..])?
            }
        }
    }
    else {
        target.is_empty()
    })
}
fn glob_match_class(set: &[char], target: char) -> Result<bool> {
    // ! があるなら反転
    let (exclusion_flag, set) = if set.first().is_some_and(|c| *c == '!') {
        (true, &set[1..])
    }
    else {
        (false, set)
    };
    // 範囲を分解
    enum MatchSet {
        Normal(char),
        Range(char, char),
    }
    let mut iter = set.iter().peekable();
    let mut set = vec![];
    while let Some(&c) = iter.next() {
        // 範囲の左側っぽそう
        if iter.peek().is_some_and(|c| **c == '-') {
            let _ = iter.next();
            // その次があれば範囲
            if let Some(&r) = iter.next() {
                if c > r {
                    return Err(Error::InvalidGlobPattern);
                }
                set.push(MatchSet::Range(c, r));
            }
            // 無ければ個別
            else {
                set.push(MatchSet::Normal('-'));
                set.push(MatchSet::Normal(c));
            }
        }
        else {
            set.push(MatchSet::Normal(c));
        }
    }

    // マッチ確認
    let match_flag = set.iter().any(|pattern| match pattern {
        MatchSet::Normal(c) => *c == target,
        MatchSet::Range(l, r) => (*l..=*r).contains(&target),
    });

    // 除外を考慮して返す
    Ok(exclusion_flag ^ match_flag)
}
