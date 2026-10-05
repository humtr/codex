//! Physical execution lease. Manager alone owns registration/lifecycle policy.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path};

unsafe extern "C" {
    fn geteuid() -> u32;
}
// AArch64 uses these flags on both musl Linux and Android/Bionic.
#[cfg(target_arch = "aarch64")]
const DIRECTORY: i32 = 0o40000;
#[cfg(not(target_arch = "aarch64"))]
const DIRECTORY: i32 = 0o200000;
#[cfg(target_arch = "aarch64")]
const NOFOLLOW: i32 = 0o100000;
#[cfg(not(target_arch = "aarch64"))]
const NOFOLLOW: i32 = 0o400000;

fn real_directory(path: &Path) -> io::Result<File> {
    let mut current = std::path::PathBuf::new();
    for part in path.components() {
        if !matches!(part, Component::RootDir | Component::Normal(_)) {
            return Err(io::Error::other("invalid profile execution path"));
        }
        current.push(part);
        let m = fs::symlink_metadata(&current)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(io::Error::other("invalid profile execution path"));
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(DIRECTORY | NOFOLLOW)
        .open(path)?;
    let m = file.metadata()?;
    if m.uid() != unsafe { geteuid() } || m.mode() & 0o022 != 0 {
        return Err(io::Error::other("unsafe profile execution directory"));
    }
    Ok(file)
}

fn shared_lock(file: &File) -> io::Result<()> {
    if unsafe { super::flock(file.as_raw_fd(), 1 | super::FLOCK_NB) } != 0 {
        return Err(io::Error::other(
            "profile is being changed; retry after the change",
        ));
    }
    Ok(())
}

pub(super) fn acquire(home: &Path, execution: &Path) -> io::Result<Option<File>> {
    let profiles = home.join(".local/share/codex/manager/profiles");
    let Ok(relative) = execution.strip_prefix(&profiles) else {
        return Ok(None);
    };
    let parts: Vec<_> = relative.components().collect();
    let [Component::Normal(id), Component::Normal(leaf)] = parts.as_slice() else {
        return Ok(None);
    };
    if *leaf != "home"
        || id.to_str().is_none_or(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
    {
        return Ok(None);
    }
    let parent = real_directory(&profiles)?;
    shared_lock(&parent)?;
    let profile = real_directory(execution)?;
    if profile.metadata()?.mode() & 0o7777 != 0o700 {
        return Err(io::Error::other("unsafe profile execution home"));
    }
    shared_lock(&profile)?;
    let physical = fs::symlink_metadata(execution)?;
    let opened = profile.metadata()?;
    if physical.dev() != opened.dev() || physical.ino() != opened.ino() {
        return Err(io::Error::other("profile execution home changed"));
    }
    // Reserve an inheritable descriptor for native TUI and persistent server.
    // Other runtime sources are duplicated above all reserved descriptors.
    let safe = unsafe {
        super::fcntl(
            profile.as_raw_fd(),
            super::F_DUPFD_CLOEXEC,
            super::SAFE_MIN_FD,
        )
    };
    if safe < 0 {
        return Err(io::Error::last_os_error());
    }
    let safe = unsafe { File::from_raw_fd(safe) };
    // Either original directory can itself occupy FD36 under descriptor pressure.
    // Close those owners before installing the reserved inheritable descriptor.
    drop(profile);
    drop(parent);
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
        const ROLE: &str = "CODEX_PROFILE_LEASE_TEST";
        if std::env::var(ROLE).as_deref() == Ok(name) {
            return true;
        }
        let selector = format!("profile_lease::tests::{name}");
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
    fn profile_lease_blocks_mutation_until_release_and_rejects_unsafe_paths() {
        if !isolated("profile_lease_blocks_mutation_until_release_and_rejects_unsafe_paths") {
            return;
        }
        let root = super::super::tests::temp_root("profile-lease");
        let profiles = root.join(".local/share/codex/manager/profiles");
        let home = profiles.join("work/home");
        fs::create_dir_all(&home).unwrap();
        fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
        let lease = acquire(&root, &home).unwrap().unwrap();
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
        assert!(acquire(&root, &home).is_err());
        drop(competing);
        let parent = File::open(&profiles).unwrap();
        assert_eq!(unsafe { super::super::flock(parent.as_raw_fd(), 2 | 4) }, 0);
        assert!(acquire(&root, &home).is_err());
        drop(parent);
        fs::set_permissions(&home, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(acquire(&root, &home).is_err());
        fs::remove_dir(&home).unwrap();
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, &home).unwrap();
        assert!(acquire(&root, &home).is_err());
        assert!(acquire(&root, &outside).unwrap().is_none());
        super::super::tests::remove_temp_root(root);
    }

    #[test]
    fn profile_lease_survives_reserved_descriptor_pressure() {
        if !isolated("profile_lease_survives_reserved_descriptor_pressure") {
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
        let lease = acquire(&root, &home).unwrap().unwrap();
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
        drop(fillers);
        super::super::tests::remove_temp_root(root);
    }
}
