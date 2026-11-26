//! Opt-in external profile shared-home preparation (`CODEX_SHARED_HOME`).
//! Minimal Wrapper owns no profile registry or CRUD; callers set `CODEX_HOME`
//! for per-account auth/daemon state and `CODEX_SHARED_HOME` to share
//! conversations, SQLite state, and configuration without touching `auth.json`.
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{
    DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::{Path, PathBuf};

use crate::fs_util;

const SHARED_DIRS: &[&str] = &[
    "sessions",
    "archived_sessions",
    "thread-writer-locks",
    "rollout-migrations",
    "memories",
    "memories_v2",
    "tui-thread-reference-capabilities",
    "skills",
    "plugins",
    "rules",
];

const SHARED_APPEND_LOGS: &[&str] = &["session_index.jsonl", "history.jsonl"];
const SHARED_OPTIONAL_FILES: &[&str] = &["installation_id", "config.toml"];
const OPTIONAL_FILE_MAX_BYTES: u64 = 4 * 1024 * 1024;
const MERGE_MAX_BYTES: u64 = 64 * 1024 * 1024;

fn extra_files(raw: &OsStr) -> io::Result<Vec<&str>> {
    let invalid_name =
        || err_invalid("CODEX_SHARED_EXTRA_FILES contains an invalid or reserved filename");
    let raw = raw.to_str().ok_or_else(invalid_name)?;
    let mut files = Vec::new();
    for name in raw.split(':').filter(|item| !item.is_empty()) {
        if name.len() > 128
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b'/')
            || matches!(
                name,
                "." | ".."
                    | "auth.json"
                    | "auth.json.lock"
                    | "auth.lock"
                    | "version.json"
                    | "models_cache.json"
                    | "app-server-control"
                    | "app-server-daemon"
            )
            || SHARED_DIRS.contains(&name)
            || SHARED_APPEND_LOGS.contains(&name)
            || SHARED_OPTIONAL_FILES.contains(&name)
            || name.starts_with("state_")
            || [".sqlite", ".sqlite-wal", ".sqlite-shm", ".lock"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        {
            return Err(invalid_name());
        }
        if !files.contains(&name) {
            if files.len() == 16 {
                return Err(err_invalid(
                    "CODEX_SHARED_EXTRA_FILES exceeds 16 unique filenames",
                ));
            }
            files.push(name);
        }
    }
    Ok(files)
}

fn err_invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

fn validate_owned_real_dir(path: &Path, create: bool) -> io::Result<()> {
    if !fs_util::wrapper_safe_absolute_path(path) {
        return Err(err_invalid(format!(
            "CODEX_SHARED_HOME/CODEX_HOME path must be safe and absolute: {}",
            path.display()
        )));
    }
    let expected_uid = fs_util::current_uid()?;
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if fs_util::is_real_directory(&meta) => {}
            Ok(_) => {
                return Err(err_invalid(format!(
                    "directory path contains non-directory or symlink component: {}",
                    ancestor.display()
                )));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            if !create {
                return Ok(());
            }
            // Create parent chain privately when needed without altering existing dirs.
            let mut cursor = PathBuf::from("/");
            for component in path.components().skip(1) {
                cursor.push(component);
                match fs::symlink_metadata(&cursor) {
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        match DirBuilder::new().mode(0o700).create(&cursor) {
                            Ok(()) => {
                                if let Some(p) = cursor.parent() {
                                    let _ = fs_util::sync_directory(p);
                                }
                            }
                            Err(e2) if e2.kind() == io::ErrorKind::AlreadyExists => {}
                            Err(e2) => return Err(e2),
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Err(err) => return Err(err),
    }
    let meta = fs::symlink_metadata(path)?;
    if !fs_util::is_owned_directory(&meta, expected_uid) || meta.permissions().mode() & 0o022 != 0 {
        return Err(err_invalid(format!(
            "directory must be an owned, non-group/world-writable real directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_symlink_target(link_path: &Path, expected_target: &Path) -> io::Result<()> {
    let parent = link_path
        .parent()
        .ok_or_else(|| err_invalid("symlink path has no parent"))?;
    let tmp_name = format!(
        ".{}.link.{}.tmp",
        link_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("shared"),
        std::process::id()
    );
    let tmp_path = parent.join(tmp_name);
    let _ = fs::remove_file(&tmp_path);
    std::os::unix::fs::symlink(expected_target, &tmp_path)?;
    if let Err(err) = fs::rename(&tmp_path, link_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }
    fs_util::sync_directory(parent)?;
    Ok(())
}

fn check_shared_dir_entry(
    codex_home: &Path,
    shared_home: &Path,
    name: &str,
    prepare: bool,
) -> io::Result<()> {
    let shared_target = shared_home.join(name);
    validate_owned_real_dir(&shared_target, prepare)?;
    let link_path = codex_home.join(name);
    match fs::symlink_metadata(&link_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(&link_path)?;
            if target == shared_target {
                Ok(())
            } else {
                Err(err_invalid(format!(
                    "shared directory symlink '{}' points to '{}' instead of '{}'",
                    link_path.display(),
                    target.display(),
                    shared_target.display()
                )))
            }
        }
        Ok(meta) if meta.is_dir() => {
            if fs::read_dir(&link_path)?.next().is_none() {
                if prepare {
                    fs::remove_dir(&link_path)?;
                    std::os::unix::fs::symlink(&shared_target, &link_path)?;
                    fs_util::sync_directory(codex_home)?;
                }
                Ok(())
            } else {
                Err(err_invalid(format!(
                    "profile directory '{}' is non-empty while CODEX_SHARED_HOME is set; migrate it before sharing",
                    link_path.display()
                )))
            }
        }
        Ok(_) => Err(err_invalid(format!(
            "profile entry '{}' must be a symlink to '{}'",
            link_path.display(),
            shared_target.display()
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            if prepare {
                std::os::unix::fs::symlink(&shared_target, &link_path)?;
                fs_util::sync_directory(codex_home)?;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn read_capped_text(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let meta = file.metadata()?;
    if meta.len() > MERGE_MAX_BYTES {
        return Err(err_invalid(format!(
            "shared log '{}' exceeds 64 MiB merge bound",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(MERGE_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MERGE_MAX_BYTES {
        return Err(err_invalid(format!(
            "shared log '{}' exceeds 64 MiB merge bound",
            path.display()
        )));
    }
    String::from_utf8(bytes)
        .map_err(|_| err_invalid(format!("shared log '{}' is not UTF-8", path.display())))
}

fn reconcile_append_log(
    codex_home: &Path,
    shared_home: &Path,
    name: &str,
    prepare: bool,
) -> io::Result<()> {
    let shared_file = shared_home.join(name);
    let expected_uid = fs_util::current_uid()?;
    match fs::symlink_metadata(&shared_file) {
        Ok(meta) if meta.file_type().is_file() && meta.uid() == expected_uid => {}
        Ok(_) => {
            return Err(err_invalid(format!(
                "shared log target '{}' must be an owned regular file",
                shared_file.display()
            )));
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound && !prepare => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&shared_file)
            {
                Ok(file) => {
                    file.sync_all()?;
                    fs_util::sync_directory(shared_home)?;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        Err(err) => return Err(err),
    }

    let profile_file = codex_home.join(name);
    match fs::symlink_metadata(&profile_file) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(&profile_file)?;
            if target == shared_file {
                Ok(())
            } else {
                Err(err_invalid(format!(
                    "profile log symlink '{}' points to '{}' instead of '{}'",
                    profile_file.display(),
                    target.display(),
                    shared_file.display()
                )))
            }
        }
        Ok(meta) if meta.file_type().is_file() && meta.uid() == expected_uid => {
            if !prepare {
                return Ok(());
            }
            // Upstream atomic rename can detach symlinked JSONL files; merge unseen lines into
            // the shared log under flock before relinking.
            let lock_path = shared_home.join(format!(".{name}.wrapper-merge.lock"));
            let lock_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .custom_flags(fs_util::O_NOFOLLOW)
                .open(&lock_path)?;
            fs_util::flock_with_timeout(
                &lock_file,
                fs_util::LOCK_EX,
                std::time::Duration::from_secs(5),
                "timed out locking shared session log for merge",
            )?;

            let shared_text = read_capped_text(&shared_file)?;
            let profile_fd = OpenOptions::new()
                .read(true)
                .custom_flags(fs_util::O_NOFOLLOW | 0o4000)
                .open(&profile_file)?;
            let profile_meta_before = profile_fd.metadata()?;
            if !profile_meta_before.is_file()
                || profile_meta_before.uid() != expected_uid
                || profile_meta_before.nlink() != 1
            {
                return Err(err_invalid(
                    "detached profile log changed or is not a private regular file",
                ));
            }
            let mut profile_bytes = Vec::new();
            profile_fd
                .try_clone()?
                .take(MERGE_MAX_BYTES + 1)
                .read_to_end(&mut profile_bytes)?;
            if profile_bytes.len() as u64 > MERGE_MAX_BYTES {
                return Err(err_invalid("profile log exceeds merge bound"));
            }
            let profile_meta_after = profile_fd.metadata()?;
            if profile_meta_before.len() != profile_meta_after.len()
                || profile_meta_before.mtime() != profile_meta_after.mtime()
                || profile_meta_before.mtime_nsec() != profile_meta_after.mtime_nsec()
            {
                return Err(err_invalid(
                    "profile log changed while being merged; retry after writer is idle",
                ));
            }
            let profile_text = String::from_utf8(profile_bytes)
                .map_err(|_| err_invalid("profile log is not UTF-8"))?;
            let mut seen: HashSet<&str> = shared_text.split_terminator('\n').collect();
            let mut to_append = String::new();
            for line in profile_text
                .split_terminator('\n')
                .filter(|line| !line.trim().is_empty())
            {
                if seen.insert(line) {
                    to_append.push_str(line);
                    to_append.push('\n');
                }
            }
            if !to_append.is_empty() {
                let mut shared_out = OpenOptions::new()
                    .read(true)
                    .append(true)
                    .custom_flags(fs_util::O_NOFOLLOW)
                    .open(&shared_file)?;
                let existing_meta = shared_out.metadata()?;
                if existing_meta.len() > 0 && !shared_text.ends_with('\n') {
                    shared_out.write_all(b"\n")?;
                }
                shared_out.write_all(to_append.as_bytes())?;
                shared_out.sync_all()?;
            }
            // Keep the detached source inode in place: upstream writers do not
            // participate in our merge lock and may still hold this inode open.
            // A later reconciliation can merge subsequent appends safely.
            Ok(())
        }
        Ok(_) => Err(err_invalid(format!(
            "profile log '{}' has unsupported type or owner",
            profile_file.display()
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            if prepare {
                std::os::unix::fs::symlink(&shared_file, &profile_file)?;
                fs_util::sync_directory(codex_home)?;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn ensure_optional_shared_file(
    codex_home: &Path,
    shared_home: &Path,
    name: &str,
    prepare: bool,
) -> io::Result<()> {
    let shared_file = shared_home.join(name);
    let expected_uid = fs_util::current_uid()?;
    match fs::symlink_metadata(&shared_file) {
        Ok(meta) if meta.file_type().is_file() && meta.uid() == expected_uid => {}
        Ok(_) => {
            return Err(err_invalid(format!(
                "shared file '{}' must be an owned regular file",
                shared_file.display()
            )));
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    }

    let profile_file = codex_home.join(name);
    match fs::symlink_metadata(&profile_file) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(&profile_file)?;
            if target == shared_file {
                Ok(())
            } else {
                Err(err_invalid(format!(
                    "profile symlink '{}' points to '{}' instead of '{}'",
                    profile_file.display(),
                    target.display(),
                    shared_file.display()
                )))
            }
        }
        Ok(meta) if meta.file_type().is_file() && meta.uid() == expected_uid => {
            let read = |path| {
                fs_util::read_bounded_regular(
                    path,
                    OPTIONAL_FILE_MAX_BYTES,
                    false,
                    "shared file must be a bounded, owned regular file",
                )
            };
            if read(&shared_file)? == read(&profile_file)? {
                if prepare {
                    ensure_symlink_target(&profile_file, &shared_file)?;
                }
                Ok(())
            } else {
                Err(err_invalid(format!(
                    "profile file '{}' differs from shared '{}' while CODEX_SHARED_HOME is set; unify or remove it before sharing",
                    profile_file.display(),
                    shared_file.display()
                )))
            }
        }
        Ok(_) => Err(err_invalid(format!(
            "profile file '{}' has unsupported type or owner",
            profile_file.display()
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            if prepare {
                std::os::unix::fs::symlink(&shared_file, &profile_file)?;
                fs_util::sync_directory(codex_home)?;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

/// Ensures `codex_home` shares conversations, SQLite DBs, and shared config with `shared_home`
/// while leaving `auth.json` and daemon sockets isolated per `codex_home`.
pub(super) fn prepare_shared_home(
    codex_home: &Path,
    shared_home_raw: &OsStr,
) -> io::Result<Option<PathBuf>> {
    let extra = std::env::var_os("CODEX_SHARED_EXTRA_FILES").unwrap_or_default();
    bind_shared_home(codex_home, shared_home_raw, extra_files(&extra)?, true)
}

/// Checks the same paths and bindings as a launch without reconciling them.
pub(super) fn inspect_shared_home(
    codex_home: &Path,
    shared_home_raw: &OsStr,
) -> io::Result<Option<PathBuf>> {
    let extra = std::env::var_os("CODEX_SHARED_EXTRA_FILES").unwrap_or_default();
    bind_shared_home(codex_home, shared_home_raw, extra_files(&extra)?, false)
}

fn bind_shared_home(
    codex_home: &Path,
    shared_home_raw: &OsStr,
    extra: Vec<&str>,
    prepare: bool,
) -> io::Result<Option<PathBuf>> {
    let shared_home = Path::new(shared_home_raw);
    validate_owned_real_dir(shared_home, prepare)?;
    validate_owned_real_dir(codex_home, prepare)?;

    // Validation has excluded symlinks in both paths. Normalize separators even
    // when the read-only check encounters a directory that does not exist yet.
    let shared_home: PathBuf = shared_home.components().collect();
    if shared_home == codex_home {
        return Ok(None);
    }

    for dir in SHARED_DIRS {
        check_shared_dir_entry(codex_home, &shared_home, dir, prepare)?;
    }
    for log_name in SHARED_APPEND_LOGS {
        reconcile_append_log(codex_home, &shared_home, log_name, prepare)?;
    }
    for file_name in SHARED_OPTIONAL_FILES.iter().copied().chain(extra) {
        ensure_optional_shared_file(codex_home, &shared_home, file_name, prepare)?;
    }
    Ok(Some(shared_home))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{remove_temp_root, temp_root};

    #[test]
    fn shares_sessions_skills_config_and_merges_detached_session_index_while_isolating_auth() {
        let root = temp_root("shared-home");
        let shared = root.join("shared");
        let profile = root.join("profile-a/home");
        fs::create_dir_all(&shared).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir_all(profile.parent().unwrap()).unwrap();
        fs::set_permissions(profile.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();

        fs::write(shared.join("auth.json"), b"{\"token\":\"default\"}\n").unwrap();
        fs::write(shared.join("config.toml"), b"model = \"gpt-6.2-sol\"\n").unwrap();
        fs::write(
            shared.join("session_index.jsonl"),
            b"{\"id\":\"t1\",\"thread_name\":\"First\"}\n",
        )
        .unwrap();

        fs::create_dir_all(&profile).unwrap();
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(profile.join("auth.json"), b"{\"token\":\"profile-a\"}\n").unwrap();
        // Simulate upstream atomic rename breaking session_index.jsonl symlink with an extra thread.
        fs::write(
            profile.join("session_index.jsonl"),
            b"{\"id\":\"t1\",\"thread_name\":\"First\"}\n{\"id\":\"t2\",\"thread_name\":\"FromA\"}\n",
        )
        .unwrap();

        let sqlite_dir = prepare_shared_home(&profile, shared.as_os_str())
            .unwrap()
            .unwrap();
        assert_eq!(sqlite_dir, fs::canonicalize(&shared).unwrap());
        assert_eq!(
            fs::read_link(profile.join("sessions")).unwrap(),
            fs::canonicalize(&shared).unwrap().join("sessions")
        );
        assert_eq!(
            fs::read_link(profile.join("config.toml")).unwrap(),
            fs::canonicalize(&shared).unwrap().join("config.toml")
        );
        // Keep the detached profile inode until the upstream writer is known idle.
        assert!(fs::symlink_metadata(profile.join("session_index.jsonl"))
            .unwrap()
            .is_file());
        let merged = fs::read_to_string(shared.join("session_index.jsonl")).unwrap();
        assert!(merged.contains("\"id\":\"t1\""));
        assert!(merged.contains("\"id\":\"t2\""));
        assert_eq!(
            fs::read_to_string(profile.join("auth.json")).unwrap(),
            "{\"token\":\"profile-a\"}\n"
        );
        assert_eq!(
            fs::read_to_string(shared.join("auth.json")).unwrap(),
            "{\"token\":\"default\"}\n"
        );

        remove_temp_root(root);
    }

    #[test]
    fn opt_in_extra_files_are_validated_before_binding_and_bounded_on_conflict() {
        let root = temp_root("extra-shared-files");
        let profile = root.join("profile");
        let shared = root.join("shared");
        fs::create_dir(&shared).unwrap();
        fs::write(shared.join("extra.toml"), b"shared content").unwrap();
        fs::write(shared.join("private.toml"), b"not opted in").unwrap();
        let guard = crate::tests::EnvRestoreGuard::new(&[(
            "CODEX_SHARED_EXTRA_FILES",
            Some(Path::new("auth.json:extra.toml")),
        )]);
        assert_eq!(
            prepare_shared_home(&profile, shared.as_os_str())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert!(!profile.exists());
        assert!(!shared.join("sessions").exists());
        drop(guard);
        let guard = crate::tests::EnvRestoreGuard::new(&[(
            "CODEX_SHARED_EXTRA_FILES",
            Some(Path::new(":extra.toml::extra.toml:")),
        )]);
        prepare_shared_home(&profile, shared.as_os_str()).unwrap();
        assert_eq!(
            fs::read_link(profile.join("extra.toml")).unwrap(),
            shared.join("extra.toml")
        );
        assert!(!profile.join("private.toml").exists());
        fs::remove_file(profile.join("extra.toml")).unwrap();
        File::create(profile.join("extra.toml"))
            .unwrap()
            .set_len(OPTIONAL_FILE_MAX_BYTES + 1)
            .unwrap();
        assert_eq!(
            prepare_shared_home(&profile, shared.as_os_str())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert!(profile.join("extra.toml").is_file());
        drop(guard);
        remove_temp_root(root);
    }

    #[test]
    fn extra_file_names_reject_sensitive_paths_and_enforce_unique_count() {
        use std::os::unix::ffi::OsStrExt as _;

        for name in [
            "auth.json",
            "auth.json.lock",
            "auth.lock",
            ".",
            "..",
            "a/b",
            "a b",
            "a\0b",
            "한",
            "config.toml",
            "installation_id",
            "session_index.jsonl",
            "history.jsonl",
            "version.json",
            "models_cache.json",
            "state_data",
            "a.sqlite",
            "a.sqlite-wal",
            "a.sqlite-shm",
            "a.lock",
        ] {
            assert_eq!(
                extra_files(OsStr::new(name)).unwrap_err().kind(),
                io::ErrorKind::InvalidData,
                "{name:?}"
            );
        }
        assert!(extra_files(OsStr::from_bytes(b"extra-\xff")).is_err());
        assert!(extra_files(OsStr::new(&"a".repeat(129))).is_err());
        assert_eq!(extra_files(OsStr::new(&"a".repeat(128))).unwrap().len(), 1);
        let names = (0..16)
            .map(|n| format!("file-{n}"))
            .collect::<Vec<_>>()
            .join(":");
        assert_eq!(
            extra_files(OsStr::new(&format!("{names}:file-0")))
                .unwrap()
                .len(),
            16
        );
        assert!(extra_files(OsStr::new(&format!("{names}:file-16"))).is_err());
    }

    #[test]
    fn session_index_retains_same_id_renames_and_deduplicates_exact_lines() {
        let root = temp_root("shared-home-rename");
        let shared = root.join("shared");
        let profile = root.join("profile");
        fs::create_dir(&shared).unwrap();
        fs::create_dir(&profile).unwrap();
        let name = "session_index.jsonl";
        let original = r#"{"id":"t1","thread_name":"initial"}"#;
        let renamed = r#"{"id":"t1","thread_name":"renamed"}"#;
        fs::write(shared.join(name), original).unwrap();
        fs::write(
            profile.join(name),
            format!("{original}\n{renamed}\n{renamed}\n"),
        )
        .unwrap();
        assert_eq!(
            inspect_shared_home(&profile, shared.as_os_str()).unwrap(),
            Some(shared.clone())
        );
        assert_eq!(fs::read_to_string(shared.join(name)).unwrap(), original);
        assert!(fs::symlink_metadata(profile.join(name)).unwrap().is_file());
        assert!(!shared.join("sessions").exists());
        prepare_shared_home(&profile, shared.as_os_str()).unwrap();
        assert_eq!(
            fs::read_to_string(shared.join(name)).unwrap(),
            format!("{original}\n{renamed}\n")
        );
        // The source is retained because an upstream writer may still hold its inode.
        assert!(fs::symlink_metadata(profile.join(name)).unwrap().is_file());
        remove_temp_root(root);
    }

    #[test]
    fn rejects_conflicting_non_empty_sessions_or_config_without_overwrite() {
        let root = temp_root("shared-home-conflict");
        let shared = root.join("shared");
        let profile = root.join("profile-b/home");
        fs::create_dir_all(&shared).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(shared.join("config.toml"), b"a = 1\n").unwrap();
        fs::create_dir_all(profile.join("sessions")).unwrap();
        fs::set_permissions(profile.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(profile.join("sessions"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(profile.join("sessions/local.jsonl"), b"keep-me").unwrap();

        assert!(prepare_shared_home(&profile, shared.as_os_str()).is_err());
        assert_eq!(
            fs::read(profile.join("sessions/local.jsonl")).unwrap(),
            b"keep-me"
        );
        fs::remove_dir_all(profile.join("sessions")).unwrap();

        fs::write(profile.join("config.toml"), b"a = 2\n").unwrap();
        assert!(prepare_shared_home(&profile, shared.as_os_str()).is_err());
        assert_eq!(fs::read(profile.join("config.toml")).unwrap(), b"a = 2\n");

        remove_temp_root(root);
    }
}
