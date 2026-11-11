//! Observe Core-published Native processes and kernel writer ownership.
//! No account selection, Native execution, approval or history mutation.
use crate::{Binding, FAMILIES};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_SERVERS: usize = 64;
fn invalid() -> io::Error {
    failure(
        "native_owner_unsafe",
        "identity or ownership could not be verified",
    )
}
fn failure(code: &str, reason: &str) -> io::Error {
    io::Error::other(format!("{code}: {reason}"))
}
#[derive(PartialEq, Eq)]
enum PathPresence {
    Missing,
    Present,
}
#[derive(Debug, Clone)]
pub struct Server {
    pub home: PathBuf,
    pub program: PathBuf,
    pub socket: PathBuf,
    pub pid: u32,
    pub start: String,
}
impl Server {
    pub fn token(&self) -> String {
        format!("{}:{}", self.pid, self.start)
    }
    pub fn identity(&self) -> Result<(), io::Error> {
        if process_start(self.pid)? != self.start
            || fs::read_link(format!("/proc/{}/exe", self.pid)).map_err(|_| invalid())?
                != self.program
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn stream(&self, deadline: Instant) -> Result<UnixStream, io::Error> {
        self.identity()?;
        let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        address.sun_family = libc::AF_UNIX as libc::sa_family_t;
        let bytes = self.socket.as_os_str().as_bytes();
        if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
            return Err(invalid());
        }
        for (target, byte) in address.sun_path.iter_mut().zip(bytes) {
            *target = *byte as libc::c_char;
        }
        let raw = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        };
        if raw < 0 {
            return Err(invalid());
        }
        let stream = unsafe { UnixStream::from_raw_fd(raw) };
        loop {
            if Instant::now() >= deadline {
                return Err(invalid());
            }
            if unsafe {
                libc::connect(
                    raw,
                    (&address as *const libc::sockaddr_un).cast(),
                    (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1)
                        as libc::socklen_t,
                )
            } == 0
            {
                break;
            }
            match io::Error::last_os_error().raw_os_error() {
                Some(libc::EAGAIN | libc::EINTR) => thread::sleep(Duration::from_millis(10)),
                _ => return Err(invalid()),
            }
        }
        stream.set_nonblocking(false).map_err(|_| invalid())?;
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut peer as *mut libc::ucred).cast(),
                &mut len,
            )
        } != 0
            || peer.pid != self.pid as i32
            || peer.uid != unsafe { libc::geteuid() }
        {
            return Err(invalid());
        }
        self.identity()?;
        Ok(stream)
    }
}
pub fn process_start(pid: u32) -> Result<String, io::Error> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).map_err(|_| invalid())?;
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or_else(invalid)?
        .1
        .split_whitespace()
        .collect();
    if matches!(fields.first(), Some(&"Z" | &"X")) {
        return Err(invalid());
    }
    let start = fields.get(19).ok_or_else(invalid)?;
    if !start.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    Ok((*start).to_owned())
}
pub fn process_exited(pid: u32) -> Result<bool, io::Error> {
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(_) => return Err(invalid()),
    };
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or_else(invalid)?
        .1
        .split_whitespace()
        .collect();
    Ok(matches!(fields.first(), Some(&"Z" | &"X")) && fields.get(17) == Some(&"1"))
}
pub fn private(path: &Path, directory: bool) -> Result<(), io::Error> {
    if inspect_path(path)? != PathPresence::Present {
        return Err(invalid());
    }
    let m = fs::symlink_metadata(path).map_err(|_| invalid())?;
    if m.uid() != unsafe { libc::geteuid() }
        || m.permissions().mode() & 0o7777 != if directory { 0o700 } else { 0o600 }
        || if directory { !m.is_dir() } else { !m.is_file() }
    {
        return Err(invalid());
    }
    Ok(())
}
pub fn record(path: &Path) -> Result<Vec<u8>, io::Error> {
    bounded_record(path, 16384)
}
fn bounded_record(path: &Path, limit: u64) -> Result<Vec<u8>, io::Error> {
    private(path, false)?;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| invalid())?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > limit {
        return Err(invalid());
    }
    Ok(bytes)
}
pub fn servers(home_root: &Path, deadline: Instant) -> Result<Vec<Server>, io::Error> {
    Ok(registry(home_root, deadline, false)?.0)
}
fn registry(
    home_root: &Path,
    deadline: Instant,
    retain_writer_homes: bool,
) -> Result<(Vec<Server>, BTreeSet<PathBuf>), io::Error> {
    let mut servers = Vec::new();
    let mut homes = BTreeSet::from([home_root.join(".codex")]);
    for family in FAMILIES {
        servers.extend(server_family(
            home_root,
            family,
            deadline,
            retain_writer_homes.then_some(&mut homes),
        )?);
        if servers.len() > MAX_SERVERS || homes.len() > MAX_SERVERS {
            return Err(invalid());
        }
    }
    Ok((servers, homes))
}

