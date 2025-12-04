#![allow(unused)]
use crate::parse::ShellCommand;
use std::{fmt::Display, path::PathBuf};

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
            let Some(name) = find_executable(&name)
            else {
                return Err(Error::CommandError(
                    "fail to find command".to_string(),
                ));
            };
            let status =
                match std::process::Command::new(name).args(args).status() {
                    Ok(status) => status,
                    Err(e) => return Err(Error::CommandError(e.to_string())),
                };
        }
        Ok(())
    }
}

/// 実行可能ファイルのフルパスを探索
fn find_executable(name: &str) -> Option<PathBuf> {
    let name = PathBuf::from(name);
    // 既に拡張子があるか、パス区切りを含む場合はそのまま
    if name.extension().is_some() || name.ancestors().count() > 2 {
        return if name.exists() { Some(name) } else { None };
    }

    let pathext = get_pathext();
    let path_dirs = get_path();

    for dir in path_dirs {
        for ext in &pathext {
            let candidate =
                dir.join(&name).with_extension(ext.trim_start_matches('.'));
            println!("{candidate:?}");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
fn get_pathext() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or(".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .map(|s| s.to_string())
        .collect()
}
fn get_path() -> Vec<PathBuf> {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(';')
        .map(PathBuf::from)
        .collect()
}
