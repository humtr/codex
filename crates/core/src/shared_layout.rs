//! Filesystem topology only. Upstream owns every conversation and SQLite schema.
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) const DIRECTORIES: &[&str] = &[
    "sessions",
    "archived_sessions",
    "thread-writer-locks",
    "rollout-migrations",
    "memories",
    "memories_v2",
    "tui-thread-reference-capabilities",
];
pub(super) const FILES: &[&str] = &["session_index.jsonl", "history.jsonl", "installation_id"];

fn invalid() -> io::Error {
    io::Error::other("declared profile requires the canonical shared conversation topology; complete the bounded legacy transition first")
}
fn identity(value: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let b = value.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && (b[0].is_ascii_alphanumeric() || matches!(b[0], b'-' | b'_'))
        && b.iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}
fn declared(home: &Path, profile: &Path) -> bool {
    if profile == home.join(".codex") {
        return true;
    }
    if profile.parent() == Some(home.join(".codex-profiles").as_path()) {
        return profile.file_name().is_some_and(identity);
    }
    profile.file_name() == Some(OsStr::new("home"))
        && profile.parent().is_some_and(|p| {
            p.parent() == Some(home.join(".local/share/codex/manager/profiles").as_path())
                && p.file_name().is_some_and(identity)
        })
}

pub(super) fn prepare(home: &Path, selected: Option<&OsStr>) -> io::Result<Option<PathBuf>> {
    let shared = home.join(".codex");
    let profile = selected
        .map(PathBuf::from)
        .unwrap_or_else(|| shared.clone());
    if !declared(home, &profile) {
        return Ok(None);
    }
    // Preflight all profile entries before any canonical or profile mutation.
    if profile != shared {
        for name in DIRECTORIES.iter().chain(FILES) {
            let path = profile.join(name);
            match fs::symlink_metadata(&path) {
                Ok(m)
                    if m.file_type().is_symlink() && fs::read_link(&path)? == shared.join(name) => {
                }
                Ok(m)
                    if m.is_dir()
                        && DIRECTORIES.contains(name)
                        && fs::read_dir(&path)?.next().is_none() => {}
                Ok(_) => return Err(invalid()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
        }
    }
    super::shared_server::private_dir(&shared)?;
    if profile != shared {
        super::shared_server::private_dir(&profile)?;
    }
    for name in DIRECTORIES {
        super::shared_server::private_dir(&shared.join(name))?;
    }
    for name in FILES {
        match fs::symlink_metadata(shared.join(name)) {
            Ok(m) if m.is_file() && !m.file_type().is_symlink() => (),
            Ok(_) => return Err(invalid()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    if profile != shared {
        for name in DIRECTORIES.iter().chain(FILES) {
            let path = profile.join(name);
            match fs::symlink_metadata(&path) {
                Ok(m) if m.file_type().is_symlink() => (),
                Ok(_) => {
                    fs::remove_dir(&path)?;
                    std::os::unix::fs::symlink(shared.join(name), &path)?;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    std::os::unix::fs::symlink(shared.join(name), &path)?
                }
                Err(e) => return Err(e),
            }
        }
        fs::File::open(&profile)?.sync_all()?;
    }
    Ok(Some(shared))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_identity_shares_only_conversations_and_rejects_legacy_or_substitution() {
        for name in ["account.a", "a", "_account", "-account"] {
            assert!(identity(OsStr::new(name)));
        }
        for name in ["", ".", "..", ".hidden", "../a", "é"] {
            assert!(!identity(OsStr::new(name)));
        }
        assert!(!identity(OsStr::new(&"a".repeat(65))));
        let home = crate::tests::temp_root("shared-layout");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir(home.join(".codex-profiles")).unwrap();
        let a = home.join(".codex-profiles/a");
        let b = home.join(".codex-profiles/b");
        let shared = prepare(&home, None).unwrap().unwrap();
        for p in [&a, &b] {
            assert_eq!(
                prepare(&home, Some(p.as_os_str())).unwrap(),
                Some(shared.clone())
            );
            fs::write(
                p.join("auth.json"),
                if p == &a {
                    b"identity-a"
                } else {
                    b"identity-b"
                },
            )
            .unwrap();
            fs::write(p.join("config.toml"), b"profile-only").unwrap();
        }
        fs::write(a.join("sessions/thread.jsonl"), b"upstream-owned").unwrap();
        assert_eq!(
            fs::read(b.join("sessions/thread.jsonl")).unwrap(),
            b"upstream-owned"
        );
        assert_ne!(
            fs::read(a.join("auth.json")).unwrap(),
            fs::read(b.join("auth.json")).unwrap()
        );
        assert!(!shared.join("auth.json").exists());
        assert!(!shared.join("config.toml").exists());
        let manager = home.join(".local/share/codex/manager/profiles/m/home");
        fs::create_dir_all(manager.parent().unwrap()).unwrap();
        assert_eq!(
            prepare(&home, Some(manager.as_os_str())).unwrap(),
            Some(shared.clone())
        );
        assert_eq!(
            fs::read_link(manager.join("sessions")).unwrap(),
            shared.join("sessions")
        );
        assert_eq!(
            prepare(&home, Some(home.join("arbitrary").as_os_str())).unwrap(),
            None
        );
        fs::remove_file(b.join("sessions")).unwrap();
        fs::create_dir(b.join("sessions")).unwrap();
        fs::write(b.join("sessions/legacy.jsonl"), b"retained").unwrap();
        assert!(prepare(&home, Some(b.as_os_str())).is_err());
        assert_eq!(
            fs::read(b.join("sessions/legacy.jsonl")).unwrap(),
            b"retained"
        );
        fs::remove_file(b.join("sessions/legacy.jsonl")).unwrap();
        assert!(prepare(&home, Some(b.as_os_str())).is_ok());
        fs::remove_file(b.join("sessions")).unwrap();
        std::os::unix::fs::symlink(a.join("sessions"), b.join("sessions")).unwrap();
        assert!(prepare(&home, Some(b.as_os_str())).is_err());
        crate::tests::remove_temp_root(&home);
    }
}
