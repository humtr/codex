use crate::error::LocalProductError;
use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read as _};
use std::os::fd::AsRawFd as _;
use std::os::raw::c_int;
const O_NONBLOCK_LINUX: c_int = 0o4000;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub(super) const O_NOFOLLOW: c_int = 0o100000;
#[cfg(not(any(target_arch = "aarch64", target_arch = "arm")))]
pub(super) const O_NOFOLLOW: c_int = 0o400000;

pub(super) const LOCK_SH: c_int = 1;
pub(super) const LOCK_EX: c_int = 2;
pub(super) const LOCK_NB: c_int = 4;
pub(super) const LOCK_UN: c_int = 8;

unsafe extern "C" {
    pub(super) fn flock(fd: c_int, operation: c_int) -> c_int;
    pub(super) fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
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

pub(super) fn is_real_directory(metadata: &Metadata) -> bool {
    metadata.file_type().is_dir()
}

pub(super) fn is_owned_directory(metadata: &Metadata, uid: u32) -> bool {
    is_real_directory(metadata) && metadata.uid() == uid
}

pub(super) fn ensure_real_directories<'a>(
    directories: impl Iterator<Item = &'a Path>,
    invalid: fn() -> io::Error,
) -> io::Result<()> {
    for path in directories {
        if !is_real_directory(&fs::symlink_metadata(path)?) {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn private_dir(path: &Path) -> io::Result<()> {
    if !wrapper_safe_absolute_path(path) {
        return Err(invalid());
    }
    // Reject substituted parents as well as the final directory.
    ensure_real_directories(path.ancestors().skip(1), invalid)?;
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => sync_directory(path.parent().ok_or_else(invalid)?)?,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    }
    let m = fs::symlink_metadata(path)?;
    if !is_real_directory(&m)
        || m.uid() != current_uid()?
        || m.permissions().mode() & 0o777 != 0o700
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) static LOCAL_STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);
fn inspect_real_directory(
    path: &Path,
    operation: &'static str,
    message: &'static str,
    allow_missing: bool,
) -> Result<(), LocalProductError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if is_real_directory(&meta) => Ok(()),
        Ok(_) => Err(LocalProductError::UnsafeSource(message)),
        Err(err) if allow_missing && err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LocalProductError::Io { operation, source }),
    }
}

pub(super) fn ensure_real_directory(
    path: &Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    inspect_real_directory(path, operation, message, false)
}

pub(super) fn ensure_real_directory_if_present(
    path: &Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    inspect_real_directory(path, operation, message, true)
}

pub(super) fn ensure_real_directory_tree(
    path: &Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    if !wrapper_safe_absolute_path(path) {
        return Err(LocalProductError::UnsafeSource(message));
    }
    let mut cursor = PathBuf::new();
    for component in path.components() {
        cursor.push(component);
        if component == std::path::Component::RootDir {
            continue;
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    // Termux may have symlinked ancestors. The leaf must be a real directory.
                    if cursor == path {
                        return Err(LocalProductError::UnsafeSource(message));
                    }
                    let target_meta = fs::metadata(&cursor)
                        .map_err(|source| LocalProductError::Io { operation, source })?;
                    if !target_meta.is_dir() {
                        return Err(LocalProductError::UnsafeSource(message));
                    }
                } else if !is_real_directory(&meta) {
                    return Err(LocalProductError::UnsafeSource(message));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                builder.mode(0o700);
                builder
                    .create(&cursor)
                    .map_err(|source| LocalProductError::Io { operation, source })?;
            }
            Err(source) => return Err(LocalProductError::Io { operation, source }),
        }
    }
    Ok(())
}

pub(super) fn ensure_regular_file(
    path: &Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    let meta =
        fs::symlink_metadata(path).map_err(|source| LocalProductError::Io { operation, source })?;
    if !meta.file_type().is_file() {
        return Err(LocalProductError::UnsafeSource(message));
    }
    Ok(())
}

