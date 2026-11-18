use crate::{
    apply_child_env_plan_and_fence, plan_termux_env, termux_browser_helper_paths, GenerationLayout,
    LocalPublicDispatchContext, QualifiedRuntimeAssets, RuntimeFdSources, TermuxProcessEnvError,
    TermuxProcessEnvSnapshot, DOCTOR_OUTPUT_MAX_BYTES,
};
use std::ffi::{OsStr, OsString};
use std::io::Read as _;
use std::os::fd::AsRawFd as _;
use std::os::raw::{c_int, c_short};
use std::os::unix::process::CommandExt as _;
use std::time::{Duration, Instant};

// Upstream `doctor` walks `$CODEX_HOME` (including `packages/standalone` and large session histories); allow >40s flash scans.
const DOCTOR_PROBE_TIMEOUT: Duration = Duration::from_secs(90);
const F_GETFL: c_int = 3;
const F_SETFL: c_int = 4;
const O_NONBLOCK_LINUX: c_int = 0o4000;
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
    fn setpgid(pid: c_int, pgid: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: c_int) -> c_int;
}

#[derive(Debug)]
pub(super) enum QualifiedUpstreamDoctorProbeError {
    WrapperPreparation(String),
    Io(std::io::Error),
    TimedOut,
    OutputTooLarge,
}

impl std::fmt::Display for QualifiedUpstreamDoctorProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedUpstreamDoctorProbeError::WrapperPreparation(msg) => f.write_str(msg),
            QualifiedUpstreamDoctorProbeError::Io(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::TimedOut => f.write_str("upstream doctor timed out"),
            QualifiedUpstreamDoctorProbeError::OutputTooLarge => {
                f.write_str("upstream doctor output exceeded size limit")
            }
        }
    }
}

impl std::error::Error for QualifiedUpstreamDoctorProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QualifiedUpstreamDoctorProbeError::Io(err) => Some(err),
            QualifiedUpstreamDoctorProbeError::WrapperPreparation(_)
            | QualifiedUpstreamDoctorProbeError::TimedOut
            | QualifiedUpstreamDoctorProbeError::OutputTooLarge => None,
        }
    }
}

