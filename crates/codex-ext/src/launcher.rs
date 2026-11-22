use crate::paths::{is_live_matching_process, now_millis, ExtPaths};
use crate::profiles::{
    apply_profile_env, effective_current_profile, resolve_profile, ProfileTarget,
};
use crate::session::{
    find_matching_session, register_current_session, remove_session_file, take_handoff_ticket,
    write_handoff_ticket, HandoffTicket, SessionMatch,
};
use codex_daemon_client::{
    daemon_stop_via_cli, wait_writer_lock_released, DaemonClient, DaemonError, ThreadSummary,
};
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static HANDOFF_SIGNALED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigusr1(_sig: libc::c_int) {
    HANDOFF_SIGNALED.store(true, Ordering::SeqCst);
}

struct SignalGuard {
    prev: libc::sigaction,
}

impl SignalGuard {
    fn install_sigusr1() -> Result<Self, String> {
        HANDOFF_SIGNALED.store(false, Ordering::SeqCst);
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = handle_sigusr1 as *const () as usize;
            libc::sigemptyset(&mut action.sa_mask);
            action.sa_flags = 0; // Do not SA_RESTART so wait loops wake promptly.
            let mut prev: libc::sigaction = std::mem::zeroed();
            if libc::sigaction(libc::SIGUSR1, &action, &mut prev) != 0 {
                return Err(format!(
                    "failed to install SIGUSR1 handler: {}",
                    io::Error::last_os_error()
                ));
            }
            Ok(Self { prev })
        }
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::sigaction(libc::SIGUSR1, &self.prev, std::ptr::null_mut());
        }
    }
}

struct TermiosGuard {
    saved: Option<libc::termios>,
}

impl TermiosGuard {
    fn capture() -> Self {
        let saved = unsafe {
            if libc::isatty(0) == 1 {
                let mut t: libc::termios = std::mem::zeroed();
                if libc::tcgetattr(0, &mut t) == 0 {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            }
        };
        Self { saved }
    }

    fn restore_terminal(&self) {
        if let Some(ref t) = self.saved {
            unsafe {
                let _ = libc::tcsetattr(0, libc::TCSAFLUSH, t);
            }
        }
        if unsafe { libc::isatty(1) == 1 } {
            // Leave alt-screen, show cursor, disable bracketed paste & mouse modes cleanly.
            let _ = io::stdout().write_all(
                b"\x1b[?1049l\x1b[?25h\x1b[?2004l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\r\n",
            );
            let _ = io::stdout().flush();
        }
    }
}

impl Drop for TermiosGuard {
    fn drop(&mut self) {
        if let Some(ref t) = self.saved {
            unsafe {
                let _ = libc::tcsetattr(0, libc::TCSAFLUSH, t);
            }
        }
    }
}

struct SessionCleanup<'a> {
    paths: &'a ExtPaths,
    pid: u32,
}

impl Drop for SessionCleanup<'_> {
    fn drop(&mut self) {
        remove_session_file(self.paths, self.pid);
    }
}

struct DaemonProbeResult {
    running: bool,
    busy: Vec<ThreadSummary>,
    auto_thread_id: Option<String>,
}

fn inspect_daemon_for_switch(paths: &ExtPaths, codex_home: &Path, cwd: &Path) -> DaemonProbeResult {
    let mut client = match DaemonClient::connect(
        codex_home,
        &paths.tmpdir,
        Duration::from_secs(2),
        "codex-ext",
        env!("CARGO_PKG_VERSION"),
    ) {
        Ok(c) => c,
        Err(DaemonError::SocketNotFound(_)) => {
            return DaemonProbeResult {
                running: false,
                busy: Vec::new(),
                auto_thread_id: None,
            };
        }
        Err(_) => {
            return DaemonProbeResult {
                running: false,
                busy: Vec::new(),
                auto_thread_id: None,
            };
        }
    };

    let cwd_str = cwd.display().to_string();
    let loaded_ids = client.loaded_threads().unwrap_or_default();
    let mut loaded_summaries = Vec::new();
    let mut busy = Vec::new();
    for id in &loaded_ids {
        if let Ok(summary) = client.read_thread(id, false) {
            if summary.is_active() {
                busy.push(summary.clone());
            }
            loaded_summaries.push(summary);
        }
    }

    // Auto-select most recently updated loaded thread in matching cwd, else single loaded thread.
    let mut auto_thread_id = loaded_summaries
        .iter()
        .filter(|t| t.cwd.as_deref() == Some(cwd_str.as_str()))
        .max_by_key(|t| t.updated_at.unwrap_or(0))
        .map(|t| t.id.clone());
    if auto_thread_id.is_none() && loaded_summaries.len() == 1 {
        auto_thread_id = Some(loaded_summaries[0].id.clone());
    }
    if auto_thread_id.is_none() {
        if let Ok(page) = client.list_threads(10, None) {
            auto_thread_id = page
                .threads
                .iter()
                .filter(|t| t.cwd.as_deref() == Some(cwd_str.as_str()))
                .max_by_key(|t| t.updated_at.unwrap_or(0))
                .map(|t| t.id.clone());
        }
    }

    client.close();
    DaemonProbeResult {
        running: true,
        busy,
        auto_thread_id,
    }
}

