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
const SHARED_OPTIONAL_FILES: &[&str] = &[
    "installation_id",
    "config.toml",
    "ais.config.toml",
    "chatgpt.config.toml",
];
const MERGE_MAX_BYTES: u64 = 64 * 1024 * 1024;

fn err_invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

fn ensure_owned_real_dir(path: &Path) -> io::Result<()> {
    if !fs_util::wrapper_safe_absolute_path(path) {
        return Err(err_invalid(format!(
            "CODEX_SHARED_HOME/CODEX_HOME path must be safe and absolute: {}",
            path.display()
        )));
    }
    let expected_uid = fs_util::current_uid()?;
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_dir() && !meta.file_type().is_symlink() => {}
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
                                    if let Ok(d) = File::open(p) {
                                        let _ = d.sync_all();
                                    }
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
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != expected_uid
        || meta.permissions().mode() & 0o022 != 0
    {
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
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn ensure_shared_dir_entry(codex_home: &Path, shared_home: &Path, name: &str) -> io::Result<()> {
    let shared_target = shared_home.join(name);
    ensure_owned_real_dir(&shared_target)?;
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
                fs::remove_dir(&link_path)?;
                std::os::unix::fs::symlink(&shared_target, &link_path)?;
                File::open(codex_home)?.sync_all()?;
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
            std::os::unix::fs::symlink(&shared_target, &link_path)?;
            File::open(codex_home)?.sync_all()?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn extract_json_string_id(line: &str) -> Option<&str> {
    let key = "\"id\":";
    let start = line.find(key)? + key.len();
    let rest = line[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
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

fn reconcile_append_log(codex_home: &Path, shared_home: &Path, name: &str) -> io::Result<()> {
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
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&shared_file)
            {
                Ok(file) => {
                    file.sync_all()?;
                    File::open(shared_home)?.sync_all()?;
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
            let profile_text = read_capped_text(&profile_file)?;
            let by_id = name == "session_index.jsonl";
            let mut seen_keys: HashSet<String> = HashSet::new();
            for line in shared_text.lines().filter(|l| !l.trim().is_empty()) {
                let key = if by_id {
                    extract_json_string_id(line).unwrap_or(line)
                } else {
                    line
                };
                seen_keys.insert(key.to_owned());
            }
            let mut to_append = String::new();
            for line in profile_text.lines().filter(|l| !l.trim().is_empty()) {
                let key = if by_id {
                    extract_json_string_id(line).unwrap_or(line)
                } else {
                    line
                };
                if seen_keys.insert(key.to_owned()) {
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
            ensure_symlink_target(&profile_file, &shared_file)?;
            Ok(())
        }
        Ok(_) => Err(err_invalid(format!(
            "profile log '{}' has unsupported type or owner",
            profile_file.display()
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            std::os::unix::fs::symlink(&shared_file, &profile_file)?;
            File::open(codex_home)?.sync_all()?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn ensure_optional_shared_file(
    codex_home: &Path,
    shared_home: &Path,
    name: &str,
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
            let shared_bytes = fs::read(&shared_file)?;
            let profile_bytes = fs::read(&profile_file)?;
            if shared_bytes == profile_bytes {
                ensure_symlink_target(&profile_file, &shared_file)?;
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
            std::os::unix::fs::symlink(&shared_file, &profile_file)?;
            File::open(codex_home)?.sync_all()?;
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
    let shared_home = PathBuf::from(shared_home_raw);
    ensure_owned_real_dir(&shared_home)?;
    ensure_owned_real_dir(codex_home)?;

    let canonical_shared = fs::canonicalize(&shared_home)?;
    let canonical_codex = fs::canonicalize(codex_home)?;
    if canonical_shared == canonical_codex {
        return Ok(None);
    }

    for dir in SHARED_DIRS {
        ensure_shared_dir_entry(codex_home, &canonical_shared, dir)?;
    }
    for log_name in SHARED_APPEND_LOGS {
        reconcile_append_log(codex_home, &canonical_shared, log_name)?;
    }
    for file_name in SHARED_OPTIONAL_FILES {
        ensure_optional_shared_file(codex_home, &canonical_shared, file_name)?;
    }
    Ok(Some(canonical_shared))
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
        assert_eq!(
            fs::read_link(profile.join("session_index.jsonl")).unwrap(),
            fs::canonicalize(&shared)
                .unwrap()
                .join("session_index.jsonl")
        );
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
