//! Starts only the signed generation's foreground app-server; upstream discovers
//! its ordinary socket and therefore never enters package/installer lifecycle.
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn invalid() -> io::Error {
    io::Error::other("shared server state is unavailable or does not match the qualified runtime")
}

pub(super) fn eligible(args: &[OsString]) -> bool {
    let command = super::upstream_root_command_index(args).and_then(|i| args[i].to_str());
    !matches!(
        command,
        Some(
            "tcp-tunnel"
                | "exec"
                | "e"
                | "review"
                | "login"
                | "logout"
                | "mcp"
                | "plugin"
                | "app-server"
                | "remote-control"
                | "app"
                | "completion"
                | "update"
                | "doctor"
                | "sandbox"
                | "debug"
                | "execpolicy"
                | "apply"
                | "a"
                | "queue"
                | "archive"
                | "delete"
                | "migrate-rollouts"
                | "unarchive"
                | "cloud"
                | "cloud-tasks"
                | "responses-api-proxy"
                | "stdio-to-uds"
                | "exec-server"
                | "features"
                | "help"
        )
    ) && !args.iter().any(|arg| {
        let Some(arg) = arg.to_str() else {
            return false;
        };
        arg == "--no-daemon"
            || arg == "--oss"
            || arg == "--strict-config"
            || arg == "--dangerously-bypass-hook-trust"
            || arg == "--search"
            || arg == "-p"
            || arg.starts_with("--profile")
            || arg == "--remote"
            || arg.starts_with("--remote=")
            || arg == "--enable"
            || arg.starts_with("--enable=")
            || arg == "--disable"
            || arg.starts_with("--disable=")
            || arg.starts_with("-c")
            || arg.starts_with("--config")
            || arg == "--help"
            || arg == "-h"
            || arg == "--version"
            || arg == "-V"
    }) && std::env::var_os("CODEX_EXEC_SERVER_URL").is_none()
}

