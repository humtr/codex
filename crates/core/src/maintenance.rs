//! One local retention boundary. No conversation or credential contents are read.
use super::*;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

fn invalid() -> io::Error {
    io::Error::other("installed artifact maintenance is unavailable")
}
fn owned_directory(path: &Path) -> io::Result<()> {
    for part in path.ancestors() {
        let m = fs::symlink_metadata(part)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(invalid());
        }
    }
    if fs::metadata(path)?.uid() != fs::metadata("/proc/self")?.uid() {
        return Err(invalid());
    }
    Ok(())
}

/// CLOEXEC releases this lease only after the kernel opens the selected program.
pub(super) fn launch_lease(root: &Path) -> io::Result<Option<File>> {
    if !root.exists() {
        return Ok(None);
    }
    owned_directory(root)?;
    let parent = root.parent().ok_or_else(invalid)?;
    owned_directory(parent)?;
    let file = File::open(parent)?;
    if unsafe { flock(file.as_raw_fd(), 1) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Some(file))
}

pub(super) struct UpdateLease<'a> {
    roots: &'a LocalCoreRoots,
    file: Option<File>,
}
impl<'a> UpdateLease<'a> {
    pub(super) fn acquire(roots: &'a LocalCoreRoots) -> Result<Self, LocalProductError> {
        let file =
            launch_lease(&roots.generation_root).map_err(|source| LocalProductError::Io {
                operation: "lease installed generations during update",
                source,
            })?;
        Ok(Self { roots, file })
    }
}
impl Drop for UpdateLease<'_> {
    fn drop(&mut self) {
        drop(self.file.take());
        let _ = prune(self.roots);
    }
}

fn dependencies(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
) -> io::Result<HashSet<String>> {
    let mut keep = HashSet::from([state.current.clone()]);
    if let Some(previous) = state.previous.as_ref() {
        keep.insert(previous.clone());
    }
    // Use the existing descriptor/provenance parsers, not a second state format.
    for id in keep.clone() {
        let path = roots.generation_root.join(&id);
        let loaded = load_local_generation(&path).map_err(|_| invalid())?;
        if loaded.generation_id != id {
            return Err(invalid());
        }
        if loaded
            .manifest
            .creation_metadata
            .starts_with(LOCAL_DERIVED_METADATA_FORMAT)
        {
            let (_, manifest) = read_local_release_manifest(&path).map_err(|_| invalid())?;
            if let Some(baseline) =
                parse_local_derived_metadata(&manifest, &loaded).map_err(|_| invalid())?
            {
                keep.insert(baseline.generation_component);
            }
        }
    }
    if let Some(guard) = rollback_guard::read_guard(roots).map_err(|_| invalid())? {
        keep.insert(guard.target_generation_id);
        keep.insert(guard.held_generation_id);
    }
    if let Some(hold) = read_update_hold(roots).map_err(|_| invalid())? {
        keep.insert(hold.generation_id);
    }
    Ok(keep)
}

fn pending_candidate(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
) -> io::Result<Option<String>> {
    let (_, current) = read_local_release_manifest(&roots.generation_root.join(&state.current))
        .map_err(|_| invalid())?;
    if current.generation_id != state.current {
        return Err(invalid());
    }
    let mut latest: Option<(u64, String)> = None;
    for entry in fs::read_dir(&roots.generation_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if m2_generation_state::validate_generation_identity(&id, "pending candidate").is_err() {
            continue;
        }
        let Ok((_, release)) = read_local_release_manifest(&entry.path()) else {
            continue;
        };
        if release.generation_id != id || release.release_sequence <= current.release_sequence {
            continue;
        }
        let Ok(loaded) = load_local_generation(&entry.path()) else {
            continue;
        };
        if loaded.generation_id != id {
            continue;
        }
        let candidate = (release.release_sequence, id);
        if latest.as_ref().is_none_or(|old| candidate > *old) {
            latest = Some(candidate);
        }
    }
    Ok(latest.map(|(_, id)| id))
}

