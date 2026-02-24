fn main() {
    let mut exit_status = 0;
    for dir in std::env::args().skip(1) {
        match std::fs::create_dir_all(dir) {
            Ok(_) => {}
            Err(e) => {
                exit_status = 1;
                eprintln!("{e}");
            }
        }
    }
    std::process::exit(exit_status);
}
