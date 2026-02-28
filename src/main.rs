mod check;
mod cli;
mod parse;
mod runtime;

use cli::*;

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use clap::Parser;

static CURRENT_EXE: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::current_exe().expect("自身のファイルパスの取得に失敗しました")
});

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Subst { payload_path }) => {
            run_subst_mode(payload_path);
        }
        None => {
            run_shell_mode()?;
        }
    }
    Ok(())
}

fn run_subst_mode<P: AsRef<Path>>(payload_path: P) -> ! {
    let mut payload = match runtime::subst::read_payload(&payload_path) {
        Ok(payload) => payload,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    };
    if let Err(e) = runtime::exec::execute_command_line(
        &payload.command,
        &mut payload.context,
    ) {
        eprintln!("{e}");
    }
    std::process::exit(payload.context.last_status)
}

fn run_shell_mode() -> anyhow::Result<()> {
    welcome();

    let mut shell = runtime::Shell::new();

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
                if let Err(e) = shell.execute(&command) {
                    if let Some(code) = e.is_exit() {
                        std::process::exit(code);
                    }
                    else {
                        eprintln!("コマンドの実行に失敗しました : {e}");
                    }
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
