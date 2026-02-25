use super::eval::Context;
use crate::parse::CommandLine;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SubstPayload {
    pub command: CommandLine,
    pub context: Context,
}

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ParseError(serde_json::Error),
}
impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
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

// 一時ファイルへファイルを保存
pub fn write_payload(payload: &SubstPayload) -> std::io::Result<TempFile> {
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

    let json = serde_json::to_vec(payload).unwrap();
    std::fs::write(&path, json)?;
    Ok(TempFile(path))
}

pub fn read_payload<P: AsRef<Path>>(path: P) -> Result<SubstPayload, Error> {
    let data = std::fs::read(path).map_err(Error::IoError)?;
    serde_json::from_slice(&data).map_err(Error::ParseError)
}
