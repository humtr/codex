use crate::fs_util;
use crate::json::{JsonParser, JsonValue};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
const DEFAULT_SETTINGS_BYTES: &[u8] = b"{\"updater\":{\"autoUpdateEnabled\":false}}\n";

static SETTINGS_TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn inspect_read_only(codex_home: &Path) -> io::Result<()> {
    if codex_home.as_os_str().is_empty() {
        return Ok(());
    }
    match fs::symlink_metadata(codex_home) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if !fs::metadata(codex_home)?.is_dir() {
                return Err(io::Error::other("CODEX_HOME must be a directory"));
            }
        }
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(io::Error::other("CODEX_HOME must be a directory")),
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    }
    let physical_home = fs::canonicalize(codex_home)?;
    let daemon_dir = physical_home.join("app-server-daemon");
    match fs::symlink_metadata(&daemon_dir) {
        Ok(meta) if meta.file_type().is_dir() => {}
        Ok(_) => {
            return Err(io::Error::other(
                "app-server-daemon path must be a real directory",
            ))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    }
    let settings_path = daemon_dir.join("settings.json");
    match fs::symlink_metadata(&settings_path) {
        Ok(meta) if meta.file_type().is_file() => {
            if metadata_exceeds_limit(&meta) {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            let file = File::open(&settings_path)?;
            let mut bytes = Vec::with_capacity(meta.len() as usize);
            file.take(MAX_SETTINGS_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_SETTINGS_BYTES {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            let _ = update_settings_bytes(Some(&bytes))?;
            Ok(())
        }
        Ok(_) => Err(io::Error::other(
            "app-server-daemon settings.json must be a regular file",
        )),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn metadata_exceeds_limit(meta: &fs::Metadata) -> bool {
    meta.len() > MAX_SETTINGS_BYTES as u64
}

pub(super) fn ensure(codex_home: &Path) -> io::Result<()> {
    if codex_home.as_os_str().is_empty() {
        return Ok(());
    }
    match fs::symlink_metadata(codex_home) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if !fs::metadata(codex_home)?.is_dir() {
                return Err(io::Error::other("CODEX_HOME must be a directory"));
            }
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(io::Error::other("CODEX_HOME must be a directory")),
        Err(err) if err.kind() == ErrorKind::NotFound => fs::create_dir_all(codex_home)?,
        Err(err) => return Err(err),
    }

    let physical_home = fs::canonicalize(codex_home)?;
    let daemon_dir = physical_home.join("app-server-daemon");
    match fs::symlink_metadata(&daemon_dir) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => {
            return Err(io::Error::other(
                "app-server-daemon path must be a real directory",
            ))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let mut builder = DirBuilder::new();
            builder.mode(0o700);
            match builder.create(&daemon_dir) {
                Ok(()) => {}
                Err(create_err) if create_err.kind() == ErrorKind::AlreadyExists => {
                    let meta = fs::symlink_metadata(&daemon_dir)?;
                    if !meta.file_type().is_dir() {
                        return Err(io::Error::other(
                            "app-server-daemon path must be a real directory",
                        ));
                    }
                }
                Err(create_err) => return Err(create_err),
            }
        }
        Err(err) => return Err(err),
    }

    let settings_path = daemon_dir.join("settings.json");
    let existing = match fs::symlink_metadata(&settings_path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            if metadata.len() > MAX_SETTINGS_BYTES as u64 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            let file = File::open(&settings_path)?;
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            file.take(MAX_SETTINGS_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_SETTINGS_BYTES {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            Some(bytes)
        }
        Ok(_) => {
            return Err(io::Error::other(
                "app-server-daemon settings.json must be a regular file",
            ))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };

    let Some(updated) = update_settings_bytes(existing.as_deref())? else {
        return Ok(());
    };

    let sequence = SETTINGS_TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_path = daemon_dir.join(format!(
        ".settings.json.{}.{sequence}.tmp",
        std::process::id()
    ));
    let write_result = (|| -> io::Result<()> {
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)?;
        temp.write_all(&updated)?;
        temp.sync_all()?;
        drop(temp);
        fs::rename(&temp_path, &settings_path)?;
        fs_util::sync_directory(&daemon_dir)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

const EXISTING_PACKAGE_INDICATORS: &[&str] = &[
    "current",
    "releases",
    "selection-history",
    "auto-update-version",
    "install.lock",
];

const PRIOR_DAEMON_TRACE_FILES: &[&str] = &[
    "daemon.pid",
    "daemon.stderr.log",
    "daemon-updater.pid",
    "daemon-updater.stderr.log",
    "app-server.pid",
    "app-server.stderr.log",
    "app-server-updater.pid",
    "app-server-updater.stderr.log",
];

fn current_uid() -> io::Result<u32> {
    fs_util::current_uid()
}

fn inspect_owned_real_dir(path: &Path, expected_uid: u32) -> io::Result<Option<bool>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(fs_util::is_owned_directory(&meta, expected_uid))),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn ensure_private_real_dir_no_chmod(path: &Path, expected_uid: u32) -> io::Result<bool> {
    if let Some(valid) = inspect_owned_real_dir(path, expected_uid)? {
        return Ok(valid);
    }
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            Ok(inspect_owned_real_dir(path, expected_uid)? == Some(true))
        }
        Err(err) => Err(err),
    }
}

fn may_exist(path: &Path) -> bool {
    !matches!(fs::symlink_metadata(path), Err(err) if err.kind() == ErrorKind::NotFound)
}

/// Atomically create `$CODEX_HOME/packages/app-server-daemon/current -> standalone_current`
/// on first managed-daemon initialization without overwriting any existing or partial state.
pub(super) fn ensure_default_package_link(
    codex_home: &Path,
    standalone_current: &Path,
    standalone_root: &Path,
) -> io::Result<()> {
    if codex_home.as_os_str().is_empty() {
        return Ok(());
    }
    // Zero-write, zero-fsync fast path when `current` (or broken link) is already present.
    if may_exist(&codex_home.join("packages/app-server-daemon/current")) {
        return Ok(());
    }
    if fs::symlink_metadata(standalone_current).is_err()
        || fs::metadata(standalone_current).is_err()
    {
        return Ok(());
    }

    let physical_home = match fs::canonicalize(codex_home) {
        Ok(path) => path,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if let Ok(canonical_standalone) = fs::canonicalize(standalone_root) {
        if physical_home.starts_with(&canonical_standalone) {
            return Ok(());
        }
    }

    let uid = current_uid()?;
    let packages_dir = physical_home.join("packages");
    let packages_state = inspect_owned_real_dir(&packages_dir, uid)?;
    if packages_state == Some(false) {
        return Ok(());
    }
    let pkg_daemon_dir = packages_dir.join("app-server-daemon");
    if packages_state == Some(true) {
        let daemon_pkg_state = inspect_owned_real_dir(&pkg_daemon_dir, uid)?;
        if daemon_pkg_state == Some(false) {
            return Ok(());
        }
        if daemon_pkg_state == Some(true) {
            for indicator in EXISTING_PACKAGE_INDICATORS {
                if may_exist(&pkg_daemon_dir.join(indicator)) {
                    return Ok(());
                }
            }
        }
    }

    // Upstream 0.170.0 chooses an existing dedicated current first, then modern
    // traces, then legacy traces. Leave the choice to upstream for all of them.
    let state_dir = physical_home.join("app-server-daemon");
    for trace in PRIOR_DAEMON_TRACE_FILES {
        if may_exist(&state_dir.join(trace)) {
            return Ok(());
        }
    }
    if may_exist(&physical_home.join("app-server-control/app-server-control.sock")) {
        return Ok(());
    }

    if !ensure_private_real_dir_no_chmod(&packages_dir, uid)? {
        return Ok(());
    }
    if !ensure_private_real_dir_no_chmod(&pkg_daemon_dir, uid)? {
        return Ok(());
    }

    let current_link = pkg_daemon_dir.join("current");
    match std::os::unix::fs::symlink(standalone_current, &current_link) {
        Ok(()) => {
            fs_util::sync_directory(&pkg_daemon_dir)?;
            Ok(())
        }
        // Another launcher or upstream installer made the selection.
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

pub(super) fn has_daemon_pid_file(codex_home: &Path) -> bool {
    let state_dir = codex_home.join("app-server-daemon");
    may_exist(&state_dir.join("daemon.pid")) || may_exist(&state_dir.join("app-server.pid"))
}

pub(super) fn has_daemon_activity(codex_home: &Path) -> bool {
    may_exist(&codex_home.join("packages/app-server-daemon/current"))
        || has_daemon_pid_file(codex_home)
}

/// Discover owned execution homes (`active_codex_home`, `$HOME/.codex`, and
/// `$HOME/.local/share/codex/manager/profiles/*/home`) in deterministic order,
/// deduplicated by canonical path.
pub(super) fn candidate_daemon_homes(
    home_dir: &Path,
    active_codex_home: Option<&Path>,
    standalone_root: &Path,
) -> Vec<PathBuf> {
    let Ok(uid) = current_uid() else {
        return Vec::new();
    };
    let canonical_standalone = fs::canonicalize(standalone_root).ok();
    let mut raw_candidates = Vec::new();
    if let Some(active) = active_codex_home.filter(|p| !p.as_os_str().is_empty()) {
        raw_candidates.push(active.to_path_buf());
    }
    if !home_dir.as_os_str().is_empty() {
        raw_candidates.push(home_dir.join(".codex"));
        let profiles_root = home_dir.join(".local/share/codex/manager/profiles");
        if matches!(inspect_owned_real_dir(&profiles_root, uid), Ok(Some(true))) {
            if let Ok(entries) = fs::read_dir(&profiles_root) {
                let mut profile_names: Vec<_> = entries
                    .filter_map(|e| e.ok().map(|entry| entry.file_name()))
                    .collect();
                profile_names.sort();
                for name in profile_names {
                    let profile_dir = profiles_root.join(name);
                    if matches!(inspect_owned_real_dir(&profile_dir, uid), Ok(Some(true))) {
                        raw_candidates.push(profile_dir.join("home"));
                    }
                }
            }
        }
    }

    let mut homes = Vec::new();
    for candidate in raw_candidates {
        if !matches!(inspect_owned_real_dir(&candidate, uid), Ok(Some(true))) {
            continue;
        }
        let Ok(physical) = fs::canonicalize(&candidate) else {
            continue;
        };
        if canonical_standalone
            .as_ref()
            .is_some_and(|standalone| physical.starts_with(standalone))
        {
            continue;
        }
        if !homes.contains(&physical) {
            homes.push(physical);
        }
    }
    homes
}

/// Ensure an initialized daemon home's `packages/app-server-daemon/current`
/// symlink points to `standalone_current`. Returns `Ok(true)` if the link was
/// created or re-pointed, or `Ok(false)` if unchanged or not initialized.
pub(super) fn align_package_link_to_standalone(
    codex_home: &Path,
    standalone_current: &Path,
    standalone_root: &Path,
) -> io::Result<bool> {
    if codex_home.as_os_str().is_empty() || !has_daemon_activity(codex_home) {
        return Ok(false);
    }
    if fs::symlink_metadata(standalone_current).is_err()
        || fs::metadata(standalone_current).is_err()
    {
        return Ok(false);
    }
    let physical_home = match fs::canonicalize(codex_home) {
        Ok(path) => path,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    if let Ok(canonical_standalone) = fs::canonicalize(standalone_root) {
        if physical_home.starts_with(&canonical_standalone) {
            return Ok(false);
        }
    }

    let uid = current_uid()?;
    let packages_dir = physical_home.join("packages");
    if !ensure_private_real_dir_no_chmod(&packages_dir, uid)? {
        return Err(io::Error::other(
            "packages directory is not an owned real directory",
        ));
    }
    let pkg_daemon_dir = packages_dir.join("app-server-daemon");
    if !ensure_private_real_dir_no_chmod(&pkg_daemon_dir, uid)? {
        return Err(io::Error::other(
            "packages/app-server-daemon is not an owned real directory",
        ));
    }

    let current_link = pkg_daemon_dir.join("current");
    match fs::symlink_metadata(&current_link) {
        Ok(meta) => {
            if !meta.file_type().is_symlink() {
                return Err(io::Error::other(
                    "packages/app-server-daemon/current must be a symlink",
                ));
            }
            if fs::read_link(&current_link).ok().as_deref() == Some(standalone_current) {
                return Ok(false);
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    let seq = SETTINGS_TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_link = pkg_daemon_dir.join(format!(".current.{}.{seq}.tmp", std::process::id()));
    let _ = fs::remove_file(&temp_link);
    std::os::unix::fs::symlink(standalone_current, &temp_link)?;
    if let Err(err) = fs::rename(&temp_link, &current_link) {
        let _ = fs::remove_file(&temp_link);
        return Err(err);
    }
    fs_util::sync_directory(&pkg_daemon_dir)?;
    Ok(true)
}

fn update_settings_bytes(existing: Option<&[u8]>) -> io::Result<Option<Vec<u8>>> {
    let Some(raw) = existing else {
        return Ok(Some(DEFAULT_SETTINGS_BYTES.to_vec()));
    };
    let mut parser = JsonParser::new(raw);
    let mut root = parser.parse_document()?;
    let JsonValue::Object(ref mut entries) = root else {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "app-server-daemon settings.json must be a JSON object",
        ));
    };

    let mut changed = false;
    let mut found_updater = false;
    for (_, updater) in entries.iter_mut().filter(|(key, _)| key == "updater") {
        found_updater = true;
        if matches!(updater, JsonValue::Null) {
            *updater = JsonValue::Object(Vec::new());
            changed = true;
        }
        let JsonValue::Object(fields) = updater else {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "app-server-daemon settings.json updater field is not an object",
            ));
        };
        let mut found_flag = false;
        for (_, flag) in fields
            .iter_mut()
            .filter(|(key, _)| key == "autoUpdateEnabled")
        {
            found_flag = true;
            if !matches!(flag, JsonValue::Bool(false)) {
                *flag = JsonValue::Bool(false);
                changed = true;
            }
        }
        if !found_flag {
            fields.push(("autoUpdateEnabled".to_owned(), JsonValue::Bool(false)));
            changed = true;
        }
    }
    if !found_updater {
        entries.push((
            "updater".to_owned(),
            JsonValue::Object(vec![(
                "autoUpdateEnabled".to_owned(),
                JsonValue::Bool(false),
            )]),
        ));
        changed = true;
    }

    if !changed {
        return Ok(None);
    }

    let mut out = String::new();
    root.write_json(&mut out);
    out.push('\n');
    Ok(Some(out.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "codex-daemon-settings-{label}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_default_settings_and_preserves_idempotency() {
        let root = test_root("create");
        let home = root.join("codex-home");
        ensure(&home).unwrap();
        let settings = home.join("app-server-daemon/settings.json");
        assert_eq!(
            fs::read(&settings).unwrap(),
            b"{\"updater\":{\"autoUpdateEnabled\":false}}\n"
        );
        ensure(&home).unwrap();
        assert_eq!(
            fs::read(&settings).unwrap(),
            b"{\"updater\":{\"autoUpdateEnabled\":false}}\n"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn merges_existing_settings_preserving_unrelated_keys() {
        let root = test_root("merge");
        let home = root.join("codex-home");
        let daemon = home.join("app-server-daemon");
        fs::create_dir_all(&daemon).unwrap();
        let settings = daemon.join("settings.json");
        fs::write(
            &settings,
            br#"{"custom":42,"updater":{"lastCheck":"2026-11-15","autoUpdateEnabled":true},"tags":["a\nb"]}"#,
        )
        .unwrap();

        ensure(&home).unwrap();
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            "{\"custom\":42,\"updater\":{\"lastCheck\":\"2026-11-15\",\"autoUpdateEnabled\":false},\"tags\":[\"a\\nb\"]}\n"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_updater_flag_is_false_even_with_duplicate_escaped_keys() {
        let input = br#"{"updater":{"autoUpdateEnabled":true,"setting":5,"autoUpdateEnabled":false},"updat\u0065r":{"autoUpdateEnabled":true,"autoUpdateEnabl\u0065d":false},"updater":null}"#;
        let updated = update_settings_bytes(Some(input)).unwrap().unwrap();
        let text = std::str::from_utf8(&updated).unwrap();
        assert_eq!(text.matches(r#""autoUpdateEnabled":false"#).count(), 5);
        assert!(!text.contains("true"));
        assert!(text.contains(r#""setting":5"#));
        assert!(update_settings_bytes(Some(&updated)).unwrap().is_none());
    }

    #[test]
    fn json_unicode_escape_requires_four_hex_digits() {
        for input in [
            br#"{"key":"\u+041"}"#.as_slice(),
            br#"{"key":"\u00g1"}"#,
            br#"{"key":"\u 041"}"#,
        ] {
            assert_eq!(
                update_settings_bytes(Some(input)).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
        }
        assert!(update_settings_bytes(Some(br#"{"key":"\u0041"}"#))
            .unwrap()
            .is_some());
    }

    #[test]
    fn rejects_symlink_settings_or_corrupt_json() {
        let root = test_root("reject");
        let home = root.join("codex-home");
        let daemon = home.join("app-server-daemon");
        fs::create_dir_all(&daemon).unwrap();
        let outside = root.join("outside.json");
        fs::write(&outside, b"{}").unwrap();
        symlink(&outside, daemon.join("settings.json")).unwrap();
        assert!(ensure(&home).is_err());

        fs::remove_file(daemon.join("settings.json")).unwrap();
        fs::write(daemon.join("settings.json"), b"not-json").unwrap();
        assert!(ensure(&home).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    fn make_standalone_fixture(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let standalone = root.join(".codex/packages/standalone");
        let rel = standalone.join("releases/0.170.0-3f7e1a");
        fs::create_dir_all(rel.join("bin")).unwrap();
        fs::write(rel.join("bin/codex"), b"binary").unwrap();
        let current = standalone.join("current");
        symlink("releases/0.170.0-3f7e1a", &current).unwrap();
        (standalone, current)
    }

    #[test]
    fn creates_default_package_link_only_for_uninitialized_home() {
        let root = test_root("pkg-link-fresh");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);
        let codex_home = root.join("fresh-home");
        fs::create_dir_all(&codex_home).unwrap();
        fs::set_permissions(&codex_home, fs::Permissions::from_mode(0o755)).unwrap();

        ensure_default_package_link(&codex_home, &standalone_current, &standalone_root).unwrap();
        let link = codex_home.join("packages/app-server-daemon/current");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_link(&link).unwrap(), standalone_current);
        assert_eq!(
            fs::metadata(&codex_home).unwrap().permissions().mode() & 0o777,
            0o755
        );

        // Second call hits fast path without replacing or failing.
        ensure_default_package_link(&codex_home, &standalone_current, &standalone_root).unwrap();
        assert_eq!(fs::read_link(&link).unwrap(), standalone_current);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn preserves_existing_dedicated_broken_root_symlink_and_partial_daemon_state() {
        let root = test_root("pkg-link-preserve");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);

        // 1. Existing pinned/different symlink target is preserved.
        let h_pinned = root.join("h-pinned");
        let pinned_target = root.join("other-gen");
        fs::create_dir_all(pinned_target.join("bin")).unwrap();
        fs::create_dir_all(h_pinned.join("packages/app-server-daemon")).unwrap();
        symlink(
            &pinned_target,
            h_pinned.join("packages/app-server-daemon/current"),
        )
        .unwrap();
        ensure_default_package_link(&h_pinned, &standalone_current, &standalone_root).unwrap();
        assert_eq!(
            fs::read_link(h_pinned.join("packages/app-server-daemon/current")).unwrap(),
            pinned_target
        );

        // 2. Broken symlink is preserved rather than overwritten.
        let h_broken = root.join("h-broken");
        fs::create_dir_all(h_broken.join("packages/app-server-daemon")).unwrap();
        symlink(
            "/nonexistent/target",
            h_broken.join("packages/app-server-daemon/current"),
        )
        .unwrap();
        ensure_default_package_link(&h_broken, &standalone_current, &standalone_root).unwrap();
        assert_eq!(
            fs::read_link(h_broken.join("packages/app-server-daemon/current")).unwrap(),
            Path::new("/nonexistent/target")
        );

        // 3. Symlinked packages/app-server-daemon root is never traversed or mutated.
        let h_root_sym = root.join("h-root-sym");
        let external_store = root.join("external-store");
        fs::create_dir_all(&external_store).unwrap();
        fs::create_dir_all(h_root_sym.join("packages")).unwrap();
        symlink(
            &external_store,
            h_root_sym.join("packages/app-server-daemon"),
        )
        .unwrap();
        ensure_default_package_link(&h_root_sym, &standalone_current, &standalone_root).unwrap();
        assert!(!external_store.join("current").exists());

        // 4. Incomplete/partial package state (`releases/`, `install.lock`, `selection-history`) prevents auto-link.
        for indicator in [
            "releases",
            "install.lock",
            "selection-history",
            "auto-update-version",
        ] {
            let h_partial = root.join(format!("h-partial-{indicator}"));
            let pkg_dir = h_partial.join("packages/app-server-daemon");
            fs::create_dir_all(&pkg_dir).unwrap();
            if indicator == "releases" {
                fs::create_dir_all(pkg_dir.join(indicator)).unwrap();
            } else {
                fs::write(pkg_dir.join(indicator), b"").unwrap();
            }
            ensure_default_package_link(&h_partial, &standalone_current, &standalone_root).unwrap();
            assert!(!pkg_dir.join("current").exists());
        }

        // Modern and legacy launch traces each preserve upstream's selection,
        // even if only a stale log or dangling symlink remains.
        for trace in PRIOR_DAEMON_TRACE_FILES {
            let home = root.join(format!("h-{trace}"));
            let state = home.join("app-server-daemon");
            fs::create_dir_all(&state).unwrap();
            symlink("missing", state.join(trace)).unwrap();
            ensure_default_package_link(&home, &standalone_current, &standalone_root).unwrap();
            assert!(!home.join("packages").exists(), "{trace}");
        }

        let h_stopped_empty = root.join("h-stopped-empty");
        ensure(&h_stopped_empty).unwrap();
        fs::write(h_stopped_empty.join("app-server-daemon/daemon.lock"), b"").unwrap();
        ensure_default_package_link(&h_stopped_empty, &standalone_current, &standalone_root)
            .unwrap();
        assert_eq!(
            fs::read_link(h_stopped_empty.join("packages/app-server-daemon/current")).unwrap(),
            standalone_current
        );

        // 6. Never creates files inside standalone release root.
        let inside_release = standalone_root.join("releases/0.170.0-3f7e1a");
        ensure_default_package_link(&inside_release, &standalone_current, &standalone_root)
            .unwrap();
        assert!(!inside_release.join("packages").exists());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn handles_concurrent_first_initialization_with_eexist() {
        let root = test_root("pkg-link-concurrent");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);
        let codex_home = root.join("shared-concurrent-home");
        fs::create_dir_all(&codex_home).unwrap();

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let mut handles = Vec::new();
        for _ in 0..4 {
            let b = barrier.clone();
            let h = codex_home.clone();
            let sc = standalone_current.clone();
            let sr = standalone_root.clone();
            handles.push(std::thread::spawn(move || {
                b.wait();
                ensure_default_package_link(&h, &sc, &sr).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(
            fs::read_link(codex_home.join("packages/app-server-daemon/current")).unwrap(),
            standalone_current
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn align_package_link_repoints_pinned_daemon_and_discovers_profile_homes() {
        let root = test_root("align-and-discover");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);
        let default_home = root.join(".codex");
        let prof_a = root.join(".local/share/codex/manager/profiles/janmori101/home");
        let prof_b = root.join(".local/share/codex/manager/profiles/unused/home");
        for dir in [&default_home, &prof_a, &prof_b] {
            fs::create_dir_all(dir).unwrap();
        }

        let homes = candidate_daemon_homes(&root, Some(&default_home), &standalone_root);
        assert_eq!(homes.len(), 3);

        // Unused profile home without daemon activity is left untouched.
        assert!(
            !align_package_link_to_standalone(&prof_b, &standalone_current, &standalone_root)
                .unwrap()
        );
        assert!(!prof_b.join("packages").exists());

        // Pinned local release link in default_home is re-pointed to standalone_current.
        let local_rel = default_home.join("packages/app-server-daemon/releases/local-abc");
        fs::create_dir_all(&local_rel).unwrap();
        symlink(
            &local_rel,
            default_home.join("packages/app-server-daemon/current"),
        )
        .unwrap();
        assert!(align_package_link_to_standalone(
            &default_home,
            &standalone_current,
            &standalone_root
        )
        .unwrap());
        assert_eq!(
            fs::read_link(default_home.join("packages/app-server-daemon/current")).unwrap(),
            standalone_current
        );
        // Second alignment is a no-op returning Ok(false).
        assert!(!align_package_link_to_standalone(
            &default_home,
            &standalone_current,
            &standalone_root
        )
        .unwrap());

        // Home with daemon.pid but missing current link gets aligned to standalone_current.
        fs::create_dir_all(prof_a.join("app-server-daemon")).unwrap();
        fs::write(prof_a.join("app-server-daemon/daemon.pid"), b"{\"pid\":1}").unwrap();
        assert!(
            align_package_link_to_standalone(&prof_a, &standalone_current, &standalone_root)
                .unwrap()
        );
        assert_eq!(
            fs::read_link(prof_a.join("packages/app-server-daemon/current")).unwrap(),
            standalone_current
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
