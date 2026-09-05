#[cfg(unix)]
fn main() {
    std::process::exit(codex_manager::run(std::env::args_os().skip(1)));
}

#[cfg(not(unix))]
fn main() {
    eprintln!("codex-manager: unsupported platform");
    std::process::exit(2);
}
