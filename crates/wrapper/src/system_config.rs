//! Immutable opt-in FD 34 system configuration snapshots for Stage 2 extensions.
//! Bare Standalone execution bypasses this module and retains the zero-write fast path.
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::fs_util;
use crate::unix_app::{
    apply_child_env_plan_and_fence, rename_noreplace, RuntimeFdSources, TermuxBaseEnvPlan,
    LOCAL_STAGING_COUNTER, TERMUX_WRAPPER_CONFIG_CONTENT,
};

pub(super) const FRAGMENT_LIMIT: usize = 8 * 1024;
pub(super) const CONFIG_LIMIT: u64 = 16 * 1024;
const VALIDATION_MARKER_PREFIX: &[u8] = b"codex-system-validation-v1\0";
const LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const VALIDATION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ExtensionFd34Input<'a> {
    pub(super) fragment: Option<&'a [u8]>,
    pub(super) requirements: Option<&'a [u8]>,
}

impl ExtensionFd34Input<'_> {
    pub(super) fn is_empty(self) -> bool {
        self.fragment.is_none() && self.requirements.is_none()
    }
}

pub(super) struct PreparedSnapshot {
    pub(super) path: PathBuf,
    // Shared flock kept alive across exec via inherited FD 34 protects active snapshots from pruning.
    pub(super) lease: File,
}

fn invalid() -> io::Error {
    io::Error::other("wrapper FD 34 system configuration snapshot is unavailable")
}

/// Deterministic length-prefixed 64-bit identity matching `codex-runtime-binding` `config-v1`.
pub(super) fn settings_identity(config: &[u8], requirements: Option<&[u8]>) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut field = |bytes: Option<&[u8]>| {
        let length = bytes.map_or(u64::MAX, |b| b.len() as u64).to_le_bytes();
        for byte in length.iter().chain(bytes.unwrap_or_default()) {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
        }
    };
    field(Some(b"config-v1"));
    field(Some(b"/"));
    field(Some(b"core-system-config-v1"));
    field(None);
    field(Some(config));
    field(requirements);
    format!("{hash:016x}")
}

pub(super) fn render_snapshot_config(fragment: &[u8]) -> io::Result<Vec<u8>> {
    if fragment.len() > FRAGMENT_LIMIT || fragment.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "extension FD 34 config fragment exceeds limit or contains NUL",
        ));
    }
    let mut bytes = Vec::with_capacity(
        TERMUX_WRAPPER_CONFIG_CONTENT.len()
            + "thread_unload_delay_secs = 0\n\n".len()
            + fragment.len(),
    );
    bytes.extend_from_slice(TERMUX_WRAPPER_CONFIG_CONTENT.as_bytes());
    bytes.extend_from_slice(b"thread_unload_delay_secs = 0\n\n");
    bytes.extend_from_slice(fragment);
    if bytes.len() as u64 > CONFIG_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rendered FD 34 system config exceeds size limit",
        ));
    }
    Ok(bytes)
}

