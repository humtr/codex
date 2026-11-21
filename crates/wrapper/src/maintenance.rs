//! One local retention boundary. No conversation or credential contents are read.
use crate::error::LocalProductError;
use crate::fs_util::{self, ensure_real_directory_tree, flock_fd, LOCK_EX, LOCK_NB, LOCK_SH};
use crate::generation::{valid_sha256_hex, LocalWrapperRoots};
use crate::runtime_env::dup_above_safe_min;
use crate::state::generation_state;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path};

fn invalid() -> io::Error {
    io::Error::other("installed artifact maintenance is unavailable")
}
fn owned_directory(path: &Path) -> io::Result<()> {
    fs_util::ensure_real_directories(path.ancestors(), invalid)?;
    if fs::metadata(path)?.uid() != fs_util::current_uid()? {
        return Err(invalid());
    }
    Ok(())
}

/// CLOEXEC releases this lease only after the kernel opens the selected program.
pub(super) fn launch_lease(root: &Path) -> io::Result<Option<File>> {
    match fs::symlink_metadata(root) {
        Ok(_) => (),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    }
    owned_directory(root)?;
    let parent = root.parent().ok_or_else(invalid)?;
    owned_directory(parent)?;
    let file = File::open(parent)?;
    flock_fd(&file, LOCK_SH)?;
    // Runtime descriptor publication may replace 33–36. Keep generation
    // retention independent of those bindings even under descriptor pressure.
    Ok(Some(File::from(dup_above_safe_min(file.as_raw_fd())?)))
}

pub(super) struct UpdateLease<'a> {
    roots: &'a LocalWrapperRoots,
    file: Option<File>,
    writer: Option<File>,
    #[cfg(test)]
    proc_root: Option<std::path::PathBuf>,
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
        // Launches and read-only checks can proceed while an update owns the
        // complete activation and daemon synchronization operation.
        let writer = File::open(&roots.generation_root)
            .and_then(|dir| {
                flock_fd(&dir, LOCK_EX | LOCK_NB)?;
                Ok(dir)
            })
            .map_err(|source| LocalProductError::Io {
                operation: "acquire exclusive upstream update lease",
                source,
            })?;
        Ok(Self {
            roots,
            file,
            writer: Some(writer),
            #[cfg(test)]
            proc_root: None,
        })
    }

    #[cfg(test)]
    fn acquire_with_proc(
        roots: &'a LocalWrapperRoots,
        proc_root: std::path::PathBuf,
    ) -> Result<Self, LocalProductError> {
        let mut lease = Self::acquire(roots)?;
        lease.proc_root = Some(proc_root);
        Ok(lease)
    }
}
impl Drop for UpdateLease<'_> {
    fn drop(&mut self) {
        drop(self.writer.take());
        drop(self.file.take());
        #[cfg(test)]
        if let Some(proc_root) = self.proc_root.as_deref() {
            let _ = prune_with_proc(self.roots, proc_root);
            return;
        }
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

fn process_references_in(proc_root: &Path, roots: &[&Path]) -> io::Result<HashSet<String>> {
    let uid = fs_util::current_uid()?;
    let mut keep = HashSet::new();
    let mut record = |link: &Path| -> io::Result<()> {
        match fs::read_link(link) {
            Ok(path) => {
                keep.extend(
                    roots
                        .iter()
                        .filter_map(|root| referenced_generation(&path, root)),
                );
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => (),
            Err(err) => return Err(err),
        }
        Ok(())
    };
    let mut exit_budget = std::time::Duration::from_millis(10);
    for entry in fs::read_dir(proc_root)? {
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
            record(&process.join("exe"))?;
            record(&process.join("cwd"))?;
            for file in fs::read_dir(process.join("fd"))? {
                record(&file?.path())?;
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
            || metadata.uid() != fs_util::current_uid()?
        {
            continue;
        }
        fs::remove_dir_all(entry.path())?;
        removed += 1;
    }
    if removed != 0 {
        fs_util::sync_directory(root)?;
    }
    Ok(removed)
}

fn prune_orphaned_headers_with_proc(root: &Path, proc_root: &Path) -> io::Result<usize> {
    match fs::symlink_metadata(root) {
        Ok(_) => owned_directory(root)?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(err),
    }
    let lease = OpenOptions::new()
        .read(true)
        .custom_flags(crate::fs_util::O_NOFOLLOW)
        .open(root)?;
    let metadata = lease.metadata()?;
    if !metadata.is_dir()
        || metadata.uid() != crate::fs_util::current_uid()?
        || metadata.mode() & 0o077 != 0
    {
        return Err(invalid());
    }
    // The downloader uses this same directory lease for part and header files.
    flock_fd(&lease, LOCK_EX | LOCK_NB)?;
    let mut removed = 0;
    let mut references = None;
    let uid = crate::fs_util::current_uid()?;
    for item in fs::read_dir(root)? {
        let item = item?;
        let filename = item.file_name();
        let Some(filename) = filename.to_str() else {
            continue;
        };
        let Some(digest) = filename.strip_suffix(".hdr") else {
            continue;
        };
        if !valid_sha256_hex(digest) {
            continue;
        }
        match fs::symlink_metadata(root.join(format!("{digest}.part"))) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => (),
            Ok(_) => continue,
            Err(err) => return Err(err),
        }

        let path = item.path();
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.nlink() != 1
            || metadata.mode() & 0o022 != 0
        {
            continue;
        }
        // A downloader killed before cleanup can leave a curl child with
        // an open header after the directory lease has been released.
        let references = match references.as_mut() {
            Some(references) => references,
            None => references.insert(process_references_in(proc_root, &[root])?),
        };
        if references.contains(filename) {
            continue;
        }
        let header = OpenOptions::new()
            .read(true)
            .custom_flags(crate::fs_util::O_NOFOLLOW | 0o4000)
            .open(&path)?;
        let opened = header.metadata()?;
        if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
            continue;
        }
        match flock_fd(&header, LOCK_EX | LOCK_NB) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
            Err(err) => return Err(err),
            Ok(()) => (),
        }
        fs::remove_file(path)?;
        removed += 1;
    }
    if removed != 0 {
        lease.sync_all()?;
    }
    Ok(removed)
}

