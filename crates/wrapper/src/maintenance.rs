//! One local retention boundary. No conversation or credential contents are read.
use super::*;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
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
    // Runtime descriptor publication may replace 33–36. Keep generation
    // retention independent of those bindings even under descriptor pressure.
    let safe =
        unsafe { super::fcntl(file.as_raw_fd(), super::F_DUPFD_CLOEXEC, super::SAFE_MIN_FD) };
    if safe < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Some(unsafe { File::from_raw_fd(safe) }))
}

pub(super) struct UpdateLease<'a> {
    roots: &'a LocalWrapperRoots,
    file: Option<File>,
}
impl<'a> UpdateLease<'a> {
    pub(super) fn acquire(roots: &'a LocalWrapperRoots) -> Result<Self, LocalProductError> {
        ensure_real_directory_tree(
            &roots.generation_root,
            "prepare wrapper generation root",
            "wrapper generation root must be a real directory",
        )?;
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

fn dependencies(state: &generation_state::GenerationPointerState) -> HashSet<String> {
    let mut keep = HashSet::from([state.current.clone()]);
    if let Some(previous) = state.previous.as_ref() {
        keep.insert(previous.clone());
    }
    keep
}

fn referenced_generation(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let Component::Normal(id) = relative.components().next()? else {
        return None;
    };
    let id = id.to_str()?;
    if generation_state::validate_generation_identity(id, "retained generation").is_err()
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

pub(super) fn process_references(root: &Path) -> io::Result<HashSet<String>> {
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
fn matching_prunable_entry(name: &str, keep: &HashSet<String>, staging_only: bool) -> bool {
    if keep.contains(name) {
        return false;
    }
    let staging = staging_name(name, ".acquire-")
        || staging_name(name, ".candidate-")
        || staging_name(name, ".local-update-");
    let generation = !staging_only
        && generation_state::validate_generation_identity(name, "disposable generation").is_ok();
    staging || generation
}

fn has_prunable_entries(
    root: &Path,
    keep: &HashSet<String>,
    staging_only: bool,
) -> io::Result<bool> {
    if !root.exists() {
        return Ok(false);
    }
    owned_directory(root)?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if matching_prunable_entry(name, keep, staging_only) {
            return Ok(true);
        }
    }
    Ok(false)
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
        if !matching_prunable_entry(name, keep, staging_only) {
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

pub(super) fn prune(roots: &LocalWrapperRoots) -> io::Result<usize> {
    owned_directory(&roots.generation_root)?;
    let parent = roots.generation_root.parent().ok_or_else(invalid)?;
    owned_directory(parent)?;
    let lease = File::open(parent)?;
    if unsafe { flock(lease.as_raw_fd(), 2 | 4) } != 0 {
        return Err(io::Error::last_os_error());
    }
    owned_directory(&roots.state_root)?;
    let paths =
        generation_state::WrapperStatePaths::new(&roots.state_root).map_err(|_| invalid())?;
    let _writer = generation_state::acquire_activation_lock(&paths).map_err(|_| invalid())?;
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
    let state = generation_state::read_pointer_state(&paths)
        .map_err(|_| invalid())?
        .ok_or_else(invalid)?;
    let mut keep = dependencies(&state);
    let publications = roots.state_root.join("publications");
    if !has_prunable_entries(&roots.generation_root, &keep, false)?
        && !has_prunable_entries(&publications, &keep, false)?
        && !has_prunable_entries(&roots.state_root, &keep, true)?
    {
        return Ok(0);
    }
    keep.extend(process_references(&roots.generation_root)?);
    // No state mutation can occur under this writer lock, including rollback.
    let mut removed = prune_directory(&roots.generation_root, &keep, false)?;
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
            // Publisher scheduling is not the production deadline under test.
            // The preceding live-state case proves exact budget exhaustion.
            let mut budget = std::time::Duration::from_secs(1);
            let confirmed = process_has_no_handles(&root, &mut budget);
            publish.join().unwrap();
            assert!(confirmed.unwrap());
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

    #[test]
    fn update_lease_creates_missing_generation_root_and_blocks_prune() {
        let root = crate::tests::temp_root("update-lease-fresh");
        let home = root.join("home");
        let generation_root = home.join(".codex/packages/standalone/releases");
        let state_root = home.join(".local/share/codex/wrapper");
        let config_dir = state_root.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let state_paths = generation_state::WrapperStatePaths::new(&state_root).unwrap();
        generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();
        let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
        let mut io = generation_state::FsActivationIo;
        let ptr = generation_state::plan_initial_pointer_state("0.169.1-abcdef").unwrap();
        generation_state::activate_pointer_state_locked(&state_paths, None, &ptr, &mut io, &lock)
            .unwrap();
        drop(lock);

        let roots = LocalWrapperRoots {
            generation_root: generation_root.clone(),
            state_root: state_root.clone(),
            config_dir,
            resolver_path: root.join("resolv.conf"),
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
            openssl: crate::tests::test_openssl_path(),
            curl: root.join("curl"),
        };
        assert!(!generation_root.exists());

        let lease = UpdateLease::acquire(&roots).unwrap();
        assert!(fs::symlink_metadata(&generation_root).unwrap().is_dir());

        let blocked = prune(&roots).unwrap_err();
        assert_eq!(blocked.kind(), io::ErrorKind::WouldBlock);

        drop(lease);
        assert_eq!(prune(&roots).unwrap(), 0);
        crate::tests::remove_temp_root(root);
    }
}
