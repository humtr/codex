use crate::m2_generation_state;
use crate::maintenance;
use crate::remote_download;
use crate::{
    capture_termux_process_env, copy_local_directory_tree, ensure_openssl_available,
    ensure_real_directory_tree, ensure_regular_file, load_local_generation, openssl_sha256,
    prepare_core_config, probe_release_candidate, rename_noreplace, sync_directory,
    valid_sha256_hex, LoadedLocalGeneration, LocalCoreRoots, LocalProductError, ReleaseFileEntry,
    TermuxProcessEnvSnapshot, DEFAULT_UPSTREAM_LATEST_URL, REMOTE_RELEASE_URL_MAX_BYTES,
    REMOTE_TRANSFER_TIMEOUT_SECONDS, UPSTREAM_PACKAGE_ASSET, UPSTREAM_RELEASE_METADATA_BASE,
    UPSTREAM_RELEASE_METADATA_MAX_BYTES,
};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

const UPDATE_VERSION_ENV: &str = "CODEX_TERMUX_UPDATE_VERSION";
const WRAPPER_CREATION_METADATA: &str = "codex-minimal-wrapper-v1";
const UPDATE_USAGE: &str = "usage: codex update [--upstream] [--version <MAJOR.MINOR.PATCH>] | --path <FILE_OR_DIR> | --rollback | --help";
const UPDATE_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub(super) fn print_update_usage() {
    println!("{UPDATE_USAGE}");
    println!("  default / --upstream      download official Codex package, verify SHA-256, adapt locally for Termux, and activate");
    println!("  --version <M.m.p>         target a specific official release version (with default/--upstream)");
    println!("  --path <FILE_OR_DIR>      adapt an offline official package .tar.gz or stage a prepared generation directory");
    println!("  --rollback                atomically switch back to the previous generation");
    println!("  --help                    show this help");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateAction {
    Help,
    Upstream { version: Option<String> },
    Path(PathBuf),
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateCliError {
    Usage(&'static str),
    InvalidVersion,
}

impl std::fmt::Display for UpdateCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateCliError::Usage(msg) => write!(f, "{msg}\n{UPDATE_USAGE}"),
            UpdateCliError::InvalidVersion => {
                f.write_str("codex update: --version must be MAJOR.MINOR.PATCH")
            }
        }
    }
}

pub(super) fn valid_update_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return false;
    }
    let components = value.split('.');
    let mut count = 0;
    for component in components {
        count += 1;
        if component.is_empty()
            || !component.bytes().all(|byte| byte.is_ascii_digit())
            || (component.len() > 1 && component.starts_with('0'))
            || component.parse::<u64>().is_err()
        {
            return false;
        }
    }
    count == 3
}

pub(super) fn parse_update_args(args: &[OsString]) -> Result<UpdateAction, UpdateCliError> {
    if args.is_empty() {
        return Ok(UpdateAction::Upstream { version: None });
    }
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        return Ok(UpdateAction::Help);
    }
    if args.len() == 1 && args[0] == OsStr::new("--rollback") {
        return Ok(UpdateAction::Rollback);
    }

    let mut upstream_flag = false;
    let mut version: Option<String> = None;
    let mut local_path: Option<PathBuf> = None;
    let mut idx = 0;
    while idx < args.len() {
        let Some(flag) = args[idx].to_str() else {
            return Err(UpdateCliError::Usage(
                "codex update: unsupported non-UTF-8 argument",
            ));
        };
        if flag == "--upstream" {
            if upstream_flag {
                return Err(UpdateCliError::Usage(
                    "codex update: duplicate --upstream flag",
                ));
            }
            upstream_flag = true;
            idx += 1;
        } else if flag == "--version" {
            if version.is_some() {
                return Err(UpdateCliError::Usage(
                    "codex update: duplicate --version flag",
                ));
            }
            let Some(next) = args.get(idx + 1).and_then(|v| v.to_str()) else {
                return Err(UpdateCliError::Usage(
                    "codex update: --version requires a value",
                ));
            };
            if !valid_update_version(next) {
                return Err(UpdateCliError::InvalidVersion);
            }
            version = Some(next.to_owned());
            idx += 2;
        } else if let Some(v) = flag.strip_prefix("--version=") {
            if version.is_some() {
                return Err(UpdateCliError::Usage(
                    "codex update: duplicate --version flag",
                ));
            }
            if !valid_update_version(v) {
                return Err(UpdateCliError::InvalidVersion);
            }
            version = Some(v.to_owned());
            idx += 1;
        } else if flag == "--path" {
            if local_path.is_some() {
                return Err(UpdateCliError::Usage("codex update: duplicate --path flag"));
            }
            let Some(next) = args.get(idx + 1).filter(|v| !v.is_empty()) else {
                return Err(UpdateCliError::Usage(
                    "codex update: --path requires a non-empty file or directory",
                ));
            };
            local_path = Some(PathBuf::from(next));
            idx += 2;
        } else if let Some(p) = args[idx]
            .to_str()
            .and_then(|s| s.strip_prefix("--path="))
            .filter(|s| !s.is_empty())
        {
            if local_path.is_some() {
                return Err(UpdateCliError::Usage("codex update: duplicate --path flag"));
            }
            local_path = Some(PathBuf::from(p));
            idx += 1;
        } else {
            return Err(UpdateCliError::Usage("codex update: unsupported option"));
        }
    }

    if let Some(path) = local_path {
        if upstream_flag || version.is_some() {
            return Err(UpdateCliError::Usage(
                "codex update: --path cannot be combined with --upstream or --version",
            ));
        }
        return Ok(UpdateAction::Path(path));
    }

    Ok(UpdateAction::Upstream { version })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OfficialReleaseMetadata {
    pub(super) version: String,
    pub(super) package_sha256: String,
}

