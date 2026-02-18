#![allow(unused)]
use crate::value::Value;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum Error {
    CommandNotFound,
    Exit(i32),
    InvalidArgs,
    Runtime(String),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
type Result<T> = ::std::result::Result<T, Error>;

pub fn run(name: &str, args: &[Value]) -> Result<i32> {
    match name {
        "echo" => echo(args),
        "cd" => cd(args),
        "exit" => exit(args),
        "mkdir" => mkdir(args),
        _ => Err(Error::CommandNotFound),
    }
}
fn echo(args: &[Value]) -> Result<i32> {
    let args = args.iter().flat_map(Value::to_args).collect::<Vec<_>>();
    if !args.is_empty() {
        println!("{}", args.join(" "));
    }
    Ok(0)
}
fn cd(args: &[Value]) -> Result<i32> {
    if 1 < args.len() {
        return Err(Error::InvalidArgs);
    }

    if let Some(dir) = args.first() {
        let Value::String(dir) = dir
        else {
            return Err(Error::InvalidArgs);
        };
        let next_dir = std::env::current_dir()
            .map_err(|_| {
                Error::Runtime(
                    "現在のディレクトリが見つかりませんでした".to_string(),
                )
            })?
            .join(dir);
        if next_dir.exists() && next_dir.is_dir() {
            std::env::set_current_dir(next_dir).map_err(|_| {
                Error::Runtime("ディレクトリの移動に失敗しました".to_string())
            })?;
        }
        else {
            return Err(Error::InvalidArgs);
        }
    }
    else {
        let home_dir = dirs::home_dir().ok_or(Error::Runtime(
            "ホームディレクトリの取得に失敗しました".to_string(),
        ))?;
        std::env::set_current_dir(home_dir).map_err(|_| {
            Error::Runtime("ディレクトリの移動に失敗しました".to_string())
        })?;
    }
    Ok(0)
}
// 終了優先
fn exit(args: &[Value]) -> Result<i32> {
    if 1 < args.len() {
        eprintln!("引数が2つ以上です");
    }

    let mut exit_code = 0;
    if let Some(arg) = args.first() {
        if let Value::Int(arg) = arg {
            if let Ok(code) = i32::try_from(*arg) {
                exit_code = code;
            }
            else {
                eprintln!("引数が終了コードの範囲外です");
                exit_code = -1;
            }
        }
        else {
            eprintln!("引数が整数ではありません");
            exit_code = -1;
        }
    }
    Err(Error::Exit(exit_code))
}
fn mkdir(args: &[Value]) -> Result<i32> {
    if args.is_empty() {
        return Err(Error::InvalidArgs);
    }

    let mut exit_status = 0;
    for dir in args.iter().flat_map(Value::to_args) {
        match std::fs::create_dir_all(dir) {
            Ok(_) => {}
            Err(e) => {
                exit_status = 1;
                eprintln!("{e}");
            }
        }
    }
    Ok(exit_status)
}