impl From<TermuxProcessEnvError> for QualifiedUpstreamDoctorProbeError {
    fn from(value: TermuxProcessEnvError) -> Self {
        Self::WrapperPreparation(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QualifiedUpstreamDoctorResult {
    pub(super) status: UpstreamDoctorStatus,
    pub(super) output: String,
    pub(super) reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoctorCaptureOptions {
    json: bool,
    use_color: bool,
    force_color: bool,
}

fn sanitize_doctor_terminal_controls(value: &str, preserve_sgr: bool) -> String {
    fn erase_current_line(output: &mut String) {
        if let Some(newline) = output.rfind('\n') {
            output.truncate(newline + 1);
        } else {
            output.clear();
        }
    }

    let mut result = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                if bytes.get(index + 1) != Some(&b'\n') {
                    erase_current_line(&mut result);
                }
                index += 1;
            }
            b'\n' | b'\t' => {
                result.push(bytes[index] as char);
                index += 1;
            }
            0x1b => {
                if bytes.get(index + 1) == Some(&b'[') {
                    let parameter_start = index + 2;
                    let Some(final_offset) = bytes[parameter_start..]
                        .iter()
                        .position(|byte| (b'@'..=b'~').contains(byte))
                    else {
                        break;
                    };
                    let final_index = parameter_start + final_offset;
                    let parameters = &bytes[parameter_start..final_index];
                    if bytes[final_index] == b'K' {
                        erase_current_line(&mut result);
                    } else if preserve_sgr
                        && bytes[final_index] == b'm'
                        && parameters
                            .iter()
                            .all(|byte| byte.is_ascii_digit() || *byte == b';')
                    {
                        result.push_str(&value[index..=final_index]);
                    }
                    index = final_index + 1;
                } else if bytes.get(index + 1) == Some(&b']') {
                    index += 2;
                    while index < bytes.len() {
                        if bytes[index] == 0x07 {
                            index += 1;
                            break;
                        }
                        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
                            index += 2;
                            break;
                        }
                        index += 1;
                    }
                } else {
                    index = (index + 2).min(bytes.len());
                }
            }
            byte if byte.is_ascii_control() => {
                index += 1;
            }
            _ => {
                let character = value[index..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 input has a character at every byte boundary");
                result.push(character);
                index += character.len_utf8();
            }
        }
    }
    result
}

fn replace_doctor_redaction(
    line: &mut String,
    boundaries: &mut Vec<usize>,
    start: usize,
    end: usize,
) -> (usize, usize) {
    const REDACTION: &str = "[redacted]";

    let original_start = boundaries[start];
    let original_end = boundaries[end];
    line.replace_range(start..end, REDACTION);

    let mut updated = Vec::with_capacity(boundaries.len() - (end - start) + REDACTION.len());
    updated.extend_from_slice(&boundaries[..=start]);
    for _ in 1..REDACTION.len() {
        updated.push(original_start);
    }
    updated.push(original_end);
    updated.extend_from_slice(&boundaries[end + 1..]);
    *boundaries = updated;
    (original_start, original_end)
}

fn redact_doctor_line_with_ranges(line: &mut String) -> (String, Vec<(usize, usize)>) {
    const SENSITIVE_KEYS: [&str; 11] = [
        "access_token",
        "refresh_token",
        "oauth_token",
        "token",
        "api_key",
        "authorization",
        "cookie",
        "client_secret",
        "secret",
        "private_key",
        "password",
    ];
    let mut boundaries: Vec<usize> = (0..=line.len()).collect();
    let mut ranges = Vec::new();

    for key in SENSITIVE_KEYS {
        let mut search_from = 0;
        loop {
            let lowered = line.to_ascii_lowercase();
            let Some(key_offset) = lowered[search_from..].find(key) else {
                break;
            };
            let key_start = search_from + key_offset;
            let after_key = key_start + key.len();
            let Some(delimiter_offset) = line[after_key..].find([':', '=']) else {
                search_from = after_key;
                continue;
            };
            let delimiter = after_key + delimiter_offset;
            let mut value_start = delimiter + 1;
            while let Some(character) = line[value_start..].chars().next() {
                if character == ' ' || character == '\t' {
                    value_start += character.len_utf8();
                } else {
                    break;
                }
            }
            let quoted = line[value_start..].starts_with('"');
            if quoted {
                value_start += 1;
            }
            let mut value_end = value_start;
            for (offset, character) in line[value_start..].char_indices() {
                let is_end = if quoted {
                    character == '"'
                } else if matches!(key, "authorization" | "cookie") {
                    character == ',' || character == '}' || character == ']'
                } else {
                    character == '"'
                        || character == ','
                        || character == '}'
                        || character.is_ascii_whitespace()
                };
                if is_end {
                    break;
                }
                value_end = value_start + offset + character.len_utf8();
            }
            if value_end == value_start {
                search_from = after_key;
                continue;
            }
            if line[value_start..].starts_with("[redacted]") {
                search_from = value_start + "[redacted]".len();
                continue;
            }
            ranges.push(replace_doctor_redaction(
                line,
                &mut boundaries,
                value_start,
                value_end,
            ));
            search_from = value_start + "[redacted]".len();
            if search_from >= line.len() {
                break;
            }
        }
    }

    for marker in ["bearer ", "sk-", "sess-", "ghp_"] {
        let mut search_from = 0;
        loop {
            let lowered = line.to_ascii_lowercase();
            let Some(marker_offset) = lowered[search_from..].find(marker) else {
                break;
            };
            let start = search_from + marker_offset;
            let value_start = start + marker.len();
            let mut value_end = value_start;
            for (offset, character) in line[value_start..].char_indices() {
                if character == '"'
                    || character == '\''
                    || character == ','
                    || character.is_ascii_whitespace()
                {
                    break;
                }
                value_end = value_start + offset + character.len_utf8();
            }
            if value_end == value_start {
                search_from = value_start;
                continue;
            }
            ranges.push(replace_doctor_redaction(
                line,
                &mut boundaries,
                value_start,
                value_end,
            ));
            search_from = value_start + "[redacted]".len();
            if search_from >= line.len() {
                break;
            }
        }
    }
    (line.to_owned(), ranges)
}

fn doctor_sgr_end(value: &str, start: usize) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.get(start) != Some(&0x1b) || bytes.get(start + 1) != Some(&b'[') {
        return None;
    }
    let mut index = start + 2;
    while index < bytes.len() {
        if bytes[index] == b'm' {
            return Some(index + 1);
        }
        index += 1;
    }
    None
}

fn doctor_styled_boundaries(styled: &str) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut plain_length = 0;
    let mut index = 0;
    while index < styled.len() {
        if let Some(end) = doctor_sgr_end(styled, index) {
            index = end;
            boundaries[plain_length] = index;
            continue;
        }
        let character = styled[index..]
            .chars()
            .next()
            .expect("valid UTF-8 styled doctor output has a character at every byte boundary");
        let width = character.len_utf8();
        for offset in 1..=width {
            boundaries.push(index + offset);
        }
        plain_length += width;
        index += width;
    }
    boundaries
}

fn doctor_styled_redaction_segment(segment: &str) -> String {
    let mut replacement = String::new();
    let mut inserted = false;
    let mut index = 0;
    while index < segment.len() {
        if let Some(end) = doctor_sgr_end(segment, index) {
            replacement.push_str(&segment[index..end]);
            index = end;
            continue;
        }
        let character = segment[index..]
            .chars()
            .next()
            .expect("valid UTF-8 styled doctor segment has a character at every byte boundary");
        if !inserted {
            replacement.push_str("[redacted]");
            inserted = true;
        }
        index += character.len_utf8();
    }
    if !inserted {
        replacement.push_str("[redacted]");
    }
    replacement
}

fn doctor_styled_redactions(styled: &str, ranges: &[(usize, usize)]) -> Option<String> {
    if ranges.is_empty() {
        return Some(styled.to_owned());
    }
    let boundaries = doctor_styled_boundaries(styled);
    let mut output = styled.to_owned();
    let mut ranges = ranges.to_vec();
    ranges.sort_by_key(|range| std::cmp::Reverse(range.0));
    for (start, end) in ranges {
        let styled_start = *boundaries.get(start)?;
        let styled_end = *boundaries.get(end)?;
        if styled_start > styled_end || styled_end > output.len() {
            return None;
        }
        let replacement = doctor_styled_redaction_segment(&output[styled_start..styled_end]);
        output.replace_range(styled_start..styled_end, &replacement);
    }
    Some(output)
}

