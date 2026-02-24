use std::{
    env::args,
    io::{Write, stdout},
};

fn main() {
    let result =
        writeln!(stdout(), "{}", args().skip(1).collect::<Vec<_>>().join(" "));
    if let Err(e) = result
        && e.kind() != std::io::ErrorKind::BrokenPipe
    {
        eprintln!("{e}");
    }
}