struct JsonCursor<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> JsonCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn error<T>(&self) -> Result<T, LocalProductError> {
        Err(LocalProductError::LocalUpdate(
            "upstream release metadata is malformed",
        ))
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.index += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), LocalProductError> {
        self.skip_whitespace();
        if self.bytes.get(self.index) != Some(&expected) {
            return self.error();
        }
        self.index += 1;
        Ok(())
    }

    fn string(&mut self) -> Result<String, LocalProductError> {
        self.skip_whitespace();
        if self.bytes.get(self.index) != Some(&b'"') {
            return self.error();
        }
        self.index += 1;
        let mut value = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.index) else {
                return self.error();
            };
            match byte {
                b'"' => {
                    self.index += 1;
                    return String::from_utf8(value).map_err(|_| {
                        LocalProductError::LocalUpdate("upstream release metadata is malformed")
                    });
                }
                b'\\' => {
                    self.index += 1;
                    let Some(&escape) = self.bytes.get(self.index) else {
                        return self.error();
                    };
                    self.index += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => value.push(escape),
                        b'b' => value.push(0x08),
                        b'f' => value.push(0x0c),
                        b'n' => value.push(b'\n'),
                        b'r' => value.push(b'\r'),
                        b't' => value.push(b'\t'),
                        b'u' => {
                            let Some(codepoint) = self.hex_codepoint() else {
                                return self.error();
                            };
                            let Some(character) = char::from_u32(codepoint) else {
                                return self.error();
                            };
                            let mut encoded = [0; 4];
                            value.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                        }
                        _ => return self.error(),
                    }
                }
                0..=0x1f => return self.error(),
                _ => {
                    let start = self.index;
                    while let Some(&next) = self.bytes.get(self.index) {
                        if matches!(next, b'"' | b'\\' | 0..=0x1f) {
                            break;
                        }
                        self.index += 1;
                    }
                    let chunk =
                        std::str::from_utf8(&self.bytes[start..self.index]).map_err(|_| {
                            LocalProductError::LocalUpdate("upstream release metadata is malformed")
                        })?;
                    value.extend_from_slice(chunk.as_bytes());
                }
            }
        }
    }

    fn hex_codepoint(&mut self) -> Option<u32> {
        let end = self.index.checked_add(4)?;
        let bytes = self.bytes.get(self.index..end)?;
        let mut value: u32 = 0;
        for byte in bytes {
            value = value.checked_mul(16)?;
            value += match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a' + 10),
                b'A'..=b'F' => u32::from(byte - b'A' + 10),
                _ => return None,
            };
        }
        self.index = end;
        Some(value)
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), LocalProductError> {
        self.skip_whitespace();
        let end = self.index.saturating_add(literal.len());
        if self.bytes.get(self.index..end) != Some(literal) {
            return self.error();
        }
        self.index = end;
        Ok(())
    }

    fn skip_number(&mut self) -> Result<(), LocalProductError> {
        self.skip_whitespace();
        let mut index = self.index;
        if self.bytes.get(index) == Some(&b'-') {
            index += 1;
        }
        match self.bytes.get(index) {
            Some(b'0') => {
                index += 1;
                if self
                    .bytes
                    .get(index)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    return self.error();
                }
            }
            Some(b'1'..=b'9') => {
                index += 1;
                while self
                    .bytes
                    .get(index)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    index += 1;
                }
            }
            _ => return self.error(),
        }
        if self.bytes.get(index) == Some(&b'.') {
            index += 1;
            let start = index;
            while self
                .bytes
                .get(index)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                index += 1;
            }
            if start == index {
                return self.error();
            }
        }
        if self
            .bytes
            .get(index)
            .is_some_and(|byte| matches!(*byte, b'e' | b'E'))
        {
            index += 1;
            if self
                .bytes
                .get(index)
                .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
            {
                index += 1;
            }
            let start = index;
            while self
                .bytes
                .get(index)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                index += 1;
            }
            if start == index {
                return self.error();
            }
        }
        if self
            .bytes
            .get(index)
            .is_some_and(|byte| !matches!(*byte, b' ' | b'\t' | b'\r' | b'\n' | b',' | b']' | b'}'))
        {
            return self.error();
        }
        self.index = index;
        Ok(())
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), LocalProductError> {
        if depth > 32 {
            return self.error();
        }
        self.skip_whitespace();
        match self.bytes.get(self.index) {
            Some(b'"') => self.string().map(|_| ()),
            Some(b'{') => {
                self.index += 1;
                self.skip_whitespace();
                if self.bytes.get(self.index) == Some(&b'}') {
                    self.index += 1;
                    return Ok(());
                }
                loop {
                    self.string()?;
                    self.expect(b':')?;
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.index) {
                        Some(b',') => self.index += 1,
                        Some(b'}') => {
                            self.index += 1;
                            return Ok(());
                        }
                        _ => return self.error(),
                    }
                }
            }
            Some(b'[') => {
                self.index += 1;
                self.skip_whitespace();
                if self.bytes.get(self.index) == Some(&b']') {
                    self.index += 1;
                    return Ok(());
                }
                loop {
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.index) {
                        Some(b',') => self.index += 1,
                        Some(b']') => {
                            self.index += 1;
                            return Ok(());
                        }
                        _ => return self.error(),
                    }
                }
            }
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.skip_number(),
            _ => self.error(),
        }
    }

    fn parse_assets(&mut self, wanted_asset: &str) -> Result<Option<String>, LocalProductError> {
        self.expect(b'[')?;
        self.skip_whitespace();
        if self.bytes.get(self.index) == Some(&b']') {
            self.index += 1;
            return Ok(None);
        }
        let mut wanted_digest = None;
        loop {
            self.expect(b'{')?;
            let mut name = None;
            let mut digest = None;
            self.skip_whitespace();
            if self.bytes.get(self.index) == Some(&b'}') {
                self.index += 1;
            } else {
                loop {
                    let key = self.string()?;
                    self.expect(b':')?;
                    match key.as_str() {
                        "name" => {
                            if name.is_some() {
                                return self.error();
                            }
                            name = Some(self.string()?);
                        }
                        "digest" => {
                            if digest.is_some() {
                                return self.error();
                            }
                            digest = Some(self.string()?);
                        }
                        _ => self.skip_value(0)?,
                    }
                    self.skip_whitespace();
                    match self.bytes.get(self.index) {
                        Some(b',') => self.index += 1,
                        Some(b'}') => {
                            self.index += 1;
                            break;
                        }
                        _ => return self.error(),
                    }
                }
            }
            if name.as_deref() == Some(wanted_asset) {
                if wanted_digest.is_some() {
                    return self.error();
                }
                let digest = digest.ok_or(LocalProductError::LocalUpdate(
                    "upstream release metadata is missing the package digest",
                ))?;
                wanted_digest = Some(digest);
            }
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    return Ok(wanted_digest);
                }
                _ => return self.error(),
            }
        }
    }
}

