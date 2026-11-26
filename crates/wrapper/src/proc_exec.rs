//! Child execution primitives for bounded probes and transfers.
use crate::fs_util::fcntl;
use std::fs::File;
use std::io::{self, Read as _};
use std::os::fd::{AsRawFd as _, FromRawFd as _};
use std::os::raw::{c_int, c_short};
use std::os::unix::process::CommandExt as _;
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

const F_GETFL: c_int = 3;
const F_SETFL: c_int = 4;
pub(super) const O_NONBLOCK_LINUX: c_int = 0o4000;
const O_CLOEXEC_LINUX: c_int = 0o2000000;
const POLLIN_EVENT: c_short = 1;
const SIGTERM_NUM: c_int = 15;
const SIGKILL_NUM: c_int = 9;

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

unsafe extern "C" {
    pub(super) fn kill(pid: c_int, sig: c_int) -> c_int;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: c_int) -> c_int;
    fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
}

pub(super) fn isolate_process_group(command: &mut Command) {
    command.process_group(0);
}

/// Observe an owned child without releasing its PID (and, for a group
/// leader, its PGID). Unlike procfs polling this also detects loss of child
/// ownership (ECHILD), in which case we must not signal the numeric group.
pub(super) fn child_exit_success(child: &Child) -> io::Result<Option<bool>> {
    loop {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        } == 0
        {
            return Ok((unsafe { info.si_pid() } != 0)
                .then(|| info.si_code == libc::CLD_EXITED && unsafe { info.si_status() } == 0));
        }
        let err = io::Error::last_os_error();
        if err.kind() != io::ErrorKind::Interrupted {
            return Err(err);
        }
    }
}

/// The sole group termination/reap path. Never signal a PGID after reaping
/// the leader, including when a caller accidentally supplies a waited child.
pub(super) fn finish_child(mut child: Child, terminate_group: bool) -> io::Result<ExitStatus> {
    if terminate_group {
        // WNOWAIT proves that the leader's PID is still reserved to us.
        child_exit_success(&child)?;
        let group = -(child.id() as c_int);
        if unsafe { kill(group, SIGTERM_NUM) } == 0 {
            // A zombie leader pins the PGID even if every descendant exits.
            // Give live members the existing grace period before escalation.
            std::thread::sleep(Duration::from_millis(150));
            unsafe { kill(group, SIGKILL_NUM) };
        }
        // Also terminate the exact child if it changed its group.
        let _ = child.kill();
    }
    child.wait()
}

pub(super) fn wait_for_exit<E>(
    child: Child,
    timeout: Duration,
    io_error: impl Fn(io::Error) -> E,
    mut check: impl FnMut() -> Result<(), E>,
) -> Result<Option<ExitStatus>, E> {
    let deadline = Instant::now() + timeout;
    let result = (|| loop {
        let exited = child_exit_success(&child).map_err(&io_error)?.is_some();
        check()?;
        if exited {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    })();
    // On every outcome this function owns the only cleanup. Qualification
    // children must not leave descendants alive, even on a successful exit.
    let status = finish_child(child, true).map_err(&io_error);
    result.and_then(|exited| status.map(|status| exited.then_some(status)))
}

pub(super) enum OutputError {
    Io(io::Error),
    TimedOut(Vec<u8>),
    OutputTooLarge(Vec<u8>),
}

pub(super) fn nonblocking_output_pipe() -> io::Result<(File, File)> {
    let mut fds = [0; 2];
    // The read end alone is nonblocking, so child writes still have pipe backpressure.
    if unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC_LINUX) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (reader, writer) = unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) };
    let fd = reader.as_raw_fd();
    let flags = unsafe { fcntl(fd, F_GETFL) };
    if flags < 0 || unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK_LINUX) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((reader, writer))
}

pub(super) fn collect_bounded_output_and_wait(
    child: Child,
    mut reader: File,
    timeout: Duration,
    max_bytes: usize,
) -> Result<(Vec<u8>, ExitStatus), OutputError> {
    use OutputError::{Io, OutputTooLarge, TimedOut};

    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    let mut eof = false;
    let result = (|| loop {
        let exited = child_exit_success(&child).map_err(Io)?;
        while !eof {
            // After observing exit, drain bytes already in the pipe without
            // waiting for EOF from any descendant that inherited the writer.
            match reader.read(&mut buf) {
                Ok(0) => eof = true,
                Ok(n) => {
                    let remaining = max_bytes - bytes.len();
                    if n > remaining {
                        bytes.extend_from_slice(&buf[..remaining]);
                        return Err(OutputTooLarge(bytes));
                    }
                    bytes.extend_from_slice(&buf[..n]);
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                    if Instant::now() >= deadline {
                        return Err(TimedOut(bytes));
                    }
                }
                Err(err) => return Err(Io(err)),
            }
        }
        if let Some(success) = exited {
            return Ok((bytes, success));
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(TimedOut(bytes));
        }
        let wait = remaining.min(Duration::from_millis(25));
        if eof {
            std::thread::sleep(wait);
        } else {
            let mut pfd = PollFd {
                fd: reader.as_raw_fd(),
                events: POLLIN_EVENT,
                revents: 0,
            };
            if unsafe { poll(&mut pfd, 1, wait.as_millis() as c_int) } < 0 {
                let err = io::Error::last_os_error();
                if err.kind() != io::ErrorKind::Interrupted {
                    return Err(Io(err));
                }
            }
        }
    })();
    // A clean EOF may be an intentional daemon handoff. Preserve that
    // behavior; otherwise terminate the group while the leader is reserved.
    let terminate = !eof || !matches!(&result, Ok((_, true)));
    let status = finish_child(child, terminate).map_err(Io);
    result.and_then(|(bytes, _)| status.map(|status| (bytes, status)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn group_cleanup_refuses_already_reaped_leader() {
        let shell = if std::path::Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let mut command = Command::new(shell);
        command
            .args(["-c", "exit 7"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        isolate_process_group(&mut command);
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(success) = child_exit_success(&child).unwrap() {
                assert!(!success);
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        // Observation does not consume the status; the caller still owns it.
        assert_eq!(child_exit_success(&child).unwrap(), Some(false));
        assert_eq!(child.wait().unwrap().code(), Some(7));
        assert_eq!(
            finish_child(child, true).unwrap_err().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
