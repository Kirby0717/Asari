use crate::parse::ShellCommand;

#[derive(Clone, Debug)]
pub enum Error {
    Exit(i32),
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
            let args: Vec<_> = command.args.iter().map(Word::to_string).collect();

            match crate::builtin::run(&name, &args) {
                Ok(_) => {}
                Err(BuiltinError::Exit(code)) => return Err(Error::Exit(code)),
                _ => todo!(),
            }
        }
        todo!()
    }
}