pub(super) fn parse_official_release_metadata(
    bytes: &[u8],
    requested_version: Option<&str>,
) -> Result<OfficialReleaseMetadata, LocalProductError> {
    let mut cursor = JsonCursor::new(bytes);
    cursor.expect(b'{')?;
    let mut tag_name = None;
    let mut package_digest = None;
    let mut assets_seen = false;
    cursor.skip_whitespace();
    if cursor.bytes.get(cursor.index) == Some(&b'}') {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata is incomplete",
        ));
    }
    loop {
        let key = cursor.string()?;
        cursor.expect(b':')?;
        match key.as_str() {
            "tag_name" => {
                if tag_name.is_some() {
                    return cursor.error();
                }
                tag_name = Some(cursor.string()?);
            }
            "assets" => {
                if assets_seen {
                    return cursor.error();
                }
                assets_seen = true;
                package_digest = cursor.parse_assets(UPSTREAM_PACKAGE_ASSET)?;
            }
            _ => cursor.skip_value(0)?,
        }
        cursor.skip_whitespace();
        match cursor.bytes.get(cursor.index) {
            Some(b',') => cursor.index += 1,
            Some(b'}') => {
                cursor.index += 1;
                break;
            }
            _ => return cursor.error(),
        }
    }
    cursor.skip_whitespace();
    if cursor.index != cursor.bytes.len() {
        return cursor.error();
    }
    let tag_name = tag_name.ok_or(LocalProductError::LocalUpdate(
        "upstream release metadata has no release tag",
    ))?;
    let version = tag_name
        .strip_prefix("rust-v")
        .ok_or(LocalProductError::LocalUpdate(
            "upstream release metadata has an unsupported tag",
        ))?;
    if !valid_update_version(version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata has a non-stable version",
        ));
    }
    if requested_version.is_some_and(|requested| requested != version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata version does not match the requested version",
        ));
    }
    let package_sha256 = package_digest
        .ok_or(LocalProductError::LocalUpdate(
            "upstream release metadata has no target package",
        ))?
        .strip_prefix("sha256:")
        .filter(|digest| valid_sha256_hex(digest))
        .ok_or(LocalProductError::LocalUpdate(
            "upstream release metadata package digest is invalid",
        ))?
        .to_owned();
    Ok(OfficialReleaseMetadata {
        version: version.to_owned(),
        package_sha256,
    })
}

