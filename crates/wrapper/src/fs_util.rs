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
            Err(err) if err.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return Err(io::Error::other(timeout_message)),
        }
    }
}

pub(super) fn wrapper_safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
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
