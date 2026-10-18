//! Immutable per-launch system settings; opaque optional fragments stay outside
//! Core policy. No account, model or notification interpretation lives here.
use super::shared_server::{immutable_record, private_dir, read_bounded};
use codex_runtime_binding::CONFIG_LIMIT;
use std::fs::{self, File};
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn invalid() -> io::Error {
    io::Error::other("Core system configuration snapshot is unavailable")
}

#[derive(Clone)]
pub(super) struct Bytes {
    pub config: Vec<u8>,
    pub requirements: Option<Vec<u8>>,
}
impl Bytes {
    pub fn read(root: &Path) -> io::Result<Self> {
        let config = read_bounded(&root.join("config.toml"), CONFIG_LIMIT)?;
        let requirements = match read_bounded(&root.join("requirements.toml"), CONFIG_LIMIT) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(Self {
            config,
            requirements,
        })
    }
    pub fn materialize(&self, root: &Path) -> io::Result<()> {
        immutable_record(&root.join("config.toml"), Some(&self.config), CONFIG_LIMIT)?;
        immutable_record(
            &root.join("requirements.toml"),
            self.requirements.as_deref(),
            CONFIG_LIMIT,
        )
    }
}

pub(super) struct Prepared {
    pub path: PathBuf,
    pub shared: Option<PathBuf>,
    _lease: File,
}

