use std::{path::PathBuf, process::exit};

fn main() {
    let current_dir = std::env::current_dir()
        .expect("現在のディレクトリの取得に失敗しました");

    let dir = if let Some(path) = std::env::args().nth(1) {
        let path = PathBuf::from(path);
        if path.is_relative() {
            current_dir.join(path)
        }
        else {
            path
        }
    }
    else {
        current_dir
    };

    let read_dir = match dir.read_dir() {
        Ok(read_dir) => read_dir,
        Err(e) => {
            eprintln!("{e}");
            exit(1)
        }
    };
    let mut files = read_dir
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|file_name| !file_name.starts_with('.'))
        .collect::<Vec<_>>();
    files.sort();
    println!("{}", files.join("\n"));
}