fn immutable_record(path: &Path, expected: Option<&[u8]>, limit: u64) -> io::Result<()> {
    match (
        fs_util::read_bounded_regular(
            path,
            limit,
            true,
            "immutable FD 34 snapshot file must be bounded, owned, and non-writable",
        ),
        expected,
    ) {
        (Ok(old), Some(bytes)) if old == bytes => return Ok(()),
        (Err(e), None) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        (Err(e), Some(_)) if e.kind() == io::ErrorKind::NotFound => (),
        _ => return Err(invalid()),
    }
    let bytes = expected.ok_or_else(invalid)?;
    if bytes.len() as u64 > limit {
        return Err(invalid());
    }
    let seq = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let parent = path.parent().ok_or_else(invalid)?;
    let temp = parent.join(format!(".snapshot-{}-{seq}.tmp", std::process::id(),));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    let result = (|| -> io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        rename_noreplace(&temp, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub(super) fn prepare(
    config_root: &Path,
    input: ExtensionFd34Input<'_>,
) -> io::Result<PreparedSnapshot> {
    if input.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "snapshot preparation requires an extension config fragment or requirements",
        ));
    }
    if let Some(req) = input.requirements {
        if req.len() as u64 > CONFIG_LIMIT || req.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "extension FD 34 requirements exceed limit or contain NUL",
            ));
        }
    }
    let config = render_snapshot_config(input.fragment.unwrap_or(&[]))?;
    let requirements = input.requirements;

    fs_util::private_dir(config_root)?;
    let parent = File::open(config_root)?;
    fs_util::flock_with_timeout(
        &parent,
        fs_util::LOCK_EX,
        LOCK_TIMEOUT,
        "wrapper FD 34 configuration directory lock busy",
    )?;
    let snapshots = config_root.join("snapshots");
    fs_util::private_dir(&snapshots)?;
    drop(parent);

    let settings_id = settings_identity(&config, requirements);
    let path = snapshots.join(&settings_id);
    fs_util::private_dir(&path)?;
    let lease = File::open(&path)?;
    fs_util::flock_with_timeout(
        &lease,
        fs_util::LOCK_SH,
        LOCK_TIMEOUT,
        "wrapper FD 34 snapshot lock busy",
    )?;

    immutable_record(&path.join("config.toml"), Some(&config), CONFIG_LIMIT)?;
    immutable_record(&path.join("requirements.toml"), requirements, CONFIG_LIMIT)?;

    Ok(PreparedSnapshot { path, lease })
}

pub(super) fn validate(
    prepared: &PreparedSnapshot,
    program: &OsStr,
    resolver_path: &Path,
    env_plan: &TermuxBaseEnvPlan,
) -> io::Result<bool> {
    validate_with_timeout(
        prepared,
        program,
        resolver_path,
        env_plan,
        VALIDATION_TIMEOUT,
    )
}

fn validate_with_timeout(
    prepared: &PreparedSnapshot,
    program: &OsStr,
    resolver_path: &Path,
    env_plan: &TermuxBaseEnvPlan,
    timeout: Duration,
) -> io::Result<bool> {
    let marker = [VALIDATION_MARKER_PREFIX, program.as_bytes()].concat();
    let key = settings_identity(&marker, None);
    let stamp = prepared.path.join(format!("validated-{key}"));
    match fs_util::read_bounded_regular(
        &stamp,
        CONFIG_LIMIT,
        true,
        "FD 34 snapshot validation marker is unsafe",
    ) {
        Ok(old) if old == marker => return Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        _ => return Err(invalid()),
    }

    let seq = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = std::env::temp_dir().join(format!("cv-{}-{seq}", std::process::id()));
    fs_util::private_dir(&temporary)?;
    let run = (|| -> io::Result<bool> {
        let fds = RuntimeFdSources::open_with_config_file(resolver_path, &prepared.lease)?;
        let mut command = Command::new(program);
        command.args(["features", "list"]);
        apply_child_env_plan_and_fence(&mut command, Some(env_plan));
        command
            .env("HOME", &temporary)
            .env("CODEX_HOME", &temporary)
            .current_dir(&temporary)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        fds.configure(&mut command);
        let mut child = command.spawn()?;
        let until = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status.success()),
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return match result {
                        Err(err) => Err(err),
                        _ => Ok(false),
                    };
                }
            }
        }
    })();
    let _ = fs::remove_dir_all(&temporary);
    if run.as_ref().is_ok_and(|&valid| valid) {
        match immutable_record(&stamp, Some(&marker), CONFIG_LIMIT) {
            Ok(()) => (),
            Err(_)
                if fs_util::read_bounded_regular(
                    &stamp,
                    CONFIG_LIMIT,
                    true,
                    "FD 34 snapshot validation marker is unsafe",
                )
                .is_ok_and(|old| old == marker) => {}
            Err(err) => return Err(err),
        }
    }
    run
}