fn validate_https_url(value: &str) -> Result<(), LocalProductError> {
    if value.is_empty()
        || value.len() > REMOTE_RELEASE_URL_MAX_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
        || value.contains(['?', '#', '\\'])
    {
        return Err(LocalProductError::LocalUpdate(
            "upstream URL is not canonical",
        ));
    }
    let Some(remainder) = value.strip_prefix("https://") else {
        return Err(LocalProductError::LocalUpdate(
            "upstream URL must use HTTPS",
        ));
    };
    let Some((authority, path)) = remainder.split_once('/') else {
        return Err(LocalProductError::LocalUpdate(
            "upstream URL has no resource path",
        ));
    };
    if authority.is_empty() || path.is_empty() || path.ends_with('/') {
        return Err(LocalProductError::LocalUpdate(
            "upstream URL is not canonical",
        ));
    }
    Ok(())
}

fn resolve_official_release_metadata(
    roots: &LocalCoreRoots,
    staging_root: &Path,
    cli_version: Option<&str>,
) -> Result<OfficialReleaseMetadata, LocalProductError> {
    let env_requested = std::env::var_os(UPDATE_VERSION_ENV);
    let env_version = if let Some(raw) = env_requested.as_ref() {
        let s = raw.to_str().ok_or(LocalProductError::LocalUpdate(
            "local update version is not UTF-8",
        ))?;
        if !valid_update_version(s) {
            return Err(LocalProductError::LocalUpdate(
                "local update version must be MAJOR.MINOR.PATCH",
            ));
        }
        Some(s)
    } else {
        None
    };
    let requested_version = cli_version.or(env_version);
    let url = requested_version
        .map(|version| format!("{UPSTREAM_RELEASE_METADATA_BASE}/{version}/release.json"))
        .unwrap_or_else(|| DEFAULT_UPSTREAM_LATEST_URL.to_owned());
    validate_https_url(&url)?;
    let path = staging_root.join("upstream-release.json");
    let output = crate::create_remote_output(&path)?;
    remote_download::fetch(
        roots,
        &url,
        &output,
        UPSTREAM_RELEASE_METADATA_MAX_BYTES as u64,
        remote_download::TransferTimeout::Fixed(REMOTE_TRANSFER_TIMEOUT_SECONDS),
        0,
        None,
    )?;
    let bytes = crate::read_bounded_regular_file(
        &path,
        UPSTREAM_RELEASE_METADATA_MAX_BYTES,
        "read upstream release metadata",
        LocalProductError::LocalUpdate("upstream release metadata exceeds its byte bound"),
        LocalProductError::LocalUpdate("upstream release metadata is not a regular file"),
    )?;
    parse_official_release_metadata(&bytes, requested_version)
}

fn update_progress_enabled(stdout_is_terminal: bool, stderr_is_terminal: bool) -> bool {
    stdout_is_terminal && stderr_is_terminal
}

enum UpdateTransientCommand {
    Update { frame_index: usize, message: String },
    Stop,
}

struct UpdateTransientWorker {
    sender: std::sync::mpsc::Sender<UpdateTransientCommand>,
    worker: Option<std::thread::JoinHandle<()>>,
}

struct UpdatePresentation {
    enabled: bool,
    transient_visible: bool,
    transient_worker: Option<UpdateTransientWorker>,
}

fn render_transient_frame(frame_index: usize, message: &str) {
    let frame = UPDATE_SPINNER_FRAMES[frame_index % UPDATE_SPINNER_FRAMES.len()];
    let mut stderr = std::io::stderr().lock();
    let _ = write!(stderr, "\r\x1b[2K{frame} {message}");
    let _ = stderr.flush();
}

impl UpdatePresentation {
    fn new() -> Self {
        use std::io::IsTerminal as _;
        Self {
            enabled: update_progress_enabled(
                std::io::stdout().is_terminal(),
                std::io::stderr().is_terminal(),
            ),
            transient_visible: false,
            transient_worker: None,
        }
    }

