use std::fs::{self, File};
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::Path;

fn invalid() -> io::Error {
    io::Error::other("private directory path is unavailable")
}

pub(super) fn core_safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}

pub(super) fn private_dir(path: &Path) -> io::Result<()> {
    if !core_safe_absolute_path(path) {
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
        || m.uid() != fs::metadata("/proc/self")?.uid()
        || m.permissions().mode() & 0o777 != 0o700
    {
        return Err(invalid());
    }
    Ok(())
}
