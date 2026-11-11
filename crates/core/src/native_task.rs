//! Native rendezvous and execution-home handoff, independent of optional UI policy.
use codex_runtime_binding::native;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const DEADLINE: Duration = Duration::from_secs(10);

pub(super) fn handles(args: &[OsString]) -> bool {
    matches!(
        args.first().and_then(|value| value.to_str()),
        Some("__task-connection-v1" | "__task-reentry-v1")
    )
}

fn connection(home: &Path, id: &str) -> io::Result<String> {
    let value = match native::owner(home, id, Instant::now() + DEADLINE)? {
        None => "null".to_owned(),
        Some(owner) => {
            let text = |path: &Path| {
                path.to_str()
                    .map(super::json_escape)
                    .ok_or_else(|| io::Error::other("Native connection path is not UTF-8"))
            };
            format!(
                "{{\"execution_home\":\"{}\",\"socket\":\"{}\"}}",
                text(&owner.home)?,
                text(&owner.socket)?,
            )
        }
    };
    // Already-built frontends consume this exact private protocol.
    Ok(format!(
        "{{\"schema\":\"codex-manager-task-connection-v1\",\"connection\":{value}}}\n"
    ))
}

pub(super) fn run(args: &[OsString]) -> io::Result<String> {
    let usage = || io::Error::other("invalid Native task connection/handoff arguments");
    let id = args
        .get(1)
        .and_then(|value| value.to_str())
        .filter(|id| native::canonical_session_id(id))
        .ok_or_else(usage)?;
    let home = std::env::var_os("HOME")
        .ok_or_else(|| io::Error::other("Native process root HOME is unavailable"))?;
    match args[0].to_str() {
        Some("__task-connection-v1") if args.len() == 2 => connection(Path::new(&home), id),
        Some("__task-reentry-v1") if args.len() > 3 && args[2] == OsStr::new("--") => {
            let owner = native::owner(Path::new(&home), id, Instant::now() + DEADLINE)?
                .ok_or_else(|| {
                    io::Error::other("Native writer is no longer available for handoff")
                })?;
            let core = super::validated_core_entrypoint()?;
            let mut command = Command::new(core);
            command
                .args(&args[3..])
                .env_remove(super::MANAGER_CORE_API_ENV)
                .env_remove(super::MANAGER_CORE_ENTRYPOINT_ENV)
                .env_remove(super::CODEX_SQLITE_HOME_ENV)
                .env("CODEX_HOME", &owner.home);
            if !native::owns_writer(Path::new(&home), id, &owner)? {
                return Err(io::Error::other(
                    "Native writer changed before context handoff",
                ));
            }
            Err(command.exec())
        }
        _ => Err(usage()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn connection_requires_confirmed_free_writer_without_manager() {
        let root = std::env::temp_dir().join(format!("core-native-owner-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let id = "12345678-1234-1234-1234-123456789abc";
        assert_eq!(
            connection(&root, id).unwrap(),
            "{\"schema\":\"codex-manager-task-connection-v1\",\"connection\":null}\n"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        let locks = root.join(".codex/thread-writer-locks");
        fs::create_dir_all(&locks).unwrap();
        let path = locks.join(format!("{id}.lock"));
        fs::write(&path, b"unknown writer").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(connection(&root, id).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"unknown writer");
        let coordination = locks.join(".coordination.lock");
        fs::write(&coordination, b"").unwrap();
        fs::set_permissions(&coordination, fs::Permissions::from_mode(0o600)).unwrap();
        let writer = fs::File::open(&path).unwrap();
        writer.try_lock().unwrap();
        let error = connection(&root, id).unwrap_err();
        assert!(error
            .to_string()
            .starts_with("native_owner_writer_unresolved:"));
        assert_eq!(fs::read(&path).unwrap(), b"unknown writer");
        drop(writer);
        assert!(connection(&root, id)
            .unwrap()
            .contains("\"connection\":null"));
        assert_eq!(fs::read(&path).unwrap(), b"unknown writer");
        fs::remove_dir_all(root).unwrap();
    }
}
