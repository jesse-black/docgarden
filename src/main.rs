fn main() {
    if let Err(error) = docgarden::run() {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}
