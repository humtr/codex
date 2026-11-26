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

pub(super) fn kill_and_wait(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(super) fn wait_for_exit<E>(
    child: &mut Child,
    timeout: Duration,
    io_error: impl Fn(io::Error) -> E,
    mut check: impl FnMut() -> Result<(), E>,
) -> Result<Option<ExitStatus>, E> {
    let deadline = Instant::now() + timeout;
    loop {
        // Inspect procfs rather than Child::try_wait: try_wait reaps the group
        // leader, allowing its numeric PID/PGID to be reused before descendants
        // are terminated. A zombie leader pins that identity until cleanup.
        let state = std::fs::read_to_string(format!("/proc/{}/stat", child.id()))
            .ok()
            .and_then(|stat| {
                stat.rsplit_once(')')?
                    .1
                    .split_whitespace()
                    .next()
                    .map(str::to_owned)
            });
        check()?;
        if state.as_deref() == Some("Z") {
            reap_process_group(child);
            return child.try_wait().map_err(&io_error);
        }
        if state.is_none() {
            return child.try_wait().map_err(&io_error);
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(super) enum OutputError {
    Io(io::Error),
    TimedOut(Vec<u8>),
    OutputTooLarge(Vec<u8>),
}

pub(super) fn reap_process_group(child: &mut Child) {
    // Do not reap the group leader until the group is gone: its zombie PID
    // pins the numeric PGID and prevents reuse during TERM/KILL escalation.
    let group = -(child.id() as c_int);
    // The leader may already be reaped while descendants still hold the pipe.
    if unsafe { kill(group, SIGTERM_NUM) } == 0 {
        let deadline = Instant::now() + Duration::from_millis(150);
        loop {
            if unsafe { kill(group, 0) } != 0 {
                break;
            }
            if Instant::now() >= deadline {
                unsafe { kill(group, SIGKILL_NUM) };
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    kill_and_wait(child);
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
    child: &mut Child,
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
        let status = child.try_wait().map_err(Io)?;
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
        if let Some(status) = status {
            return Ok((bytes, status));
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
    if result.is_err() || !eof {
        reap_process_group(child);
    }
    result
}
