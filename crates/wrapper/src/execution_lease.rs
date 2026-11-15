//! Physical directory lease only; no account/registry or sharing policy.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

unsafe extern "C" {
    fn geteuid() -> u32;
}
#[cfg(target_arch = "aarch64")]
const DIRECTORY: i32 = 0o40000;
#[cfg(not(target_arch = "aarch64"))]
const DIRECTORY: i32 = 0o200000;
#[cfg(target_arch = "aarch64")]
const NOFOLLOW: i32 = 0o100000;
#[cfg(not(target_arch = "aarch64"))]
const NOFOLLOW: i32 = 0o400000;

pub(super) fn acquire(execution: &Path) -> io::Result<Option<File>> {
    // Preserve native relative/alias paths and creation of missing homes. The
    // declared shared-home guard is separate and not part of this primitive.
    let physical = match fs::canonicalize(execution) {
        Ok(path) => path,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(DIRECTORY | NOFOLLOW)
        .open(&physical)?;
    let opened = directory.metadata()?;
    if opened.uid() != unsafe { geteuid() } {
        return Ok(None);
    }
    if unsafe { super::flock(directory.as_raw_fd(), 1 | super::FLOCK_NB) } != 0 {
        return Err(io::Error::other(
            "execution home is being changed; retry after the change",
        ));
    }
    let current = fs::metadata(execution)?;
    if current.dev() != opened.dev() || current.ino() != opened.ino() {
        return Err(io::Error::other("execution home changed"));
    }
    // Reserve the native FD only after the owned directory is locked and bound.
    let safe = unsafe {
        super::fcntl(
            directory.as_raw_fd(),
            super::F_DUPFD_CLOEXEC,
            super::SAFE_MIN_FD,
        )
    };
    if safe < 0 {
        return Err(io::Error::last_os_error());
    }
    let safe = unsafe { File::from_raw_fd(safe) };
    drop(directory);
    if unsafe { super::dup2(safe.as_raw_fd(), 36) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Some(unsafe { File::from_raw_fd(36) }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn isolated(name: &str) -> bool {
        const ROLE: &str = "CODEX_EXECUTION_LEASE_TEST";
        if std::env::var(ROLE).as_deref() == Ok(name) {
            return true;
        }
        let selector = format!("execution_lease::tests::{name}");
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &selector, "--nocapture"])
            .env(ROLE, name)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        false
    }
    #[test]
    fn lease_tracks_inode_and_descendants_without_registry_policy() {
        if !isolated("lease_tracks_inode_and_descendants_without_registry_policy") {
            return;
        }
        let root = super::super::tests::temp_root("profile-lease");
        let profiles = root.join(".local/share/codex/manager/profiles");
        let home = profiles.join("work/home");
        fs::create_dir_all(&home).unwrap();
        fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
        let lease = acquire(&home).unwrap().unwrap();
        let competing = File::open(&home).unwrap();
        assert_ne!(
            unsafe { super::super::flock(competing.as_raw_fd(), 2 | 4) },
            0
        );
        assert_eq!(
            fs::metadata("/proc/self/fd/36").unwrap().ino(),
            fs::metadata(&home).unwrap().ino()
        );
        let mut child = std::process::Command::new(
            std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into()),
        )
        .args(["-c", "test -d /proc/self/fd/36 && sleep 1"])
        .spawn()
        .unwrap();
        drop(lease);
        assert_ne!(
            unsafe { super::super::flock(competing.as_raw_fd(), 2 | 4) },
            0
        );
        assert!(child.wait().unwrap().success());
        assert_eq!(
            unsafe { super::super::flock(competing.as_raw_fd(), 2 | 4) },
            0
        );
        assert!(acquire(&home).is_err());
        drop(competing);
        // A registry-only lock no longer denies unrelated Core execution.
        let parent = File::open(&profiles).unwrap();
        assert_eq!(unsafe { super::super::flock(parent.as_raw_fd(), 2 | 4) }, 0);
        drop(acquire(&home).unwrap().unwrap());
        drop(parent);
        // Mode/managed-path policy belongs to the declared-layout guard.
        fs::set_permissions(&home, fs::Permissions::from_mode(0o755)).unwrap();
        drop(acquire(&home).unwrap().unwrap());
        fs::remove_dir(&home).unwrap();
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, &home).unwrap();
        let aliased = acquire(&home).unwrap().unwrap();
        assert_eq!(
            aliased.metadata().unwrap().ino(),
            fs::metadata(&outside).unwrap().ino()
        );
        drop(aliased);
        assert!(acquire(&root.join("not-created")).unwrap().is_none());
        for path in [root.join(".codex"), root.join("isolated")] {
            fs::create_dir(&path).unwrap();
            let lease = acquire(&path).unwrap().unwrap();
            let peer = File::open(&path).unwrap();
            assert_ne!(unsafe { crate::flock(peer.as_raw_fd(), 2 | 4) }, 0);
            drop(lease);
            assert_eq!(unsafe { crate::flock(peer.as_raw_fd(), 2 | 4) }, 0);
        }
        let previous_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        let relative = acquire(Path::new("outside")).unwrap().unwrap();
        assert_eq!(
            relative.metadata().unwrap().ino(),
            fs::metadata(&outside).unwrap().ino()
        );
        drop(relative);
        std::env::set_current_dir(previous_cwd).unwrap();

        super::super::tests::remove_temp_root(root);
    }

    #[test]
    fn retention_and_execution_leases_survive_reserved_descriptor_pressure() {
        if !isolated("retention_and_execution_leases_survive_reserved_descriptor_pressure") {
            return;
        }
        let root = super::super::tests::temp_root("profile-lease-fd-pressure");
        let home = root.join(".local/share/codex/manager/profiles/work/home");
        fs::create_dir_all(&home).unwrap();
        fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
        // Only this test process's inherited descriptor is closed. No real path
        // is opened or mutated; reserve every lower gap so acquisition reaches 36.
        if unsafe { super::super::fcntl(36, 1) } >= 0 {
            drop(unsafe { File::from_raw_fd(36) });
        }
        let mut fillers = Vec::new();
        loop {
            let file = File::open("/dev/null").unwrap();
            if file.as_raw_fd() == 36 {
                drop(file);
                break;
            }
            assert!(file.as_raw_fd() < 36);
            fillers.push(file);
        }
        let generations = root.join("generations");
        fs::create_dir(&generations).unwrap();
        let retention = crate::maintenance::launch_lease(&generations)
            .unwrap()
            .unwrap();
        assert!(retention.as_raw_fd() >= crate::SAFE_MIN_FD);
        let retention_peer = File::open(&root).unwrap();
        assert_ne!(
            unsafe { crate::flock(retention_peer.as_raw_fd(), 2 | 4) },
            0
        );
        drop(retention_peer);
        let lease = acquire(&home).unwrap().unwrap();
        assert_eq!(lease.as_raw_fd(), 36);
        assert_eq!(
            lease.metadata().unwrap().ino(),
            fs::metadata(&home).unwrap().ino()
        );
        let competing = File::open(&home).unwrap();
        assert_ne!(
            unsafe { super::super::flock(competing.as_raw_fd(), 2 | 4) },
            0
        );
        drop(lease);
        assert_eq!(
            unsafe { super::super::flock(competing.as_raw_fd(), 2 | 4) },
            0
        );
        drop(competing);
        let retention_peer = File::open(&root).unwrap();
        assert_ne!(
            unsafe { crate::flock(retention_peer.as_raw_fd(), 2 | 4) },
            0
        );
        drop(retention);
        assert_eq!(
            unsafe { crate::flock(retention_peer.as_raw_fd(), 2 | 4) },
            0
        );
        drop(retention_peer);
        drop(fillers);
        super::super::tests::remove_temp_root(root);
    }
}