fn wait_until_daemon_idle(
    paths: &ExtPaths,
    codex_home: &Path,
    cwd: &Path,
    timeout: Duration,
) -> Result<bool, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let probe = inspect_daemon_for_switch(paths, codex_home, cwd);
        if !probe.running || probe.busy.is_empty() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn terminate_child_gracefully(child: &mut Child, timeout: Duration) {
    let pid = child.id() as libc::pid_t;
    unsafe {
        let _ = libc::kill(pid, libc::SIGTERM);
    }
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return,
        }
    }
}

fn exit_status_to_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        code
    } else if let Some(sig) = status.signal() {
        128 + sig
    } else {
        1
    }
}

pub fn validate_provider_name(paths: &ExtPaths, name: &str) -> Result<(), String> {
    if name == "openai" {
        return Ok(());
    }
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(format!("invalid provider name: '{name}'"));
    }
    let cfg = paths.shared_home.join(format!("{name}.config.toml"));
    if !cfg.is_file() {
        return Err(format!(
            "provider config not found: {} (expected ~/.codex/{name}.config.toml)",
            cfg.display()
        ));
    }
    Ok(())
}

/// Non-PTY parent launcher: inherits Termux TTY fd 0/1/2 directly (zero PTY proxy,
/// lossless OSC 52 `/copy` and `SIGWINCH`), saves/restores `termios`, and handles
/// `SIGUSR1` handoff tickets from in-TUI `!cdx switch` / `!cdx provider`.
pub fn run_launcher(
    paths: &ExtPaths,
    initial_target: ProfileTarget,
    initial_provider: Option<String>,
    initial_args: Vec<OsString>,
) -> Result<i32, String> {
    if !paths.codex_bin.is_file() {
        return Err(format!(
            "codex binary not found: {}",
            paths.codex_bin.display()
        ));
    }
    let cwd = env::current_dir().map_err(|e| format!("cannot determine cwd: {e}"))?;
    let termios = TermiosGuard::capture();
    let _sig = SignalGuard::install_sigusr1()?;
    let _cleanup = SessionCleanup {
        paths,
        pid: std::process::id(),
    };

    let mut current_target = initial_target;
    let mut current_provider = initial_provider;
    let mut next_args = initial_args;

    loop {
        let cur_home = current_target.codex_home(paths);
        let _ = register_current_session(
            paths,
            current_target.name(),
            &cur_home,
            &cwd,
            current_provider.as_deref(),
        )?;

        let mut cmd = Command::new(&paths.codex_bin);
        apply_profile_env(&mut cmd, &current_target, paths);
        cmd.args(&next_args);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", paths.codex_bin.display()))?;

        let mut handoff: Option<HandoffTicket> = None;
        let final_status: Option<ExitStatus> = loop {
            if HANDOFF_SIGNALED.swap(false, Ordering::SeqCst) {
                match take_handoff_ticket(paths, std::process::id()) {
                    Ok(Some(ticket)) => {
                        handoff = Some(ticket);
                        break None;
                    }
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("codex-ext: ignoring invalid handoff ticket: {err}");
                    }
                }
            }
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(err) => return Err(format!("waitpid failed: {err}")),
            }
        };

        let Some(ticket) = handoff else {
            return Ok(exit_status_to_code(final_status.unwrap()));
        };

        // Brief grace so the daemon-spawned `!cdx` child finishes returning exit 0 cleanly.
        std::thread::sleep(Duration::from_millis(30));
        terminate_child_gracefully(&mut child, Duration::from_secs(3));
        termios.restore_terminal();

        match ticket.kind.as_str() {
            "switch" => {
                let new_target = match resolve_profile(paths, &ticket.target) {
                    Ok(t) => t,
                    Err(err) => {
                        eprintln!("codex-ext: switch aborted: {err}");
                        next_args = Vec::new();
                        continue;
                    }
                };
                let old_home = current_target.codex_home(paths);
                let probe = inspect_daemon_for_switch(paths, &old_home, &cwd);
                let tid = ticket.thread_id.or(probe.auto_thread_id);

                if !probe.busy.is_empty() {
                    if ticket.wait {
                        eprintln!(
                            "codex-ext: waiting for {} active turn(s) on '{}' to finish...",
                            probe.busy.len(),
                            current_target.name()
                        );
                        let _ = wait_until_daemon_idle(
                            paths,
                            &old_home,
                            &cwd,
                            Duration::from_secs(600),
                        );
                    } else {
                        eprintln!(
                            "codex-ext: switch aborted — '{}' still has {} active turn(s); resuming original session",
                            current_target.name(),
                            probe.busy.len()
                        );
                        next_args = match tid {
                            Some(ref id) => vec![OsString::from("resume"), OsString::from(id)],
                            None => Vec::new(),
                        };
                        continue;
                    }
                }

                if probe.running && current_target != new_target {
                    let _ = daemon_stop_via_cli(
                        &paths.codex_bin,
                        &old_home,
                        current_target.shared_home_opt(paths),
                    );
                }
                if let Some(ref id) = tid {
                    match wait_writer_lock_released(&paths.shared_home, id, Duration::from_secs(3))
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            eprintln!(
                                "codex-ext: warning: writer lock for {id} is still held after 3s; proceeding to resume"
                            );
                        }
                        Err(err) => {
                            eprintln!("codex-ext: warning: writer-lock check failed: {err}");
                        }
                    }
                }

                current_target = new_target;
                current_provider = None;
                next_args = match tid {
                    Some(id) => vec![OsString::from("resume"), OsString::from(id)],
                    None => Vec::new(),
                };
                eprintln!(
                    "codex-ext: switched account -> '{}'{}.",
                    current_target.name(),
                    if next_args.len() == 2 {
                        format!(" (resuming {})", next_args[1].to_string_lossy())
                    } else {
                        String::new()
                    }
                );
            }
            "provider" => {
                if let Err(err) = validate_provider_name(paths, &ticket.target) {
                    eprintln!("codex-ext: provider switch aborted: {err}");
                    next_args = Vec::new();
                    continue;
                }
                current_target = ProfileTarget::Default;
                if ticket.target == "openai" {
                    current_provider = None;
                    next_args = Vec::new();
                } else {
                    current_provider = Some(ticket.target.clone());
                    next_args = vec![OsString::from("-p"), OsString::from(&ticket.target)];
                }
                eprintln!(
                    "codex-ext: switched provider -> '{}' (new thread on ~/.codex).",
                    ticket.target
                );
            }
            other => {
                eprintln!("codex-ext: unknown handoff ticket kind '{other}'");
                next_args = Vec::new();
            }
        }
    }
}