pub(super) fn read_bounded_regular_file(
    path: &Path,
    limit: usize,
    operation: &'static str,
    over_limit: LocalProductError,
    not_regular: LocalProductError,
) -> Result<Vec<u8>, LocalProductError> {
    let sym_meta =
        fs::symlink_metadata(path).map_err(|source| LocalProductError::Io { operation, source })?;
    if !sym_meta.file_type().is_file() {
        return Err(not_regular);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK_LINUX)
        .open(path)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    let file_meta = file
        .metadata()
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if !file_meta.is_file()
        || file_meta.nlink() != 1
        || file_meta.dev() != sym_meta.dev()
        || file_meta.ino() != sym_meta.ino()
    {
        return Err(not_regular);
    }
    let mut bytes = Vec::new();
    io::Read::take(file, limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if bytes.len() > limit {
        return Err(over_limit);
    }
    Ok(bytes)
}

pub(super) fn copy_local_directory_tree(
    source: &Path,
    destination: &Path,
) -> Result<(), LocalProductError> {
    copy_local_directory_tree_inner(source, destination, true)
}

fn copy_local_directory_tree_inner(
    source: &Path,
    destination: &Path,
    is_generation_root: bool,
) -> Result<(), LocalProductError> {
    ensure_real_directory(
        source,
        "inspect source directory for copy",
        "source tree must contain real directories only",
    )?;
    let src_meta = fs::symlink_metadata(source).map_err(|source| LocalProductError::Io {
        operation: "inspect source directory permissions",
        source,
    })?;
    fs::DirBuilder::new()
        .mode(src_meta.permissions().mode() & 0o777)
        .create(destination)
        .map_err(|source| LocalProductError::Io {
            operation: "create destination directory during copy",
            source,
        })?;

    let entries = fs::read_dir(source).map_err(|source| LocalProductError::Io {
        operation: "read source directory for copy",
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "iterate source directory for copy",
            source,
        })?;
        let src_child = entry.path();
        let dst_child = destination.join(entry.file_name());
        let meta = fs::symlink_metadata(&src_child).map_err(|source| LocalProductError::Io {
            operation: "inspect entry in source tree",
            source,
        })?;
        if meta.file_type().is_dir() {
            copy_local_directory_tree_inner(&src_child, &dst_child, false)?;
        } else if meta.file_type().is_symlink()
            && is_generation_root
            && entry.file_name() == OsStr::new("codex")
            && fs::read_link(&src_child).ok().as_deref() == Some(Path::new("bin/codex"))
        {
            std::os::unix::fs::symlink("bin/codex", &dst_child).map_err(|source| {
                LocalProductError::Io {
                    operation: "copy upstream codex entrypoint symlink",
                    source,
                }
            })?;
        } else if meta.file_type().is_file() {
            let mut reader = OpenOptions::new()
                .read(true)
                .custom_flags(O_NOFOLLOW | O_NONBLOCK_LINUX)
                .open(&src_child)
                .map_err(|source| LocalProductError::Io {
                    operation: "open source file for copy",
                    source,
                })?;
            let opened = reader.metadata().map_err(|source| LocalProductError::Io {
                operation: "verify opened source file",
                source,
            })?;
            if !opened.is_file()
                || opened.nlink() != 1
                || opened.dev() != meta.dev()
                || opened.ino() != meta.ino()
            {
                return Err(LocalProductError::UnsafeSource(
                    "generation source changed during copy",
                ));
            }
            let mut writer = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(meta.permissions().mode() & 0o777)
                .open(&dst_child)
                .map_err(|source| LocalProductError::Io {
                    operation: "create destination file during copy",
                    source,
                })?;
            io::copy(&mut reader, &mut writer).map_err(|source| LocalProductError::Io {
                operation: "copy file contents",
                source,
            })?;
            writer.sync_all().map_err(|source| LocalProductError::Io {
                operation: "sync copied file",
                source,
            })?;
        } else {
            return Err(LocalProductError::UnsafeSource(
                "generation tree must contain only regular files and directories",
            ));
        }
    }
    sync_generation_directory(destination)
}

pub(super) fn rename_noreplace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;

    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
    #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
    unsafe extern "C" {
        fn renameat2(
            olddirfd: i32,
            oldpath: *const std::ffi::c_char,
            newdirfd: i32,
            newpath: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "source path contains NUL"))?;
    let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination path contains NUL")
    })?;
    // Android API 24 does not export renameat2 in libc, though the arm64 kernel provides it.
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    let result = unsafe {
        unsafe extern "C" {
            fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
        }
        const SYS_RENAMEAT2_AARCH64: std::ffi::c_long = 276;
        syscall(
            SYS_RENAMEAT2_AARCH64,
            AT_FDCWD as std::ffi::c_long,
            source.as_ptr() as std::ffi::c_long,
            AT_FDCWD as std::ffi::c_long,
            destination.as_ptr() as std::ffi::c_long,
            RENAME_NOREPLACE as std::ffi::c_long,
        ) as i32
    };
    #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            source.as_ptr(),
            AT_FDCWD,
            destination.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(super) fn sync_directory(path: &Path) -> io::Result<()> {
    sync_directory_with(path, std::convert::identity, std::convert::identity)
}

pub(super) fn sync_directory_with<E>(
    path: &Path,
    open_error: impl FnOnce(io::Error) -> E,
    sync_error: impl FnOnce(io::Error) -> E,
) -> Result<(), E> {
    File::open(path)
        .map_err(open_error)?
        .sync_all()
        .map_err(sync_error)
}

pub(super) fn sync_generation_directory(path: &Path) -> Result<(), LocalProductError> {
    sync_directory_with(
        path,
        |source| LocalProductError::Io {
            operation: "open directory for sync",
            source,
        },
        |source| LocalProductError::Io {
            operation: "sync directory",
            source,
        },
    )
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
