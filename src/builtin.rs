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
        _ => Err(Error::CommandNotFound),
    }
}
fn cd(args: &[String]) -> Result<i32> {
    todo!()
}
fn exit(args: &[String]) -> Result<i32> {
    let code = args
        .first()
        .map(|code| code.parse::<i32>().map_err(|_| Error::InvalidArgs))
        .transpose()?
        .unwrap_or(0);
    Err(Error::Exit(code))
}
fn mkdir(args: &[String]) -> Result<i32> {
    todo!()
}
