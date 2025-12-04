#![allow(unused)]
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

pub fn run(name: &str, args: &[String]) -> Result<i32> {
    match name {
        "cd" => cd(args),
        "exit" => exit(args),
        "mkdir" => mkdir(args),
        _ => Err(Error::CommandNotFound),
    }
}
fn cd(args: &[String]) -> Result<i32> {
    if 1 < args.len() {
        return Err(Error::InvalidArgs);
    }

    let current_dir = std::env::current_dir().map_err(|_| {
        Error::Runtime("現在のディレクトリが見つかりませんでした".to_string())
    })?;
    if let Some(dir) = args.first() {
        let next_dir = current_dir.join(dir);
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
fn exit(args: &[String]) -> Result<i32> {
    if 1 < args.len() {
        return Err(Error::InvalidArgs);
    }

    let code = args
        .first()
        .map(|code| code.parse::<i32>().map_err(|_| Error::InvalidArgs))
        .transpose()?
        .unwrap_or(0);
    Err(Error::Exit(code))
}
fn mkdir(args: &[String]) -> Result<i32> {
    if args.is_empty() {
        return Err(Error::InvalidArgs);
    }

    for dir in args {
        use std::io::ErrorKind;
        match std::fs::create_dir_all(dir) {
            Ok(_) => {}
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(0)
}