    fn transient(&mut self, message: &str) {
        if !self.enabled {
            return;
        }
        if let Some(worker) = self.transient_worker.as_ref() {
            if worker
                .sender
                .send(UpdateTransientCommand::Update {
                    frame_index: 0,
                    message: message.to_owned(),
                })
                .is_ok()
            {
                self.transient_visible = true;
                return;
            }
        }
        render_transient_frame(0, message);
        let (sender, receiver) = std::sync::mpsc::channel::<UpdateTransientCommand>();
        let initial_message = message.to_owned();
        let worker = std::thread::Builder::new()
            .name("codex-update-spinner".to_owned())
            .spawn(move || {
                let mut current_message = initial_message;
                let mut current_frame = 0usize;
                loop {
                    match receiver.recv_timeout(std::time::Duration::from_millis(80)) {
                        Ok(UpdateTransientCommand::Update {
                            frame_index,
                            message,
                        }) => {
                            current_message = message;
                            current_frame = frame_index % UPDATE_SPINNER_FRAMES.len();
                            render_transient_frame(current_frame, &current_message);
                        }
                        Ok(UpdateTransientCommand::Stop)
                        | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            current_frame = (current_frame + 1) % UPDATE_SPINNER_FRAMES.len();
                            render_transient_frame(current_frame, &current_message);
                        }
                    }
                }
            });
        if let Ok(worker) = worker {
            self.transient_worker = Some(UpdateTransientWorker {
                sender,
                worker: Some(worker),
            });
        }
        self.transient_visible = true;
    }

    fn clear_transient(&mut self) {
        if let Some(mut transient) = self.transient_worker.take() {
            let _ = transient.sender.send(UpdateTransientCommand::Stop);
            if let Some(worker) = transient.worker.take() {
                let _ = worker.join();
            }
        }
        if !self.transient_visible {
            return;
        }
        let mut stderr = std::io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[2K");
        let _ = stderr.flush();
        self.transient_visible = false;
    }

    fn finish_activated(
        &mut self,
        previous_version: Option<&str>,
        current_version: &str,
        generation_id: &str,
    ) {
        self.clear_transient();
        match previous_version {
            Some(prev) if prev != current_version => {
                println!("Updated Codex {prev} -> {current_version} (generation {generation_id}).");
            }
            _ => {
                println!("Codex {current_version} is now active (generation {generation_id}).");
            }
        }
    }

    fn finish_current(&mut self, current_version: &str) {
        self.clear_transient();
        println!("Codex {current_version} is already up to date.");
    }

    fn finish_rollback(
        &mut self,
        previous_version: &str,
        current_version: &str,
        generation_id: &str,
    ) {
        self.clear_transient();
        if previous_version == current_version {
            println!("Rolled back Codex {current_version} (generation {generation_id}).");
        } else {
            println!("Rolled back Codex {previous_version} -> {current_version} (generation {generation_id}).");
        }
    }

    fn fail(&mut self, error: &LocalProductError) {
        self.clear_transient();
        eprintln!("codex update: {error}");
    }
}

impl Drop for UpdatePresentation {
    fn drop(&mut self) {
        self.clear_transient();
    }
}

fn local_update_generation_id() -> String {
    let sequence = crate::LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("local-{timestamp}-{}-{sequence}", std::process::id())
}

fn running_wrapper_artifact() -> Result<PathBuf, LocalProductError> {
    #[cfg(test)]
    if let Some(input) = std::env::var_os("CODEX_SOURCE_ACCEPTANCE_CORE") {
        return std::fs::canonicalize(input).map_err(|source| LocalProductError::Io {
            operation: "resolve test wrapper artifact",
            source,
        });
    }
    let input = std::env::current_exe().map_err(|source| LocalProductError::Io {
        operation: "resolve running wrapper binary",
        source,
    })?;
    std::fs::canonicalize(input).map_err(|source| LocalProductError::Io {
        operation: "resolve running wrapper binary",
        source,
    })
}

fn create_staging_root(parent: &Path, prefix: &str) -> Result<PathBuf, LocalProductError> {
    use std::os::unix::fs::DirBuilderExt as _;
    ensure_real_directory_tree(
        parent,
        "prepare wrapper staging parent",
        "wrapper staging parent must be a real directory",
    )?;
    for _ in 0..32 {
        let sequence = crate::LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        let path = parent.join(format!("{prefix}{}-{sequence}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "create private update staging directory",
                    source,
                })
            }
        }
    }
    Err(LocalProductError::LocalUpdate(
        "could not allocate a unique update staging directory",
    ))
}

fn sync_tree_recursive(root: &Path) -> Result<(), LocalProductError> {
    let entries = std::fs::read_dir(root).map_err(|source| LocalProductError::Io {
        operation: "read generation directory for sync",
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "read generation entry for sync",
            source,
        })?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).map_err(|source| LocalProductError::Io {
            operation: "inspect generation entry for sync",
            source,
        })?;
        if meta.file_type().is_dir() {
            sync_tree_recursive(&path)?;
        } else if meta.file_type().is_file() {
            std::fs::File::open(&path)
                .and_then(|file| file.sync_all())
                .map_err(|source| LocalProductError::Io {
                    operation: "sync generation file",
                    source,
                })?;
        }
    }
    sync_directory(root)
}

#[derive(Debug)]
struct ActivatedOutcome {
    generation_id: String,
    previous_version: Option<String>,
    current_version: String,
}

fn ensure_writable_roots(roots: &LocalCoreRoots) -> Result<(), LocalProductError> {
    ensure_real_directory_tree(
        &roots.state_root,
        "prepare wrapper state directory",
        "wrapper state directory must be a real directory",
    )?;
    ensure_real_directory_tree(
        &roots.config_dir,
        "prepare wrapper config directory",
        "wrapper config directory must be a real directory",
    )?;
    ensure_real_directory_tree(
        &roots.generation_root,
        "prepare wrapper generation root",
        "wrapper generation root must be a real directory",
    )?;
    Ok(())
}