pub(super) fn redact_doctor_output(bytes: &[u8], preserve_sgr: bool) -> String {
    let text = String::from_utf8_lossy(bytes);
    let styled = sanitize_doctor_terminal_controls(&text, preserve_sgr);
    let plain = sanitize_doctor_terminal_controls(&text, false);
    let mut redacted = String::with_capacity(plain.len());
    let mut styled_chunks = styled.split_inclusive('\n');
    for chunk in plain.split_inclusive('\n') {
        let (line, newline) = chunk
            .strip_suffix('\n')
            .map_or((chunk, ""), |line| (line, "\n"));
        let styled_chunk = styled_chunks.next().unwrap_or_default();
        let (styled_line, _) = styled_chunk
            .strip_suffix('\n')
            .map_or((styled_chunk, ""), |line| (line, "\n"));
        let mut plain_line = line.to_owned();
        let (redacted_line, ranges) = redact_doctor_line_with_ranges(&mut plain_line);
        if preserve_sgr {
            if let Some(styled_line) = doctor_styled_redactions(styled_line, &ranges) {
                redacted.push_str(&styled_line);
            } else {
                redacted.push_str(&redacted_line);
            }
        } else {
            redacted.push_str(&redacted_line);
        }
        redacted.push_str(newline);
    }
    redacted
}

fn doctor_color_enabled(force_color: bool) -> bool {
    use std::io::IsTerminal as _;

    doctor_color_policy(
        std::io::stdout().is_terminal(),
        std::env::var_os("NO_COLOR").is_some(),
        force_color,
    )
}

fn doctor_color_policy(
    stdout_is_terminal: bool,
    no_color_present: bool,
    force_color: bool,
) -> bool {
    stdout_is_terminal && (force_color || !no_color_present)
}

fn apply_doctor_color_override(
    cmd: &mut std::process::Command,
    use_color: bool,
    force_color: bool,
) {
    if use_color && force_color {
        cmd.env_remove("NO_COLOR");
    }
}

fn doctor_shell_quote(value: &OsStr) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let mut quoted = Vec::with_capacity(value.as_bytes().len() + 2);
    quoted.push(b'\'');
    for byte in value.as_bytes() {
        if *byte == b'\'' {
            quoted.extend_from_slice(b"'\"'\"'");
        } else {
            quoted.push(*byte);
        }
    }
    quoted.push(b'\'');
    OsString::from_vec(quoted)
}

fn doctor_script_command(runtime: &OsStr, json: bool) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let quoted_runtime = doctor_shell_quote(runtime);
    let mut command = quoted_runtime.as_bytes().to_vec();
    command.extend_from_slice(b" doctor");
    if json {
        command.extend_from_slice(b" --json");
    }
    OsString::from_vec(command)
}

fn doctor_pty_path(process_env: &TermuxProcessEnvSnapshot) -> Option<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let prefix = process_env.prefix.as_deref()?;
    let path = std::path::Path::new(prefix).join("bin/script");
    let metadata = std::fs::symlink_metadata(&path).ok()?;
    (metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0).then_some(path)
}

fn reap_doctor_child(child: &mut std::process::Child) {
    let pid = child.id() as c_int;
    if pid > 0 {
        // Put in its own process group at spawn; signal the group first so `script`
        // grandchildren exit too.
        unsafe {
            kill(-pid, SIGTERM_NUM);
        }
        let grace_deadline = Instant::now() + Duration::from_millis(150);
        while Instant::now() < grace_deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        unsafe {
            kill(-pid, SIGKILL_NUM);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn collect_bounded_output_and_wait(
    child: &mut std::process::Child,
    mut stdout: std::process::ChildStdout,
    timeout: Duration,
) -> Result<(Vec<u8>, std::process::ExitStatus), QualifiedUpstreamDoctorProbeError> {
    let fd = stdout.as_raw_fd();
    let flags = unsafe { crate::fcntl(fd, F_GETFL) };
    if flags < 0 {
        let err = std::io::Error::last_os_error();
        reap_doctor_child(child);
        return Err(QualifiedUpstreamDoctorProbeError::Io(err));
    }
    if unsafe { crate::fcntl(fd, F_SETFL, flags | O_NONBLOCK_LINUX) } < 0 {
        let err = std::io::Error::last_os_error();
        reap_doctor_child(child);
        return Err(QualifiedUpstreamDoctorProbeError::Io(err));
    }

    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    let mut eof = false;

    loop {
        if !eof {
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(n) => {
                        bytes.extend_from_slice(&buf[..n]);
                        if bytes.len() > DOCTOR_OUTPUT_MAX_BYTES {
                            reap_doctor_child(child);
                            return Err(QualifiedUpstreamDoctorProbeError::OutputTooLarge);
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(err) => {
                        reap_doctor_child(child);
                        return Err(QualifiedUpstreamDoctorProbeError::Io(err));
                    }
                }
            }
        }

        if eof {
            match child.try_wait() {
                Ok(Some(status)) => return Ok((bytes, status)),
                Ok(None) => {}
                Err(err) => {
                    reap_doctor_child(child);
                    return Err(QualifiedUpstreamDoctorProbeError::Io(err));
                }
            }
        }

        let now = Instant::now();
        if now >= deadline {
            reap_doctor_child(child);
            return Err(QualifiedUpstreamDoctorProbeError::TimedOut);
        }
        let rem = deadline.saturating_duration_since(now);
        if eof {
            std::thread::sleep(rem.min(Duration::from_millis(15)));
        } else {
            let wait_ms = rem.as_millis().min(200) as c_int;
            let mut pfd = PollFd {
                fd,
                events: POLLIN_EVENT,
                revents: 0,
            };
            let poll_rc = unsafe { poll(&mut pfd, 1, wait_ms) };
            if poll_rc < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() != std::io::ErrorKind::Interrupted {
                    reap_doctor_child(child);
                    return Err(QualifiedUpstreamDoctorProbeError::Io(err));
                }
            }
        }
    }
}

fn capture_qualified_upstream_doctor<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    options: DoctorCaptureOptions,
) -> Result<QualifiedUpstreamDoctorResult, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    capture_qualified_upstream_doctor_with_timeout(
        assets,
        process_env,
        cert_file,
        cert_dir,
        resolver_path,
        config_dir,
        options,
        DOCTOR_PROBE_TIMEOUT,
    )
}

