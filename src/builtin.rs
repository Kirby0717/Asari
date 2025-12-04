#![allow(unused)]
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum Error {
    CommandNotFound,
    Exit(i32),
    InvalidArgs,
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

    let current_dir = std::env::current_dir().unwrap();
    if let Some(dir) = args.first() {
        let next_dir = current_dir.join(dir);
        if next_dir.exists() {
            std::env::set_current_dir(next_dir);
        }
        else {
            return Err(Error::InvalidArgs);
        }
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
        std::fs::create_dir_all(dir);
    }
    Ok(0)
}
