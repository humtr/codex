//! Explicit account preference and lifecycle; upstream owns shared conversations.
use super::*;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;

const PREFERENCE: &str = "default-profile-v1";
const HEADER: &str = "codex-manager-default-profile-v1";
pub(super) const METADATA_V2: &[u8] = b"codex-manager-profile-v2\n";
const ERR_BUSY: ManagerError =
    ManagerError::operation("codex termux: profile is in use; no change made");
const ERR_DEFAULT: ManagerError =
    ManagerError::operation("codex termux: default profile is invalid");

pub(super) fn read_default(context: &Context) -> Result<Option<ProfileTarget>, ManagerError> {
    let path = manager_base(context).join("manager").join(PREFERENCE);
    if inspect_path(&path)? == PathPresence::Missing {
        return Ok(None);
    }
    existing_manager_profiles(context)?;
    let parent = fs::symlink_metadata(path.parent().ok_or(ERR_PATH)?).map_err(|_| ERR_PATH)?;
    if parent.uid() != unsafe { libc::geteuid() } {
        return Err(ERR_PATH);
    }
    let bytes = read_record(&path)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| ERR_DEFAULT)?;
    let id = text
        .strip_prefix(&format!("{HEADER}\nprofile\t"))
        .and_then(|s| s.strip_suffix('\n'))
        .ok_or(ERR_DEFAULT)?;
    let target = parse_target(&OsString::from(id)).map_err(|_| ERR_DEFAULT)?;
    if target_name(&target) != id {
        return Err(ERR_DEFAULT);
    }
    validate_target(context, &target)?;
    Ok(Some(target))
}

pub(super) fn target_name(target: &ProfileTarget) -> &str {
    match target {
        ProfileTarget::Default => "default",
        ProfileTarget::Custom(id) => id,
    }
}

pub(super) fn validate_target(
    context: &Context,
    target: &ProfileTarget,
) -> Result<(), ManagerError> {
    if let ProfileTarget::Custom(id) = target {
        let dirs = existing_manager_profiles(context)?.ok_or(ERR_PROFILE)?;
        if !profile_complete(&dirs, id) {
            return Err(ERR_PROFILE);
        }
    }
    Ok(())
}