fn server_family(
    home_root: &Path,
    family: &str,
    deadline: Instant,
    mut writer_homes: Option<&mut BTreeSet<PathBuf>>,
) -> Result<Vec<Server>, io::Error> {
    let root = home_root.join(".local/share/codex/core").join(family);
    if inspect_path(&root)? == PathPresence::Missing {
        return Ok(Vec::new());
    }
    private(&root, true)?;
    let mut result = Vec::new();
    for entry in fs::read_dir(&root).map_err(|_| invalid())? {
        if Instant::now() >= deadline {
            return Err(invalid());
        }
        let dir = entry.map_err(|_| invalid())?.path();
        private(&dir, true)?;
        // Core holds this directory's exclusive flock while publishing/restarting.
        // Never classify an old PID as retired during a new owner's publication.
        let snapshot = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(&dir)
            .map_err(|_| invalid())?;
        if unsafe { libc::flock(snapshot.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } != 0 {
            return Err(invalid());
        }
        let text = String::from_utf8(record(&dir.join("pid"))?).map_err(|_| invalid())?;
        let pid = text
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|p| *p > 1)
            .ok_or_else(invalid)?;
        // Parse the owned record before excluding an exited or reused PID.
        // Only the two known legacy codecs may bypass namespace decoding;
        // unknown codecs and malformed current records never prove absence.
        let owner_record = record(&dir.join("owner"))?;
        let fields = owner_record.split(|byte| *byte == 0).collect::<Vec<_>>();
        let legacy = matches!(fields.as_slice(), [_, _] | [_, _, b"catalog-v1"]);
        let binding = if legacy {
            None
        } else {
            Some(
                Binding::read(&dir, |path, limit| {
                    if inspect_path(path)
                        .map_err(|_| io::Error::other("unsafe execution record"))?
                        == PathPresence::Missing
                    {
                        return Err(io::Error::from(io::ErrorKind::NotFound));
                    }
                    bounded_record(path, limit)
                        .map_err(|_| io::Error::other("unsafe execution record"))
                })
                .map_err(|_| {
                    failure(
                        "native_owner_binding_invalid",
                        "execution binding is unsupported or invalid",
                    )
                })?,
            )
        };
        let home = fields
            .first()
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| PathBuf::from(OsString::from_vec(bytes.to_vec())))
            .ok_or_else(invalid)?;
        let program = fields
            .get(1)
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| PathBuf::from(OsString::from_vec(bytes.to_vec())))
            .ok_or_else(|| {
                failure(
                    "native_owner_registry_invalid",
                    "execution identity is malformed",
                )
            })?;
        let generations = home_root.join(".local/lib/codex/core/generations");
        if !is_safe_absolute_path(&home)
            || !is_safe_absolute_path(&program)
            || program.file_name() != Some(OsStr::new("runtime"))
            || program.parent().and_then(Path::parent) != Some(generations.as_path())
        {
            return Err(invalid());
        }
        // A retired process has no execution authority. Its verified record
        // still identifies a writer namespace that may have another holder.
        if let Some(homes) = writer_homes.as_mut() {
            if inspect_path(&home)? == PathPresence::Present {
                private(&home, true)?;
            }
            homes.insert(home);
            if homes.len() > MAX_SERVERS {
                return Err(invalid());
            }
        }
        let start = match process_start(pid) {
            Ok(start) => start,
            Err(_) if process_exited(pid)? => continue,
            Err(_) => return Err(invalid()),
        };
        match fs::read_link(format!("/proc/{pid}/exe")) {
            Ok(actual)
                if actual.as_os_str().as_bytes().strip_suffix(b" (deleted)")
                    == Some(program.as_os_str().as_bytes()) =>
            {
                return Err(invalid());
            }
            Ok(actual) if actual != program => continue,
            Ok(_) => (),
            Err(_) if process_exited(pid)? => continue,
            Err(_) => return Err(invalid()),
        }
        let Binding { home, program } = binding.ok_or_else(|| {
            failure(
                "native_owner_binding_invalid",
                "matching live legacy execution binding is unsupported",
            )
        })?;
        private(&home, true)?;
        if !is_existing_executable(&program)? {
            return Err(invalid());
        }
        let m = fs::metadata(&program).map_err(|_| invalid())?;
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
            return Err(invalid());
        }
        let socket = dir.join("s");
        let m = fs::symlink_metadata(&socket).map_err(|_| invalid())?;
        let physical = if m.file_type().is_symlink() {
            fs::read_link(&socket).map_err(|_| invalid())?
        } else {
            socket.clone()
        };
        if !is_safe_absolute_path(&physical) || inspect_path(&physical)? != PathPresence::Present {
            return Err(invalid());
        }
        let m = fs::symlink_metadata(&physical).map_err(|_| invalid())?;
        if !m.file_type().is_socket()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o077 != 0
        {
            return Err(invalid());
        }
        result.push(Server {
            home,
            program,
            socket: physical,
            pid,
            start,
        });
        if result.len() > MAX_SERVERS {
            return Err(invalid());
        }
    }
    result.sort_by_key(|s| (s.home.clone(), s.pid));
    Ok(result)
}

