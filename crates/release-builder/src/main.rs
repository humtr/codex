#[cfg(unix)]
fn main() {
    std::process::exit(codex_release_builder::run_from_args(
        std::env::args_os().skip(1),
    ));
}

#[cfg(not(unix))]
fn main() {
    eprintln!("codex-release-builder: this tool requires a Unix release environment");
    std::process::exit(1);
}