fn current_loaded_generation(
    roots: &LocalCoreRoots,
) -> Result<
    (
        Option<m2_generation_state::GenerationPointerState>,
        Option<LoadedLocalGeneration>,
    ),
    LocalProductError,
> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?;
    let loaded = match before.as_ref() {
        Some(state) => Some(load_local_generation(
            &roots.generation_root.join(&state.current),
        )?),
        None => None,
    };
    Ok((before, loaded))
}

fn install_built_generation_dir(
    built_dir: &Path,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    before: Option<&m2_generation_state::GenerationPointerState>,
    before_loaded: Option<&LoadedLocalGeneration>,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    let loaded = load_local_generation(built_dir)?;
    let generation_id = loaded.generation_id.clone();
    let version = loaded.manifest.upstream_package_version.clone();

    ensure_writable_roots(roots)?;
    let final_dir = roots.generation_root.join(&generation_id);
    match std::fs::symlink_metadata(&final_dir) {
        Ok(_) => return Err(LocalProductError::GenerationCollision),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect destination generation path",
                source,
            })
        }
    }

    let candidate_parent = roots.generation_root.clone();
    let candidate_seq = crate::LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let candidate_dir =
        candidate_parent.join(format!(".candidate-{}-{candidate_seq}", std::process::id()));

    let result = (|| -> Result<ActivatedOutcome, LocalProductError> {
        copy_local_directory_tree(built_dir, &candidate_dir)?;
        sync_tree_recursive(&candidate_dir)?;

        let staged_loaded = load_local_generation(&candidate_dir)?;
        presentation.transient(&format!("Checking Codex {version} candidate runtime..."));
        prepare_core_config(&roots.config_dir).map_err(|source| LocalProductError::Io {
            operation: "prepare wrapper config.toml",
            source,
        })?;
        probe_release_candidate(&staged_loaded, roots, process_env)?;

        presentation.transient(&format!("Activating Codex {version}..."));
        rename_noreplace(&candidate_dir, &final_dir).map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                LocalProductError::GenerationCollision
            } else {
                LocalProductError::Io {
                    operation: "commit generation directory",
                    source,
                }
            }
        })?;
        sync_directory(&roots.generation_root)?;

        let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
            .map_err(LocalProductError::StateFormat)?;
        m2_generation_state::prepare_core_state_paths(&state_paths)
            .map_err(LocalProductError::State)?;
        let lock = m2_generation_state::acquire_activation_lock(&state_paths)
            .map_err(LocalProductError::State)?;
        let authoritative = m2_generation_state::read_pointer_state(&state_paths)
            .map_err(LocalProductError::State)?;
        if authoritative.as_ref() != before {
            return Err(LocalProductError::State(
                m2_generation_state::ActivationTransactionError::StaleAuthoritativeState,
            ));
        }
        let after = match authoritative.as_ref() {
            Some(prev) => m2_generation_state::plan_activation_pointer_state(prev, &generation_id),
            None => m2_generation_state::plan_initial_pointer_state(&generation_id),
        }
        .map_err(LocalProductError::StateFormat)?;
        let mut io = m2_generation_state::FsActivationIo;
        m2_generation_state::activate_pointer_state_locked(
            &state_paths,
            authoritative.as_ref(),
            &after,
            &mut io,
            &lock,
        )
        .map_err(LocalProductError::State)?;

        Ok(ActivatedOutcome {
            generation_id,
            previous_version: before_loaded.map(|l| l.manifest.upstream_package_version.clone()),
            current_version: version,
        })
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&candidate_dir);
    }
    result
}

fn gzip_tool_path(roots: &LocalCoreRoots) -> Result<PathBuf, LocalProductError> {
    roots
        .curl
        .parent()
        .ok_or(LocalProductError::LocalUpdate(
            "Termux tool directory is unavailable",
        ))
        .map(|bin| bin.join("gzip"))
}

struct ArchiveBuildInputs<'a> {
    archive_path: &'a Path,
    version: &'a str,
    archive_sha256: &'a str,
    staging_root: &'a Path,
}

fn build_from_archive_and_activate(
    inputs: ArchiveBuildInputs<'_>,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    before: Option<&m2_generation_state::GenerationPointerState>,
    before_loaded: Option<&LoadedLocalGeneration>,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    if !codex_release_builder::binary_adaptation_available(inputs.version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream Codex version is outside adaptive binary patch policy support",
        ));
    }
    let core = running_wrapper_artifact()?;
    let generation_id = local_update_generation_id();
    let built_dir = inputs.staging_root.join("built-generation");
    let gzip = gzip_tool_path(roots)?;
    presentation.transient(&format!(
        "Building Termux generation for Codex {}...",
        inputs.version
    ));
    codex_release_builder::build_generation(&codex_release_builder::GenerationRequest {
        version: inputs.version.to_owned(),
        archive: inputs.archive_path.to_path_buf(),
        archive_sha256: inputs.archive_sha256.to_owned(),
        generation_id,
        core,
        legacy_activation_doctor_unsupported: false,
        creation_metadata: WRAPPER_CREATION_METADATA.to_owned(),
        gzip,
        openssl: roots.openssl.clone(),
        output: built_dir.clone(),
    })
    .map_err(LocalProductError::Builder)?;
    install_built_generation_dir(
        &built_dir,
        roots,
        process_env,
        before,
        before_loaded,
        presentation,
    )
}

