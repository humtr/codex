use crate::fs_util;
use crate::json::{JsonParser, JsonValue};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
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
    let mut temp = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp_path)?;
    let write_result = (|| -> io::Result<()> {
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

pub(super) const DAEMON_PID_FILES: [&str; 2] = ["daemon.pid", "app-server.pid"];

pub(super) fn has_daemon_activity(codex_home: &Path) -> bool {
    may_exist(&codex_home.join("packages/app-server-daemon/current"))
        || DAEMON_PID_FILES
            .iter()
            .any(|name| may_exist(&codex_home.join("app-server-daemon").join(name)))
}

fn owned_directory_chain(base: &Path, path: &Path, uid: u32) -> bool {
    fs_util::wrapper_safe_absolute_path(path)
        && path.starts_with(base)
        && path
            .ancestors()
            .take_while(|p| *p != base)
            .all(|p| matches!(inspect_owned_real_dir(p, uid), Ok(Some(true))))
}

fn directory_identity(meta: &std::fs::Metadata) -> (u64, u64) {
    (meta.dev(), meta.ino())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DaemonHomeLabel {
    Named(String),
    Path(String),
}

impl DaemonHomeLabel {
    pub(super) fn short_name(&self) -> &str {
        match self {
            DaemonHomeLabel::Named(name) | DaemonHomeLabel::Path(name) => name,
        }
    }

    pub(super) fn warning_target(&self, home: &Path) -> String {
        match self {
            DaemonHomeLabel::Named(name) => format!("{name} ({})", home.display()),
            DaemonHomeLabel::Path(_) => home.display().to_string(),
        }
    }
}

pub(super) fn daemon_home_label(home_dir: &Path, codex_home: &Path) -> DaemonHomeLabel {
    let canonical_home = fs::canonicalize(home_dir).unwrap_or_else(|_| home_dir.to_path_buf());
    let normalized = codex_home
        .strip_prefix(home_dir)
        .map_or_else(|_| codex_home.to_path_buf(), |rel| canonical_home.join(rel));
    if normalized == canonical_home.join(".codex") {
        return DaemonHomeLabel::Named("default".to_owned());
    }
    if let Ok(rel) =
        normalized.strip_prefix(canonical_home.join(".local/share/codex/manager/profiles"))
    {
        let mut components = rel.components();
        if let (
            Some(std::path::Component::Normal(name)),
            Some(std::path::Component::Normal(leaf)),
            None,
        ) = (components.next(), components.next(), components.next())
        {
            if leaf == "home" {
                if let Some(name) = name.to_str().filter(|s| !s.is_empty()) {
                    return DaemonHomeLabel::Named(name.to_owned());
                }
            }
        }
    }
    DaemonHomeLabel::Path(codex_home.display().to_string())
}

/// Discover local execution homes in deterministic order (`default`, sorted
/// manager profiles, then any distinct active `$CODEX_HOME`) without following
/// symlinks within the registry or the candidate home. HOME itself can have
/// platform aliases.
pub(super) fn candidate_daemon_homes(
    home_dir: &Path,
    active_codex_home: Option<&Path>,
    standalone_root: &Path,
) -> Vec<PathBuf> {
    let Ok(uid) = current_uid() else {
        return Vec::new();
    };
    let Ok(home) = fs::canonicalize(home_dir) else {
        return Vec::new();
    };
    let Ok(standalone) = fs::metadata(standalone_root) else {
        return Vec::new();
    };
    let standalone = directory_identity(&standalone);
    let mut homes = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |candidate: PathBuf| {
        let safe = if candidate.starts_with(&home) {
            owned_directory_chain(&home, &candidate, uid)
        } else {
            fs_util::wrapper_safe_absolute_path(&candidate)
                && fs_util::ensure_real_directories(candidate.ancestors(), || {
                    io::Error::other("daemon home traverses a symlink")
                })
                .is_ok()
        };
        if !safe {
            return;
        }
        let Ok(meta) = fs::symlink_metadata(&candidate) else {
            return;
        };
        // Android can reach the same directory through distinct mount paths.
        if fs_util::is_owned_directory(&meta, uid)
            && !candidate
                .ancestors()
                .any(|p| fs::metadata(p).is_ok_and(|meta| directory_identity(&meta) == standalone))
            && seen.insert(directory_identity(&meta))
        {
            homes.push(candidate);
        }
    };
    add(home.join(".codex"));
    let profiles = home.join(".local/share/codex/manager/profiles");
    if owned_directory_chain(&home, &profiles, uid) {
        if let Ok(entries) = fs::read_dir(&profiles) {
            let mut names: Vec<_> = entries
                .filter_map(|e| e.ok().map(|e| e.file_name()))
                .collect();
            names.sort();
            for name in names {
                add(profiles.join(name).join("home"));
            }
        }
    }
    if let Some(active) = active_codex_home {
        add(active
            .strip_prefix(home_dir)
            .map_or_else(|_| active.to_path_buf(), |rel| home.join(rel)));
    }
    homes
}

/// Use upstream's existing operation lock while inspecting or changing the
/// package selection. Upstream lifecycle commands take it themselves.
pub(super) fn lock_daemon_home(codex_home: &Path) -> io::Result<File> {
    let uid = current_uid()?;
    if !matches!(inspect_owned_real_dir(codex_home, uid), Ok(Some(true))) {
        return Err(io::Error::other(
            "daemon home must be an owned real directory",
        ));
    }
    let dir = codex_home.join("app-server-daemon");
    if !ensure_private_real_dir_no_chmod(&dir, uid)? {
        return Err(io::Error::other(
            "daemon state must be an owned real directory",
        ));
    }
    let lock_path = dir.join("daemon.lock");
    if let Ok(meta) = fs::symlink_metadata(&lock_path) {
        if !meta.is_file() || meta.uid() != uid || meta.mode() & 0o022 != 0 {
            return Err(io::Error::other(
                "daemon operation lock must be an owned regular file",
            ));
        }
    }
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(fs_util::O_NOFOLLOW | crate::proc_exec::O_NONBLOCK_LINUX)
        .open(lock_path)?;
    let meta = lock.metadata()?;
    if !meta.is_file() || meta.uid() != uid || meta.mode() & 0o022 != 0 {
        return Err(io::Error::other(
            "daemon operation lock must be an owned regular file",
        ));
    }
    fs_util::flock_fd(&lock, fs_util::LOCK_EX | fs_util::LOCK_NB).map_err(|err| {
        if err.kind() == ErrorKind::WouldBlock {
            io::Error::new(err.kind(), "upstream daemon operation is in progress")
        } else {
            err
        }
    })?;
    Ok(lock)
}

/// Check the package path before interrupting a running daemon.
pub(super) fn prepare_daemon_package_dir(codex_home: &Path) -> io::Result<PathBuf> {
    let uid = current_uid()?;
    let packages = codex_home.join("packages");
    let daemon_package = packages.join("app-server-daemon");
    for dir in [&packages, &daemon_package] {
        match inspect_owned_real_dir(dir, uid)? {
            Some(true) => {}
            Some(false) => {
                return Err(io::Error::other(
                    "daemon package path must be an owned real directory",
                ))
            }
            None => {
                if !ensure_private_real_dir_no_chmod(dir, uid)? {
                    return Err(io::Error::other(
                        "daemon package path must be an owned real directory",
                    ));
                }
                fs_util::sync_directory(dir.parent().expect("package path has a parent"))?;
            }
        }
    }
    match fs::symlink_metadata(daemon_package.join("current")) {
        Ok(meta) if !meta.file_type().is_symlink() => Err(io::Error::other(
            "packages/app-server-daemon/current must be a symlink",
        )),
        Ok(_) => Ok(daemon_package),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(daemon_package),
        Err(err) => Err(err),
    }
}

/// Atomically align the selection while the caller holds the operation lock.
pub(super) fn align_package_link_to_standalone(
    pkg_daemon_dir: &Path,
    standalone_current: &Path,
) -> io::Result<bool> {
    let current_link = pkg_daemon_dir.join("current");
    match fs::read_link(&current_link) {
        Ok(target) if target == standalone_current => return Ok(false),
        Ok(_) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {
            std::os::unix::fs::symlink(standalone_current, &current_link)?;
            fs_util::sync_directory(pkg_daemon_dir)?;
            return Ok(true);
        }
        Err(err) => return Err(err),
    }
    let seq = SETTINGS_TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_link = pkg_daemon_dir.join(format!(".current.{}.{seq}.tmp", std::process::id()));
    std::os::unix::fs::symlink(standalone_current, &temp_link)?;
    if let Err(err) = fs::rename(&temp_link, &current_link) {
        let _ = fs::remove_file(&temp_link);
        return Err(err);
    }
    fs_util::sync_directory(pkg_daemon_dir)?;
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

        let homes = candidate_daemon_homes(&root, Some(&prof_a), &standalone_root);
        assert_eq!(homes.len(), 3);
        assert_eq!(
            daemon_home_label(&root, &homes[0]),
            DaemonHomeLabel::Named("default".to_owned())
        );
        assert_eq!(
            daemon_home_label(&root, &homes[1]),
            DaemonHomeLabel::Named("janmori101".to_owned())
        );
        assert_eq!(
            daemon_home_label(&root, &homes[2]),
            DaemonHomeLabel::Named("unused".to_owned())
        );
        let custom = root.join("custom-codex-home");
        fs::create_dir_all(&custom).unwrap();
        let with_custom = candidate_daemon_homes(&root, Some(&custom), &standalone_root);
        assert_eq!(with_custom.len(), 4);
        assert_eq!(
            daemon_home_label(&root, &with_custom[3]),
            DaemonHomeLabel::Path(with_custom[3].display().to_string())
        );

        assert!(!has_daemon_activity(&prof_b));
        let local_rel = default_home.join("packages/app-server-daemon/releases/local-abc");
        fs::create_dir_all(&local_rel).unwrap();
        let package_dir = prepare_daemon_package_dir(&default_home).unwrap();
        let link = package_dir.join("current");
        // A broken dedicated pin is replaced without visiting the old target.
        symlink(local_rel.join("missing"), &link).unwrap();
        assert!(align_package_link_to_standalone(&package_dir, &standalone_current).unwrap());
        assert_eq!(fs::read_link(&link).unwrap(), standalone_current);
        assert!(!align_package_link_to_standalone(&package_dir, &standalone_current).unwrap());
        fs::remove_file(&link).unwrap();
        fs::create_dir(&link).unwrap();
        assert!(prepare_daemon_package_dir(&default_home).is_err());
        assert!(link.is_dir());
        assert!(local_rel.is_dir());

        let manager = root.join(".local/share/codex/manager");
        fs::rename(&manager, root.join("other-manager")).unwrap();
        symlink(root.join("other-manager"), &manager).unwrap();
        assert_eq!(
            candidate_daemon_homes(&root, Some(&prof_a), &standalone_root),
            vec![default_home]
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