fn native_file(path: &Path) -> Result<Option<File>, io::Error> {
    if inspect_path(path)? == PathPresence::Missing {
        return Ok(None);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| invalid())?;
    let m = file.metadata().map_err(|_| invalid())?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err(invalid());
    }
    Ok(Some(file))
}
pub fn writer_free(home_root: &Path, id: &str) -> Result<bool, io::Error> {
    writer_free_until(home_root, id, Instant::now() + Duration::from_secs(2))
}
/// Observe all known writer namespaces without opening a server or a writer.
/// The result is an observation, not a lease for a later Native acquisition.
pub fn writer_free_until(home_root: &Path, id: &str, deadline: Instant) -> Result<bool, io::Error> {
    if !canonical_session_id(id) {
        return Err(invalid());
    }
    let (_, homes) = registry(home_root, deadline, true)?;
    for directory in writer_directories(home_root, homes)? {
        if Instant::now() >= deadline {
            return Err(invalid());
        }
        if !writer_free_directory(&directory, id)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn writer_directory(home_root: &Path, execution_home: &Path) -> Result<PathBuf, io::Error> {
    if inspect_path(execution_home)? == PathPresence::Present {
        private(execution_home, true)?;
    }
    let directory = execution_home.join("thread-writer-locks");
    let shared = home_root.join(".codex/thread-writer-locks");
    let directory = match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            // This is the single topology alias produced by Core shared_layout.
            // An arbitrary canonicalized alias is never a writer authority.
            if directory == shared || fs::read_link(&directory)? != shared {
                return Err(invalid());
            }
            if inspect_path(&shared)? != PathPresence::Present {
                return Err(invalid());
            }
            shared
        }
        Ok(_) => directory,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(directory),
        Err(_) => return Err(invalid()),
    };
    if inspect_path(&directory)? != PathPresence::Present {
        return Err(invalid());
    }
    let metadata = fs::symlink_metadata(&directory).map_err(|_| invalid())?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
    {
        return Err(invalid());
    }
    Ok(directory)
}
fn writer_directories(
    home_root: &Path,
    homes: BTreeSet<PathBuf>,
) -> Result<BTreeSet<PathBuf>, io::Error> {
    homes
        .into_iter()
        .map(|home| writer_directory(home_root, &home))
        .collect()
}
fn writer_free_directory(directory: &Path, id: &str) -> Result<bool, io::Error> {
    if inspect_path(directory)? == PathPresence::Missing {
        return Ok(true);
    }
    let Some(coordination) = native_file(&directory.join(".coordination.lock"))? else {
        return if inspect_path(&directory.join(format!("{id}.lock")))? == PathPresence::Missing {
            Ok(true)
        } else {
            Err(invalid())
        };
    };
    match coordination.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(false),
        Err(_) => return Err(invalid()),
    }
    let Some(writer) = native_file(&directory.join(format!("{id}.lock")))? else {
        return Ok(true);
    };
    match writer.try_lock() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(_) => Err(invalid()),
    }
}
pub fn owns_writer(home_root: &Path, id: &str, server: &Server) -> Result<bool, io::Error> {
    server.identity()?;
    if !canonical_session_id(id) {
        return Err(invalid());
    }
    let path = writer_directory(home_root, &server.home)?.join(format!("{id}.lock"));
    let Some(file) = native_file(&path)? else {
        return Ok(false);
    };
    let target = file.metadata().map_err(|_| invalid())?;
    let dir = PathBuf::from(format!("/proc/{}/fd", server.pid));
    for (count, entry) in fs::read_dir(dir).map_err(|_| invalid())?.enumerate() {
        if count >= 4096 {
            return Err(invalid());
        }
        let entry = entry.map_err(|_| invalid())?;
        let m = match fs::metadata(entry.path()) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => return Err(invalid()),
        };
        if m.ino() != target.ino() || m.dev() != target.dev() {
            continue;
        }
        let info = fs::read_to_string(format!(
            "/proc/{}/fdinfo/{}",
            server.pid,
            entry.file_name().to_string_lossy()
        ))
        .map_err(|_| invalid())?;
        if info.lines().any(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            fields.len() >= 6
                && fields[0] == "lock:"
                && fields[2] == "FLOCK"
                && fields[4] == "WRITE"
                && fields[5] == server.pid.to_string()
        }) {
            server.identity()?;
            return Ok(true);
        }
    }
    Ok(false)
}
fn is_safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn inspect_path(path: &Path) -> Result<PathPresence, io::Error> {
    if !is_safe_absolute_path(path) {
        return Err(invalid());
    }
    let components: Vec<OsString> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_owned()),
            Component::RootDir => None,
            _ => None,
        })
        .collect();
    let mut current = PathBuf::from("/");
    let component_count = components.len();
    for (index, component) in components.into_iter().enumerate() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || (index + 1 != component_count && !metadata.is_dir())
                {
                    return Err(invalid());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(PathPresence::Missing);
            }
            Err(_) => return Err(invalid()),
        }
    }
    Ok(PathPresence::Present)
}

