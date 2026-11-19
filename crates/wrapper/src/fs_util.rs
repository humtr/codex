use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _};
use std::os::fd::AsRawFd as _;
use std::os::raw::c_int;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, Instant};

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub(super) const O_NOFOLLOW: c_int = 0o100000;
#[cfg(not(any(target_arch = "aarch64", target_arch = "arm")))]
pub(super) const O_NOFOLLOW: c_int = 0o400000;

pub(super) const LOCK_SH: c_int = 1;
pub(super) const LOCK_EX: c_int = 2;
pub(super) const LOCK_NB: c_int = 4;

unsafe extern "C" {
    fn flock(fd: c_int, operation: c_int) -> c_int;
}

fn invalid() -> io::Error {
    io::Error::other("private directory path is unavailable")
}

pub(super) fn current_uid() -> io::Result<u32> {
    fs::metadata("/proc/self").map(|m| m.uid())
}

pub(super) fn flock_fd(file: &File, operation: c_int) -> io::Result<()> {
    if unsafe { flock(file.as_raw_fd(), operation) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(super) fn flock_with_timeout(
    file: &File,
    mode: c_int,
    timeout: Duration,
    timeout_message: &'static str,
) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        match flock_fd(file, mode | LOCK_NB) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, timeout_message));
                }
                std::thread::sleep(remaining.min(Duration::from_millis(10)));
            }
            Err(err) => return Err(err),
        }
    }
}

pub(super) fn wrapper_safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && !path.as_os_str().as_encoded_bytes().contains(&0)
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}

pub(super) fn read_bounded_regular(
    path: &Path,
    max_bytes: u64,
    reject_group_world_write: bool,
    invalid_message: &'static str,
) -> io::Result<Vec<u8>> {
    if !wrapper_safe_absolute_path(path) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, invalid_message));
    }
    let sym_meta = fs::symlink_metadata(path)?;
    if !sym_meta.file_type().is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, invalid_message));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)?;
    let meta = file.metadata()?;
    let expected_uid = current_uid()?;
    if !meta.file_type().is_file()
        || meta.dev() != sym_meta.dev()
        || meta.ino() != sym_meta.ino()
        || meta.uid() != expected_uid
        || (reject_group_world_write && meta.permissions().mode() & 0o022 != 0)
        || meta.len() > max_bytes
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, invalid_message));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, invalid_message));
    }
    Ok(bytes)
}

pub(super) fn private_dir(path: &Path) -> io::Result<()> {
    if !wrapper_safe_absolute_path(path) {
        return Err(invalid());
    }
    // Reject substituted parents as well as the final directory.
    for parent in path.ancestors().skip(1) {
        let m = fs::symlink_metadata(parent)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(invalid());
        }
    }
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => File::open(path.parent().ok_or_else(invalid)?)?.sync_all()?,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    }
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != current_uid()?
        || m.permissions().mode() & 0o777 != 0o700
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flock_waits_for_contention_and_propagates_os_errors() {
        let root = crate::tests::temp_root("flock-timeout");
        let holder = File::open(&root).unwrap();
        let waiter = File::open(&root).unwrap();
        flock_fd(&holder, LOCK_EX).unwrap();
        let start = Instant::now();
        let err = flock_with_timeout(
            &waiter,
            LOCK_EX,
            Duration::from_millis(25),
            "lease timed out",
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert_eq!(err.to_string(), "lease timed out");
        assert!(start.elapsed() >= Duration::from_millis(25));
        let unlock = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            drop(holder);
        });
        flock_with_timeout(&waiter, LOCK_EX, Duration::from_secs(2), "lease timed out").unwrap();
        unlock.join().unwrap();

        // LOCK_NB without a lock operation is invalid at the OS boundary.
        let expected = flock_fd(&waiter, LOCK_NB).unwrap_err().raw_os_error();
        let actual = flock_with_timeout(&waiter, 0, Duration::ZERO, "lease timed out").unwrap_err();
        assert_eq!(actual.raw_os_error(), expected);
        assert!(expected.is_some());
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn bounded_regular_read_rejects_symlink_size_and_unsafe_mode() {
        let root = crate::tests::temp_root("bounded-regular");
        let file = root.join("file");
        let link = root.join("link");
        fs::write(&file, b"four").unwrap();
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let read =
            |path, size, private| read_bounded_regular(path, size, private, "invalid test file");
        assert_eq!(read(&file, 4, true).unwrap(), b"four");
        assert_eq!(
            read(&link, 4, false).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            read(&file, 3, false).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::set_permissions(&file, fs::Permissions::from_mode(0o622)).unwrap();
        assert_eq!(
            read(&file, 4, true).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(read(&file, 4, false).unwrap(), b"four");
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn safe_absolute_paths_reject_parent_traversal_and_relative_paths() {
        for path in ["/", "/a/b", "/a//b", "/a/./b"] {
            assert!(wrapper_safe_absolute_path(Path::new(path)), "{path}");
        }
        for path in ["", ".", "a/b", "../a", "/a/../b", "/..", "/a/\0b"] {
            assert!(!wrapper_safe_absolute_path(&PathBuf::from(path)), "{path}");
        }
    }
}
