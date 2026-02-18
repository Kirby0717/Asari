use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

use super::{
    parse::{
        CommandPart, Expr, ExprInfix, ExprPostfix, ExprPrefix, Primary,
        Spanned, SpecialVar,
    },
    value::*,
};

#[derive(Clone, Debug)]
pub enum Error {
    InvalidType,
    OverFlow,
    UnwrapNone,
    UnknownShellVar,
    FailCast,
    NoHomeDir,
    InvalidUtf8Path,
    InvalidGlobPattern,
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

pub fn eval_command_part(
    command_part: &CommandPart,
    env: &Context,
) -> Result<Value> {
    Ok(match command_part {
        CommandPart::Unquoted(string) => tilde_expand(&string.inner)?.into(),
        CommandPart::SimpleExpr(expr) => eval_expr(expr, env)?,
    })
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
        _ => return Err(Error::InvalidType),
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
        _ => return Err(Error::InvalidType),
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
                return Err(Error::InvalidType);
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
        _ => return Err(Error::InvalidType),
    })
}
fn eval_primary(primary: &Spanned<Primary>, env: &Context) -> Result<Value> {
    use Primary::*;
    Ok(match &primary.inner {
        String(str) => str.clone().into(),
        PathString(str) => eval_path_string(str, env)?,
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
fn eval_path_string(path_string: &str, _env: &Context) -> Result<Value> {
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