pub(super) fn read_record(path: &Path) -> Result<Vec<u8>, ManagerError> {
    if inspect_path(path)? != PathPresence::Present {
        return Err(ERR_PATH);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ERR_PATH)?;
    let metadata = file.metadata().map_err(|_| ERR_PATH)?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o7777 != PRIVATE_FILE_MODE
    {
        return Err(ERR_PATH);
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((RECORD_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ERR_PATH)?;
    if bytes.len() > RECORD_MAX_BYTES {
        return Err(ERR_PATH);
    }
    Ok(bytes)
}

fn publish_record(path: &Path, bytes: &[u8]) -> Result<(), ManagerError> {
    if inspect_path(path)? == PathPresence::Present {
        read_record(path)?;
    }
    let parent = path.parent().ok_or(ERR_PATH)?;
    let temporary = parent.join(format!(
        ".profile-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        write_new_record(&temporary, bytes)?;
        fs::rename(&temporary, path).map_err(|_| ERR_CREATE)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub(super) fn run_default(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    let target = match args {
        [] => read_default(context)?.unwrap_or(ProfileTarget::Default),
        [value] => {
            let target = parse_target(value)?;
            validate_target(context, &target)?;
            // Serialize with creation/lifecycle through the existing directory.
            let dirs = manager_profiles_for_create(context)?;
            let _lock = directory_lock(&dirs, libc::LOCK_EX)?;
            read_default(context)?;
            validate_target(context, &target)?;
            publish_record(
                &dirs.parent().ok_or(ERR_PATH)?.join(PREFERENCE),
                format!("{HEADER}\nprofile\t{}\n", target_name(&target)).as_bytes(),
            )?;
            target
        }
        _ => return Err(ERR_USAGE),
    };
    Ok(Some(format!("default: {}\n", target_name(&target))))
}

pub(super) fn directory_lock(path: &Path, kind: i32) -> Result<File, ManagerError> {
    if inspect_path(path)? != PathPresence::Present {
        return Err(ERR_PATH);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(path)
        .map_err(|_| ERR_PATH)?;
    let m = file.metadata().map_err(|_| ERR_PATH)?;
    if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o7777 != PRIVATE_DIR_MODE {
        return Err(ERR_PATH);
    }
    use std::os::fd::AsRawFd;
    if unsafe { libc::flock(file.as_raw_fd(), kind | libc::LOCK_NB) } != 0 {
        return Err(ERR_BUSY);
    }
    Ok(file)
}

pub(super) fn launch_default(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if args
        .first()
        .is_some_and(|arg| matches!(arg.to_str(), Some("termux" | "doctor" | "update")))
    {
        return Err(ERR_USAGE);
    }
    // Invalid optional settings cannot make ordinary native launch unavailable.
    let target = read_default(context)
        .ok()
        .flatten()
        .unwrap_or(ProfileTarget::Default);
    launch_core(context, &target, args)?;
    Ok(None)
}

pub(super) fn run_lifecycle(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    let (id, renamed) = match args {
        [command, id] if command == "delete" => (parse_create_id(id)?, None),
        [command, old, new] if command == "rename" => {
            (parse_create_id(old)?, Some(parse_create_id(new)?))
        }
        _ => return Err(ERR_USAGE),
    };
    let dirs = existing_manager_profiles(context)?.ok_or(ERR_PROFILE)?;
    let _root_lock = directory_lock(&dirs, libc::LOCK_EX)?;
    if read_default(context)? == Some(ProfileTarget::Custom(id.clone())) {
        return Err(ManagerError::operation(
            "codex termux: choose another default profile before deleting or renaming this one",
        ));
    }
    validate_target(context, &ProfileTarget::Custom(id.clone()))?;
    let old = profile_path(&dirs, &id);
    if renamed
        .as_ref()
        .is_some_and(|new| fs::symlink_metadata(profile_path(&dirs, new)).is_ok())
    {
        return Err(ERR_COLLISION);
    }
    let home = profile_home_path(&dirs, &id);
    let _home_lock = directory_lock(&home, libc::LOCK_EX)?;
    if task::profile_in_use(context, &home)? || runtime_in_use(context, &home)? {
        return Err(ERR_BUSY);
    }
    if let Some(new) = renamed {
        // v2 remains valid under either name, including interruption before rename.
        if read_record(&old.join(PROFILE_META))? != METADATA_V2 {
            publish_record(&old.join(PROFILE_META), METADATA_V2)?;
        }
        rename_noreplace(&old, &profile_path(&dirs, &new)).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                ERR_COLLISION
            } else {
                ERR_CREATE
            }
        })?;
        sync_directory(&dirs)?;
        Ok(Some(format!("renamed: {id} -> {new}\n")))
    } else {
        bounded_tree(&old, 0, &mut 0)?;
        let retired = dirs.join(format!(
            ".delete-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        rename_noreplace(&old, &retired).map_err(|_| ERR_CREATE)?;
        sync_directory(&dirs)?;
        remove_private_tree(&retired).map_err(|_| ERR_CREATE)?;
        sync_directory(&dirs)?;
        Ok(Some(format!("deleted: {id}\n")))
    }
}

fn bounded_tree(path: &Path, depth: usize, count: &mut usize) -> Result<(), ManagerError> {
    *count += 1;
    if depth > 128 || *count > 65536 {
        return Err(ERR_CREATE);
    }
    let m = fs::symlink_metadata(path).map_err(|_| ERR_CREATE)?;
    if m.is_dir() && !m.file_type().is_symlink() {
        for entry in fs::read_dir(path).map_err(|_| ERR_CREATE)? {
            bounded_tree(&entry.map_err(|_| ERR_CREATE)?.path(), depth + 1, count)?;
        }
    }
    Ok(())
}

// Only execution identity is retained. All unrelated environment values are
// discarded while streaming; neither argv nor account payload is inspected.
fn execution_home(reader: impl Read) -> Result<Option<PathBuf>, ManagerError> {
    let mut key = Vec::new();
    let mut value = Vec::new();
    let mut in_value = false;
    let mut selected = false;
    let mut home = None;
    for (count, byte) in std::io::BufReader::new(reader).bytes().enumerate() {
        if count >= 1024 * 1024 {
            return Err(ERR_BUSY);
        }
        let byte = byte.map_err(|_| ERR_BUSY)?;
        if byte == 0 {
            if selected {
                if home.is_some() || value.len() > 4096 {
                    return Err(ERR_BUSY);
                }
                home = Some(PathBuf::from(OsString::from_vec(std::mem::take(
                    &mut value,
                ))));
            }
            key.clear();
            in_value = false;
            selected = false;
        } else if !in_value && byte == b'=' {
            in_value = true;
            selected = key == b"CODEX_HOME";
        } else if !in_value {
            if key.len() < 64 {
                key.push(byte);
            }
        } else if selected {
            if value.len() >= 4096 {
                return Err(ERR_BUSY);
            }
            value.push(byte);
        }
    }
    if in_value || !key.is_empty() {
        return Err(ERR_BUSY);
    }
    Ok(home)
}

fn process_alive(path: &Path) -> Result<bool, ManagerError> {
    let stat = match fs::read_to_string(path.join("stat")) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(ERR_BUSY),
    };
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or(ERR_BUSY)?
        .1
        .split_whitespace()
        .collect();
    let state = fields.first().ok_or(ERR_BUSY)?;
    let threads = fields
        .get(17)
        .ok_or(ERR_BUSY)?
        .parse::<u64>()
        .map_err(|_| ERR_BUSY)?;
    Ok(!(threads == 1 && matches!(*state, "Z" | "X")))
}

fn local_process(path: &Path, directory_uid: u32, uid: u32) -> Result<bool, ManagerError> {
    if directory_uid == uid {
        return Ok(true);
    }
    if directory_uid != 0 {
        return Ok(false);
    }
    // A non-dumpable local process may have root-owned proc entries.
    let status = match fs::read_to_string(path.join("status")) {
        Ok(status) => status,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(ERR_BUSY),
    };
    let mut lines = status.lines().filter_map(|line| line.strip_prefix("Uid:"));
    let owners: Vec<u32> = lines
        .next()
        .ok_or(ERR_BUSY)?
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| ERR_BUSY)?;
    if owners.len() != 4 || lines.next().is_some() {
        return Err(ERR_BUSY);
    }
    Ok(owners.contains(&uid))
}

fn owned_account_path(path: &Path, home: &Path) -> bool {
    path.starts_with(home) || fs::canonicalize(path).is_ok_and(|p| p.starts_with(home))
}

fn runtime_in_use(context: &Context, home: &Path) -> Result<bool, ManagerError> {
    let generations = context.home.join(".local/lib/codex/core/generations");
    for (count, entry) in fs::read_dir("/proc").map_err(|_| ERR_BUSY)?.enumerate() {
        if count >= 32768 {
            return Err(ERR_BUSY);
        }
        let entry = entry.map_err(|_| ERR_BUSY)?;
        if !entry.file_name().as_bytes().iter().all(u8::is_ascii_digit) {
            continue;
        }
        let path = entry.path();
        let m = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => return Err(ERR_BUSY),
        };
        if !local_process(&path, m.uid(), unsafe { libc::geteuid() })? || !process_alive(&path)? {
            continue;
        }
        let executable = match fs::read_link(path.join("exe")) {
            Ok(p) => p,
            Err(_) if !process_alive(&path)? => continue,
            Err(_) => return Err(ERR_BUSY),
        };
        let bytes = executable.as_os_str().as_bytes();
        let executable = PathBuf::from(OsString::from_vec(
            bytes.strip_suffix(b" (deleted)").unwrap_or(bytes).to_vec(),
        ));
        let native_runtime = executable.file_name() == Some(OsStr::new("runtime"))
            && executable.parent().and_then(Path::parent) == Some(generations.as_path());
        if !native_runtime
            && executable != context.core_entrypoint
            && executable
                != context
                    .home
                    .join(".local/share/codex/core/core-entrypoint-rollback")
        {
            continue;
        }
        let in_use = (|| {
            if execution_home(File::open(path.join("environ")).map_err(|_| ERR_BUSY)?)?
                .is_some_and(|p| owned_account_path(&p, home))
            {
                return Ok(true);
            }
            if owned_account_path(
                &fs::read_link(path.join("cwd")).map_err(|_| ERR_BUSY)?,
                home,
            ) {
                return Ok(true);
            }
            for (count, fd) in fs::read_dir(path.join("fd"))
                .map_err(|_| ERR_BUSY)?
                .enumerate()
            {
                if count >= 8192 {
                    return Err(ERR_BUSY);
                }
                let fd = fd.map_err(|_| ERR_BUSY)?;
                match fs::read_link(fd.path()) {
                    Ok(p) if owned_account_path(&p, home) => return Ok(true),
                    Ok(_) => (),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                    Err(_) => return Err(ERR_BUSY),
                }
            }
            Ok(false)
        })();
        match in_use {
            Ok(true) if process_alive(&path)? => return Ok(true),
            Ok(_) => (),
            Err(_) if !process_alive(&path)? => (),
            Err(e) => return Err(e),
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_execution_metadata_is_bounded_and_keeps_only_home() {
        assert_eq!(
            execution_home(b"OTHER=discard\0CODEX_HOME=/owned/home\0TOKEN=discard\0".as_slice())
                .unwrap(),
            Some(PathBuf::from("/owned/home"))
        );
        assert!(execution_home(b"CODEX_HOME=a\0CODEX_HOME=b\0".as_slice()).is_err());
        assert!(execution_home(b"CODEX_HOME=unfinished".as_slice()).is_err());
        assert!(execution_home(vec![b'a'; 1024 * 1024 + 1].as_slice()).is_err());
        assert!(execution_home(format!("CODEX_HOME={}\0", "x".repeat(4097)).as_bytes()).is_err());
        assert!(execution_home(b"OTHER=value\0".as_slice())
            .unwrap()
            .is_none());
    }

    #[test]
    fn profile_delete_preflight_is_bounded_and_never_follows_links() {
        let path = std::env::temp_dir().join(format!(
            "codex-profile-bound-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        std::os::unix::fs::symlink("/nonexistent/outside", path.join("link")).unwrap();
        assert!(bounded_tree(&path, 0, &mut 0).is_ok());
        assert!(bounded_tree(&path, 0, &mut 65536).is_err());
        assert!(bounded_tree(&path, 129, &mut 0).is_err());
        assert!(bounded_tree(&path.join("missing"), 0, &mut 0).is_err());
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn profile_process_identity_never_skips_live_siblings_or_root_reassigned_uids() {
        let root = std::env::temp_dir().join(format!(
            "codex-profile-process-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        assert!(local_process(&root, 101, 101).unwrap());
        assert!(!local_process(&root, 102, 101).unwrap());
        assert!(!local_process(&root, 0, 101).unwrap());
        for (uids, owned) in [
            ("101 101 101 101", true),
            ("0 0 101 0", true),
            ("0 0 0 0", false),
        ] {
            fs::write(root.join("status"), format!("Name: owned\nUid: {uids}\n")).unwrap();
            assert_eq!(local_process(&root, 0, 101).unwrap(), owned);
        }
        for text in [
            "bad",
            "Uid: 1 2 3",
            "Uid: 1 bad 3 4",
            "Uid: 1 2 3 4\nUid: 1 2 3 4",
        ] {
            fs::write(root.join("status"), text).unwrap();
            assert!(local_process(&root, 0, 101).is_err());
        }
        for (state, threads, alive) in [
            ("Z", "1", false),
            ("X", "1", false),
            ("Z", "2", true),
            ("S", "1", true),
        ] {
            let mut fields = vec!["0"; 18];
            fields[0] = state;
            fields[17] = threads;
            fs::write(
                root.join("stat"),
                format!("1 (owned ) name) {}", fields.join(" ")),
            )
            .unwrap();
            assert_eq!(process_alive(&root).unwrap(), alive);
        }
        fs::write(root.join("stat"), "bad").unwrap();
        assert!(process_alive(&root).is_err());
        fs::remove_file(root.join("stat")).unwrap();
        assert!(!process_alive(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
