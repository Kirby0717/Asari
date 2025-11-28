mod parse;

fn main() -> anyhow::Result<()> {
    welcome();

    let current_dir = std::env::current_dir()?;
    loop {
        continuation(&current_dir);

        let stdin = std::io::stdin();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(_len) => {
                use winnow::Parser;
                let line = line.trim().to_string();
                let parsed = parse::command.parse(line.as_str());
                println!("{parsed:?}");
            }
            Err(e) => {
                eprintln!("err: {e}");
            }
        }
    }
}

fn welcome() {
    println!("Welcome to Asari!");
}

fn continuation(current_dir: &std::path::Path) {
    use std::io::Write;
    print!("{}>", format_path(current_dir));
    std::io::stdout().flush().expect("stdoutのフラッシュに失敗");
}

fn format_path(path: &std::path::Path) -> String {
    if let Some(home) = std::env::home_dir()
        && let Ok(relative) = path.strip_prefix(&home)
    {
        if path == home {
            "~".to_string()
        }
        else {
            format!("~/{}", relative.display())
        }
    }
    else {
        path.display().to_string()
    }
}
