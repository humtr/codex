use crate::generation::{termux_browser_helper_paths, GenerationLayout, QualifiedRuntimeAssets};
use crate::json::json_escape;
use crate::proc_exec::{self, nonblocking_output_pipe as doctor_output_pipe, O_NONBLOCK_LINUX};
use crate::runtime_env::{
    apply_child_env_plan_and_fence, inspect_shared_home_from_env, plan_termux_env,
    RuntimeFdSources, TermuxProcessEnvError, TermuxProcessEnvSnapshot,
    TERMUX_WRAPPER_CONFIG_CONTENT,
};
use crate::LocalPublicDispatchContext;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::fd::AsRawFd as _;
use std::os::unix::fs::{
    DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus, Stdio};
use std::sync::atomic::Ordering;
use std::time::Duration;

// Upstream `doctor` walks `$CODEX_HOME` (including `packages/standalone` and large session histories); allow >40s flash scans.
const DOCTOR_OUTPUT_MAX_BYTES: usize = 64 * 1024;
const DOCTOR_PROBE_TIMEOUT: Duration = Duration::from_secs(90);
#[derive(Debug)]
pub(super) enum QualifiedUpstreamDoctorProbeError {
    WrapperPreparation(String),
    Io(std::io::Error),
    TimedOut(Vec<u8>),
    OutputTooLarge(Vec<u8>),
}