enum UpstreamOutcome {
    AlreadyCurrent(String),
    Activated(ActivatedOutcome),
}

fn run_upstream_update(
    requested_version: Option<&str>,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> Result<UpstreamOutcome, LocalProductError> {
    ensure_openssl_available(&roots.openssl)?;
    ensure_writable_roots(roots)?;
    let (before, before_loaded) = current_loaded_generation(roots)?;
    let staging_root = create_staging_root(&roots.state_root, ".local-update-")?;
    let result = (|| -> Result<UpstreamOutcome, LocalProductError> {
        presentation.transient("Checking official Codex release...");
        let metadata = resolve_official_release_metadata(roots, &staging_root, requested_version)?;

        if let Some(current) = before_loaded.as_ref() {
            let core = running_wrapper_artifact()?;
            let core_sha256 = openssl_sha256(&roots.openssl, &core)?;
            if current.manifest.upstream_package_version == metadata.version
                && current.manifest.source_artifact_digest == metadata.package_sha256
                && current.manifest.wrapper_artifact_digest == core_sha256
            {
                return Ok(UpstreamOutcome::AlreadyCurrent(metadata.version));
            }
        }

        let archive_path = staging_root.join(UPSTREAM_PACKAGE_ASSET);
        let download_url = format!(
            "{UPSTREAM_RELEASE_METADATA_BASE}/{}/{UPSTREAM_PACKAGE_ASSET}",
            metadata.version
        );
        validate_https_url(&download_url)?;
        presentation.transient(&format!(
            "Downloading official Codex {} package...",
            metadata.version
        ));
        let archive_entry = ReleaseFileEntry {
            relative_path: UPSTREAM_PACKAGE_ASSET.to_owned(),
            sha256: metadata.package_sha256.clone(),
            mode: 0o600,
        };
        let mut last_bytes = 0u64;
        let mut progress = |bytes: u64| {
            if bytes != last_bytes {
                last_bytes = bytes;
            }
        };
        remote_download::payload(
            roots,
            &download_url,
            &archive_path,
            &archive_entry,
            remote_download::PayloadOptions {
                limit: codex_release_builder::ARCHIVE_MAX_BYTES,
                expected_size: None,
                timeout: codex_release_builder::RELEASE_TRANSFER_TIMEOUT_SECONDS,
                progress: Some(&mut progress),
            },
        )?;
        let activated = build_from_archive_and_activate(
            ArchiveBuildInputs {
                archive_path: &archive_path,
                version: &metadata.version,
                archive_sha256: &metadata.package_sha256,
                staging_root: &staging_root,
            },
            roots,
            process_env,
            before.as_ref(),
            before_loaded.as_ref(),
            presentation,
        )?;
        Ok(UpstreamOutcome::Activated(activated))
    })();
    let _ = std::fs::remove_dir_all(&staging_root);
    result
}

fn resolve_cli_path(input: &Path) -> Result<PathBuf, LocalProductError> {
    let abs = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| LocalProductError::Io {
                operation: "resolve current directory",
                source,
            })?
            .join(input)
    };
    Ok(abs)
}