pub(super) fn private_dir(path: &Path) -> io::Result<()> {
    if !super::core_notify_safe_absolute_path(path) {
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

fn namespace(home: &Path, program: &OsStr) -> String {
    // The exact owner record below makes a hash collision a rejection, not aliasing.
    let mut value = 0xcbf29ce484222325_u64;
    for b in home
        .as_os_str()
        .as_bytes()
        .iter()
        .chain([0].iter())
        .chain(program.as_bytes())
    {
        value = (value ^ u64::from(*b)).wrapping_mul(0x100000001b3);
    }
    format!("{value:016x}")
}

fn read_record(path: &Path) -> io::Result<Vec<u8>> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > 16384 {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(16385).read_to_end(&mut bytes)?;
    if bytes.len() > 16384 {
        return Err(invalid());
    }
    Ok(bytes)
}

fn write_record(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        File::open(path.parent().ok_or_else(invalid)?)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

fn bound_connection(socket: &Path, record: &Path, program: &OsStr) -> io::Result<bool> {
    let stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            return Ok(false)
        }
        Err(e) => return Err(e),
    };
    let pid = String::from_utf8(read_record(record)?)
        .map_err(|_| invalid())?
        .trim()
        .parse::<u32>()
        .map_err(|_| invalid())?;
    #[repr(C)]
    struct Peer {
        pid: i32,
        uid: u32,
        gid: u32,
    }
    unsafe extern "C" {
        fn getsockopt(fd: i32, level: i32, name: i32, value: *mut Peer, len: *mut u32) -> i32;
    }
    let mut peer = Peer {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<Peer>() as u32;
    if unsafe { getsockopt(stream.as_raw_fd(), 1, 17, &mut peer, &mut len) } != 0
        || peer.pid != pid as i32
        || peer.uid != fs::metadata("/proc/self")?.uid()
        || fs::read_link(format!("/proc/{pid}/exe"))?.as_os_str() != program
    {
        return Err(invalid());
    }
    Ok(true)
}

pub(super) fn ensure(
    program: &OsStr,
    profile: &Path,
    state: &Path,
    resolver: &Path,
    config: &Path,
    env: &super::TermuxBaseEnvPlan,
) -> io::Result<PathBuf> {
    private_dir(profile)?;
    let servers = state.join("servers");
    private_dir(&servers)?;
    let directory = servers.join(namespace(profile, program));
    private_dir(&directory)?;
    let lock = File::open(&directory)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while unsafe { super::flock(lock.as_raw_fd(), 2 | 4) } != 0 {
        if io::Error::last_os_error().kind() != io::ErrorKind::WouldBlock
            || Instant::now() >= deadline
        {
            return Err(invalid());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let mut binding = Vec::from(profile.as_os_str().as_bytes());
    binding.push(0);
    binding.extend_from_slice(program.as_bytes());
    let owner = directory.join("owner");
    match read_record(&owner) {
        Ok(b) if b == binding => (),
        Ok(_) => return Err(invalid()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => write_record(&owner, &binding)?,
        Err(e) => return Err(e),
    }
    let socket = directory.join("s");
    if socket.as_os_str().as_bytes().len() > 107 {
        return Err(invalid());
    }
    let logical_dir = profile.join("app-server-control");
    private_dir(&logical_dir)?;
    let logical = logical_dir.join("app-server-control.sock");
    match fs::symlink_metadata(&logical) {
        Ok(m) if m.file_type().is_symlink() => {
            let previous = fs::read_link(&logical)?;
            if previous != socket {
                // Only our bound namespace may be retargeted on generation change.
                if previous.parent().and_then(Path::parent) != Some(servers.as_path())
                    || previous.file_name() != Some(OsStr::new("s"))
                {
                    return Err(invalid());
                }
            }
        }
        Ok(_) => return Err(invalid()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e),
    }
    let temporary_root = env
        .assignments
        .iter()
        .find(|(name, _)| name == "TMPDIR")
        .map(|(_, value)| PathBuf::from(value))
        .ok_or_else(invalid)?;
    let pid_file = directory.join("pid");
    let running = bound_connection(&socket, &pid_file, program)?;
    if !running {
        match fs::symlink_metadata(&socket) {
            Ok(m) if m.file_type().is_socket() => fs::remove_file(&socket)?,
            Ok(m) if m.file_type().is_symlink() => {
                let target = fs::read_link(&socket)?;
                let native_root = fs::canonicalize(&temporary_root)?
                    .join(fs::metadata("/proc/self")?.uid().to_string());
                if target.parent() != Some(native_root.as_path())
                    || target.file_name().is_none_or(|n| {
                        n.as_bytes().len() != 64 || !n.as_bytes().iter().all(u8::is_ascii_hexdigit)
                    })
                {
                    return Err(invalid());
                }
                fs::remove_file(&socket)?;
            }
            Ok(_) => return Err(invalid()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    let snapshot = directory.join("config");
    private_dir(&snapshot)?;
    for name in ["config.toml", "requirements.toml"] {
        let source = config.join(name);
        match read_record(&source) {
            Ok(b) => match read_record(&snapshot.join(name)) {
                Ok(old) if old == b => (),
                Ok(_) => write_record(&snapshot.join(name), &b)?,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    write_record(&snapshot.join(name), &b)?
                }
                Err(e) => return Err(e),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                match read_record(&snapshot.join(name)) {
                    Ok(_) => fs::remove_file(snapshot.join(name))?,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e),
                }
            }
            Err(e) => return Err(e),
        }
    }
    if !running {
        let fds = super::RuntimeFdSources::open(resolver, &snapshot)?;
        let mut command = Command::new(program);
        command
            .args([OsStr::new("app-server"), OsStr::new("--listen")])
            .arg(format!("unix://{}", socket.to_str().ok_or_else(invalid)?))
            .env("CODEX_HOME", profile)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        super::apply_child_env_plan_and_fence(&mut command, Some(env));
        fds.configure(&mut command);
        unsafe extern "C" {
            fn setsid() -> i32;
        }
        unsafe {
            command.pre_exec(move || {
                if setsid() < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let mut child = command.spawn()?;
        let readiness = (|| {
            write_record(&pid_file, format!("{}\n", child.id()).as_bytes())?;
            loop {
                if bound_connection(&socket, &pid_file, program)? {
                    break;
                }
                if child.try_wait()?.is_some() || Instant::now() >= deadline {
                    return Err(invalid());
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(())
        })();
        if let Err(error) = readiness {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    }
    if fs::read_link(&logical).ok().as_ref() != Some(&socket) {
        let temporary = logical.with_extension(format!("tmp-{}", std::process::id()));
        std::os::unix::fs::symlink(&socket, &temporary)?;
        fs::rename(&temporary, &logical)?;
        File::open(&logical_dir)?.sync_all()?;
    }
    Ok(socket)
}

pub(super) fn retire_unused(
    state: &Path,
    generations: &Path,
    keep: &std::collections::HashSet<String>,
) -> io::Result<()> {
    let servers = state.join("servers");
    let entries = match fs::read_dir(&servers) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    private_dir(&servers)?;
    for entry in entries {
        let entry = entry?;
        let directory = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        private_dir(&directory)?;
        let owner = read_record(&directory.join("owner"))?;
        let mut parts = owner.split(|b| *b == 0);
        let profile = Path::new(OsStr::from_bytes(parts.next().ok_or_else(invalid)?));
        let program = OsStr::from_bytes(parts.next().ok_or_else(invalid)?);
        if parts.next().is_some() || entry.file_name() != OsStr::new(&namespace(profile, program)) {
            return Err(invalid());
        }
        let path = Path::new(program);
        let Some(relative) = path.strip_prefix(generations).ok() else {
            continue;
        };
        let mut components = relative.components();
        let Some(std::path::Component::Normal(id)) = components.next() else {
            continue;
        };
        let Some(id) = id.to_str() else { continue };
        if components.next() != Some(std::path::Component::Normal(OsStr::new("runtime")))
            || components.next().is_some()
            || super::m2_generation_state::validate_generation_identity(id, "server generation")
                .is_err()
            || keep.contains(id)
        {
            continue;
        }
        let lock = File::open(&directory)?;
        if unsafe { super::flock(lock.as_raw_fd(), 2 | 4) } != 0 {
            continue;
        }
        let pid_file = directory.join("pid");
        let pid = String::from_utf8(read_record(&pid_file)?)
            .map_err(|_| invalid())?
            .trim()
            .parse::<u32>()
            .map_err(|_| invalid())?;
        let process = PathBuf::from(format!("/proc/{pid}"));
        match fs::read_link(process.join("exe")) {
            Ok(exe) if exe.as_os_str() == program => (),
            Ok(_) => continue,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::remove_dir_all(&directory)?;
                File::open(&servers)?.sync_all()?;
                continue;
            }
            Err(e) => return Err(e),
        }
        if !bound_connection(&directory.join("s"), &pid_file, program)? {
            continue;
        }
        // Let our own peer-check connection close before counting open sockets.
        std::thread::sleep(Duration::from_millis(20));
        let mut sockets = 0;
        let mut writer = false;
        for file in fs::read_dir(process.join("fd"))? {
            let target = match fs::read_link(file?.path()) {
                Ok(target) => target,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            if target.as_os_str().as_bytes().starts_with(b"socket:[") {
                sockets += 1;
            }
            if target.components().any(|c| matches!(c,
                std::path::Component::Normal(n) if n == "sessions" || n == "archived_sessions" || n == "thread-writer-locks")) {
                writer = true;
            }
        }
        if sockets != 1 || writer {
            continue;
        }
        // Only upstream's graceful-only signal; a repeated request cannot force a turn.
        if fs::read_link(process.join("exe"))?.as_os_str() != program {
            continue;
        }
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        if unsafe { kill(pid as i32, 1) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(3) {
                return Err(error);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_server_selection_preserves_explicit_user_modes() {
        for args in [
            vec![],
            vec!["a plain prompt"],
            vec!["--model", "test-model", "a prompt"],
            vec!["agents"],
            vec!["--agents"],
            vec!["resume"],
            vec!["resume", "--all"],
            vec!["fork", "--last"],
        ] {
            let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
            assert!(eligible(&argv));
        }
        for args in [
            vec!["exec"],
            vec!["archive", "test-id"],
            vec!["help", "resume"],
            vec!["--version"],
            vec!["resume", "-c", "foo=bar"],
            vec!["--enable=foo"],
            vec!["--disable", "foo"],
            vec!["--search"],
            vec!["--no-daemon"],
            vec!["--remote=unix://x"],
            vec!["--profile", "x"],
            vec!["--strict-config"],
            vec!["--oss"],
            vec!["--help"],
        ] {
            let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
            assert!(!eligible(&argv), "{argv:?}");
        }
    }

    fn server_fixture(root: &Path) -> PathBuf {
        let fixture = root.join("server.rs");
        fs::write(
            &fixture,
            r#"
use std::{env,fs,io::Read,os::unix::net::UnixListener};
fn main() {
 let a:Vec<String>=env::args().collect();
 assert_eq!(&a[1..3], &["app-server", "--listen"]);
 let path=a[3].strip_prefix("unix://").unwrap();
 let c=fs::read_to_string("/proc/self/fd/34/config.toml").unwrap();
 assert!(c.contains("danger-full-access"));
 assert!(!fs::read("/proc/self/fd/33").unwrap().is_empty());
 assert!(fs::metadata("/proc/self/fd/35").unwrap().is_dir());
 assert!(env::var_os("LD_PRELOAD").is_none());
 if fs::exists(env::var_os("CODEX_HOME").unwrap().into_string().unwrap()+"/exit").unwrap() { std::process::exit(77); }
 let _writer=fs::File::open(std::path::PathBuf::from(env::var_os("CODEX_HOME").unwrap()).join("sessions/held.jsonl")).ok();
 let l=UnixListener::bind(path).unwrap();
 for stream in l.incoming() {
  let mut stream=stream.unwrap();let mut bytes=[0;128];
  while stream.read(&mut bytes).unwrap_or(0)!=0 {}
 }
}
"#,
        )
        .unwrap();
        let program = root.join("runtime");
        assert!(
            Command::new(crate::tests::resolve_test_tool("rustc").unwrap())
                .arg(&fixture)
                .arg("-o")
                .arg(&program)
                .status()
                .unwrap()
                .success()
        );
        program
    }

    #[test]
    fn shared_server_reuse_refreshes_config_without_restarting_active_server() {
        let root = crate::tests::temp_root("shared-server-refresh");
        let program = server_fixture(&root);
        let state = root.join("state");
        let config = root.join("config");
        private_dir(&state).unwrap();
        private_dir(&config).unwrap();
        let resolver = root.join("resolver");
        fs::write(&resolver, b"nameserver 127.0.0.1\n").unwrap();
        fs::write(
            config.join("config.toml"),
            crate::render_core_notification_config(&[]),
        )
        .unwrap();
        let env = crate::TermuxBaseEnvPlan {
            assignments: vec![("TMPDIR".into(), root.as_os_str().into())],
            removals: vec![],
        };
        let profile = root.join("profile");
        let socket = ensure(
            program.as_os_str(),
            &profile,
            &state,
            &resolver,
            &config,
            &env,
        )
        .unwrap();
        let directory = socket.parent().unwrap();
        let pid = read_record(&directory.join("pid")).unwrap();
        let pid_text = String::from_utf8(pid.clone()).unwrap();
        let server_config = PathBuf::from(format!("/proc/{}/fd/34", pid_text.trim()));
        let inode = fs::metadata(&server_config).unwrap().ino();
        for events in [vec!["SessionStart"], vec![]] {
            let bytes = crate::render_core_notification_config(&events);
            fs::write(config.join("config.toml"), &bytes).unwrap();
            fs::write(
                config.join("requirements.toml"),
                b"# refreshed requirements\n",
            )
            .unwrap();
            assert_eq!(
                ensure(
                    program.as_os_str(),
                    &profile,
                    &state,
                    &resolver,
                    &config,
                    &env
                )
                .unwrap(),
                socket
            );
            assert_eq!(read_record(&directory.join("pid")).unwrap(), pid);
            assert_eq!(fs::metadata(&server_config).unwrap().ino(), inode);
            assert_eq!(fs::read(server_config.join("config.toml")).unwrap(), bytes);
            assert_eq!(
                fs::read(server_config.join("requirements.toml")).unwrap(),
                b"# refreshed requirements\n"
            );
            let file_inode = fs::metadata(server_config.join("config.toml"))
                .unwrap()
                .ino();
            ensure(
                program.as_os_str(),
                &profile,
                &state,
                &resolver,
                &config,
                &env,
            )
            .unwrap();
            assert_eq!(
                fs::metadata(server_config.join("config.toml"))
                    .unwrap()
                    .ino(),
                file_inode
            );
        }
        fs::remove_file(config.join("requirements.toml")).unwrap();
        ensure(
            program.as_os_str(),
            &profile,
            &state,
            &resolver,
            &config,
            &env,
        )
        .unwrap();
        assert!(!server_config.join("requirements.toml").exists());
        let foreign = root.join("foreign");
        fs::write(&foreign, b"untouched").unwrap();
        let snapshot_file = directory.join("config/config.toml");
        fs::remove_file(&snapshot_file).unwrap();
        std::os::unix::fs::symlink(&foreign, &snapshot_file).unwrap();
        assert!(ensure(
            program.as_os_str(),
            &profile,
            &state,
            &resolver,
            &config,
            &env
        )
        .is_err());
        assert_eq!(fs::read(&foreign).unwrap(), b"untouched");
        assert!(Path::new(&format!("/proc/{}", pid_text.trim())).exists());
        assert!(
            Command::new(crate::tests::resolve_test_tool("kill").unwrap())
                .args(["-TERM", pid_text.trim()])
                .status()
                .unwrap()
                .success()
        );
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn shared_server_signed_process_fds_reuse_and_namespace_are_bound() {
        let root = crate::tests::temp_root("shared-server");
        let program = server_fixture(&root);
        let state = root.join("state");
        private_dir(&state).unwrap();
        let config = root.join("config");
        private_dir(&config).unwrap();
        fs::write(
            config.join("config.toml"),
            crate::render_core_notification_config(&[]),
        )
        .unwrap();
        let resolver = root.join("resolver");
        fs::write(&resolver, b"nameserver 127.0.0.1\n").unwrap();
        let env = crate::TermuxBaseEnvPlan {
            assignments: vec![("TMPDIR".into(), root.as_os_str().into())],
            removals: vec![],
        };
        let failed = root.join("failed");
        private_dir(&failed).unwrap();
        fs::write(failed.join("exit"), b"test").unwrap();
        assert!(ensure(
            program.as_os_str(),
            &failed,
            &state,
            &resolver,
            &config,
            &env
        )
        .is_err());
        let failed_pid = read_record(
            &state
                .join("servers")
                .join(namespace(&failed, program.as_os_str()))
                .join("pid"),
        )
        .unwrap();
        assert!(!Path::new("/proc")
            .join(String::from_utf8(failed_pid).unwrap().trim())
            .exists());
        assert!(!failed
            .join("app-server-control/app-server-control.sock")
            .exists());
        let one = root.join("one");
        let two = root.join("two");
        let coordinated = state
            .join("servers")
            .join(namespace(&one, program.as_os_str()));
        private_dir(&coordinated).unwrap();
        let held = File::open(&coordinated).unwrap();
        assert_eq!(unsafe { super::super::flock(held.as_raw_fd(), 2 | 4) }, 0);
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            drop(held);
        });
        let s1 = ensure(program.as_os_str(), &one, &state, &resolver, &config, &env).unwrap();
        releaser.join().unwrap();
        let pid1 = read_record(&s1.parent().unwrap().join("pid")).unwrap();
        assert_eq!(
            ensure(program.as_os_str(), &one, &state, &resolver, &config, &env).unwrap(),
            s1
        );
        assert_eq!(
            read_record(&s1.parent().unwrap().join("pid")).unwrap(),
            pid1
        );
        let s2 = ensure(program.as_os_str(), &two, &state, &resolver, &config, &env).unwrap();
        assert_ne!(s1, s2);
        assert_eq!(
            fs::read_link(one.join("app-server-control/app-server-control.sock")).unwrap(),
            s1
        );
        assert!(s1.as_os_str().as_bytes().len() <= 107);
        assert!(!one.join("packages").exists());
        let new = root.join("runtime-new");
        fs::copy(&program, &new).unwrap();
        let s3 = ensure(new.as_os_str(), &one, &state, &resolver, &config, &env).unwrap();
        assert_ne!(s1, s3);
        assert!(
            bound_connection(&s1, &s1.parent().unwrap().join("pid"), program.as_os_str()).unwrap()
        );
        assert_eq!(
            fs::read_link(one.join("app-server-control/app-server-control.sock")).unwrap(),
            s3
        );
        // A substituted owner or PID fails closed without starting another process.
        write_record(&s2.parent().unwrap().join("owner"), b"foreign").unwrap();
        assert!(ensure(program.as_os_str(), &two, &state, &resolver, &config, &env).is_err());
        write_record(&s1.parent().unwrap().join("pid"), b"1\n").unwrap();
        assert!(
            bound_connection(&s1, &s1.parent().unwrap().join("pid"), program.as_os_str()).is_err()
        );
        for (socket, saved) in [(&s1, Some(pid1)), (&s2, None), (&s3, None)] {
            let bytes = saved
                .unwrap_or_else(|| read_record(&socket.parent().unwrap().join("pid")).unwrap());
            let pid = String::from_utf8(bytes).unwrap();
            assert!(
                Command::new(crate::tests::resolve_test_tool("kill").unwrap())
                    .args(["-TERM", pid.trim()])
                    .status()
                    .unwrap()
                    .success()
            );
        }
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn shared_server_retirement_preserves_clients_writers_and_bound_roles() {
        let root = crate::tests::temp_root("server-retire");
        let generations = root.join("generations");
        private_dir(&generations).unwrap();
        let old = generations.join("old");
        private_dir(&old).unwrap();
        let program = server_fixture(&old);
        let state = root.join("state");
        private_dir(&state).unwrap();
        let config = root.join("config");
        private_dir(&config).unwrap();
        fs::write(
            config.join("config.toml"),
            crate::render_core_notification_config(&[]),
        )
        .unwrap();
        let resolver = root.join("resolver");
        fs::write(&resolver, b"nameserver 127.0.0.1\n").unwrap();
        let env = crate::TermuxBaseEnvPlan {
            assignments: vec![("TMPDIR".into(), root.as_os_str().into())],
            removals: vec![],
        };
        let busy = root.join("busy");
        private_dir(&busy).unwrap();
        private_dir(&busy.join("sessions")).unwrap();
        fs::write(busy.join("sessions/held.jsonl"), b"active").unwrap();
        let busy_socket =
            ensure(program.as_os_str(), &busy, &state, &resolver, &config, &env).unwrap();
        let idle = root.join("idle");
        let idle_socket =
            ensure(program.as_os_str(), &idle, &state, &resolver, &config, &env).unwrap();
        let busy_pid =
            String::from_utf8(read_record(&busy_socket.parent().unwrap().join("pid")).unwrap())
                .unwrap();
        let idle_pid =
            String::from_utf8(read_record(&idle_socket.parent().unwrap().join("pid")).unwrap())
                .unwrap();
        retire_unused(
            &state,
            &generations,
            &std::collections::HashSet::from(["old".into()]),
        )
        .unwrap();
        assert!(Path::new(&format!("/proc/{}/exe", idle_pid.trim())).exists());
        let connection = UnixStream::connect(&idle_socket).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let empty = std::collections::HashSet::new();
        retire_unused(&state, &generations, &empty).unwrap();
        assert!(Path::new(&format!("/proc/{}/exe", idle_pid.trim())).exists());
        assert!(Path::new(&format!("/proc/{}/exe", busy_pid.trim())).exists());
        drop(connection);
        std::thread::sleep(Duration::from_millis(30));
        retire_unused(&state, &generations, &empty).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Path::new(&format!("/proc/{}/exe", idle_pid.trim())).exists()
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!Path::new(&format!("/proc/{}/exe", idle_pid.trim())).exists());
        assert!(Path::new(&format!("/proc/{}/exe", busy_pid.trim())).exists());
        assert_eq!(
            fs::read(busy.join("sessions/held.jsonl")).unwrap(),
            b"active"
        );
        retire_unused(&state, &generations, &empty).unwrap();
        assert!(!idle_socket.parent().unwrap().exists());
        assert!(
            Command::new(crate::tests::resolve_test_tool("kill").unwrap())
                .args(["-TERM", busy_pid.trim()])
                .status()
                .unwrap()
                .success()
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        while Path::new(&format!("/proc/{}/exe", busy_pid.trim())).exists()
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        retire_unused(&state, &generations, &empty).unwrap();
        assert!(!busy_socket.parent().unwrap().exists());
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn shared_server_rejects_unsafe_records_directories_and_sockets() {
        let root = crate::tests::temp_root("server-reject");
        let target = root.join("target");
        private_dir(&target).unwrap();
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&target, &alias).unwrap();
        assert!(private_dir(&alias).is_err());
        assert!(private_dir(&alias.join("child")).is_err());
        let record = root.join("record");
        fs::write(&record, vec![0; 16385]).unwrap();
        assert!(read_record(&record).is_err());
        fs::remove_file(&record).unwrap();
        std::os::unix::fs::symlink(&target, &record).unwrap();
        assert!(read_record(&record).is_err());
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(private_dir(&target).is_err());
        assert_ne!(
            namespace(Path::new("/a"), OsStr::new("/runtime")),
            namespace(Path::new("/b"), OsStr::new("/runtime"))
        );
        crate::tests::remove_temp_root(root);
    }
}
