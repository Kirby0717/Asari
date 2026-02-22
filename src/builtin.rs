#![allow(unused)]
use crate::value::Value;

use std::fmt::Display;
use std::io::{Read, Write};

#[derive(Clone, Debug)]
pub enum Error {
    Exit(i32),
    InvalidArgs,
    Other(String),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug)]
pub enum BuiltinCommand {
    Echo,
    Cd,
    Exit,
    Mkdir,
}
pub fn find_command(name: &str) -> Option<BuiltinCommand> {
    use BuiltinCommand::*;
    Some(match name {
        "echo" => Echo,
        "cd" => Cd,
        "exit" => Exit,
        "mkdir" => Mkdir,
        _ => None?,
    })
}
pub fn run(
    command: BuiltinCommand,
    args: &[Value],
    stdin: Box<dyn Read>,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
) -> Result<i32> {
    use BuiltinCommand::*;
    match command {
        Echo => echo(args, stdin, stdout, stderr),
        Cd => cd(args, stdin, stdout, stderr),
        Exit => exit(args, stdin, stdout, stderr),
        Mkdir => mkdir(args, stdin, stdout, stderr),
    }
}
fn echo(
    args: &[Value],
    stdin: Box<dyn Read>,
    mut stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
) -> Result<i32> {
    let args = args.iter().flat_map(Value::to_args).collect::<Vec<_>>();
    if !args.is_empty() {
        writeln!(stdout, "{}", args.join(" "));
    }
    Ok(0)
}
fn cd(
    args: &[Value],
    stdin: Box<dyn Read>,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
) -> Result<i32> {
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
                Error::Other(
                    "現在のディレクトリが見つかりませんでした".to_string(),
                )
            })?
            .join(dir);
        if next_dir.exists() && next_dir.is_dir() {
            std::env::set_current_dir(next_dir).map_err(|_| {
                Error::Other("ディレクトリの移動に失敗しました".to_string())
            })?;
        }
        else {
            return Err(Error::InvalidArgs);
        }
    }
    else {
        let home_dir = dirs::home_dir().ok_or(Error::Other(
            "ホームディレクトリの取得に失敗しました".to_string(),
        ))?;
        std::env::set_current_dir(home_dir).map_err(|_| {
            Error::Other("ディレクトリの移動に失敗しました".to_string())
        })?;
    }
    Ok(0)
}
// 終了優先
fn exit(
    args: &[Value],
    stdin: Box<dyn Read>,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
) -> Result<i32> {
    if 1 < args.len() {
        eprintln!("引数が2つ以上です");
    }

    let mut exit_code = 0;
    if let Some(arg) = args.first() {
        match arg {
            Value::Int(arg) => {
                if let Ok(code) = i32::try_from(*arg) {
                    exit_code = code;
                }
                else {
                    eprintln!("数値が終了コードの範囲外です");
                    exit_code = -1;
                }
            }
            Value::String(arg) => {
                if let Ok(code) = arg.parse::<i32>() {
                    exit_code = code;
                }
                else {
                    eprintln!("文字列を終了コードに変換できませんでした");
                    exit_code = -1;
                }
            }
            _ => {
                eprintln!("引数が整数または文字列ではありません");
                exit_code = -1;
            }
        }
    }
    Err(Error::Exit(exit_code))
}
fn mkdir(
    args: &[Value],
    stdin: Box<dyn Read>,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
) -> Result<i32> {
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
