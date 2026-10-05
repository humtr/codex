use super::profile_snapshot::inventory;
use super::*;
use serde_json::{json, Value};

#[test]
fn task_snapshot_empty_usage_and_handoff_are_exact_and_read_only() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let home = root.0.join("home");
    fs::create_dir(&home).unwrap();
    let before = inventory(&home);
    let result = run_manager(&home, &core, &["__task-snapshot-v1"], None);
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap(),
        json!({"schema":"codex-manager-tasks-v1", "tasks":[]})
    );
    assert_eq!(result.stdout.last(), Some(&b'\n'));
    assert_eq!(inventory(&home), before);
    assert_eq!(fs::read_dir(&home).unwrap().count(), 0);
    for args in [
        vec!["__task-snapshot-v1", "extra"],
        vec!["__task-snapshot-v1", "--help"],
    ] {
        let result = run_manager(&home, &core, &args, None);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert_eq!(inventory(&home), before);
    }
    let result = base_manager_command(&home, &core)
        .env_remove(CORE_API_ENV)
        .arg("__task-snapshot-v1")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert_eq!(inventory(&home), before);
}

#[test]
fn task_snapshot_rejects_unsafe_server_state_without_partial_data_or_writes() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let home = root.0.join("home");
    let servers = home.join(".local/share/codex/core/servers");
    fs::create_dir_all(&servers).unwrap();
    fs::set_permissions(&servers, fs::Permissions::from_mode(0o777)).unwrap();
    let before = inventory(&home);
    let result = run_manager(&home, &core, &["__task-snapshot-v1"], None);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert_eq!(inventory(&home), before);
    assert_eq!(
        fs::metadata(&servers).unwrap().permissions().mode() & 0o777,
        0o777
    );
}