pub(super) fn prune(config_root: &Path) -> io::Result<()> {
    let snapshots = config_root.join("snapshots");
    match fs::symlink_metadata(&snapshots) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    fs_util::private_dir(&snapshots)?;
    'snapshot: for entry in fs::read_dir(&snapshots)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name.to_str() else { continue };
        if id.len() != 16
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        {
            continue;
        }
        let path = entry.path();
        fs_util::private_dir(&path)?;
        let lease = File::open(&path)?;
        match fs_util::flock_fd(&lease, fs_util::LOCK_EX | fs_util::LOCK_NB) {
            Ok(()) => (),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
            Err(err) => return Err(err),
        }
        let config = fs_util::read_bounded_regular(
            &path.join("config.toml"),
            CONFIG_LIMIT,
            true,
            "FD 34 snapshot config.toml is unsafe",
        )?;
        let requirements = match fs_util::read_bounded_regular(
            &path.join("requirements.toml"),
            CONFIG_LIMIT,
            true,
            "FD 34 snapshot requirements.toml is unsafe",
        ) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        if settings_identity(&config, requirements.as_deref()) != id {
            return Err(invalid());
        }
        for child in fs::read_dir(&path)? {
            let child = child?;
            let cname = child.file_name();
            if cname == "config.toml" || cname == "requirements.toml" {
                continue;
            }
            let Some(key) = cname.to_str().and_then(|s| s.strip_prefix("validated-")) else {
                continue 'snapshot;
            };
            let marker = fs_util::read_bounded_regular(
                &child.path(),
                CONFIG_LIMIT,
                true,
                "FD 34 snapshot validation marker is unsafe",
            )?;
            if !marker.starts_with(VALIDATION_MARKER_PREFIX)
                || settings_identity(&marker, None) != key
            {
                continue 'snapshot;
            }
        }
        fs::remove_dir_all(&path)?;
    }
    File::open(snapshots)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{remove_temp_root, temp_root};
    use crate::unix_app::plan_termux_env;
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt as _;

    fn shell_executable() -> &'static str {
        if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        }
    }

    fn test_env_and_resolver(root: &Path) -> (PathBuf, TermuxBaseEnvPlan) {
        let prefix = root.join("prefix");
        let gen = root.join("gen");
        let bin = prefix.join("bin");
        let tmp = root.join("tmp");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&gen).unwrap();
        fs::create_dir_all(&tmp).unwrap();
        let resolver = root.join("resolv.conf");
        fs::write(&resolver, b"nameserver 1.1.1.1\n").unwrap();
        let snap = crate::unix_app::TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(prefix.into_os_string()),
            tmpdir: Some(tmp.into_os_string()),
            inherited_path: Some(OsString::from("/system/bin:/bin:/usr/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let plan = plan_termux_env(
            &snap,
            gen.as_os_str(),
            None,
            OsStr::new("/etc/tls/cert.pem"),
            None,
        )
        .unwrap();
        (resolver, plan)
    }

    #[test]
    fn settings_identity_avoids_concatenation_and_absence_ambiguity() {
        let a = settings_identity(b"cfg", None);
        let b = settings_identity(b"cfg", Some(b""));
        assert_eq!(a.len(), 16);
        assert_ne!(a, b);
        assert_ne!(
            settings_identity(b"ab", Some(b"c")),
            settings_identity(b"a", Some(b"bc"))
        );
    }

    #[test]
    fn prepare_enforces_bounds_and_immutable_collision_checks() {
        let root = temp_root("sys-cfg-prepare");
        let config = root.join("config");
        let oversize = vec![b'a'; FRAGMENT_LIMIT + 1];
        assert!(prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(&oversize),
                requirements: None,
            }
        )
        .is_err());

        let first = prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(b"# ext A\n"),
                requirements: Some(b"sqlite_home = \"/tmp/s\"\n"),
            },
        )
        .unwrap();
        let same = prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(b"# ext A\n"),
                requirements: Some(b"sqlite_home = \"/tmp/s\"\n"),
            },
        )
        .unwrap();
        assert_eq!(first.path, same.path);
        let cfg_text = fs::read_to_string(first.path.join("config.toml")).unwrap();
        assert!(cfg_text.contains("thread_unload_delay_secs = 0"));
        assert!(cfg_text.ends_with("# ext A\n"));
        assert!(first.path.join("requirements.toml").is_file());

        // Tampered immutable file must fail closed on reuse.
        drop(same);
        drop(first);
        let target_cfg = config
            .join("snapshots")
            .join(settings_identity(
                &render_snapshot_config(b"# ext A\n").unwrap(),
                Some(b"sqlite_home = \"/tmp/s\"\n"),
            ))
            .join("config.toml");
        fs::write(&target_cfg, b"# tampered\n").unwrap();
        assert!(prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(b"# ext A\n"),
                requirements: Some(b"sqlite_home = \"/tmp/s\"\n"),
            },
        )
        .is_err());
        remove_temp_root(root);
    }

    #[test]
    fn validate_caches_features_list_success_and_rejects_failure() {
        let root = temp_root("sys-cfg-validate");
        let (resolver, env_plan) = test_env_and_resolver(&root);
        let config = root.join("config");

        let ok_prog = root.join("codex-ok.sh");
        let marker = root.join("features-ran");
        fs::write(
            &ok_prog,
            format!(
                "#!{}\n[ \"$1\" = features ] && [ \"$2\" = list ] && echo ok >> '{}' && exit 0\nexit 3\n",
                shell_executable(),
                marker.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&ok_prog, fs::Permissions::from_mode(0o700)).unwrap();

        let snap = prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(b"# ok\n"),
                requirements: None,
            },
        )
        .unwrap();
        assert!(validate(&snap, ok_prog.as_os_str(), &resolver, &env_plan).unwrap());
        assert_eq!(fs::read_to_string(&marker).unwrap().lines().count(), 1);
        // Second call hits immutable validation stamp without re-spawning.
        assert!(validate(&snap, ok_prog.as_os_str(), &resolver, &env_plan).unwrap());
        assert_eq!(fs::read_to_string(&marker).unwrap().lines().count(), 1);

        let bad_prog = root.join("codex-fail.sh");
        fs::write(&bad_prog, format!("#!{}\nexit 4\n", shell_executable())).unwrap();
        fs::set_permissions(&bad_prog, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!validate(&snap, bad_prog.as_os_str(), &resolver, &env_plan).unwrap());

        let slow_prog = root.join("codex-slow.sh");
        fs::write(
            &slow_prog,
            format!("#!{}\nsleep 2\nexit 0\n", shell_executable()),
        )
        .unwrap();
        fs::set_permissions(&slow_prog, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!validate_with_timeout(
            &snap,
            slow_prog.as_os_str(),
            &resolver,
            &env_plan,
            Duration::from_millis(120)
        )
        .unwrap());
        remove_temp_root(root);
    }

    #[test]
    fn prune_preserves_fd34_locked_snapshot_and_removes_unheld_snapshot() {
        let root = temp_root("sys-cfg-prune");
        let (resolver, _) = test_env_and_resolver(&root);
        let config = root.join("config");

        let live = prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(b"# live\n"),
                requirements: None,
            },
        )
        .unwrap();
        let stale = prepare(
            &config,
            ExtensionFd34Input {
                fragment: Some(b"# stale\n"),
                requirements: None,
            },
        )
        .unwrap();
        let live_path = live.path.clone();
        let stale_path = stale.path.clone();
        drop(stale);

        // Spawn a real child holding FD 34 onto `live` while dropping parent's `live.lease`.
        let ready = root.join("child-ready");
        let fds = RuntimeFdSources::open_with_config_file(&resolver, &live.lease).unwrap();
        let mut cmd = Command::new(shell_executable());
        cmd.arg("-c").arg(format!(
            "echo ok > '{}'; while [ -f '{}' ]; do sleep 0.05; done",
            ready.display(),
            ready.display()
        ));
        fds.configure(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        drop(live);
        drop(fds);

        let until = Instant::now() + Duration::from_secs(5);
        while !ready.exists() {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(10));
        }

        prune(&config).unwrap();
        assert!(live_path.exists());
        assert!(!stale_path.exists());

        let _ = fs::remove_file(&ready);
        assert!(child.wait().unwrap().success());
        prune(&config).unwrap();
        assert!(!live_path.exists());
        remove_temp_root(root);
    }
}