pub(super) fn prune(roots: &LocalWrapperRoots) -> io::Result<usize> {
    prune_with_proc(roots, Path::new("/proc"))
}

fn prune_with_proc(roots: &LocalWrapperRoots, proc_root: &Path) -> io::Result<usize> {
    owned_directory(&roots.generation_root)?;
    let parent = roots.generation_root.parent().ok_or_else(invalid)?;
    owned_directory(parent)?;
    let lease = File::open(parent)?;
    flock_fd(&lease, LOCK_EX | LOCK_NB)?;
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
    let headers = prune_orphaned_headers_with_proc(&roots.state_root.join("downloads"), proc_root);
    let publications = roots.state_root.join("publications");
    if !has_prunable_entries(&roots.generation_root, &keep, false)?
        && !has_prunable_entries(&publications, &keep, false)?
        && !has_prunable_entries(&roots.state_root, &keep, true)?
    {
        return headers;
    }
    keep.extend(process_references_in(
        proc_root,
        &[&roots.generation_root, &roots.state_root],
    )?);
    // No state mutation can occur under this writer lock, including rollback.
    let mut removed = prune_directory(&roots.generation_root, &keep, false)?;
    removed += prune_directory(&publications, &keep, false)?;
    removed += prune_directory(&roots.state_root, &keep, true)?;
    Ok(removed + headers?)
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
    fn launch_lease_rejects_dangling_generation_symlink() {
        let root = crate::tests::temp_root("dangling-generation");
        let generation_root = root.join("releases");
        assert!(launch_lease(&generation_root).unwrap().is_none());
        std::os::unix::fs::symlink(root.join("absent"), &generation_root).unwrap();
        assert!(launch_lease(&generation_root).is_err());
        assert!(fs::symlink_metadata(&generation_root).unwrap().is_symlink());
        crate::tests::remove_temp_root(root);
    }

    fn test_self_proc_root(root: &Path) -> std::path::PathBuf {
        let proc_root = root.join("proc");
        fs::create_dir_all(&proc_root).unwrap();
        let pid = std::process::id().to_string();
        std::os::unix::fs::symlink(Path::new("/proc").join(&pid), proc_root.join(&pid)).unwrap();
        proc_root
    }

    #[test]
    fn maintenance_preserves_open_state_staging_and_prunes_it_after_release() {
        let root = crate::tests::temp_root("state-staging-prune");
        let roots = crate::tests::make_test_roots(&root);
        let proc_root = test_self_proc_root(&root);
        let paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        let pointer = generation_state::plan_initial_pointer_state("gen-1").unwrap();
        fs::write(
            &paths.activation_state,
            generation_state::encode_pointer_state(&pointer).unwrap(),
        )
        .unwrap();

        let busy = roots.state_root.join(".local-update-10-1");
        let old = roots.state_root.join(".local-update-11-1");
        fs::create_dir(&busy).unwrap();
        fs::create_dir(&old).unwrap();
        let open = File::create(busy.join("proof")).unwrap();
        assert_eq!(prune_with_proc(&roots, &proc_root).unwrap(), 1);
        assert!(busy.exists());
        assert!(!old.exists());

        drop(open);
        assert_eq!(prune_with_proc(&roots, &proc_root).unwrap(), 1);
        assert!(!busy.exists());
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn maintenance_prunes_only_owned_unheld_orphan_download_headers() {
        let root = crate::tests::temp_root("header-prune");
        let roots = crate::tests::make_test_roots(&root);
        let proc_root = test_self_proc_root(&root);
        let paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        let pointer = generation_state::plan_initial_pointer_state("gen-1").unwrap();
        fs::write(
            &paths.activation_state,
            generation_state::encode_pointer_state(&pointer).unwrap(),
        )
        .unwrap();

        let downloads = roots.state_root.join("downloads");
        crate::fs_util::private_dir(&downloads).unwrap();
        let orphan = downloads.join(format!("{}.hdr", "a".repeat(64)));
        let paired = downloads.join(format!("{}.hdr", "b".repeat(64)));
        let held = downloads.join(format!("{}.hdr", "c".repeat(64)));
        let symlink = downloads.join(format!("{}.hdr", "d".repeat(64)));
        let unknown = downloads.join("user.hdr");
        for path in [
            &orphan,
            &paired,
            &held,
            &unknown,
            &paired.with_extension("part"),
        ] {
            fs::write(path, b"test").unwrap();
        }
        std::os::unix::fs::symlink(&unknown, &symlink).unwrap();
        let cache_lease = File::open(&downloads).unwrap();
        flock_fd(&cache_lease, LOCK_EX | LOCK_NB).unwrap();
        assert_eq!(
            prune_with_proc(&roots, &proc_root).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert!(orphan.exists());
        drop(cache_lease);
        let open_header = File::open(&held).unwrap();
        assert_eq!(prune_with_proc(&roots, &proc_root).unwrap(), 1);
        assert!(!orphan.exists());
        assert!(paired.exists());
        assert!(held.exists());
        assert!(symlink.is_symlink());
        assert!(unknown.exists());
        drop(open_header);
        assert_eq!(prune_with_proc(&roots, &proc_root).unwrap(), 1);
        assert!(!held.exists());
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn prune_fails_closed_when_owned_process_fd_is_unreadable() {
        use std::os::unix::fs::PermissionsExt;
        let root = crate::tests::temp_root("unreadable-proc-fd");
        let roots = crate::tests::make_test_roots(&root);
        let proc_root = test_self_proc_root(&root);
        let paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        let pointer = generation_state::plan_initial_pointer_state("gen-1").unwrap();
        fs::write(
            &paths.activation_state,
            generation_state::encode_pointer_state(&pointer).unwrap(),
        )
        .unwrap();

        let old = roots.state_root.join(".local-update-19-1");
        fs::create_dir(&old).unwrap();
        let unreadable = proc_root.join("999999");
        let unreadable_fd = unreadable.join("fd");
        fs::create_dir_all(&unreadable_fd).unwrap();
        let mut fields = vec!["0"; 18];
        fields[0] = "S";
        fields[17] = "1";
        fs::write(
            unreadable.join("stat"),
            format!("999999 (opaque) {}", fields.join(" ")),
        )
        .unwrap();
        fs::set_permissions(&unreadable_fd, fs::Permissions::from_mode(0o000)).unwrap();
        let err = prune_with_proc(&roots, &proc_root).unwrap_err();
        fs::set_permissions(&unreadable_fd, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(old.exists());
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn update_lease_creates_missing_generation_root_and_blocks_prune() {
        let root = crate::tests::temp_root("update-lease-fresh");
        let proc_root = test_self_proc_root(&root);
        let home = root.join("home");
        let generation_root = home.join(".codex/packages/standalone/releases");
        let state_root = home.join(".local/share/codex/wrapper");
        let config_dir = state_root.join("config");
        crate::tests::create_private_test_dirs(&config_dir);
        let state_paths = generation_state::WrapperStatePaths::new(&state_root).unwrap();
        generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();
        let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
        let mut io = generation_state::FsActivationIo;
        let ptr = generation_state::plan_initial_pointer_state("0.169.1-abcdef").unwrap();
        generation_state::activate_pointer_state_locked(
            &state_paths,
            generation_state::ActivationOperation::Update,
            None,
            &ptr,
            &mut io,
            &lock,
        )
        .unwrap();
        drop(lock);

        let roots = LocalWrapperRoots {
            generation_root: generation_root.clone(),
            state_root: state_root.clone(),
            config_dir,
            resolver_path: root.join("resolv.conf"),
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
            curl: root.join("curl"),
        };
        assert!(!generation_root.exists());

        let lease = UpdateLease::acquire_with_proc(&roots, proc_root.clone()).unwrap();
        assert!(fs::symlink_metadata(&generation_root).unwrap().is_dir());
        assert!(UpdateLease::acquire(&roots).is_err());
        assert!(launch_lease(&generation_root).unwrap().is_some());

        let old_staging = state_root.join(".local-update-22-1");
        fs::create_dir(&old_staging).unwrap();
        let blocked = prune_with_proc(&roots, &proc_root).unwrap_err();
        assert_eq!(blocked.kind(), io::ErrorKind::WouldBlock);

        drop(lease);
        assert!(!old_staging.exists(), "UpdateLease drop already prunes");
        assert_eq!(prune_with_proc(&roots, &proc_root).unwrap(), 0);
        crate::tests::remove_temp_root(root);
    }
}
