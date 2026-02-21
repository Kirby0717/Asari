mod builtin;
mod check;
mod eval;
mod exec;
mod parse;
mod value;

fn main() -> anyhow::Result<()> {
    welcome();

    let mut shell = exec::Shell::new();

    loop {
        continuation(&std::env::current_dir()?);

        let stdin = std::io::stdin();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(_len) => {
                // 解析
                let line = line.trim_end_matches(['\n', '\r']);
                let parsed = parse::parse_shell_command(line);
                let command = match parsed {
                    Ok(command) => command,
                    Err(e) => {
                        let display = e.inner().display(e.input());
                        eprintln!("{display}");
                        continue;
                    }
                };

                println!("parse_result:");
                println!("{command:?}");

                // 検証
                let mut errors = vec![];
                check::check_shell_command(&command, &mut errors);
                if !errors.is_empty() {
                    for error in errors {
                        eprintln!("{error:?}");
                    }
                    continue;
                }
                continue;

                // 実行
                match shell.execute(&command) {
                    Err(eval::ExecuteError::Exit(code)) => {
                        std::process::exit(code)
                    }
                    Err(e) => eprintln!("コマンドの実行に失敗しました : {e}"),
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("入力の取得に失敗しました : {e}");
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
    std::io::stdout()
        .flush()
        .expect("stdoutのフラッシュに失敗しました");
}

fn format_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir()
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
