//! Observe Core-published Native processes and kernel writer ownership.
//! No account selection, Native execution, approval or history mutation.
use crate::{Binding, FAMILIES};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_SERVERS: usize = 64;
fn invalid() -> io::Error {
    io::Error::other("Native owner is unavailable, ambiguous, unsafe or unresponsive")
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
    let mut servers = Vec::new();
    for family in FAMILIES {
        servers.extend(server_family(home_root, family, deadline)?);
        if servers.len() > MAX_SERVERS {
            return Err(invalid());
        }
    }
    Ok(servers)
}

fn server_family(
    home_root: &Path,
    family: &str,
    deadline: Instant,
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
        let start = match process_start(pid) {
            Ok(start) => start,
            Err(_) if process_exited(pid)? => continue,
            Err(_) => return Err(invalid()),
        };
        let Binding { home, program } = Binding::read(&dir, |path, limit| {
            if inspect_path(path).map_err(|_| io::Error::other("unsafe execution record"))?
                == PathPresence::Missing
            {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
            bounded_record(path, limit).map_err(|_| io::Error::other("unsafe execution record"))
        })
        .map_err(|_| invalid())?;
        let generations = home_root.join(".local/lib/codex/core/generations");
        if program.file_name() != Some(OsStr::new("runtime"))
            || program.parent().and_then(Path::parent) != Some(generations.as_path())
        {
            return Err(invalid());
        }
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
    let directory = home_root.join(".codex/thread-writer-locks");
    if inspect_path(&directory)? == PathPresence::Missing {
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
    let path = home_root.join(format!(".codex/thread-writer-locks/{id}.lock"));
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
    let mut owner = None;
    for server in servers(home_root, deadline)? {
        if owns_writer(home_root, id, &server)? {
            if owner.is_some() {
                return Err(invalid());
            }
            // Kernel peer identity, not an unrelated server's responsiveness.
            server.stream(deadline)?;
            if !owns_writer(home_root, id, &server)? {
                return Err(invalid());
            }
            owner = Some(server);
        }
    }
    if owner.is_none() && !writer_free(home_root, id)? {
        return Err(invalid());
    }
    Ok(owner)
}