fn is_existing_executable(path: &Path) -> Result<bool, io::Error> {
    if inspect_path(path)? != PathPresence::Present {
        return Ok(false);
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| invalid())?;
    Ok(metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0)
}

pub fn canonical_session_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
            }
        })
}

/// Return a verified live rendezvous; None requires confirmed free ownership.
pub fn owner(home_root: &Path, id: &str, deadline: Instant) -> io::Result<Option<Server>> {
    if !canonical_session_id(id) {
        return Err(invalid());
    }
    let (servers, homes) = registry(home_root, deadline, true)?;
    let directories = writer_directories(home_root, homes)?;
    let mut owner = None;
    let mut owned_directory = None;
    for server in servers {
        if owns_writer(home_root, id, &server)? {
            if owner.is_some() {
                return Err(failure(
                    "native_owner_ambiguous",
                    "multiple verified writers were discovered",
                ));
            }
            // Kernel peer identity, not an unrelated server's responsiveness.
            server.stream(deadline)?;
            if !owns_writer(home_root, id, &server)? {
                return Err(invalid());
            }
            owned_directory = Some(writer_directory(home_root, &server.home)?);
            owner = Some(server);
        }
    }
    for directory in directories {
        if Instant::now() >= deadline {
            return Err(invalid());
        }
        if owned_directory.as_ref() != Some(&directory) && !writer_free_directory(&directory, id)? {
            return Err(failure(
                "native_owner_writer_unresolved",
                "writer is held but its qualified owner is unavailable",
            ));
        }
    }
    if let Some(server) = owner.as_ref() {
        if !owns_writer(home_root, id, server)? {
            return Err(invalid());
        }
    }
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREAD: &str = "12345678-1234-1234-1234-123456789abc";
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "nw-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            Self(root)
        }
        fn directory(&self, relative: &str) -> PathBuf {
            let path = self.0.join(relative);
            fs::create_dir_all(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            path
        }
        fn file(&self, path: &Path, bytes: &[u8]) -> File {
            fs::write(path, bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
            File::open(path).unwrap()
        }
        fn server(&self, home: PathBuf) -> Server {
            Server {
                home,
                program: std::env::current_exe().unwrap(),
                socket: self.0.join("s"),
                pid: std::process::id(),
                start: process_start(std::process::id()).unwrap(),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn writer_identity_follows_independent_execution_home() {
        let f = Fixture::new();
        let home = f.directory("independent");
        let locks = f.directory("independent/thread-writer-locks");
        let coordination = f.file(&locks.join(".coordination.lock"), b"");
        drop(coordination);
        let writer = f.file(&locks.join(format!("{THREAD}.lock")), b"original writer");
        writer.try_lock().unwrap();
        let server = f.server(home);
        assert!(
            owns_writer(&f.0, THREAD, &server).unwrap(),
            "the registered CODEX_HOME owns its real kernel writer"
        );
        assert!(
            !f.0.join(".codex").exists(),
            "lookup must not create shared state"
        );
        assert_eq!(
            fs::read(locks.join(format!("{THREAD}.lock"))).unwrap(),
            b"original writer"
        );
    }
    #[test]
    fn canonical_writer_alias_is_exact_and_never_general_path_resolution() {
        let f = Fixture::new();
        let shared = f.directory(".codex/thread-writer-locks");
        let home = f.directory(".codex-profiles/account");
        let alias = home.join("thread-writer-locks");
        std::os::unix::fs::symlink(&shared, &alias).unwrap();
        let writer = f.file(&shared.join(format!("{THREAD}.lock")), b"shared writer");
        writer.try_lock().unwrap();
        let server = f.server(home);
        assert!(owns_writer(&f.0, THREAD, &server).unwrap());
        fs::remove_file(&alias).unwrap();
        let other = f.directory("other/thread-writer-locks");
        std::os::unix::fs::symlink(&other, &alias).unwrap();
        assert!(owns_writer(&f.0, THREAD, &server).is_err());
        fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(&shared, &alias).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(owns_writer(&f.0, THREAD, &server).is_err());
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(owns_writer(&f.0, THREAD, &server).unwrap());
        assert_eq!(
            fs::read(shared.join(format!("{THREAD}.lock"))).unwrap(),
            b"shared writer"
        );
    }

    #[test]
    fn retired_registration_retains_unknown_writer_and_publication_uncertainty() {
        let f = Fixture::new();
        let home = f.directory("independent");
        let locks = f.directory("independent/thread-writer-locks");
        let coordination = f.file(&locks.join(".coordination.lock"), b"");
        let writer = f.file(&locks.join(format!("{THREAD}.lock")), b"unresolved writer");
        let family = f.directory(".local/share/codex/core/servers");
        let entry = family.join("retained");
        fs::create_dir(&entry).unwrap();
        fs::set_permissions(&entry, fs::Permissions::from_mode(0o700)).unwrap();
        let mut bytes = home.as_os_str().as_bytes().to_vec();
        bytes.push(0);
        bytes.extend_from_slice(
            f.0.join(".local/lib/codex/core/generations/retired/runtime")
                .as_os_str()
                .as_bytes(),
        );
        f.file(&entry.join("owner"), &bytes);
        f.file(&entry.join("pid"), b"4294967294\n");
        writer.try_lock().unwrap();
        assert!(!writer_free(&f.0, THREAD).unwrap());
        assert!(owner(&f.0, THREAD, Instant::now() + Duration::from_secs(2)).is_err());
        drop(writer);
        coordination.try_lock().unwrap();
        assert!(!writer_free(&f.0, THREAD).unwrap());
        drop(coordination);
        assert!(writer_free(&f.0, THREAD).unwrap());
        assert!(owner(&f.0, THREAD, Instant::now() + Duration::from_secs(2))
            .unwrap()
            .is_none());
        assert_eq!(fs::read(entry.join("owner")).unwrap(), bytes);
        let lock = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(&entry)
            .unwrap();
        lock.try_lock().unwrap();
        assert!(
            writer_free(&f.0, THREAD).is_err(),
            "a publishing registry is not safe absence"
        );
        drop(lock);
        fs::remove_file(entry.join("owner")).unwrap();
        let substituted = f.directory("alias-home");
        std::os::unix::fs::symlink(&home, substituted.join("home")).unwrap();
        let mut changed = substituted.join("home").as_os_str().as_bytes().to_vec();
        changed.push(0);
        changed.extend_from_slice(
            f.0.join(".local/lib/codex/core/generations/retired/runtime")
                .as_os_str()
                .as_bytes(),
        );
        f.file(&entry.join("owner"), &changed);
        assert!(
            writer_free(&f.0, THREAD).is_err(),
            "an unsafe retained home cannot become null"
        );
    }
    #[test]
    fn retired_and_reused_records_validate_current_codec_and_known_legacy_shapes() {
        let f = Fixture::new();
        let home = f.directory("independent");
        let program =
            f.0.join(".local/lib/codex/core/generations/retired/runtime");
        let config = b"owned settings";
        let family = f.directory(".local/share/codex/core/servers");
        let entry = family.join(crate::configuration_namespace(
            &home,
            program.as_os_str(),
            None,
            None,
            config,
            None,
        ));
        fs::create_dir(&entry).unwrap();
        fs::set_permissions(&entry, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(entry.join("config")).unwrap();
        fs::set_permissions(entry.join("config"), fs::Permissions::from_mode(0o700)).unwrap();
        f.file(&entry.join("config/config.toml"), config);
        let current = crate::configuration_owner_bytes(&home, program.as_os_str(), false, false);
        let mut legacy = home.as_os_str().as_bytes().to_vec();
        legacy.push(0);
        legacy.extend_from_slice(program.as_os_str().as_bytes());
        let mut exited = std::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap();
        let dead_pid = exited.id();
        assert!(exited.wait().unwrap().success());
        for pid in [dead_pid, std::process::id()] {
            f.file(&entry.join("pid"), format!("{pid}\n").as_bytes());
            for valid in [
                &current,
                &legacy,
                &[legacy.as_slice(), b"\0catalog-v1"].concat(),
            ] {
                f.file(&entry.join("owner"), valid);
                assert!(servers(&f.0, Instant::now() + Duration::from_secs(2))
                    .unwrap()
                    .is_empty());
                assert!(writer_free(&f.0, THREAD).unwrap());
                assert!(owner(&f.0, THREAD, Instant::now() + Duration::from_secs(2))
                    .unwrap()
                    .is_none());
                assert_eq!(fs::read(entry.join("owner")).unwrap(), *valid);
            }
            for malformed in [
                [legacy.as_slice(), b"\0unknown-v9"].concat(),
                [current.as_slice(), b"\0unexpected"].concat(),
                [home.as_os_str().as_bytes(), b"\0"].concat(),
            ] {
                f.file(&entry.join("owner"), &malformed);
                assert!(servers(&f.0, Instant::now() + Duration::from_secs(2)).is_err());
                assert!(writer_free(&f.0, THREAD).is_err());
                assert!(owner(&f.0, THREAD, Instant::now() + Duration::from_secs(2)).is_err());
                assert_eq!(fs::read(entry.join("owner")).unwrap(), malformed);
            }
            f.file(&entry.join("owner"), &current);
            f.file(&entry.join("config/config.toml"), b"substituted settings");
            assert!(servers(&f.0, Instant::now() + Duration::from_secs(2)).is_err());
            assert!(owner(&f.0, THREAD, Instant::now() + Duration::from_secs(2)).is_err());
            f.file(&entry.join("config/config.toml"), config);
        }
    }
}