#[allow(clippy::too_many_arguments)]
fn capture_qualified_upstream_doctor_with_timeout<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    options: DoctorCaptureOptions,
    timeout: Duration,
) -> Result<QualifiedUpstreamDoctorResult, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let json = options.json;
    let force_color = options.force_color;
    let mut use_color = options.use_color;
    let selection = assets.selection();
    let browser_helpers = termux_browser_helper_paths(selection)?;
    let env_plan = plan_termux_env(
        process_env,
        selection.generation_dir,
        browser_helpers,
        cert_file,
        cert_dir,
    )?;
    let sqlite_home_override = crate::prepare_shared_home_from_env().map_err(|err| {
        QualifiedUpstreamDoctorProbeError::WrapperPreparation(format!(
            "wrapper shared-home preparation failed: {err}"
        ))
    })?;
    if let Err(err) = crate::prepare_wrapper_config(config_dir.as_ref()) {
        return Err(QualifiedUpstreamDoctorProbeError::WrapperPreparation(
            format!("wrapper config preparation failed: {err}"),
        ));
    }
    let runtime_fds = RuntimeFdSources::open(resolver_path, config_dir).map_err(|err| {
        QualifiedUpstreamDoctorProbeError::WrapperPreparation(format!(
            "wrapper runtime FD preparation failed: {err}"
        ))
    })?;
    let pty_path = (!json && use_color)
        .then(|| doctor_pty_path(process_env))
        .flatten();
    let mut cmd = if let Some(script) = pty_path.as_ref() {
        let mut command = std::process::Command::new(script);
        command
            .args(["-qefc"])
            .arg(doctor_script_command(selection.runtime.program_path, json))
            .arg("/dev/null");
        command
    } else {
        use_color = false;
        std::process::Command::new(selection.runtime.program_path)
    };
    if pty_path.is_none() {
        cmd.args(["doctor"]);
        if json {
            cmd.arg("--json");
        }
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());
    apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
    if let Some(sqlite_home) = sqlite_home_override {
        cmd.env("CODEX_SQLITE_HOME", sqlite_home);
    }
    apply_doctor_color_override(&mut cmd, use_color, force_color);
    runtime_fds.configure(&mut cmd);
    // SAFETY: setpgid(0, 0) is async-signal-safe and isolates the doctor child group for reaping.
    unsafe {
        cmd.pre_exec(|| {
            if setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            reap_doctor_child(&mut child);
            return Err(QualifiedUpstreamDoctorProbeError::Io(
                std::io::Error::other("upstream doctor stdout is unavailable"),
            ));
        }
    };
    let (bytes, status) = collect_bounded_output_and_wait(&mut child, stdout, timeout)?;
    Ok(QualifiedUpstreamDoctorResult {
        status: if status.success() {
            UpstreamDoctorStatus::Healthy
        } else {
            UpstreamDoctorStatus::Unhealthy
        },
        output: redact_doctor_output(&bytes, use_color && !json),
        reason: (!status.success()).then(|| format!("upstream doctor exited with {status}")),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpstreamDoctorStatus {
    Healthy,
    Unhealthy,
    NotRun,
    TimedOut,
    OutputExceeded,
}

impl UpstreamDoctorStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            UpstreamDoctorStatus::Healthy => "healthy",
            UpstreamDoctorStatus::Unhealthy => "unhealthy",
            UpstreamDoctorStatus::NotRun => "not_run",
            UpstreamDoctorStatus::TimedOut => "timed_out",
            UpstreamDoctorStatus::OutputExceeded => "output_exceeded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WrapperDoctorStatus {
    Healthy,
    Unhealthy,
}

impl WrapperDoctorStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            WrapperDoctorStatus::Healthy => "healthy",
            WrapperDoctorStatus::Unhealthy => "unhealthy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DoctorSummaryStatus {
    Healthy,
    Unhealthy,
}

impl DoctorSummaryStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            DoctorSummaryStatus::Healthy => "healthy",
            DoctorSummaryStatus::Unhealthy => "unhealthy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DoctorExitClass {
    Success,
    HealthFailure,
}

