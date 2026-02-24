mod check;
mod cli;
mod eval;
mod exec;
mod parse;
mod payload;
mod shell_command;
mod value;

use cli::*;

use std::path::Path;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Subst { payload_path }) => {
            run_subst_mode(payload_path)?;
        }
        None => {
            run_shell_mode()?;
        }
    }
    Ok(())
}

fn run_subst_mode<P: AsRef<Path>>(payload_path: P) -> anyhow::Result<()> {
    let mut payload = payload::read_payload(&payload_path)?;
    exec::execute_command_line(&payload.command, &mut payload.context)?;
    Ok(())
}

fn run_shell_mode() -> anyhow::Result<()> {
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

                // 検証
                let mut errors = vec![];
                check::check_command_line(&command, &mut errors);
                if !errors.is_empty() {
                    for error in errors {
                        eprintln!("{error:?}");
                    }
                    continue;
                }

                // 実行
                match shell.execute(&command) {
                    Err(exec::Error::Exit(code)) => std::process::exit(code),
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
