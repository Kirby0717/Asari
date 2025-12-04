#![allow(unused)]

#[derive(Clone, Debug)]
pub enum Error {
    CommandNotFound,
    Exit(i32),
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
        .get(0)
        .map(|code| code.parse::<i32>().ok())
        .flatten()
        .unwrap_or(0);
    Err(Error::Exit(code))
}
fn mkdir(args: &[String]) -> Result<i32> {
    todo!()
}
