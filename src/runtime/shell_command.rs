use super::value::Value;

use std::fmt::Display;
use std::io::Error as IoError;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    Exit(i32),
    InvalidArgs,
    Stdio(std::io::Error),
    Other(String),
}
impl From<IoError> for Error {
    fn from(value: IoError) -> Self {
        Error::Stdio(value)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ShellCommandKind {
    Cd,
    Exit,
}
pub fn find_shell_command(name: &str) -> Option<ShellCommandKind> {
    use ShellCommandKind::*;
    Some(match name {
        "cd" => Cd,
        "exit" => Exit,
        _ => None?,
    })
}
pub fn run(command: &ShellCommandKind, args: &[Value]) -> Result<i32> {
    use ShellCommandKind::*;
    let result = match command {
        Cd => cd(args),
        Exit => exit(args),
    };
    if let Err(Error::Stdio(e)) = &result
        && e.kind() == std::io::ErrorKind::BrokenPipe
    {
        return Ok(0);
    }
    result
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
fn exit(args: &[Value]) -> Result<i32> {
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
