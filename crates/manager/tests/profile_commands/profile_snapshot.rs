use super::*;
use serde_json::{json, Value};

pub(super) fn inventory(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut result = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            } else {
                result.push((
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                ));
            }
        }
    }
    result.sort();
    result
}

#[test]
fn profile_snapshot_tracks_registered_saved_inherited_and_external_without_writes() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let home = root.0.join("home");
    fs::create_dir(&home).unwrap();
    let empty = run_manager(&home, &core, &["__profile-snapshot-v1"], None);
    assert_eq!(empty.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&empty.stdout).unwrap(),
        json!({
            "schema":"codex-manager-profiles-v1", "profiles":["default"],
            "current":"default", "current_source":"default", "saved_default":"default"
        })
    );
    assert_eq!(inventory(&home), Vec::new());
    for id in ["work", "external"] {
        assert_eq!(
            run_manager(&home, &core, &["profile", "create", id], None)
                .status
                .code(),
            Some(0)
        );
    }
    assert_eq!(
        run_manager(&home, &core, &["profile", "default", "work"], None)
            .status
            .code(),
        Some(0)
    );
    let work = home.join(".local/share/codex/manager/profiles/work/home");
    let external = root.0.join("external-home");
    fs::create_dir(&external).unwrap();
    for (inherited, selected, source) in [
        (None, Some("work"), "saved"),
        (Some(work.as_path()), Some("work"), "inherited"),
        (
            Some(home.join(".codex").as_path()),
            Some("default"),
            "inherited",
        ),
        (Some(external.as_path()), None, "inherited"),
        (Some(Path::new("")), Some("work"), "saved"),
    ] {
        let before = inventory(&home);
        let output = run_manager(&home, &core, &["__profile-snapshot-v1"], inherited);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            json!({
                "schema":"codex-manager-profiles-v1", "profiles":["default","external","work"],
                "current":selected, "current_source":source, "saved_default":"work"
            })
        );
        assert!(output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stdout).contains(home.to_str().unwrap()));
        assert_eq!(inventory(&home), before);
        let public = run_manager(&home, &core, &["profile", "current"], inherited);
        assert_eq!(
            public.stdout,
            format!(
                "current: {}\nsource: {source}\n",
                selected.unwrap_or("external")
            )
            .as_bytes()
        );
    }
}

#[test]
fn profile_snapshot_rejects_bad_arguments_handoff_and_default_without_writes() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let home = root.0.join("home");
    fs::create_dir(&home).unwrap();
    let before = inventory(&home);
    let extra = run_manager(&home, &core, &["__profile-snapshot-v1", "extra"], None);
    assert_eq!(extra.status.code(), Some(2));
    assert!(extra.stdout.is_empty());
    let unbound = Command::new(manager_binary())
        .env_clear()
        .env("HOME", &home)
        .arg("__profile-snapshot-v1")
        .output()
        .unwrap();
    assert_eq!(unbound.status.code(), Some(1));
    assert!(unbound.stdout.is_empty());
    assert_eq!(inventory(&home), before);
    assert_eq!(
        run_manager(&home, &core, &["profile", "default", "default"], None)
            .status
            .code(),
        Some(0)
    );
    let preference = home.join(".local/share/codex/manager/default-profile-v1");
    fs::write(preference, b"bad record\n").unwrap();
    let before = inventory(&home);
    let malformed = run_manager(&home, &core, &["__profile-snapshot-v1"], None);
    assert_eq!(malformed.status.code(), Some(1));
    assert!(malformed.stdout.is_empty());
    assert_eq!(inventory(&home), before);
}

#[test]
fn profile_snapshot_bounds_inventory_without_restricting_public_list() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let home = root.0.join("home");
    fs::create_dir(&home).unwrap();
    for i in 0..256 {
        assert_eq!(
            run_manager(
                &home,
                &core,
                &["profile", "create", &format!("owned-{i:03}")],
                None
            )
            .status
            .code(),
            Some(0)
        );
    }
    let before = inventory(&home);
    let snapshot = run_manager(&home, &core, &["__profile-snapshot-v1"], None);
    assert_eq!(snapshot.status.code(), Some(1));
    assert!(snapshot.stdout.is_empty());
    let list = run_manager(&home, &core, &["profile", "list"], None);
    assert_eq!(list.status.code(), Some(0));
    assert_eq!(
        list.stdout
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .count(),
        257
    );
    assert_eq!(inventory(&home), before);
}
