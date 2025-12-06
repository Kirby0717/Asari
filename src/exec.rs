#![allow(unused)]
use crate::parse::ShellCommand;
use std::{ffi::OsString, fmt::Display, path::PathBuf};

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
                command.args.iter().map(ToString::to_string).collect();

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
                    "コマンドが見つかりませんでした".to_string(),
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

    // 探索する拡張子を取得
    let extensions = name
        .extension()
        .map(|ext| vec![ext.to_os_string()])
        .unwrap_or_else(get_pathext);
    // 探索するパスを取得
    let search_dirs = name
        .parent()
        .and_then(|parent| {
            if parent.as_os_str().is_empty() {
                None
            }
            else {
                Some(vec![parent.to_owned()])
            }
        })
        .unwrap_or_else(get_path);

    let file_name = name.file_stem()?;
    for dir in search_dirs {
        for ext in &extensions {
            let candidate = dir.join(file_name).with_extension(ext);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
fn get_pathext() -> Vec<OsString> {
    // var_osを使用するとより正確
    std::env::var("PATHEXT")
        .unwrap_or(".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .map(|s| OsString::from(s.trim_start_matches('.')))
        .collect()
}
fn get_path() -> Vec<PathBuf> {
    // var_osを使用するとより正確
    std::env::var("PATH")
        .unwrap_or_default()
        .split(';')
        .map(PathBuf::from)
        .collect()
}