impl std::fmt::Display for QualifiedUpstreamDoctorProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedUpstreamDoctorProbeError::WrapperPreparation(msg) => f.write_str(msg),
            QualifiedUpstreamDoctorProbeError::Io(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::TimedOut(_) => {
                f.write_str("upstream doctor timed out")
            }
            QualifiedUpstreamDoctorProbeError::OutputTooLarge(_) => {
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
            | QualifiedUpstreamDoctorProbeError::TimedOut(_)
            | QualifiedUpstreamDoctorProbeError::OutputTooLarge(_) => None,
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
                    if bytes[final_index] == b'K' && matches!(parameters, b"1" | b"2") {
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
                    index += 1;
                    if let Some(next) = value[index..].chars().next() {
                        index += next.len_utf8();
                    }
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
            let rest = line[after_key..].trim_start_matches([' ', '\t']);
            let rest = rest
                .strip_prefix(['"', '\''])
                .unwrap_or(rest)
                .trim_start_matches([' ', '\t']);
            if !rest.starts_with([':', '=']) {
                search_from = after_key;
                continue;
            }
            let delimiter = line.len() - rest.len();
            let mut value_start = delimiter + 1;
            while let Some(character) = line[value_start..].chars().next() {
                if character == ' ' || character == '\t' {
                    value_start += character.len_utf8();
                } else {
                    break;
                }
            }
            let quote = line[value_start..]
                .chars()
                .next()
                .filter(|c| matches!(c, '"' | '\''));
            if quote.is_some() {
                value_start += 1;
            }
            let mut value_end = value_start;
            let mut escaped = false;
            for (offset, character) in line[value_start..].char_indices() {
                let is_end = if let Some(quote) = quote {
                    let closing = character == quote && !escaped;
                    escaped = character == '\\' && !escaped;
                    closing
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
            if &line[value_start..value_end] == "[redacted]" {
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
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        if start >= end {
            continue;
        }
        if let Some((_, last_end)) = merged.last_mut().filter(|(_, last_end)| start < *last_end) {
            *last_end = end.max(*last_end);
        } else {
            merged.push((start, end));
        }
    }
    for (start, end) in merged.into_iter().rev() {
        let styled_start = *boundaries.get(start)?;
        let styled_end = *boundaries.get(end)?;
        let replacement = doctor_styled_redaction_segment(output.get(styled_start..styled_end)?);
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

fn collect_bounded_output_and_wait(
    child: Child,
    reader: File,
    timeout: Duration,
) -> Result<(Vec<u8>, ExitStatus), QualifiedUpstreamDoctorProbeError> {
    proc_exec::collect_bounded_output_and_wait(child, reader, timeout, DOCTOR_OUTPUT_MAX_BYTES)
        .map_err(|err| match err {
            proc_exec::OutputError::Io(err) => QualifiedUpstreamDoctorProbeError::Io(err),
            proc_exec::OutputError::TimedOut(bytes) => {
                QualifiedUpstreamDoctorProbeError::TimedOut(bytes)
            }
            proc_exec::OutputError::OutputTooLarge(bytes) => {
                QualifiedUpstreamDoctorProbeError::OutputTooLarge(bytes)
            }
        })
}

struct DoctorTemp(PathBuf);

impl DoctorTemp {
    fn create() -> io::Result<Self> {
        let root = std::env::temp_dir();
        for _ in 0..32 {
            let sequence = crate::fs_util::LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(".codex-doctor-{}-{sequence}", std::process::id()));
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            }
        }
        Err(io::Error::other(
            "could not create temporary doctor directory",
        ))
    }
}

impl Drop for DoctorTemp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn open_doctor_config(
    config_dir: &Path,
    temp: &DoctorTemp,
    warning: &mut Option<&'static str>,
) -> io::Result<File> {
    use crate::fs_util;
    let expected = TERMUX_WRAPPER_CONFIG_CONTENT.as_bytes();
    for dir in config_dir.ancestors() {
        match fs::symlink_metadata(dir) {
            Ok(meta) if meta.is_dir() => (),
            Ok(_) => {
                return Err(io::Error::other(
                    "wrapper config path is not a real directory",
                ))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => (),
            Err(err) => return Err(err),
        }
    }

    let config = OpenOptions::new()
        .read(true)
        .custom_flags(fs_util::O_NOFOLLOW | O_NONBLOCK_LINUX)
        .open(config_dir);
    *warning = Some(match config {
        Ok(dir) => {
            let meta = dir.metadata()?;
            if !meta.is_dir()
                || meta.uid() != fs_util::current_uid()?
                || meta.permissions().mode() & 0o022 != 0
            {
                return Err(io::Error::other("wrapper config directory is unsafe"));
            }
            // Inspect the file under the exact directory being passed to FD 34.
            let config_path =
                PathBuf::from(format!("/proc/self/fd/{}/config.toml", dir.as_raw_fd()));
            match fs::symlink_metadata(&config_path) {
                Ok(meta) => {
                    if !meta.is_file()
                        || meta.uid() != fs_util::current_uid()?
                        || meta.permissions().mode() & 0o022 != 0
                    {
                        return Err(io::Error::other(
                            "wrapper config.toml is not a safe regular file",
                        ));
                    }
                    if meta.len() == expected.len() as u64
                        && fs_util::read_bounded_regular(
                            &config_path,
                            expected.len() as u64,
                            true,
                            "unsafe wrapper config.toml",
                        )? == expected
                    {
                        return Ok(dir);
                    }
                    "wrapper config.toml has drifted; using temporary Termux defaults"
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    "wrapper config.toml is absent; using temporary Termux defaults"
                }
                Err(err) => return Err(err),
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            "wrapper config.toml is absent; using temporary Termux defaults"
        }
        Err(err) => return Err(err),
    });

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(temp.0.join("config.toml"))?;
    file.write_all(expected)?;
    File::open(&temp.0)
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
    config_warning: &mut Option<&'static str>,
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
    let env_plan = plan_termux_env(process_env, browser_helpers, cert_file, cert_dir)?;
    let sqlite_home_override = inspect_shared_home_from_env().map_err(|err| {
        QualifiedUpstreamDoctorProbeError::WrapperPreparation(format!(
            "wrapper shared-home inspection failed: {err}"
        ))
    })?;
    let config_error = |err| {
        QualifiedUpstreamDoctorProbeError::WrapperPreparation(format!(
            "wrapper config inspection or temporary FD preparation failed: {err}"
        ))
    };
    let temp = DoctorTemp::create().map_err(config_error)?;
    let config =
        open_doctor_config(config_dir.as_ref(), &temp, config_warning).map_err(config_error)?;
    let runtime_fds = RuntimeFdSources::open_with_resolver_cache(resolver_path, &config, false)
        .map_err(|err| {
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
    let (reader, writer) = doctor_output_pipe().map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    cmd.stderr(Stdio::from(
        writer
            .try_clone()
            .map_err(QualifiedUpstreamDoctorProbeError::Io)?,
    ))
    .stdout(Stdio::from(writer))
    .stdin(Stdio::null());
    proc_exec::isolate_process_group(&mut cmd);
    apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
    crate::timezone::configure_with_cache(&mut cmd, Some(&temp.0));
    if let Some(sqlite_home) = sqlite_home_override {
        cmd.env("CODEX_SQLITE_HOME", sqlite_home);
    }
    apply_doctor_color_override(&mut cmd, use_color, force_color);
    runtime_fds.configure(&mut cmd);
    let child = cmd.spawn().map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    drop(cmd);
    let (bytes, status) = collect_bounded_output_and_wait(child, reader, timeout)?;
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
    if report.termux_wrapper.generation_id.is_empty() {
        doctor_row(
            &mut output,
            "current",
            "generation unavailable",
            false,
            color,
        );
        failure_count += 1;
    } else {
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
    }

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
pub(super) enum DoctorOutputMode {
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

pub(super) fn doctor_output_mode<I, S>(args: I) -> Result<DoctorOutputMode, LocalDoctorCommandError>
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

pub(super) fn doctor_preparation_failure(
    mode: DoctorOutputMode,
    reason: &str,
) -> DoctorCommandOutcome {
    let report = compose_doctor_report(
        QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::NotRun,
            output: String::new(),
            reason: Some("wrapper preparation failed before running upstream".to_owned()),
        },
        TermuxWrapperDoctorReport {
            status: WrapperDoctorStatus::Unhealthy,
            generation_id: String::new(),
            generation_layout: GenerationLayout::PackageRoot,
            has_code_mode_host: false,
            reason: Some(redact_doctor_output(reason.as_bytes(), false)),
            warnings: vec![],
        },
    );
    doctor_command_outcome(
        &report,
        mode,
        doctor_color_enabled(mode == DoctorOutputMode::Color),
    )
}

fn doctor_command_outcome(
    report: &DoctorReport,
    mode: DoctorOutputMode,
    color: bool,
) -> DoctorCommandOutcome {
    let output = match mode {
        DoctorOutputMode::Human | DoctorOutputMode::Color => render_doctor_human(report, color),
        DoctorOutputMode::Json => render_doctor_json(report),
    };
    DoctorCommandOutcome {
        output,
        exit_class: doctor_exit_class(report),
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
    if let Some(codex_home) = crate::runtime_env::resolve_codex_home() {
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
    let mut config_warning = None;
    let upstream = match capture_qualified_upstream_doctor_with_timeout(
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
        DOCTOR_PROBE_TIMEOUT,
        &mut config_warning,
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
        Err(QualifiedUpstreamDoctorProbeError::TimedOut(bytes)) => QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::TimedOut,
            output: redact_doctor_output(&bytes, color),
            reason: Some("upstream doctor exceeded time limit".to_owned()),
        },
        Err(QualifiedUpstreamDoctorProbeError::OutputTooLarge(bytes)) => {
            QualifiedUpstreamDoctorResult {
                status: UpstreamDoctorStatus::OutputExceeded,
                output: redact_doctor_output(&bytes, color),
                reason: Some("upstream doctor output exceeded 64 KiB limit".to_owned()),
            }
        }
        Err(QualifiedUpstreamDoctorProbeError::Io(err)) => QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::Unhealthy,
            output: String::new(),
            reason: Some(format!("upstream doctor execution failed: {err}")),
        },
    };
    warnings.extend(config_warning.map(str::to_owned));
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
    Ok(doctor_command_outcome(&report, mode, color))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proc_exec::kill;
    use crate::tests::{os_vec, remove_temp_root, temp_root, EnvRestoreGuard};
    use std::fs;
    use std::os::unix::process::CommandExt as _;
    use std::time::Instant;

    #[test]
    fn redaction_merges_overlapping_styled_ranges_at_utf8_boundaries() {
        let raw = "한\x1b[31m암호\x1b[0m값 완료";
        assert_eq!(
            doctor_styled_redactions(raw, &[(3, 9), (6, 12), (3, 6)]).unwrap(),
            "한\x1b[31m[redacted]\x1b[0m 완료"
        );
        let raw = "표시 token = \x1b[32m비\x1b[1m밀\x1b[0m done\n";
        let styled = redact_doctor_output(raw.as_bytes(), true);
        assert!(styled.contains("\x1b[32m[redacted]"));
        assert!(styled.contains("\x1b[1m"));
        assert!(styled.contains("\x1b[0m done"));
        assert!(!styled.contains('비'));
        assert!(!styled.contains('밀'));
        assert_eq!(
            redact_doctor_output(raw.as_bytes(), false),
            "표시 token = [redacted] done\n"
        );
    }

    #[test]
    fn redaction_requires_adjacent_delimiters_and_erase_end_preserves_prefix() {
        let raw =
            b"token count: 5\x1b[0K\nsecret missing; status=ok\x1b[K\n\"api_key\" \t: \"hidden\"\n";
        let expected =
            "token count: 5\nsecret missing; status=ok\n\"api_key\" \t: \"[redacted]\"\n";
        assert_eq!(redact_doctor_output(raw, true), expected);
        assert_eq!(redact_doctor_output(raw, false), expected);
        let raw = br#"secret = 'first second' "api_key": "escaped\" quote""#;
        assert_eq!(
            redact_doctor_output(raw, true),
            r#"secret = '[redacted]' "api_key": "[redacted]""#
        );
        let raw = b"progress 50%\r\x1b[2Kfinished\x1b[K\n";
        assert_eq!(redact_doctor_output(raw, true), "finished\n");
    }

    #[test]
    fn non_pty_capture_drains_stderr_and_stops_grandchild_after_leader_exits() {
        let root = temp_root("doctor-open-pipe");
        let marker = root.join("grandchild.pid");
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let (reader, writer) = doctor_output_pipe().unwrap();
        let mut command = std::process::Command::new(shell);
        // Ignore TERM in the background process to exercise group escalation.
        command
            .args([
                "-c",
                r#"trap '' TERM
                (sleep 30) &
                printf '%s\n' "$!" > "$1"
                printf 'upstream info\n'
                printf 'upstream error\n' >&2
                exit 7"#,
                "doctor",
                marker.to_str().unwrap(),
            ])
            .stdout(Stdio::from(writer.try_clone().unwrap()))
            .stderr(Stdio::from(writer))
            .process_group(0);
        let child = command.spawn().unwrap();
        drop(command);
        let start = Instant::now();
        let (bytes, status) =
            collect_bounded_output_and_wait(child, reader, Duration::from_secs(5)).unwrap();
        assert_eq!(status.code(), Some(7));
        assert_eq!(bytes, b"upstream info\nupstream error\n");
        assert!(start.elapsed() < Duration::from_secs(2));
        let pid = fs::read_to_string(marker).unwrap();
        let process = PathBuf::from("/proc").join(pid.trim());
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut no_wait = Duration::ZERO;
        while !crate::maintenance::process_has_no_handles(&process, &mut no_wait).unwrap() {
            assert!(Instant::now() < deadline, "grandchild still running");
            std::thread::sleep(Duration::from_millis(10));
        }
        remove_temp_root(root);
    }

    #[test]
    fn doctor_snapshot_uses_read_only_bindings_and_ephemeral_config() {
        const CHILD: &str = "CODEX_WP3_DOCTOR_SNAPSHOT_CHILD";
        if std::env::var_os(CHILD).is_some() {
            std::process::exit(crate::run_public_main(os_vec(&["doctor", "--json"])));
        }

        use std::collections::BTreeMap;
        type TreeSnapshot = BTreeMap<PathBuf, (u32, u64, (i64, i64), Vec<u8>)>;

        fn snapshot(root: &Path) -> TreeSnapshot {
            fn collect(root: &Path, dir: &Path, entries: &mut TreeSnapshot) {
                for entry in fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    let meta = fs::symlink_metadata(&path).unwrap();
                    let bytes = if meta.is_symlink() {
                        fs::read_link(&path)
                            .unwrap()
                            .into_os_string()
                            .into_encoded_bytes()
                    } else if meta.is_file() {
                        fs::read(&path).unwrap()
                    } else {
                        assert!(meta.is_dir());
                        collect(root, &path, entries);
                        Vec::new()
                    };
                    entries.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        (
                            meta.mode(),
                            meta.ino(),
                            (meta.mtime(), meta.mtime_nsec()),
                            bytes,
                        ),
                    );
                }
            }
            let mut entries = BTreeMap::new();
            collect(root, root, &mut entries);
            entries
        }

        let root = temp_root("doctor-snapshot");
        let home = root.join("home");
        let codex_home = root.join("profile");
        let shared = root.join("shared");
        let prefix = root.join("prefix");
        let tmp = root.join("tmp");
        let xdg_cache = home.join(".cache");
        for dir in [&home, &codex_home, &shared, &prefix, &tmp] {
            crate::tests::create_private_test_dirs(dir);
        }
        let env = EnvRestoreGuard::new(&[
            ("HOME", Some(&home)),
            ("CODEX_HOME", Some(&codex_home)),
            ("CODEX_SHARED_HOME", Some(&shared)),
            ("CODEX_SQLITE_HOME", None),
            ("PREFIX", Some(&prefix)),
            ("TMPDIR", Some(&tmp)),
            ("XDG_CACHE_HOME", Some(&xdg_cache)),
        ]);
        let roots = crate::LocalWrapperRoots::from_environment().unwrap();
        let stage = roots.generation_root.join("gen-1");
        crate::tests::create_private_test_dirs(&roots.state_root);
        crate::tests::write_sample_generation(&stage, "gen-1", false);
        crate::tests::create_private_test_dirs(roots.resolver_path.parent().unwrap());
        fs::write(&roots.resolver_path, b"nameserver 1.1.1.1\n").unwrap();
        fs::write(
            codex_home.join("session_index.jsonl"),
            b"{\"id\":\"local\"}\n",
        )
        .unwrap();
        let state = crate::state::generation_state::WrapperStatePaths::new(&roots.state_root)
            .unwrap()
            .with_generation_root(&roots.generation_root);
        let pointer = crate::state::generation_state::plan_initial_pointer_state("gen-1").unwrap();
        fs::write(
            &state.activation_state,
            crate::state::generation_state::encode_pointer_state(&pointer).unwrap(),
        )
        .unwrap();
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let runtime = stage.join("bin/codex");
        fs::write(
            &runtime,
            format!(
                r#"#!{shell}
IFS= read -r config < "/proc/$$/fd/34/config.toml" || exit 11
[ "$config" = '# codex-termux-wrapper' ] || exit 12
printf 'sqlite=%s\n' "$CODEX_SQLITE_HOME"
printf 'upstream stderr captured\n' >&2
"#
            ),
        )
        .unwrap();
        fs::set_permissions(runtime, fs::Permissions::from_mode(0o700)).unwrap();

        let invoke = || {
            let before = snapshot(&root);
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "doctor::tests::doctor_snapshot_uses_read_only_bindings_and_ephemeral_config",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            let mut after = snapshot(&root);
            if let Some(tmp_before) = before.get(Path::new("tmp")) {
                after.get_mut(Path::new("tmp")).unwrap().2 = tmp_before.2;
            }
            assert_eq!(after, before);
            let output = String::from_utf8(result.stdout).unwrap();
            (result.status, output)
        };

        // Compare all protected trees. The disposable TMPDIR is allowed to
        // change directory timestamps while creating and removing ephemeral FDs.
        let (status, output) = invoke();
        assert!(status.success(), "{output}");
        assert!(output.contains("wrapper config.toml is absent"), "{output}");
        assert!(output.contains("upstream stderr captured"), "{output}");
        assert!(
            output.contains(&format!("sqlite={}", shared.display())),
            "{output}"
        );

        crate::tests::create_private_test_dirs(&roots.config_dir);
        fs::write(roots.config_dir.join("config.toml"), "drifted contents").unwrap();
        let (status, output) = invoke();
        assert!(status.success(), "{output}");
        assert!(
            output.contains("wrapper config.toml has drifted"),
            "{output}"
        );

        fs::write(
            roots.config_dir.join("config.toml"),
            TERMUX_WRAPPER_CONFIG_CONTENT,
        )
        .unwrap();
        let (status, output) = invoke();
        assert!(status.success(), "{output}");
        assert!(!output.contains("temporary Termux defaults"), "{output}");

        let pending = crate::state::generation_state::ActivationJournal {
            operation: crate::state::generation_state::ActivationOperation::Update,
            before: None,
            after: pointer,
        };
        fs::write(
            &state.activation_journal,
            crate::state::generation_state::encode_activation_journal(&pending).unwrap(),
        )
        .unwrap();
        let (status, output) = invoke();
        assert_eq!(status.code(), Some(1));
        assert!(output.contains("\"status\":\"not_run\""), "{output}");
        assert!(output.contains("pending transaction"), "{output}");
        assert_eq!(fs::read_dir(&tmp).unwrap().count(), 0);
        drop(env);
        remove_temp_root(root);
    }

    fn doctor_test_roots(root: &Path) -> crate::LocalWrapperRoots {
        let home = root.join("home");
        let state_root = home.join(".local/share/codex/wrapper");
        let config_dir = state_root.join("config");
        let generation_root = home.join(".codex/packages/standalone/releases");
        crate::tests::create_private_test_dirs(&config_dir);
        fs::create_dir_all(&generation_root).unwrap();
        let resolver_path = root.join("resolv.conf");
        fs::write(&resolver_path, b"nameserver 1.1.1.1\n").unwrap();
        let cert_file = root.join("cert.pem");
        let cert_dir = root.join("certs");
        fs::write(&cert_file, b"").unwrap();
        fs::create_dir_all(&cert_dir).unwrap();
        let curl = root.join("prefix/bin/curl");
        crate::LocalWrapperRoots {
            generation_root,
            state_root,
            config_dir,
            resolver_path,
            cert_file,
            cert_dir,
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
        let daemon_settings = root.join("home/.codex/app-server-daemon/settings.json");
        fs::create_dir_all(daemon_settings.parent().unwrap()).unwrap();
        fs::write(&daemon_settings, "{corrupt").unwrap();

        // Break resolver_path so RuntimeFdSources::open fails during wrapper preparation.
        fs::remove_file(&roots.resolver_path).unwrap();
        let loaded = crate::generation::load_local_generation(&gen_dir).unwrap();
        let process_env = crate::tests::test_process_env(&roots);

        let outcome = crate::generation::with_qualified_loaded_runtime(&loaded, |assets| {
            let ctx = LocalPublicDispatchContext {
                runtime_assets: assets,
                process_env: &process_env,
                cert_file: roots.cert_file.as_os_str(),
                cert_dir: Some(roots.cert_dir.as_os_str()),
                resolver_path: &roots.resolver_path,
                config_dir: &roots.config_dir,
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
        let sleep_cmd = ["/system/bin/sleep", "/bin/sleep", "/usr/bin/sleep"]
            .into_iter()
            .find(|path| Path::new(path).exists())
            .unwrap_or("sleep");
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

        let loaded = crate::generation::load_local_generation(&gen_dir).unwrap();
        let process_env = crate::tests::test_process_env(&roots);
        let opts = DoctorCaptureOptions {
            json: true,
            use_color: false,
            force_color: false,
        };
        let started = Instant::now();
        let res = crate::generation::with_qualified_loaded_runtime(&loaded, |assets| {
            Ok(capture_qualified_upstream_doctor_with_timeout(
                assets,
                &process_env,
                roots.cert_file.as_os_str(),
                Some(roots.cert_dir.as_os_str()),
                &roots.resolver_path,
                &roots.config_dir,
                opts,
                Duration::from_millis(200),
                &mut None,
            ))
        })
        .unwrap();
        assert!(
            matches!(res, Err(QualifiedUpstreamDoctorProbeError::TimedOut(_))),
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

        // The bound applies to either stream and to their combined bytes.
        for redirects in ["", ">&2", ">&2; echo \"$line\""] {
            fs::write(
                gen_dir.join("bin/codex"),
                format!("#!{shell}\ni=0\nline='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nwhile [ \"$i\" -lt 1200 ]; do echo \"$line\" {redirects}; i=$((i+1)); done\n"),
            )
            .unwrap();
            let loaded2 = crate::generation::load_local_generation(&gen_dir).unwrap();
            let res2 = crate::generation::with_qualified_loaded_runtime(&loaded2, |assets| {
                Ok(capture_qualified_upstream_doctor_with_timeout(
                    assets,
                    &process_env,
                    roots.cert_file.as_os_str(),
                    Some(roots.cert_dir.as_os_str()),
                    &roots.resolver_path,
                    &roots.config_dir,
                    opts,
                    Duration::from_secs(5),
                    &mut None,
                ))
            })
            .unwrap();
            assert!(
                matches!(&res2, Err(QualifiedUpstreamDoctorProbeError::OutputTooLarge(bytes)) if bytes.len() == DOCTOR_OUTPUT_MAX_BYTES),
                "{res2:?}"
            );
        }
        remove_temp_root(root);
    }
}
