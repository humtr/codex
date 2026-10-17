//! Bounded owned stdio client. Only account/model reads; no thread or inference.
use super::*;
use serde_json::{json, Value};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStdin, ChildStdout};

pub(super) struct Client {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    buffered: Vec<u8>,
    next: u64,
    deadline: Instant,
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Client {
    pub(super) fn new(
        context: &Context,
        home: &Path,
        deadline: Instant,
    ) -> Result<Self, ManagerError> {
        let mut command = Command::new(&context.core_entrypoint);
        let parent = std::process::id() as libc::pid_t;
        // The reader must not survive a cancelled/killed Manager or TUI. It
        // owns only this new stdio process, never an existing account server.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM, 0, 0, 0) != 0
                    || libc::getppid() != parent
                {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .arg("app-server")
            .env_remove(CORE_API_ENV)
            .env_remove(CORE_ENTRYPOINT_ENV)
            .env_remove(CODEX_SQLITE_HOME_ENV)
            .env(CODEX_HOME_ENV, home)
            .current_dir(home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ERR_LAUNCH)?;
        let input = child.stdin.take().ok_or(ERR_LAUNCH)?;
        let output = child.stdout.take().ok_or(ERR_LAUNCH)?;
        let mut client = Self {
            child,
            input,
            output,
            buffered: Vec::new(),
            next: 0,
            deadline,
        };
        client.call("initialize", json!({"clientInfo":{"name":"codex_termux_usage","version":"1"},"capabilities":{"experimentalApi":true}}))?;
        writeln!(client.input, "{}", json!({"method":"initialized"})).map_err(|_| ERR_LAUNCH)?;
        Ok(client)
    }
    pub(super) fn call(&mut self, method: &str, params: Value) -> Result<Value, ManagerError> {
        if !matches!(
            method,
            "initialize" | "account/read" | "account/rateLimits/read" | "model/list"
        ) {
            return Err(ERR_USAGE);
        }
        self.next += 1;
        writeln!(
            self.input,
            "{}",
            json!({"id":self.next,"method":method,"params":params})
        )
        .map_err(|_| ERR_LAUNCH)?;
        let mut total = 0;
        loop {
            if let Some(index) = self.buffered.iter().position(|byte| *byte == b'\n') {
                let line: Vec<_> = self.buffered.drain(..=index).collect();
                let value: Value = serde_json::from_slice(&line).map_err(|_| ERR_LAUNCH)?;
                // Never answer server requests (auth, tools or review) from a read client.
                if value.get("method").is_some() {
                    if value.get("id").is_some() {
                        return Err(ERR_LAUNCH);
                    }
                    continue;
                }
                if value["id"].as_u64() != Some(self.next) || value.get("error").is_some() {
                    return Err(ERR_LAUNCH);
                }
                return value.get("result").cloned().ok_or(ERR_LAUNCH);
            }
            let left = self
                .deadline
                .checked_duration_since(Instant::now())
                .ok_or(ERR_LAUNCH)?;
            let mut descriptor = libc::pollfd {
                fd: self.output.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe {
                libc::poll(
                    &mut descriptor,
                    1,
                    left.as_millis().min(i32::MAX as u128) as i32,
                )
            };
            if ready < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if ready <= 0 {
                return Err(ERR_LAUNCH);
            }
            let mut bytes = [0; 8192];
            let read = self.output.read(&mut bytes).map_err(|_| ERR_LAUNCH)?;
            total += read;
            if read == 0 || total > 1024 * 1024 || self.buffered.len() + read > 256 * 1024 {
                return Err(ERR_LAUNCH);
            }
            self.buffered.extend_from_slice(&bytes[..read]);
        }
    }
}