fn referenced_generation(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let Component::Normal(id) = relative.components().next()? else {
        return None;
    };
    let id = id.to_str()?;
    if m2_generation_state::validate_generation_identity(id, "retained generation").is_err()
        && !staging_name(id, ".acquire-")
        && !staging_name(id, ".candidate-")
        && !staging_name(id, ".local-update-")
    {
        return None;
    }
    Some(id.to_owned())
}

// Exit can clear /proc visibility before the task's state becomes Z. Never
// infer released handles from PF_EXITING alone; confirm disappearance/death.
pub(super) fn process_has_no_handles(
    process: &Path,
    budget: &mut std::time::Duration,
) -> io::Result<bool> {
    let deadline = std::time::Instant::now() + *budget;
    let result = (|| loop {
        let status = match fs::read_to_string(process.join("stat")) {
            Ok(status) => status,
            Err(e) if e.kind() == io::ErrorKind::NotFound || e.raw_os_error() == Some(3) => {
                return Ok(true);
            }
            Err(e) => return Err(e),
        };
        let fields: Vec<_> = status
            .rsplit_once(')')
            .ok_or_else(invalid)?
            .1
            .split_whitespace()
            .collect();
        if fields.len() < 18 {
            return Err(invalid());
        }
        let threads = fields[17].parse::<u64>().map_err(|_| invalid())?;
        let flags = fields[6].parse::<u64>().map_err(|_| invalid())?;
        if threads == 1 && matches!(fields[0], "Z" | "X") {
            return Ok(true);
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        *budget = remaining;
        if threads != 1 || flags & 4 == 0 || remaining.is_zero() {
            return Ok(false);
        }
        std::thread::sleep(remaining.min(std::time::Duration::from_millis(1)));
    })();
    *budget = deadline.saturating_duration_since(std::time::Instant::now());
    result
}

fn process_is_owned(process: &Path, directory_uid: u32, uid: u32) -> io::Result<bool> {
    if directory_uid == uid {
        return Ok(true);
    }
    if directory_uid != 0 {
        return Ok(false);
    }
    // Linux may reassign non-dumpable proc entries to root. Confirm actual UID
    // before skipping them; inaccessible local references still stop pruning.
    let status = match fs::read_to_string(process.join("status")) {
        Ok(status) => status,
        Err(e) if e.kind() == io::ErrorKind::NotFound || e.raw_os_error() == Some(3) => {
            return Ok(false)
        }
        Err(e) => return Err(e),
    };
    let mut lines = status.lines().filter_map(|line| line.strip_prefix("Uid:"));
    let owners = lines.next().ok_or_else(invalid)?;
    if lines.next().is_some() {
        return Err(invalid());
    }
    let owners = owners
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;
    if owners.len() != 4 {
        return Err(invalid());
    }
    Ok(owners.contains(&uid))
}

fn process_references(root: &Path) -> io::Result<HashSet<String>> {
    let uid = fs::metadata("/proc/self")?.uid();
    let mut keep = HashSet::new();
    let mut exit_budget = std::time::Duration::from_millis(10);
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if !entry
            .file_name()
            .as_encoded_bytes()
            .iter()
            .all(u8::is_ascii_digit)
        {
            continue;
        }
        let process = entry.path();
        let metadata = match fs::metadata(&process) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        if !process_is_owned(&process, metadata.uid(), uid)? {
            continue;
        }
        let mut no_wait = std::time::Duration::ZERO;
        if process_has_no_handles(&process, &mut no_wait)? {
            continue;
        }
        let references = (|| -> io::Result<()> {
            match fs::read_link(process.join("exe")) {
                Ok(path) => {
                    if let Some(id) = referenced_generation(&path, root) {
                        keep.insert(id);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
            let files = fs::read_dir(process.join("fd"))?;
            for file in files {
                match fs::read_link(file?.path()) {
                    Ok(path) => {
                        if let Some(id) = referenced_generation(&path, root) {
                            keep.insert(id);
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        })();
        if let Err(error) = references {
            if (matches!(
                error.kind(),
                io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound
            ) || error.raw_os_error() == Some(3))
                && process_has_no_handles(&process, &mut exit_budget).unwrap_or(false)
            {
                continue;
            }
            return Err(error);
        }
    }
    Ok(keep)
}

fn staging_name(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(pid, sequence)| {
            !pid.is_empty()
                && !sequence.is_empty()
                && pid.bytes().all(|b| b.is_ascii_digit())
                && sequence.bytes().all(|b| b.is_ascii_digit())
        })
}
fn prune_directory(root: &Path, keep: &HashSet<String>, staging_only: bool) -> io::Result<usize> {
    if !root.exists() {
        return Ok(0);
    }
    owned_directory(root)?;
    let mut removed = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if keep.contains(name) {
            continue;
        }
        let staging = staging_name(name, ".acquire-")
            || staging_name(name, ".candidate-")
            || staging_name(name, ".local-update-");
        let generation = !staging_only
            && m2_generation_state::validate_generation_identity(name, "disposable generation")
                .is_ok();
        if !staging && !generation {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != fs::metadata("/proc/self")?.uid()
        {
            continue;
        }
        fs::remove_dir_all(entry.path())?;
        removed += 1;
    }
    if removed != 0 {
        File::open(root)?.sync_all()?;
    }
    Ok(removed)
}

pub(super) fn prune(roots: &LocalCoreRoots) -> io::Result<usize> {
    owned_directory(&roots.generation_root)?;
    let parent = roots.generation_root.parent().ok_or_else(invalid)?;
    owned_directory(parent)?;
    let lease = File::open(parent)?;
    if unsafe { flock(lease.as_raw_fd(), 2 | 4) } != 0 {
        return Err(io::Error::last_os_error());
    }
    owned_directory(&roots.state_root)?;
    let paths =
        m2_generation_state::CoreStatePaths::new(&roots.state_root).map_err(|_| invalid())?;
    let _writer = m2_generation_state::acquire_activation_lock(&paths).map_err(|_| invalid())?;
    for name in [
        "activation-journal",
        "activation-journal.tmp",
        "activation-state.tmp",
    ] {
        match fs::symlink_metadata(roots.state_root.join(name)) {
            Ok(_) => return Err(invalid()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    let state = m2_generation_state::read_pointer_state(&paths)
        .map_err(|_| invalid())?
        .ok_or_else(invalid)?;
    let mut keep = dependencies(roots, &state)?;
    if let Some(candidate) = pending_candidate(roots, &state)? {
        keep.insert(candidate);
    }
    shared_server::retire_unused(&roots.state_root, &roots.generation_root, &keep)?;
    keep.extend(process_references(&roots.generation_root)?);
    // No state mutation can occur under this writer lock, including rollback.
    let mut removed = prune_directory(&roots.generation_root, &keep, false)?;
    let publications = roots
        .generation_root
        .parent()
        .ok_or_else(invalid)?
        .join("publications");
    removed += prune_directory(&publications, &keep, false)?;
    removed += prune_directory(&roots.state_root, &keep, true)?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_ownership_checks_native_uids_when_proc_entries_belong_to_root() {
        use std::os::unix::fs::PermissionsExt;
        let root = crate::tests::temp_root("proc-native-owner");
        assert!(process_is_owned(&root, 101, 101).unwrap());
        assert!(!process_is_owned(&root, 102, 101).unwrap());
        assert!(!process_is_owned(&root, 0, 101).unwrap());
        for (uids, owned) in [
            ("101 101 101 101", true),
            ("0 0 101 0", true),
            ("0 0 0 0", false),
        ] {
            fs::write(root.join("status"), format!("Name: fixture\nUid: {uids}\n")).unwrap();
            assert_eq!(process_is_owned(&root, 0, 101).unwrap(), owned);
        }
        for status in [
            "broken",
            "Uid: 1 2 3",
            "Uid: 1 bad 3 4",
            "Uid: 1 2 3 4\nUid: 1 2 3 4",
        ] {
            fs::write(root.join("status"), status).unwrap();
            assert!(process_is_owned(&root, 0, 101).is_err());
        }
        fs::write(root.join("status"), "Uid: 101 101 101 101").unwrap();
        fs::set_permissions(root.join("status"), fs::Permissions::from_mode(0o0)).unwrap();
        let denied = process_is_owned(&root, 0, 101);
        fs::set_permissions(root.join("status"), fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(denied.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn process_exit_confirmation_never_skips_live_or_unreadable_handles() {
        let root = crate::tests::temp_root("proc-exit-status");
        fs::create_dir_all(&root).unwrap();
        let mut no_wait = std::time::Duration::ZERO;
        let write_status = |state: &str, flags: &str, threads: &str| {
            let mut fields = vec!["0"; 18];
            fields[0] = state;
            fields[6] = flags;
            fields[17] = threads;
            fs::write(
                root.join("stat"),
                format!("1 (name with ) space) {}", fields.join(" ")),
            )
            .unwrap();
        };
        for (state, flags, threads, dead) in [
            ("Z", "4", "1", true),
            ("X", "4", "1", true),
            ("Z", "4", "2", false),
            ("R", "0", "1", false),
            ("R", "4", "2", false),
        ] {
            write_status(state, flags, threads);
            assert_eq!(process_has_no_handles(&root, &mut no_wait).unwrap(), dead);
        }
        write_status("R", "4", "1");
        let mut budget = std::time::Duration::from_millis(2);
        assert!(!process_has_no_handles(&root, &mut budget).unwrap());
        assert!(budget.is_zero());
        for disappear in [false, true] {
            write_status("R", "4", "1");
            let source = root.clone();
            let publish = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1));
                if disappear {
                    fs::remove_file(source.join("stat")).unwrap();
                } else {
                    let mut fields = vec!["0"; 18];
                    fields[0] = "Z";
                    fields[6] = "4";
                    fields[17] = "1";
                    fs::write(
                        source.join("next"),
                        format!("1 (done) {}", fields.join(" ")),
                    )
                    .unwrap();
                    fs::rename(source.join("next"), source.join("stat")).unwrap();
                }
            });
            let mut budget = std::time::Duration::from_millis(10);
            let confirmed = process_has_no_handles(&root, &mut budget);
            publish.join().unwrap();
            assert!(confirmed.unwrap());
            assert!(budget < std::time::Duration::from_millis(10));
        }
        for (flags, threads) in [("bad", "1"), ("4", "bad")] {
            write_status("R", flags, threads);
            assert!(process_has_no_handles(&root, &mut no_wait).is_err());
        }
        for malformed in ["broken", "1 (name) R"] {
            fs::write(root.join("stat"), malformed).unwrap();
            assert!(process_has_no_handles(&root, &mut no_wait).is_err());
        }
        use std::os::unix::fs::PermissionsExt;
        write_status("R", "0", "1");
        fs::set_permissions(root.join("stat"), fs::Permissions::from_mode(0o0)).unwrap();
        let denied = process_has_no_handles(&root, &mut no_wait);
        fs::set_permissions(root.join("stat"), fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(denied.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        fs::remove_file(root.join("stat")).unwrap();
        fs::create_dir(root.join("stat")).unwrap();
        assert!(process_has_no_handles(&root, &mut no_wait).is_err());
        fs::remove_dir(root.join("stat")).unwrap();
        assert!(process_has_no_handles(&root, &mut no_wait).unwrap());
        crate::tests::remove_temp_root(root);
    }
}