pub(super) fn doctor_exit_code(class: DoctorExitClass) -> i32 {
    match class {
        DoctorExitClass::Success => 0,
        DoctorExitClass::HealthFailure => 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxWrapperDoctorReport {
    status: WrapperDoctorStatus,
    generation_id: String,
    generation_layout: GenerationLayout,
    has_code_mode_host: bool,
    reason: Option<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorReport {
    upstream: QualifiedUpstreamDoctorResult,
    termux_wrapper: TermuxWrapperDoctorReport,
    summary: DoctorSummaryStatus,
}

fn compose_doctor_report(
    upstream: QualifiedUpstreamDoctorResult,
    termux_wrapper: TermuxWrapperDoctorReport,
) -> DoctorReport {
    let summary = if upstream.status != UpstreamDoctorStatus::Healthy
        || termux_wrapper.status == WrapperDoctorStatus::Unhealthy
    {
        DoctorSummaryStatus::Unhealthy
    } else {
        DoctorSummaryStatus::Healthy
    };
    DoctorReport {
        upstream,
        termux_wrapper,
        summary,
    }
}

fn doctor_exit_class(report: &DoctorReport) -> DoctorExitClass {
    match report.summary {
        DoctorSummaryStatus::Healthy => DoctorExitClass::Success,
        DoctorSummaryStatus::Unhealthy => DoctorExitClass::HealthFailure,
    }
}

fn doctor_color(text: &str, code: u16, enabled: bool) -> String {
    if enabled {
        format!("\x1b[38;5;{code}m{text}\x1b[39m")
    } else {
        text.to_owned()
    }
}

fn doctor_bold(text: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[1m{text}\x1b[22m")
    } else {
        text.to_owned()
    }
}

fn doctor_dim(text: &str, enabled: bool) -> String {
    doctor_color(text, 240, enabled)
}

fn doctor_row(output: &mut String, name: &str, description: &str, healthy: bool, color: bool) {
    let marker = if healthy {
        doctor_color("✓", 10, color)
    } else {
        doctor_color("✗", 196, color)
    };
    let description = if healthy {
        doctor_dim(description, color)
    } else {
        description.to_owned()
    };
    output.push_str(&format!("  {marker} {name:<14} {description}\n"));
}

fn doctor_warning(output: &mut String, name: &str, description: &str, color: bool) {
    let marker = doctor_color("⚠", 214, color);
    output.push_str(&format!("  {marker} {name:<14} {description}\n"));
}

fn doctor_detail(output: &mut String, name: &str, value: &str, color: bool) {
    output.push_str("    ");
    output.push_str(&doctor_dim(name, color));
    output.push_str(": ");
    output.push_str(value);
    output.push('\n');
}

fn code_mode_status(termux_wrapper: &TermuxWrapperDoctorReport) -> (&'static str, bool) {
    let _ = termux_wrapper.generation_layout;
    if termux_wrapper.has_code_mode_host {
        ("healthy", true)
    } else {
        ("not_packaged", true)
    }
}

fn render_doctor_human(report: &DoctorReport, color: bool) -> String {
    let mut output = String::new();
    if report.upstream.output.is_empty() {
        output.push_str("Upstream doctor output unavailable: ");
        output.push_str(report.upstream.status.as_str());
        if let Some(reason) = report.upstream.reason.as_deref() {
            output.push_str(" (");
            output.push_str(reason);
            output.push(')');
        }
        output.push('\n');
    } else {
        output.push_str(&report.upstream.output);
        if !report.upstream.output.ends_with('\n') {
            output.push('\n');
        }
        if color {
            output.push_str("\x1b[0m");
        }
    }

    let (code_mode_status, code_mode_healthy) = code_mode_status(&report.termux_wrapper);
    let wrapper_healthy = report.termux_wrapper.status == WrapperDoctorStatus::Healthy;
    let summary_healthy = report.summary == DoctorSummaryStatus::Healthy;
    let mut ok_count = 0;
    let mut warning_count = 1usize;
    let mut failure_count = 0;

    output.push_str("\n[Termux doctor]\n");
    let header = format!(
        "Codex Termux Wrapper Doctor · generation {} · status {}",
        report.termux_wrapper.generation_id,
        report.termux_wrapper.status.as_str()
    );
    output.push_str(&doctor_bold(&header, color));
    output.push('\n');

    output.push('\n');
    output.push_str(&doctor_bold("Runtime", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "runtime",
        if wrapper_healthy {
            "selected runtime is qualified"
        } else {
            "wrapper runtime preparation failed"
        },
        wrapper_healthy,
        color,
    );
    if wrapper_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    doctor_detail(
        &mut output,
        "generation_id",
        &report.termux_wrapper.generation_id,
        color,
    );
    doctor_detail(
        &mut output,
        "layout",
        report.termux_wrapper.generation_layout.as_str(),
        color,
    );
    doctor_row(
        &mut output,
        "code host",
        "code-mode companion status",
        code_mode_healthy,
        color,
    );
    if code_mode_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    doctor_detail(&mut output, "code_mode_host", code_mode_status, color);

    output.push('\n');
    output.push_str(&doctor_bold("Support", color));
    output.push('\n');
    doctor_warning(
        &mut output,
        "sandbox",
        "Linux namespace/mount sandboxing is unsupported on Termux; bwrap is not used",
        color,
    );
    doctor_detail(
        &mut output,
        "sandbox",
        "unsupported (bwrap is not used)",
        color,
    );
    for warning in &report.termux_wrapper.warnings {
        doctor_warning(&mut output, "config", warning, color);
        warning_count += 1;
    }

    output.push('\n');
    output.push_str(&doctor_bold("Wrapper", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "wrapper",
        report
            .termux_wrapper
            .reason
            .as_deref()
            .unwrap_or("wrapper API and selected generation are compatible"),
        wrapper_healthy,
        color,
    );
    if wrapper_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    doctor_detail(
        &mut output,
        "wrapper",
        report.termux_wrapper.status.as_str(),
        color,
    );

    output.push('\n');
    output.push_str(&doctor_bold("State", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "current",
        "one complete generation is selected",
        true,
        color,
    );
    ok_count += 1;
    doctor_detail(
        &mut output,
        "generation",
        &report.termux_wrapper.generation_id,
        color,
    );

    output.push_str("\n[Summary]\n");
    doctor_row(
        &mut output,
        "status",
        report.summary.as_str(),
        summary_healthy,
        color,
    );
    if summary_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    output.push('\n');
    output.push_str(&doctor_dim(
        "─────────────────────────────────────────────────────────────",
        color,
    ));
    output.push('\n');
    let summary_status = if summary_healthy {
        doctor_color("healthy", 10, color)
    } else {
        doctor_color(report.summary.as_str(), 196, color)
    };
    output.push_str(&format!(
        "{} ok · {} warn · {} fail {}\n",
        ok_count, warning_count, failure_count, summary_status
    ));
    output
}

pub(super) fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write as _;
                write!(&mut escaped, "\\u{:04x}", c as u32)
                    .expect("writing into String cannot fail");
            }
            c => escaped.push(c),
        }
    }
    escaped
}

fn render_doctor_json(report: &DoctorReport) -> String {
    let (code_mode_status, _) = code_mode_status(&report.termux_wrapper);
    let runtime_status = if report.termux_wrapper.status == WrapperDoctorStatus::Healthy {
        "healthy"
    } else {
        "unhealthy"
    };
    let upstream_reason = report
        .upstream
        .reason
        .as_deref()
        .map(|r| format!(",\"reason\":\"{}\"", json_escape(r)))
        .unwrap_or_default();
    let wrapper_reason = report
        .termux_wrapper
        .reason
        .as_deref()
        .map(|r| format!(",\"reason\":\"{}\"", json_escape(r)))
        .unwrap_or_default();
    let wrapper_warnings = if report.termux_wrapper.warnings.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = report
            .termux_wrapper
            .warnings
            .iter()
            .map(|w| format!("\"{}\"", json_escape(w)))
            .collect();
        format!(",\"warnings\":[{}]", items.join(","))
    };
    format!(
        "{{\"schema_version\":1,\"upstream\":{{\"status\":\"{}\",\"output\":\"{}\"{}}},\"termux_wrapper\":{{\"status\":\"{}\"{}{},\"generation_id\":\"{}\",\"layout\":\"{}\",\"runtime\":{{\"status\":\"{}\"}},\"code_mode_host\":{{\"status\":\"{}\"}},\"sandbox\":{{\"status\":\"unsupported\",\"reason\":\"bwrap is not used\"}}}},\"summary\":{{\"status\":\"{}\"}}}}\n",
        report.upstream.status.as_str(),
        json_escape(&report.upstream.output),
        upstream_reason,
        report.termux_wrapper.status.as_str(),
        wrapper_reason,
        wrapper_warnings,
        json_escape(&report.termux_wrapper.generation_id),
        report.termux_wrapper.generation_layout.as_str(),
        runtime_status,
        code_mode_status,
        report.summary.as_str(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorOutputMode {
    Human,
    Color,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DoctorCommandOutcome {
    pub(super) output: String,
    pub(super) exit_class: DoctorExitClass,
}

#[derive(Debug)]
pub(super) enum LocalDoctorCommandError {
    Usage,
}

impl std::fmt::Display for LocalDoctorCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalDoctorCommandError::Usage => f.write_str("usage: codex doctor [--json|--color]"),
        }
    }
}

impl std::error::Error for LocalDoctorCommandError {}

fn doctor_output_mode<I, S>(args: I) -> Result<DoctorOutputMode, LocalDoctorCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    match (args.next(), args.next()) {
        (None, None) => Ok(DoctorOutputMode::Human),
        (Some(arg), None) if arg.as_os_str() == OsStr::new("--color") => {
            Ok(DoctorOutputMode::Color)
        }
        (Some(arg), None) if arg.as_os_str() == OsStr::new("--json") => Ok(DoctorOutputMode::Json),
        _ => Err(LocalDoctorCommandError::Usage),
    }
}

pub(super) fn run_local_doctor_command<'context, 'runtime_selection, 'runtime_asset, I, S>(
    args: I,
    context: LocalPublicDispatchContext<'context, 'runtime_selection, 'runtime_asset>,
) -> Result<DoctorCommandOutcome, LocalDoctorCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mode = doctor_output_mode(args)?;
    let mut warnings = Vec::new();
    if let Err(err) = crate::extensions::WrapperSettings::read(context.state_root) {
        warnings.push(format!("invalid wrapper.toml: {err}"));
    }
    if let Some(codex_home) = crate::resolve_codex_home() {
        if let Err(err) = crate::daemon_settings::inspect_read_only(&codex_home) {
            warnings.push(format!("invalid daemon settings.json: {err}"));
        }
    }
    let color = match mode {
        DoctorOutputMode::Human => doctor_color_enabled(false),
        DoctorOutputMode::Color => doctor_color_enabled(true),
        DoctorOutputMode::Json => false,
    };
    let json = mode == DoctorOutputMode::Json;
    let force_color = mode == DoctorOutputMode::Color;
    let mut wrapper_status = context.wrapper_doctor_status;
    let mut wrapper_reason = None;
    let upstream = match capture_qualified_upstream_doctor(
        context.runtime_assets,
        context.process_env,
        context.cert_file,
        context.cert_dir,
        context.resolver_path,
        context.config_dir,
        DoctorCaptureOptions {
            json,
            use_color: color,
            force_color,
        },
    ) {
        Ok(result) => result,
        Err(QualifiedUpstreamDoctorProbeError::WrapperPreparation(msg)) => {
            wrapper_status = WrapperDoctorStatus::Unhealthy;
            wrapper_reason = Some(msg);
            QualifiedUpstreamDoctorResult {
                status: UpstreamDoctorStatus::NotRun,
                output: String::new(),
                reason: Some("wrapper preparation failed before running upstream".to_owned()),
            }
        }
        Err(QualifiedUpstreamDoctorProbeError::TimedOut) => QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::TimedOut,
            output: String::new(),
            reason: Some("upstream doctor exceeded time limit".to_owned()),
        },
        Err(QualifiedUpstreamDoctorProbeError::OutputTooLarge) => QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::OutputExceeded,
            output: String::new(),
            reason: Some("upstream doctor output exceeded 64 KiB limit".to_owned()),
        },
        Err(QualifiedUpstreamDoctorProbeError::Io(err)) => QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::Unhealthy,
            output: String::new(),
            reason: Some(format!("upstream doctor execution failed: {err}")),
        },
    };
    let report = compose_doctor_report(
        upstream,
        TermuxWrapperDoctorReport {
            status: wrapper_status,
            generation_id: context.generation_id.to_owned(),
            generation_layout: context.generation_layout,
            has_code_mode_host: context.has_code_mode_host,
            reason: wrapper_reason,
            warnings,
        },
    );
    let output = match mode {
        DoctorOutputMode::Human | DoctorOutputMode::Color => render_doctor_human(&report, color),
        DoctorOutputMode::Json => render_doctor_json(&report),
    };
    Ok(DoctorCommandOutcome {
        output,
        exit_class: doctor_exit_class(&report),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{os_vec, remove_temp_root, temp_root, EnvRestoreGuard};
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;

    fn doctor_test_roots(root: &Path) -> crate::LocalWrapperRoots {
        let home = root.join("home");
        let state_root = home.join(".local/share/codex/wrapper");
        let config_dir = state_root.join("config");
        let generation_root = home.join(".codex/packages/standalone/releases");
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&generation_root).unwrap();
        let resolver_path = root.join("resolv.conf");
        fs::write(&resolver_path, b"nameserver 1.1.1.1\n").unwrap();
        let cert_file = root.join("cert.pem");
        let cert_dir = root.join("certs");
        fs::write(&cert_file, b"").unwrap();
        fs::create_dir_all(&cert_dir).unwrap();
        let openssl = crate::tests::test_openssl_path();
        let curl = openssl.parent().unwrap().join("curl");
        crate::LocalWrapperRoots {
            generation_root,
            state_root,
            config_dir,
            resolver_path,
            cert_file,
            cert_dir,
            openssl,
            curl,
        }
    }

    #[test]
    fn doctor_attributes_wrapper_preparation_failure_and_warnings() {
        let root = temp_root("p2-09-prep");
        let roots = doctor_test_roots(&root);
        let gen_dir = roots.generation_root.join("1.2.3-aaaaaa");
        crate::tests::write_sample_generation(&gen_dir, "1.2.3-aaaaaa", false);
        let marker = root.join("upstream-was-invoked");
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        fs::write(
            gen_dir.join("bin/codex"),
            format!(
                "#!{shell}\n: > '{}'\necho '{{\"ok\":true}}'\n",
                marker.display()
            ),
        )
        .unwrap();
        fs::set_permissions(gen_dir.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();

        let _env =
            EnvRestoreGuard::new(&[("HOME", Some(&root.join("home"))), ("CODEX_HOME", None)]);
        fs::write(roots.state_root.join("wrapper.toml"), "invalid = [\n").unwrap();
        let daemon_settings = root.join("home/.codex/app-server-daemon/settings.json");
        fs::create_dir_all(daemon_settings.parent().unwrap()).unwrap();
        fs::write(&daemon_settings, "{corrupt").unwrap();

        // Break resolver_path so RuntimeFdSources::open fails during wrapper preparation.
        fs::remove_file(&roots.resolver_path).unwrap();
        let loaded = crate::load_local_generation(&gen_dir).unwrap();
        let process_env = crate::tests::test_process_env(&roots);

        let outcome = crate::with_qualified_loaded_runtime(&loaded, |assets| {
            let ctx = LocalPublicDispatchContext {
                runtime_assets: assets,
                process_env: &process_env,
                cert_file: roots.cert_file.as_os_str(),
                cert_dir: Some(roots.cert_dir.as_os_str()),
                resolver_path: &roots.resolver_path,
                config_dir: &roots.config_dir,
                state_root: &roots.state_root,
                wrapper_doctor_status: WrapperDoctorStatus::Healthy,
                generation_id: &loaded.generation_id,
                generation_layout: loaded.generation_layout,
                has_code_mode_host: false,
            };
            Ok(run_local_doctor_command(os_vec(&["--json"]), ctx).unwrap())
        })
        .unwrap();

        assert_eq!(outcome.exit_class, DoctorExitClass::HealthFailure);
        assert!(
            !marker.exists(),
            "upstream doctor must not execute when wrapper prep fails"
        );
        assert!(
            outcome
                .output
                .contains("\"upstream\":{\"status\":\"not_run\""),
            "{}",
            outcome.output
        );
        assert!(
            outcome
                .output
                .contains("\"termux_wrapper\":{\"status\":\"unhealthy\""),
            "{}",
            outcome.output
        );
        assert!(
            outcome
                .output
                .contains("\"summary\":{\"status\":\"unhealthy\"}"),
            "{}",
            outcome.output
        );
        assert!(
            outcome.output.contains("invalid wrapper.toml"),
            "{}",
            outcome.output
        );
        assert!(
            outcome.output.contains("invalid daemon settings.json"),
            "{}",
            outcome.output
        );
        assert_eq!(fs::read_to_string(&daemon_settings).unwrap(), "{corrupt");

        drop(_env);
        remove_temp_root(root);
    }

    #[test]
    fn doctor_timeout_reaps_process_group_and_caps_oversized_output() {
        let root = temp_root("p2-09-proc");
        let roots = doctor_test_roots(&root);
        let gen_dir = roots.generation_root.join("1.2.3-bbbbbb");
        crate::tests::write_sample_generation(&gen_dir, "1.2.3-bbbbbb", false);
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let sleep_bin = Path::new("/system/bin/sleep");
        let sleep_cmd = if sleep_bin.exists() {
            "/system/bin/sleep"
        } else {
            "sleep"
        };
        let child_pid_file = root.join("bg.pid");
        fs::write(
            gen_dir.join("bin/codex"),
            format!(
                "#!{shell}\n{sleep_cmd} 30 &\necho \"$!\" > '{}'\nwait\n",
                child_pid_file.display()
            ),
        )
        .unwrap();
        fs::set_permissions(gen_dir.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();

        let loaded = crate::load_local_generation(&gen_dir).unwrap();
        let process_env = crate::tests::test_process_env(&roots);
        let opts = DoctorCaptureOptions {
            json: true,
            use_color: false,
            force_color: false,
        };
        let started = Instant::now();
        let res = crate::with_qualified_loaded_runtime(&loaded, |assets| {
            Ok(capture_qualified_upstream_doctor_with_timeout(
                assets,
                &process_env,
                roots.cert_file.as_os_str(),
                Some(roots.cert_dir.as_os_str()),
                &roots.resolver_path,
                &roots.config_dir,
                opts,
                Duration::from_millis(200),
            ))
        })
        .unwrap();
        assert!(
            matches!(res, Err(QualifiedUpstreamDoctorProbeError::TimedOut)),
            "{res:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));

        if let Ok(raw_pid) = fs::read_to_string(&child_pid_file) {
            if let Ok(pid) = raw_pid.trim().parse::<i32>() {
                let deadline = Instant::now() + Duration::from_secs(2);
                while Instant::now() < deadline {
                    if unsafe { kill(pid, 0) } != 0 {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                assert_ne!(
                    unsafe { kill(pid, 0) },
                    0,
                    "grandchild {pid} must be reaped"
                );
            }
        }

        // Now test > 64 KiB stdout -> OutputTooLarge
        fs::write(
            gen_dir.join("bin/codex"),
            format!("#!{shell}\ni=0\nline='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nwhile [ \"$i\" -lt 1200 ]; do echo \"$line\"; i=$((i+1)); done\n"),
        )
        .unwrap();
        fs::set_permissions(gen_dir.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();
        let loaded2 = crate::load_local_generation(&gen_dir).unwrap();
        let res2 = crate::with_qualified_loaded_runtime(&loaded2, |assets| {
            Ok(capture_qualified_upstream_doctor_with_timeout(
                assets,
                &process_env,
                roots.cert_file.as_os_str(),
                Some(roots.cert_dir.as_os_str()),
                &roots.resolver_path,
                &roots.config_dir,
                opts,
                Duration::from_secs(5),
            ))
        })
        .unwrap();
        assert!(
            matches!(res2, Err(QualifiedUpstreamDoctorProbeError::OutputTooLarge)),
            "{res2:?}"
        );

        remove_temp_root(root);
    }
}
