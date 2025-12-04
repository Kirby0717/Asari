mod builtin;
mod exec;
mod parse;

fn main() -> anyhow::Result<()> {
    welcome();

    let mut shell = exec::Shell::new();

    loop {
        continuation(&std::env::current_dir()?);

        let stdin = std::io::stdin();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(_len) => {
                use winnow::Parser;
                let line = line.trim_end_matches(['\n', '\r']).to_string();
                let parsed = parse::shell_command.parse(line.as_str());
                //println!("{parsed:?}");
                let Ok(command) = parsed
                else {
                    eprintln!("{}", parsed.unwrap_err());
                    continue;
                };

                use exec::Error;
                match shell.execute(&command) {
                    Err(Error::Exit(code)) => std::process::exit(code),
                    Err(e) => eprintln!("fail to run command: {e}"),
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("fail to get line: {e}");
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
