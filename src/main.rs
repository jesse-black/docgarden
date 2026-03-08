fn main() {
    if let Err(error) = dglint::run() {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}
