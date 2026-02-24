use std::path::PathBuf;

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

    let mut files = dir
        .read_dir()
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|file_name| !file_name.starts_with('.'))
        .collect::<Vec<_>>();
    files.sort();
    println!("{}", files.join("\n"));
}
