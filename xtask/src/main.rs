fn main() {
    std::process::exit(xtask::run(std::env::args_os().skip(1)));
}
