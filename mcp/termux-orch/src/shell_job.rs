use crate::ledger::{
    ensure_private_dir, now_unix, read_private_file, write_atomic_private, Ledger,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::os::unix::io::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_TAIL: usize = 8192;
const START_GRACE_SECS: u64 = 10;

extern "C" fn keep_receipt_writer_alive(_: libc::c_int) {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    pid: u32,
    start_ticks: u64,
}

#[derive(Serialize, Deserialize)]
struct ExitReceipt {
    pid: u32,
    start_ticks: u64,
    exit_code: i32,
    finished_at_unix: u64,
}

pub struct ShellJob {
    dir: PathBuf,
    id: String,
}

impl ShellJob {
    pub fn new(ledger: &Ledger, work_id: &str, id: &str) -> io::Result<Self> {
        let digits = id.strip_prefix('W').or_else(|| id.strip_prefix('V'));
        if !digits
            .is_some_and(|d| d.parse::<u32>().is_ok() && d.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid shell receipt ID",
            ));
        }
        Ok(Self {
            dir: ledger.work_dir(work_id)?.join("shell"),
            id: id.to_owned(),
        })
    }

    fn path(&self, suffix: &str) -> PathBuf {
        self.dir.join(format!("{}.{}", self.id, suffix))
    }

    fn identity(&self) -> io::Result<Identity> {
        let bytes = read_private_file(&self.path("meta.json"), 4096)?;
        let identity: Identity = serde_json::from_slice(&bytes)?;
        if identity.pid == 0 || identity.start_ticks == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid shell process identity",
            ));
        }
        Ok(identity)
    }

    pub fn has_owner(&self) -> io::Result<bool> {
        match self.identity() {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn confirmed_exit(&self) -> io::Result<bool> {
        match self.identity() {
            Ok(identity) => {
                if self.receipt(identity)?.is_none() {
                    return Ok(false);
                }
                // A recycled group leader cannot represent the old process group.
                if process_stat(identity.pid).is_ok_and(|(start, _)| start != identity.start_ticks)
                {
                    return Ok(true);
                }
                Ok(!live_group_members(identity.pid)?)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) => Err(error),
        }
    }

    pub fn validate_attach(&self) -> io::Result<()> {
        let identity = self.identity()?;
        if self.receipt(identity)?.is_some() || process_alive(identity)? {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "shell identity has no live process or exit receipt",
        ))
    }

    pub fn spawn(
        &self,
        root: &Path,
        work_id: &str,
        command: &str,
        cwd: Option<&str>,
    ) -> io::Result<()> {
        ensure_private_dir(&self.dir)?;
        match self.path("meta.json").symlink_metadata() {
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "shell receipt ID already exists",
                ))
            }
        }
        let mut runner = Command::new(std::env::current_exe()?);
        runner
            .arg("shell-job")
            .arg(root)
            .args([work_id, &self.id, command]);
        if let Some(cwd) = cwd {
            runner.current_dir(cwd);
        }
        runner
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // Calling setsid here detaches the receipt writer before this spawn returns.
        unsafe {
            runner.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = runner.spawn()?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }

    pub fn run(&self, ledger: &Ledger, work_id: &str, command: &str) -> io::Result<()> {
        let pid = std::process::id();
        let (start_ticks, _) = process_stat(pid)?;
        let identity = Identity { pid, start_ticks };
        {
            let _lock = ledger.lock_work(work_id)?;
            let work = ledger.load_work(work_id)?;
            if !work.can_start()
                || !work
                    .participants
                    .iter()
                    .any(|p| p.backend == "shell" && p.ref_id == self.id && !p.is_terminal())
            {
                return Err(io::Error::other("shell intent no longer active"));
            }
            // Multiple relay attempts can launch receipt writers after a crash.
            // Under the Work lock only the first writer may start the command.
            if self.has_owner()? {
                return Ok(());
            }
            write_atomic_private(&self.path("meta.json"), &serde_json::to_vec(&identity)?)?;
        }
        let exit_code = match self.run_command(ledger, work_id, command) {
            Ok(code) => code,
            Err(e) => {
                let _ = write_atomic_private(&self.path("output.log"), e.to_string().as_bytes());
                125
            }
        };
        let receipt = ExitReceipt {
            pid,
            start_ticks,
            exit_code,
            finished_at_unix: now_unix(),
        };
        write_atomic_private(&self.path("exit.json"), &serde_json::to_vec(&receipt)?)
    }

    fn run_command(&self, ledger: &Ledger, work_id: &str, command: &str) -> io::Result<i32> {
        let writer_pid = std::process::id() as libc::pid_t;
        if unsafe { libc::getpgrp() } != writer_pid {
            return Err(io::Error::other(
                "shell receipt writer does not own its process group",
            ));
        }
        // A caught handler is reset on exec, so the shell and its commands
        // receive TERM normally while the writer can still record their exit.
        if unsafe {
            libc::signal(
                libc::SIGTERM,
                keep_receipt_writer_alive as *const () as libc::sighandler_t,
            )
        } == libc::SIG_ERR
        {
            return Err(io::Error::last_os_error());
        }
        let (mut capture, output) = UnixStream::pair()?;
        capture.set_nonblocking(true)?;
        let error_output = output.try_clone()?;
        let shell = std::env::var_os("PREFIX")
            .map(|prefix| PathBuf::from(prefix).join("bin/sh"))
            .unwrap_or_else(|| "/bin/sh".into());
        let mut child = Command::new(shell)
            .args(["-c", command])
            .stdin(Stdio::null())
            .stdout(Stdio::from(OwnedFd::from(output)))
            .stderr(Stdio::from(OwnedFd::from(error_output)))
            .spawn()?;
        let mut buf = [0; 4096];
        let mut tail = Vec::new();
        let mut last_flush = Instant::now();
        loop {
            drain_tail(&mut capture, &mut buf, &mut tail)?;
            let status = child.try_wait()?;
            if status.is_some() {
                drain_tail(&mut capture, &mut buf, &mut tail)?;
            }
            if status.is_some() || last_flush.elapsed() >= Duration::from_secs(1) {
                if status.is_none()
                    && ledger
                        .load_work(work_id)
                        .is_ok_and(|w| w.state == "cancelling")
                {
                    let _ = unsafe { libc::kill(-writer_pid, libc::SIGTERM) };
                }
                // Keep execution and its authoritative exit status independent of tail-log errors.
                let _ = write_atomic_private(&self.path("output.log"), &tail);
                last_flush = Instant::now();
            }
            if let Some(status) = status {
                if ledger
                    .load_work(work_id)
                    .is_ok_and(|w| w.state == "cancelling")
                {
                    // The command can exit before a descendant finishes handling TERM.
                    while live_group_members(writer_pid as u32)? {
                        let _ = unsafe { libc::kill(-writer_pid, libc::SIGTERM) };
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                return Ok(status
                    .code()
                    .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn receipt(&self, identity: Identity) -> io::Result<Option<ExitReceipt>> {
        match read_private_file(&self.path("exit.json"), 4096) {
            Ok(bytes) => {
                let receipt: ExitReceipt = serde_json::from_slice(&bytes)?;
                if receipt.pid != identity.pid || receipt.start_ticks != identity.start_ticks {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "exit receipt identity mismatch",
                    ));
                }
                Ok(Some(receipt))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn observe(&self, registered_at: u64) -> io::Result<Option<(String, String)>> {
        let identity = match self.identity() {
            Ok(identity) => identity,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(
                    (now_unix().saturating_sub(registered_at) >= START_GRACE_SECS)
                        .then(|| ("failed".into(), "shell receipt writer did not start".into())),
                );
            }
            Err(e) => return Err(e),
        };
        let mut receipt = self.receipt(identity)?;
        if receipt.is_none() {
            if process_alive(identity)? {
                return Ok(None);
            }
            // The writer may have published its exit between the first read and the proc check.
            receipt = self.receipt(identity)?;
        }
        let (state, detail) = match receipt {
            Some(exit) if exit.exit_code == 0 => ("completed", "exit_code=0".to_owned()),
            Some(exit) => ("failed", format!("exit_code={}", exit.exit_code)),
            None => (
                "failed",
                "shell process identity disappeared without an exit receipt".to_owned(),
            ),
        };
        let output = read_private_file(&self.path("output.log"), MAX_TAIL as u64)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        let output: String = output
            .chars()
            .rev()
            .take(1100)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Ok(Some((state.to_owned(), format!("{detail}\n{output}"))))
    }
}

fn drain_tail(capture: &mut UnixStream, buf: &mut [u8], tail: &mut Vec<u8>) -> io::Result<()> {
    // Bound each drain so a busy writer cannot starve try_wait.
    for _ in 0..32 {
        match capture.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                tail.extend_from_slice(&buf[..n]);
                if tail.len() > MAX_TAIL {
                    tail.drain(..tail.len() - MAX_TAIL);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn process_stat(pid: u32) -> io::Result<(u64, bool)> {
    let stat = std::fs::read(format!("/proc/{pid}/stat"))?;
    parse_stat(&String::from_utf8_lossy(&stat))
}

fn parse_stat(stat: &str) -> io::Result<(u64, bool)> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid /proc PID stat");
    let (_, fields) = stat.rsplit_once(')').ok_or_else(invalid)?;
    let mut fields = fields.split_whitespace();
    let dead = matches!(fields.next().ok_or_else(invalid)?, "Z" | "X" | "x");
    let start = fields
        .nth(18)
        .and_then(|value| value.parse().ok())
        .ok_or_else(invalid)?;
    Ok((start, dead))
}

fn process_alive(identity: Identity) -> io::Result<bool> {
    match process_stat(identity.pid) {
        Ok((start, dead)) => Ok(!dead && start == identity.start_ticks),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn live_group_members(writer_pid: u32) -> io::Result<bool> {
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == writer_pid {
            continue;
        }
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let Some((_, fields)) = stat.rsplit_once(')') else {
            continue;
        };
        let mut fields = fields.split_whitespace();
        if matches!(fields.next(), Some("Z" | "X" | "x")) {
            continue;
        }
        let group = fields.nth(1).and_then(|value| value.parse::<u32>().ok());
        let session = fields.next().and_then(|value| value.parse::<u32>().ok());
        if group == Some(writer_pid) && session == Some(writer_pid) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_start_ticks_and_zombie_after_last_paren() {
        let stat = "19 (test ) process) Z 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 765 23";
        assert_eq!(parse_stat(stat).unwrap(), (765, true));
        let pid = std::process::id();
        let (start_ticks, dead) = process_stat(pid).unwrap();
        assert!(!dead && start_ticks > 0);
        assert!(!process_alive(Identity {
            pid,
            start_ticks: start_ticks + 1
        })
        .unwrap());
    }
}