fn current_codex_home_from_env(paths: &ExtPaths) -> PathBuf {
    env::var_os("CODEX_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.shared_home.clone())
}

fn print_ambiguous_sessions(sessions: &[crate::session::SessionRecord]) {
    eprintln!("codex-ext: multiple live cdx sessions match this CODEX_HOME and directory:");
    for s in sessions {
        eprintln!(
            "  --session {}  (account={}, cwd={})",
            s.pid, s.account, s.cwd
        );
    }
    eprintln!("Re-run with --session <PID> to select the exact terminal window.");
}

pub fn run_switch_command(
    paths: &ExtPaths,
    target_account: &str,
    explicit_tid: Option<String>,
    wait: bool,
    explicit_session: Option<u32>,
) -> Result<i32, String> {
    let target = resolve_profile(paths, target_account)?;
    let cwd = env::current_dir().map_err(|e| format!("cannot determine cwd: {e}"))?;
    let cur_home = current_codex_home_from_env(paths);

    match find_matching_session(paths, explicit_session, &cur_home, &cwd)? {
        SessionMatch::Ambiguous(list) => {
            print_ambiguous_sessions(&list);
            return Ok(2);
        }
        SessionMatch::Exact(parent) => {
            // In-TUI `!cdx switch` path: pre-flight busy check BEFORE disturbing TUI!
            let parent_home = PathBuf::from(&parent.codex_home);
            let probe = inspect_daemon_for_switch(paths, &parent_home, &cwd);
            if !probe.busy.is_empty() && !wait {
                let ids: Vec<&str> = probe.busy.iter().map(|t| t.id.as_str()).collect();
                eprintln!(
                    "codex-ext: cannot switch while '{}' has {} active turn(s): {}",
                    parent.account,
                    probe.busy.len(),
                    ids.join(", ")
                );
                eprintln!("Finish or interrupt active turns, or re-run with --wait.");
                return Ok(1);
            }
            let resolved_tid = explicit_tid.or(probe.auto_thread_id);
            let ticket = HandoffTicket {
                kind: "switch".to_owned(),
                target: target.name().to_owned(),
                thread_id: resolved_tid,
                wait,
                requested_at_ms: now_millis(),
            };
            write_handoff_ticket(paths, parent.pid, &ticket)?;
            if !is_live_matching_process(parent.pid, parent.start_ticks) {
                return Err(format!(
                    "parent codex-ext session {} exited before handoff",
                    parent.pid
                ));
            }
            let rc = unsafe { libc::kill(parent.pid as libc::pid_t, libc::SIGUSR1) };
            if rc != 0 {
                return Err(format!(
                    "failed to signal parent codex-ext ({}): {}",
                    parent.pid,
                    io::Error::last_os_error()
                ));
            }
            println!("switching to '{}'...", target.name());
            return Ok(0);
        }
        SessionMatch::None => {}
    }

    // Standalone shell `cdx switch <acct> [<tid>]`: check old daemon before launching.
    let current_name = effective_current_profile(paths)?;
    let current_target = resolve_profile(paths, &current_name).unwrap_or(ProfileTarget::Default);
    let old_home = current_target.codex_home(paths);
    let probe = inspect_daemon_for_switch(paths, &old_home, &cwd);
    let resolved_tid = explicit_tid.or(probe.auto_thread_id);

    if !probe.busy.is_empty() && current_target != target {
        if wait {
            eprintln!(
                "codex-ext: waiting for {} active turn(s) on '{}'...",
                probe.busy.len(),
                current_target.name()
            );
            let _ = wait_until_daemon_idle(paths, &old_home, &cwd, Duration::from_secs(600));
        } else if io::stdin().is_terminal() {
            eprintln!(
                "codex-ext: '{}' currently has {} active turn(s).",
                current_target.name(),
                probe.busy.len()
            );
            eprint!(
                "Choose action: [w] wait / [r] return to '{}' / [q] cancel: ",
                current_target.name()
            );
            let _ = io::stderr().flush();
            let mut choice = String::new();
            let _ = io::stdin().read_line(&mut choice);
            match choice.trim().to_ascii_lowercase().as_str() {
                "w" | "wait" => {
                    let _ =
                        wait_until_daemon_idle(paths, &old_home, &cwd, Duration::from_secs(600));
                }
                "r" | "return" => {
                    let args = match resolved_tid {
                        Some(id) => vec![OsString::from("resume"), OsString::from(id)],
                        None => Vec::new(),
                    };
                    return run_launcher(paths, current_target, None, args);
                }
                _ => {
                    eprintln!("codex-ext: switch cancelled.");
                    return Ok(1);
                }
            }
        } else {
            return Err(format!(
                "'{}' has {} active turn(s); pass --wait or finish active work first",
                current_target.name(),
                probe.busy.len()
            ));
        }
    }

    if probe.running && current_target != target {
        let _ = daemon_stop_via_cli(
            &paths.codex_bin,
            &old_home,
            current_target.shared_home_opt(paths),
        );
    }
    if let Some(ref id) = resolved_tid {
        let _ = wait_writer_lock_released(&paths.shared_home, id, Duration::from_secs(3));
    }

    let args = match resolved_tid {
        Some(id) => vec![OsString::from("resume"), OsString::from(id)],
        None => Vec::new(),
    };
    run_launcher(paths, target, None, args)
}

pub fn run_provider_command(
    paths: &ExtPaths,
    provider_name: &str,
    explicit_session: Option<u32>,
) -> Result<i32, String> {
    validate_provider_name(paths, provider_name)?;
    let cwd = env::current_dir().map_err(|e| format!("cannot determine cwd: {e}"))?;
    let cur_home = current_codex_home_from_env(paths);

    match find_matching_session(paths, explicit_session, &cur_home, &cwd)? {
        SessionMatch::Ambiguous(list) => {
            print_ambiguous_sessions(&list);
            return Ok(2);
        }
        SessionMatch::Exact(parent) => {
            let parent_home = PathBuf::from(&parent.codex_home);
            let probe = inspect_daemon_for_switch(paths, &parent_home, &cwd);
            if !probe.busy.is_empty() {
                let ids: Vec<&str> = probe.busy.iter().map(|t| t.id.as_str()).collect();
                eprintln!(
                    "codex-ext: cannot switch provider while '{}' has {} active turn(s): {}",
                    parent.account,
                    probe.busy.len(),
                    ids.join(", ")
                );
                return Ok(1);
            }
            let ticket = HandoffTicket {
                kind: "provider".to_owned(),
                target: provider_name.to_owned(),
                thread_id: None,
                wait: false,
                requested_at_ms: now_millis(),
            };
            write_handoff_ticket(paths, parent.pid, &ticket)?;
            if !is_live_matching_process(parent.pid, parent.start_ticks) {
                return Err(format!("parent codex-ext session {} exited", parent.pid));
            }
            let rc = unsafe { libc::kill(parent.pid as libc::pid_t, libc::SIGUSR1) };
            if rc != 0 {
                return Err(format!(
                    "failed to signal parent codex-ext ({}): {}",
                    parent.pid,
                    io::Error::last_os_error()
                ));
            }
            println!("switching provider to '{provider_name}'...");
            return Ok(0);
        }
        SessionMatch::None => {}
    }

    let (provider_opt, args) = if provider_name == "openai" {
        (None, Vec::new())
    } else {
        (
            Some(provider_name.to_owned()),
            vec![OsString::from("-p"), OsString::from(provider_name)],
        )
    };
    run_launcher(paths, ProfileTarget::Default, provider_opt, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::create_profile;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn launcher_handles_in_tui_sigusr1_account_and_provider_switch() {
        let root = std::env::temp_dir().join(format!(
            "cdx-launch-{}-{}",
            std::process::id(),
            now_millis()
        ));
        let usr_bin = root.join("usr/bin");
        let work_cwd = root.join("work");
        fs::create_dir_all(&usr_bin).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::create_dir_all(&work_cwd).unwrap();
        fs::write(
            root.join(".codex/chatgpt.config.toml"),
            "model = \"gpt-5.4\"\n",
        )
        .unwrap();

        let trace_file = root.join("trace.log");
        let fake_codex = usr_bin.join("codex");
        fs::write(
            &fake_codex,
            format!(
                "#!/bin/sh\n\
                 printf 'HOME=%s SHARED=%s EXTRA=%s ARGS=%s\\n' \"${{CODEX_HOME:-DEFAULT}}\" \"${{CODEX_SHARED_HOME:-NONE}}\" \"${{CODEX_SHARED_EXTRA_FILES:-NONE}}\" \"$*\" >> '{}'\n\
                 if [ \"$1\" = 'app-server' ]; then exit 0; fi\n\
                 if [ \"$1\" = 'resume' ]; then exit 0; fi\n\
                 trap 'exit 0' TERM\n\
                 sleep 5\n",
                trace_file.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();

        let paths = ExtPaths {
            prefix: root.join("usr"),
            tmpdir: root.join("tmp"),
            codex_bin: fake_codex,
            shared_home: root.join(".codex"),
            manager_dir: root.join(".local/share/codex/manager"),
            profiles_dir: root.join(".local/share/codex/manager/profiles"),
            default_profile_file: root.join(".local/share/codex/manager/default-profile"),
            sessions_dir: root.join(".local/state/codex-ext/sessions"),
            handoff_dir: root.join(".local/state/codex-ext/handoff"),
            notify_config_file: root.join(".config/codex-ext/notify.json"),
        };
        create_profile(&paths, "szob").unwrap();

        // Spawn a background thread simulating `!cdx switch szob tid-99` after launcher registers.
        let paths_clone = paths.clone();
        let parent_pid = std::process::id();
        let switcher = std::thread::spawn(move || {
            for _ in 0..100 {
                if paths_clone
                    .sessions_dir
                    .join(format!("{parent_pid}.json"))
                    .is_file()
                {
                    std::thread::sleep(Duration::from_millis(60));
                    let ticket = HandoffTicket {
                        kind: "switch".to_owned(),
                        target: "szob".to_owned(),
                        thread_id: Some("tid-99".to_owned()),
                        wait: false,
                        requested_at_ms: now_millis(),
                    };
                    write_handoff_ticket(&paths_clone, parent_pid, &ticket).unwrap();
                    unsafe {
                        libc::kill(parent_pid as libc::pid_t, libc::SIGUSR1);
                    }
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("launcher never registered session file");
        });

        let rc = run_launcher(&paths, ProfileTarget::Default, None, Vec::new()).unwrap();
        switcher.join().unwrap();
        assert_eq!(rc, 0);

        let trace = fs::read_to_string(&trace_file).unwrap();
        let lines: Vec<&str> = trace.lines().collect();
        assert_eq!(lines.len(), 2, "trace was:\n{trace}");
        assert!(lines[0].contains("HOME=DEFAULT SHARED=NONE"));
        assert!(lines[1].contains("profiles/szob/home"));
        assert!(lines[1].contains("EXTRA=hooks.json"));
        assert!(lines[1].contains("ARGS=resume tid-99"));

        let _ = fs::remove_dir_all(&root);
    }
}