fn run_path_update(
    input_path: &Path,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    ensure_openssl_available(&roots.openssl)?;
    ensure_writable_roots(roots)?;
    let source_path = resolve_cli_path(input_path)?;
    let source_meta =
        std::fs::symlink_metadata(&source_path).map_err(|source| LocalProductError::Io {
            operation: "inspect local package path",
            source,
        })?;
    let (before, before_loaded) = current_loaded_generation(roots)?;

    if source_meta.file_type().is_dir() {
        presentation.transient("Inspecting local generation directory...");
        let descriptor = source_path.join("generation.meta");
        if !descriptor.exists() {
            return Err(LocalProductError::LocalUpdate(
                "local directory must be a prepared generation directory containing generation.meta (or pass a .tar.gz upstream package archive)",
            ));
        }
        return install_built_generation_dir(
            &source_path,
            roots,
            process_env,
            before.as_ref(),
            before_loaded.as_ref(),
            presentation,
        );
    }

    if !source_meta.file_type().is_file() {
        return Err(LocalProductError::UnsafeSource(
            "local package path must be a regular non-symlink file or generation directory",
        ));
    }
    if source_meta.len() == 0 || source_meta.len() > codex_release_builder::ARCHIVE_MAX_BYTES {
        return Err(LocalProductError::LocalUpdate(
            "local package archive exceeds its byte bound",
        ));
    }

    presentation.transient("Inspecting local Codex archive...");
    let gzip = gzip_tool_path(roots)?;
    let version = codex_release_builder::inspect_package_archive_version(&source_path, &gzip)
        .map_err(LocalProductError::Builder)?;
    if !codex_release_builder::binary_adaptation_available(&version) {
        return Err(LocalProductError::LocalUpdate(
            "local package archive version is outside adaptive binary patch policy support",
        ));
    }

    let staging_root = create_staging_root(&roots.state_root, ".local-update-")?;
    let result = (|| {
        let staged_archive = staging_root.join(UPSTREAM_PACKAGE_ASSET);
        std::fs::copy(&source_path, &staged_archive).map_err(|source| LocalProductError::Io {
            operation: "stage local package archive",
            source,
        })?;
        let sha256 = openssl_sha256(&roots.openssl, &staged_archive)?;
        build_from_archive_and_activate(
            ArchiveBuildInputs {
                archive_path: &staged_archive,
                version: &version,
                archive_sha256: &sha256,
                staging_root: &staging_root,
            },
            roots,
            process_env,
            before.as_ref(),
            before_loaded.as_ref(),
            presentation,
        )
    })();
    let _ = std::fs::remove_dir_all(&staging_root);
    result
}

fn run_rollback(roots: &LocalCoreRoots) -> Result<ActivatedOutcome, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    let after = m2_generation_state::plan_rollback_pointer_state(&before)
        .map_err(LocalProductError::StateFormat)?;

    let current_loaded = load_local_generation(&roots.generation_root.join(&before.current))?;
    let target_loaded = load_local_generation(&roots.generation_root.join(&after.current))?;
    if target_loaded.generation_id != after.current {
        return Err(LocalProductError::Descriptor(
            "rollback generation descriptor id does not match previous",
        ));
    }
    ensure_regular_file(
        &target_loaded.runtime_path,
        "inspect rollback target runtime",
        "rollback target runtime must be a regular file",
    )?;

    let lock = m2_generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let authoritative =
        m2_generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
    if authoritative.as_ref() != Some(&before) {
        return Err(LocalProductError::State(
            m2_generation_state::ActivationTransactionError::StaleAuthoritativeState,
        ));
    }
    let mut io = m2_generation_state::FsActivationIo;
    m2_generation_state::activate_pointer_state_locked(
        &state_paths,
        Some(&before),
        &after,
        &mut io,
        &lock,
    )
    .map_err(LocalProductError::State)?;
    Ok(ActivatedOutcome {
        generation_id: after.current,
        previous_version: Some(current_loaded.manifest.upstream_package_version),
        current_version: target_loaded.manifest.upstream_package_version,
    })
}

pub(super) fn run_update_command(args: Vec<OsString>) -> i32 {
    let action = match parse_update_args(&args) {
        Ok(action) => action,
        Err(err) => {
            eprintln!("{err}");
            return 2;
        }
    };
    if matches!(action, UpdateAction::Help) {
        print_update_usage();
        return 0;
    }

    let mut presentation = UpdatePresentation::new();
    let roots = match LocalCoreRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            presentation.fail(&err);
            return 1;
        }
    };
    let _lease = match maintenance::UpdateLease::acquire(&roots) {
        Ok(lease) => lease,
        Err(err) => {
            presentation.fail(&err);
            return 1;
        }
    };
    let process_env = capture_termux_process_env();

    match action {
        UpdateAction::Help => 0,
        UpdateAction::Rollback => match run_rollback(&roots) {
            Ok(outcome) => {
                presentation.finish_rollback(
                    outcome
                        .previous_version
                        .as_deref()
                        .unwrap_or(&outcome.current_version),
                    &outcome.current_version,
                    &outcome.generation_id,
                );
                0
            }
            Err(err) => {
                presentation.fail(&err);
                1
            }
        },
        UpdateAction::Path(path) => {
            match run_path_update(&path, &roots, &process_env, &mut presentation) {
                Ok(outcome) => {
                    presentation.finish_activated(
                        outcome.previous_version.as_deref(),
                        &outcome.current_version,
                        &outcome.generation_id,
                    );
                    0
                }
                Err(err) => {
                    presentation.fail(&err);
                    1
                }
            }
        }
        UpdateAction::Upstream { version } => {
            match run_upstream_update(version.as_deref(), &roots, &process_env, &mut presentation) {
                Ok(UpstreamOutcome::AlreadyCurrent(version)) => {
                    presentation.finish_current(&version);
                    0
                }
                Ok(UpstreamOutcome::Activated(outcome)) => {
                    presentation.finish_activated(
                        outcome.previous_version.as_deref(),
                        &outcome.current_version,
                        &outcome.generation_id,
                    );
                    0
                }
                Err(err) => {
                    presentation.fail(&err);
                    1
                }
            }
        }
    }
}