fn lock(file: &File, mode: i32) -> io::Result<()> {
    let until = Instant::now() + Duration::from_secs(10);
    while unsafe { super::flock(file.as_raw_fd(), mode | 4) } != 0 {
        if io::Error::last_os_error().kind() != io::ErrorKind::WouldBlock || Instant::now() >= until
        {
            return Err(invalid());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

pub(super) fn prepare(root: &Path, fragment: &[u8]) -> io::Result<Prepared> {
    // Serialize base/default generation only. Optional bytes are composed
    // locally and never published into this mutable shared directory.
    super::validate_core_config(root)?;
    let parent = File::open(root)?;
    lock(&parent, 2)?;
    let shared = super::prepare_core_shared_state_requirements(root)?;
    super::prepare_core_config(root, &[])?;
    let mut bytes = Bytes::read(root)?;
    bytes.config = super::render_core_config(fragment);
    drop(parent);
    let snapshots = root.join("snapshots");
    private_dir(&snapshots)?;
    let path = snapshots.join(codex_runtime_binding::settings_identity(
        &bytes.config,
        bytes.requirements.as_deref(),
    ));
    private_dir(&path)?;
    let lease = File::open(&path)?;
    // Equal settings can be prepared while a sibling holds its launch lease.
    // Each file uses no-replace publication and exact-byte collision checks.
    lock(&lease, 1)?;
    bytes.materialize(&path)?;
    Ok(Prepared {
        path,
        shared,
        _lease: lease,
    })
}

pub(super) fn validate(
    prepared: &Prepared,
    program: &std::ffi::OsStr,
    resolver: &Path,
    env: &super::TermuxBaseEnvPlan,
) -> io::Result<bool> {
    use std::os::unix::ffi::OsStrExt;
    use std::process::{Command, Stdio};
    let marker = [
        b"codex-system-validation-v1\0".as_slice(),
        program.as_bytes(),
    ]
    .concat();
    // One marker per exact immutable runtime path. Full bytes are compared.
    let key = codex_runtime_binding::settings_identity(&marker, None);
    let stamp = prepared.path.join(format!("validated-{key}"));
    match read_bounded(&stamp, 16384) {
        Ok(old) if old == marker => return Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        _ => return Err(invalid()),
    }
    let temporary = std::env::temp_dir().join(format!(
        "cv-{}-{}",
        std::process::id(),
        super::LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    private_dir(&temporary)?;
    let run = (|| -> io::Result<bool> {
        let fds = super::RuntimeFdSources::open(resolver, &prepared.path)?;
        let mut command = Command::new(program);
        command
            .args(["features", "list"])
            .env("HOME", &temporary)
            .env("CODEX_HOME", &temporary)
            .current_dir(&temporary)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        super::apply_child_env_plan_and_fence(&mut command, Some(env));
        fds.configure(&mut command);
        let mut child = command.spawn()?;
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status.success()),
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return match result {
                        Err(error) => Err(error),
                        _ => Ok(false),
                    };
                }
            }
        }
    })();
    let _ = fs::remove_dir_all(&temporary);
    if run.as_ref().is_ok_and(|valid| *valid) {
        match immutable_record(&stamp, Some(&marker), 16384) {
            Ok(()) => (),
            // Another launch may have completed the same validation concurrently.
            Err(_) if read_bounded(&stamp, 16384).is_ok_and(|old| old == marker) => (),
            Err(e) => return Err(e),
        }
    }
    run
}

pub(super) fn prune(root: &Path) -> io::Result<()> {
    let snapshots = root.join("snapshots");
    match fs::symlink_metadata(&snapshots) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    private_dir(&snapshots)?;
    let referenced = super::maintenance::process_references(&snapshots)?;
    'snapshot: for entry in fs::read_dir(&snapshots)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name.to_str() else { continue };
        if id.len() != 16 || !id.bytes().all(|b| b.is_ascii_hexdigit()) || referenced.contains(id) {
            continue;
        }
        let path = entry.path();
        private_dir(&path)?;
        let lease = File::open(&path)?;
        if unsafe { super::flock(lease.as_raw_fd(), 2 | 4) } != 0 {
            continue;
        }
        let bytes = Bytes::read(&path)?;
        if codex_runtime_binding::settings_identity(&bytes.config, bytes.requirements.as_deref())
            != id
        {
            return Err(invalid());
        }
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == "config.toml" || name == "requirements.toml" {
                continue;
            }
            let Some(key) = name.to_str().and_then(|s| s.strip_prefix("validated-")) else {
                continue 'snapshot;
            };
            let marker = read_bounded(&entry.path(), 16384)?;
            if !marker.starts_with(b"codex-system-validation-v1\0")
                || codex_runtime_binding::settings_identity(&marker, None) != key
            {
                continue 'snapshot;
            }
        }
        // The generation launch lease excludes new launches during this scan.
        fs::remove_dir_all(&path)?;
    }
    File::open(snapshots)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};

    #[test]
    fn preparation_probe() {
        let Some(root) = std::env::var_os("CODEX_SYSTEM_CONFIG_PROBE") else {
            return;
        };
        let root = PathBuf::from(root);
        let fragment = std::env::var("CODEX_SYSTEM_CONFIG_FRAGMENT").unwrap();
        let prepared = prepare(&root.join("config"), fragment.as_bytes()).unwrap();
        fs::write(
            root.join(format!("ready-{}", std::process::id())),
            prepared.path.as_os_str().as_encoded_bytes(),
        )
        .unwrap();
        let mut input = [0];
        std::io::stdin().read_exact(&mut input).unwrap();
    }

    #[test]
    fn concurrent_launches_keep_live_snapshots_and_pruning_preserves_owned_live_handles() {
        let root = crate::tests::temp_root("immutable-settings");
        let config = root.join("config");
        private_dir(&config).unwrap();
        let home = root.join("home");
        private_dir(&home).unwrap();
        let start = |fragment: &str| {
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "system_config::tests::preparation_probe",
                    "--exact",
                    "--nocapture",
                ])
                .env("CODEX_SYSTEM_CONFIG_PROBE", &root)
                .env("CODEX_SYSTEM_CONFIG_FRAGMENT", fragment)
                .env("HOME", &home)
                .env("CODEX_HOME", home.join(".codex"))
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let ready = root.join(format!("ready-{}", child.id()));
            let until = Instant::now() + Duration::from_secs(10);
            while !ready.exists() {
                if let Some(status) = child.try_wait().unwrap() {
                    panic!(
                        "snapshot child failed {status}: {}",
                        String::from_utf8_lossy(&child.wait_with_output().unwrap().stderr)
                    );
                }
                assert!(Instant::now() < until);
                std::thread::sleep(Duration::from_millis(10));
            }
            let path = PathBuf::from(String::from_utf8(fs::read(ready).unwrap()).unwrap());
            (child, path)
        };
        let (mut first, first_path) = start("# first optional settings\n");
        let first_bytes = fs::read(first_path.join("config.toml")).unwrap();
        let (mut equal, equal_path) = start("# first optional settings\n");
        assert_eq!(first_path, equal_path);
        equal.stdin.take().unwrap().write_all(&[1]).unwrap();
        assert!(equal.wait().unwrap().success());
        let (mut second, second_path) = start("# second optional settings\n");
        assert_ne!(first_path, second_path);
        assert_eq!(
            fs::read(first_path.join("config.toml")).unwrap(),
            first_bytes
        );
        assert_eq!(
            fs::read(config.join("config.toml")).unwrap(),
            super::super::render_core_config(&[])
        );
        prune(&config).unwrap();
        assert!(first_path.exists() && second_path.exists());
        first.stdin.take().unwrap().write_all(&[1]).unwrap();
        assert!(first.wait().unwrap().success());
        prune(&config).unwrap();
        assert!(!first_path.exists() && second_path.exists());
        fs::write(
            second_path.join("user-sentinel"),
            b"preserve unknown contents",
        )
        .unwrap();
        second.stdin.take().unwrap().write_all(&[1]).unwrap();
        assert!(second.wait().unwrap().success());
        prune(&config).unwrap();
        assert_eq!(
            fs::read(second_path.join("user-sentinel")).unwrap(),
            b"preserve unknown contents"
        );
        crate::tests::remove_temp_root(root);
    }
}
