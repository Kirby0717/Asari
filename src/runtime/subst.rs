use super::{Context, status_into_i32};
use crate::parse::CommandLine;

use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;

#[derive(Serialize, Deserialize)]
pub struct SubstPayload {
    pub command: CommandLine,
    pub context: Context,
}

type Result<T> = ::std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    FailExecuteItself(IoError),
    SaveTempFile(IoError),
    OpenTempFile(IoError),
    FailDeserialize(JsonError),
    FailSerialize(JsonError),
    OutputNotUtf8,
}
impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            FailExecuteItself(e) => {
                write!(f, "自分自身を実行するのに失敗しました : {e}")
            }
            SaveTempFile(e) => {
                write!(f, "一時ファイルの保存に失敗しました : {e}")
            }
            OpenTempFile(e) => {
                write!(f, "一時ファイルの取得に失敗しました : {e}")
            }
            FailSerialize(e) => {
                write!(f, "ASTのJSONへの変換に失敗しました : {e}")
            }
            FailDeserialize(e) => {
                write!(f, "ASTのJSONの解析に失敗しました : {e}")
            }
            OutputNotUtf8 => {
                write!(f, "コマンドの出力がUTF-8ではありません")
            }
        }
    }
}

pub struct TempFile(PathBuf);
impl AsRef<std::ffi::OsStr> for TempFile {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_os_str()
    }
}
impl AsRef<Path> for TempFile {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn execute_substitution(
    payload: &SubstPayload,
    env: &mut Context,
) -> Result<String> {
    use std::process::Stdio;

    let file = write_payload(payload)?;
    let output = std::process::Command::new(crate::CURRENT_EXE.as_path())
        .arg("subst")
        .arg(&file)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .map_err(Error::FailExecuteItself)?;
    env.last_status = status_into_i32(output.status);
    Ok(std::string::String::from_utf8(output.stdout)
        .map_err(|_| Error::OutputNotUtf8)?
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

/// 一時ファイルへASTとコンテキストを保存
pub fn write_payload(payload: &SubstPayload) -> Result<TempFile> {
    let temp_dir = std::env::temp_dir();

    // 適当に被らなさそうな名前を被らなくなるまで作る
    let path = loop {
        let name = format!(
            "asari_subst_{:X}_{:X}.txt",
            // プロセスID
            std::process::id(),
            // 現在時刻（ナノ秒）
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos()),
        );
        let path = temp_dir.join(name);
        if !path.exists() {
            break path;
        }
    };

    let json = serde_json::to_vec(payload).map_err(Error::FailSerialize)?;
    std::fs::write(&path, json).map_err(Error::SaveTempFile)?;
    Ok(TempFile(path))
}
/// 一時ファイルからASTとコンテキストを取得
pub fn read_payload<P: AsRef<Path>>(path: P) -> Result<SubstPayload> {
    let data = std::fs::read(path).map_err(Error::OpenTempFile)?;
    let payload =
        serde_json::from_slice(&data).map_err(Error::FailDeserialize)?;
    Ok(payload)
}
