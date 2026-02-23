use std::io::Write;

fn main() {
    if let Err(e) = writeln!(
        std::io::stdout(),
        "{}",
        std::env::args().skip(1).collect::<Vec<_>>().join(" ")
    ) {
        eprintln!("{e}");
    }
}
