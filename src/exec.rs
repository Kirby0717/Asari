#![allow(unused)]
use crate::parse::ShellCommand;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum Error {
    Exit(i32),
    CommandError(String),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, Default)]
pub struct Shell {
    last_status: Option<i32>,
}
impl Shell {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn execute(&mut self, cmd: &ShellCommand) -> Result<()> {
        use crate::builtin::Error as BuiltinError;
        use crate::parse::Word;
        for (command, _pipe) in &cmd.commands {
            let name = command.name.to_string();
            let args: Vec<_> =
                command.args.iter().map(Word::to_string).collect();

            // ビルトインの実行を試す
            match crate::builtin::run(&name, &args) {
                Ok(_) => {
                    continue;
                }
                Err(BuiltinError::Exit(code)) => return Err(Error::Exit(code)),
                Err(BuiltinError::CommandNotFound) => {}
                Err(e) => return Err(Error::CommandError(e.to_string())),
            }

            // 外部コマンドの実行を試す
            let status =
                match std::process::Command::new(name).args(args).status() {
                    Ok(status) => status,
                    Err(e) => return Err(Error::CommandError(e.to_string())),
                };
        }
        Ok(())
    }
}
