use std::process::Command;

#[test]
fn manager_native_and_stock_window_return_paths() {
    let output = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/window_paths.py"
        ))
        .arg(env!("CARGO_BIN_EXE_codex-manager"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|s| s.starts_with("PASS "))
            .count(),
        8
    );
}
