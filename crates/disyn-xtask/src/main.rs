use std::process::Command;

fn run(label: &str, cmd: &str, args: &[&str]) {
    println!("--- {label} ---");
    let status = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("{label} failed to start: {e}"));
    if !status.success() {
        eprintln!("{label} FAILED");
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("ci");

    match cmd {
        "ci" => {
            run("fmt", "cargo", &["fmt", "--all", "--check"]);
            run(
                "clippy",
                "cargo",
                &[
                    "clippy", "--workspace", "--all-targets", "--", "-D",
                    "warnings",
                ],
            );
            run("test", "cargo", &["test", "--workspace"]);
            run("build", "cargo", &["build", "--workspace"]);
            println!("--- all gates passed ---");
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: cargo xtask ci");
            std::process::exit(1);
        }
    }
}
