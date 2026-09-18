#![cfg(unix)]

use std::collections::BTreeSet;
use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const ARCHIVE_MAX_BYTES: u64 = 256 * 1024 * 1024;
const ENTRY_MAX_BYTES: u64 = 384 * 1024 * 1024;
const PAYLOAD_MAX_BYTES: u64 = 512 * 1024 * 1024;
const CORE_ARTIFACT_MAX_BYTES: u64 = 64 * 1024 * 1024;
const MANAGER_ARTIFACT_MAX_BYTES: u64 = 64 * 1024 * 1024;
const MANAGER_ARTIFACT_PROBE_OUTPUT_MAX_BYTES: usize = 512;
const MANAGER_ARTIFACT_PROBE_TIMEOUT_SECONDS: u64 = 5;
const MANAGER_ARTIFACT_PROBE_ARGUMENT: &str = "--artifact-probe";
const MANAGER_ARTIFACT_PROBE_ENV: &str = "CODEX_MANAGER_ARTIFACT_PROBE";
const MANAGER_ARTIFACT_PROBE_OUTPUT: &[u8] =
    b"codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n";
const MANAGER_ARTIFACT_DEFERRED_MARKER: &str = ".manager-probe-deferred";
const MANAGER_ARTIFACT_DEFERRED_MARKER_BYTES: &[u8] = b"codex-manager-probe-deferred-v1\n";
const PATH_MAX_BYTES: usize = 256;
const LOGICAL_ENTRY_MAX: usize = 256;
const PAX_PAYLOAD_MAX_BYTES: u64 = 512;
const PACKAGE_JSON_MAX_BYTES: u64 = 64 * 1024;
const JSON_DEPTH_MAX: usize = 16;
const GENERATION_ID_MAX_BYTES: usize = 512;
const TEXT_VALUE_MAX_BYTES: usize = 512;
const TAR_BLOCK_BYTES: usize = 512;
const GENERATION_FORMAT: &str = "codex-local-generation-v2";
const RELEASE_FORMAT_V3: &str = "codex-release-v3";
const RELEASE_FORMAT_V4: &str = "codex-release-v4";
const CORE_API_IDENTITY: &str = "core-api-v1";
const PERSISTENT_SCHEMA_IDENTITY: &str = "schema-v1";
const PACKAGE_IDENTITY: &str = "openai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz";
const OFFICIAL_RELEASE_BASE: &str = "https://releases.openai.com/codex/releases";
const UPSTREAM_ARCHIVE_NAME: &str = "codex-package-aarch64-unknown-linux-musl.tar.gz";
const RELEASE_CONNECT_TIMEOUT_SECONDS: &str = "15";
const RELEASE_TRANSFER_TIMEOUT_SECONDS: &str = "300";
const PATCH_POLICY_ID: &str = "termux-fd-remap-v1";
const RELEASE_INDEX_FORMAT: &str = "codex-update-index-v1";
const RELEASE_CHANNEL: &str = "stable";
const RELEASE_URL_MAX_BYTES: usize = 4096;
const RELEASE_MANIFEST_MAX_BYTES: u64 = 128 * 1024;
const RELEASE_SIGNATURE_BYTES: u64 = 64;
const RELEASE_PRIVATE_KEY_MAX_BYTES: u64 = 16 * 1024;
const RELEASE_FILE_MAX_BYTES: u64 = 512 * 1024 * 1024;
const RELEASE_TOTAL_FILE_MAX_BYTES: u64 = 1024 * 1024 * 1024;
const GENERATION_DESCRIPTOR_MAX_BYTES: u64 = 64 * 1024;
const BROWSER_HELPER_MAX_BYTES: u64 = 4096;
// webbrowser 1.2.2 treats a command whose basename is `curl` as a text browser,
// so it waits for the helper exit status before trying the next BROWSER entry.
// Keep the signed helper basename `curl` unless that dependency contract is requalified.
const TERMUX_BROWSER_OPEN_HELPER_IDENTITY: &str = "termux-browser-open-v1";
const TERMUX_BROWSER_MANUAL_HELPER_IDENTITY: &str = "termux-browser-manual-v1";
const R10_BROWSER_HELPER_BRIDGE_METADATA: &str = "r10-browser-helper-bridge-v1";
const TERMUX_BROWSER_OPEN_HELPER: &[u8] = br##"#!/system/bin/sh
if [ "$#" -ne 1 ]; then
    exit 64
fi
url=$1
case "$url" in
    http://*|https://*) ;;
    *) exit 64 ;;
esac
case "$url" in
    *[[:space:]]*|*[[:cntrl:]]*|*\\*) exit 64 ;;
esac
authority=${url#*://}
case "$authority" in
    ""|/*|\?*|\#*) exit 64 ;;
esac
hostport=${authority%%/*}
hostport=${hostport%%\?*}
hostport=${hostport%%\#*}
host=${hostport##*@}
case "$host" in
    ""|:*) exit 64 ;;
esac
case "${PREFIX-}" in
    /*) ;;
    *) exit 64 ;;
esac
opener=${CODEX_TERMUX_URL_OPENER-}
if [ "$opener" != "$PREFIX/bin/termux-open-url" ]; then
    exit 64
fi
if [ ! -f "$opener" ] || [ -L "$opener" ] || [ ! -x "$opener" ]; then
    exit 127
fi
exec "$opener" "$url"
"##;
const TERMUX_BROWSER_MANUAL_HELPER: &[u8] = br##"#!/system/bin/sh
if [ "$#" -eq 1 ]; then
    url=$1
    valid=1
    case "$url" in
        http://*|https://*) ;;
        *) valid=0 ;;
    esac
    case "$url" in
        *[[:space:]]*|*[[:cntrl:]]*|*\\*) valid=0 ;;
    esac
    authority=${url#*://}
    case "$authority" in
        ""|/*|\?*|\#*) valid=0 ;;
    esac
    hostport=${authority%%/*}
    hostport=${hostport%%\?*}
    hostport=${hostport%%\#*}
    host=${hostport##*@}
    case "$host" in
        ""|:*) valid=0 ;;
    esac
    if [ "$valid" -eq 1 ]; then
        printf 'Browser launch unavailable; open this URL manually:\n%s\n' "$url" >&2
        exit 0
    fi
fi
printf '%s\n' 'Browser launch blocked: only a single well-formed HTTP/HTTPS URL is permitted.' >&2
exit 0
"##;
const ANDROID_AARCH64_INTERPRETER: &[u8] = b"/system/bin/linker64\0";

const PATCHES: [(&[u8], &[u8], usize); 4] = [
    (b"/etc/resolv.conf", b"/proc/self/fd/33", 2),
    (b"/etc/codex/config.toml", b"/dev/fd/34/config.toml", 1),
    (
        b"/etc/codex/requirements.toml",
        b"/dev/fd/34/requirements.toml",
        1,
    ),
    (
        b"/etc/codex/managed_config.toml",
        b"/dev/fd/34/managed_config.toml",
        1,
    ),
];

const REQUIRED_ARCHIVE_FILES: [&str; 3] = [
    "bin/codex",
    "bin/codex-code-mode-host",
    "codex-package.json",
];

const USAGE: &str = concat!(
    "usage: codex-release-builder fetch --version <MAJOR.MINOR.PATCH> ",
    "--curl <ABSOLUTE_EXECUTABLE> --openssl <ABSOLUTE_EXECUTABLE> ",
    "--output <ABSENT_ABSOLUTE_FILE>\n",
    "       codex-release-builder build --version <MAJOR.MINOR.PATCH> ",
    "--archive <ABSOLUTE_FILE> --archive-sha256 <LOWERCASE_SHA256> ",
    "--generation-id <ID> --core <ABSOLUTE_FILE> [--manager <ABSOLUTE_FILE>] ",
    "[--defer-manager-probe] [--legacy-activation-doctor-unsupported] ",
    "--creation-metadata <VALUE> ",
    "--gzip <ABSOLUTE_EXECUTABLE> --openssl <ABSOLUTE_EXECUTABLE> ",
    "--output <ABSENT_ABSOLUTE_DIRECTORY>\n",
    "       codex-release-builder publish --generation <ABSOLUTE_DIRECTORY> ",
    "--release-sequence <POSITIVE_DECIMAL> --release-base <HTTPS_BASE_URL> ",
    "--private-key <ABSOLUTE_FILE> --openssl <ABSOLUTE_EXECUTABLE> ",
    "--output <ABSENT_ABSOLUTE_DIRECTORY>"
);

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
enum BuilderError {
    Usage,
    Invalid(&'static str),
    Io {
        operation: &'static str,
        source: io::Error,
    },
    Tool(&'static str),
    Archive(&'static str),
    ArchiveDigestMismatch,
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuilderError::Usage => f.write_str(USAGE),
            BuilderError::Invalid(message)
            | BuilderError::Tool(message)
            | BuilderError::Archive(message) => f.write_str(message),
            BuilderError::Io { operation, source } => write!(f, "{operation}: {source}"),
            BuilderError::ArchiveDigestMismatch => {
                f.write_str("archive SHA-256 does not match the pinned digest")
            }
        }
    }
}

impl std::error::Error for BuilderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuilderError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(operation: &'static str, source: io::Error) -> BuilderError {
    BuilderError::Io { operation, source }
}

#[derive(Debug)]
struct BuildRequest {
    version: String,
    archive: PathBuf,
    archive_sha256: String,
    generation_id: String,
    core: PathBuf,
    manager: Option<PathBuf>,
    defer_manager_probe: bool,
    legacy_activation_doctor_unsupported: bool,
    creation_metadata: String,
    gzip: PathBuf,
    openssl: PathBuf,
    output: PathBuf,
}

#[derive(Debug)]
struct FetchRequest {
    version: String,
    curl: PathBuf,
    openssl: PathBuf,
    output: PathBuf,
}

#[derive(Debug, Clone)]
struct PublishRequest {
    generation: PathBuf,
    release_sequence: String,
    release_base: String,
    private_key: PathBuf,
    openssl: PathBuf,
    output: PathBuf,
}

#[derive(Default)]
struct RequestFields {
    version: Option<String>,
    archive: Option<PathBuf>,
    archive_sha256: Option<String>,
    generation_id: Option<String>,
    core: Option<PathBuf>,
    manager: Option<PathBuf>,
    defer_manager_probe: bool,
    legacy_activation_doctor_unsupported: bool,
    creation_metadata: Option<String>,
    gzip: Option<PathBuf>,
    openssl: Option<PathBuf>,
    output: Option<PathBuf>,
}

#[derive(Default)]
struct FetchFields {
    version: Option<String>,
    curl: Option<PathBuf>,
    openssl: Option<PathBuf>,
    output: Option<PathBuf>,
}

#[derive(Default)]
struct PublishFields {
    generation: Option<PathBuf>,
    release_sequence: Option<String>,
    release_base: Option<String>,
    private_key: Option<PathBuf>,
    openssl: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn text_value(value: OsString) -> Result<String, BuilderError> {
    value.into_string().map_err(|_| BuilderError::Usage)
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), BuilderError> {
    if slot.replace(value).is_some() {
        Err(BuilderError::Usage)
    } else {
        Ok(())
    }
}

fn parse_request<I, S>(args: I) -> Result<BuildRequest, BuilderError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    if args.next().as_deref() != Some(OsStr::new("build")) {
        return Err(BuilderError::Usage);
    }
    let mut fields = RequestFields::default();
    while let Some(flag) = args.next() {
        if flag == OsStr::new("--defer-manager-probe") {
            if fields.defer_manager_probe {
                return Err(BuilderError::Usage);
            }
            fields.defer_manager_probe = true;
            continue;
        }
        if flag == OsStr::new("--legacy-activation-doctor-unsupported") {
            if fields.legacy_activation_doctor_unsupported {
                return Err(BuilderError::Usage);
            }
            fields.legacy_activation_doctor_unsupported = true;
            continue;
        }
        let value = args.next().ok_or(BuilderError::Usage)?;
        match flag.to_str() {
            Some("--version") => set_once(&mut fields.version, text_value(value)?)?,
            Some("--archive") => set_once(&mut fields.archive, PathBuf::from(value))?,
            Some("--archive-sha256") => set_once(&mut fields.archive_sha256, text_value(value)?)?,
            Some("--generation-id") => set_once(&mut fields.generation_id, text_value(value)?)?,
            Some("--core") => set_once(&mut fields.core, PathBuf::from(value))?,
            Some("--manager") => set_once(&mut fields.manager, PathBuf::from(value))?,
            Some("--creation-metadata") => {
                set_once(&mut fields.creation_metadata, text_value(value)?)?
            }
            Some("--gzip") => set_once(&mut fields.gzip, PathBuf::from(value))?,
            Some("--openssl") => set_once(&mut fields.openssl, PathBuf::from(value))?,
            Some("--output") => set_once(&mut fields.output, PathBuf::from(value))?,
            _ => return Err(BuilderError::Usage),
        }
    }
    Ok(BuildRequest {
        version: fields.version.ok_or(BuilderError::Usage)?,
        archive: fields.archive.ok_or(BuilderError::Usage)?,
        archive_sha256: fields.archive_sha256.ok_or(BuilderError::Usage)?,
        generation_id: fields.generation_id.ok_or(BuilderError::Usage)?,
        core: fields.core.ok_or(BuilderError::Usage)?,
        manager: fields.manager,
        defer_manager_probe: fields.defer_manager_probe,
        legacy_activation_doctor_unsupported: fields.legacy_activation_doctor_unsupported,
        creation_metadata: fields.creation_metadata.ok_or(BuilderError::Usage)?,
        gzip: fields.gzip.ok_or(BuilderError::Usage)?,
        openssl: fields.openssl.ok_or(BuilderError::Usage)?,
        output: fields.output.ok_or(BuilderError::Usage)?,
    })
}

fn parse_fetch_request<I, S>(args: I) -> Result<FetchRequest, BuilderError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    if args.next().as_deref() != Some(OsStr::new("fetch")) {
        return Err(BuilderError::Usage);
    }
    let mut fields = FetchFields::default();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(BuilderError::Usage)?;
        match flag.to_str() {
            Some("--version") => set_once(&mut fields.version, text_value(value)?)?,
            Some("--curl") => set_once(&mut fields.curl, PathBuf::from(value))?,
            Some("--openssl") => set_once(&mut fields.openssl, PathBuf::from(value))?,
            Some("--output") => set_once(&mut fields.output, PathBuf::from(value))?,
            _ => return Err(BuilderError::Usage),
        }
    }
    Ok(FetchRequest {
        version: fields.version.ok_or(BuilderError::Usage)?,
        curl: fields.curl.ok_or(BuilderError::Usage)?,
        openssl: fields.openssl.ok_or(BuilderError::Usage)?,
        output: fields.output.ok_or(BuilderError::Usage)?,
    })
}

fn parse_publish_request<I, S>(args: I) -> Result<PublishRequest, BuilderError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    if args.next().as_deref() != Some(OsStr::new("publish")) {
        return Err(BuilderError::Usage);
    }
    let mut fields = PublishFields::default();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(BuilderError::Usage)?;
        match flag.to_str() {
            Some("--generation") => set_once(&mut fields.generation, PathBuf::from(value))?,
            Some("--release-sequence") => {
                set_once(&mut fields.release_sequence, text_value(value)?)?
            }
            Some("--release-base") => set_once(&mut fields.release_base, text_value(value)?)?,
            Some("--private-key") => set_once(&mut fields.private_key, PathBuf::from(value))?,
            Some("--openssl") => set_once(&mut fields.openssl, PathBuf::from(value))?,
            Some("--output") => set_once(&mut fields.output, PathBuf::from(value))?,
            _ => return Err(BuilderError::Usage),
        }
    }
    Ok(PublishRequest {
        generation: fields.generation.ok_or(BuilderError::Usage)?,
        release_sequence: fields.release_sequence.ok_or(BuilderError::Usage)?,
        release_base: fields.release_base.ok_or(BuilderError::Usage)?,
        private_key: fields.private_key.ok_or(BuilderError::Usage)?,
        openssl: fields.openssl.ok_or(BuilderError::Usage)?,
        output: fields.output.ok_or(BuilderError::Usage)?,
    })
}

fn valid_stable_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return false;
    }
    let mut components = value.split('.');
    let mut count = 0;
    for component in components.by_ref() {
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

fn official_archive_url(version: &str) -> String {
    format!("{OFFICIAL_RELEASE_BASE}/{version}/{UPSTREAM_ARCHIVE_NAME}")
}

fn valid_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_line_value(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn valid_generation_id(value: &str) -> bool {
    valid_line_value(value, GENERATION_ID_MAX_BYTES)
        && value != "."
        && value != ".."
        && !value.as_bytes().contains(&b'/')
}

fn valid_positive_decimal(value: &str) -> bool {
    let Some(first) = value.as_bytes().first() else {
        return false;
    };
    matches!(*first, b'1'..=b'9') && value.as_bytes()[1..].iter().all(u8::is_ascii_digit)
}

fn valid_publish_generation_id(value: &str) -> bool {
    valid_generation_id(value)
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

fn publish_remote_port(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes().iter().all(u8::is_ascii_digit)
        && value.parse::<u16>().is_ok_and(|port| port != 0)
}

fn publish_remote_authority(value: &str) -> bool {
    if value.is_empty() || value.contains('@') {
        return false;
    }
    if let Some(ipv6) = value.strip_prefix('[') {
        let Some((address, suffix)) = ipv6.split_once(']') else {
            return false;
        };
        if address.parse::<std::net::Ipv6Addr>().is_err() {
            return false;
        }
        return suffix.is_empty() || suffix.strip_prefix(':').is_some_and(publish_remote_port);
    }

    let mut authority = value.split(':');
    let host = authority.next().unwrap_or_default();
    let port = authority.next();
    if authority.next().is_some() || port.is_some_and(|port| !publish_remote_port(port)) {
        return false;
    }
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn publish_remote_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn publish_uppercase_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn valid_publish_remote_path_component(value: &str) -> bool {
    if value.is_empty() || matches!(value, "." | "..") {
        return false;
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if publish_remote_unreserved(bytes[index]) {
            index += 1;
            continue;
        }
        if bytes[index] != b'%' || index + 2 >= bytes.len() {
            return false;
        }
        let Some(high) = publish_uppercase_hex_value(bytes[index + 1]) else {
            return false;
        };
        let Some(low) = publish_uppercase_hex_value(bytes[index + 2]) else {
            return false;
        };
        let decoded = (high << 4) | low;
        if publish_remote_unreserved(decoded)
            || decoded.is_ascii_control()
            || matches!(decoded, b'/' | b'\\')
        {
            return false;
        }
        index += 3;
    }
    true
}

fn publish_percent_encode_path(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if publish_remote_unreserved(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

fn valid_publish_release_base(value: &str, generation_id: &str) -> bool {
    if value.is_empty()
        || value.len() > RELEASE_URL_MAX_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        || value.contains(['?', '#', '\\'])
        || !value.ends_with('/')
    {
        return false;
    }
    let Some(remainder) = value.strip_prefix("https://") else {
        return false;
    };
    let Some((authority, path)) = remainder.split_once('/') else {
        return false;
    };
    let path = path.strip_suffix('/').unwrap_or_default();
    path.split('/').next_back() == Some(publish_percent_encode_path(generation_id).as_str())
        && publish_remote_authority(authority)
        && !path.is_empty()
        && path.split('/').all(valid_publish_remote_path_component)
}

fn canonical_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn ensure_regular_file(
    path: &Path,
    operation: &'static str,
    message: &'static str,
) -> Result<std::fs::Metadata, BuilderError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(operation, source))?;
    if !metadata.file_type().is_file() {
        return Err(BuilderError::Invalid(message));
    }
    Ok(metadata)
}

fn ensure_executable(path: &Path, name: &'static str) -> Result<(), BuilderError> {
    let metadata = ensure_regular_file(path, "inspect release tool", name)?;
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(BuilderError::Invalid(name));
    }
    Ok(())
}

fn validate_request(request: &BuildRequest) -> Result<(), BuilderError> {
    if request.defer_manager_probe && request.manager.is_none() {
        return Err(BuilderError::Invalid(
            "deferred Manager artifact probe requires a Manager artifact",
        ));
    }
    if !valid_stable_version(&request.version) {
        return Err(BuilderError::Invalid("release version is invalid"));
    }
    if !valid_lower_sha256(&request.archive_sha256) {
        return Err(BuilderError::Invalid("archive SHA-256 is invalid"));
    }
    if !valid_generation_id(&request.generation_id) {
        return Err(BuilderError::Invalid("generation identity is invalid"));
    }
    if !valid_line_value(&request.creation_metadata, TEXT_VALUE_MAX_BYTES) {
        return Err(BuilderError::Invalid("creation metadata is invalid"));
    }
    if request.legacy_activation_doctor_unsupported
        && request.creation_metadata != R10_BROWSER_HELPER_BRIDGE_METADATA
    {
        return Err(BuilderError::Invalid(
            "legacy activation doctor transition requires the exact R10 helper layout",
        ));
    }
    for path in [
        &request.archive,
        &request.core,
        &request.gzip,
        &request.openssl,
        &request.output,
    ] {
        if !canonical_absolute_path(path) {
            return Err(BuilderError::Invalid(
                "builder paths must be canonical absolute paths",
            ));
        }
    }
    let archive = ensure_regular_file(
        &request.archive,
        "inspect upstream archive",
        "upstream archive is not a regular file",
    )?;
    if archive.len() > ARCHIVE_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "upstream archive exceeds its byte bound",
        ));
    }
    let core = ensure_regular_file(
        &request.core,
        "inspect Core artifact",
        "Core artifact is not an executable regular file",
    )?;
    if core.permissions().mode() & 0o111 == 0 {
        return Err(BuilderError::Invalid(
            "Core artifact is not an executable regular file",
        ));
    }
    if core.len() > CORE_ARTIFACT_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "Core artifact exceeds its byte bound",
        ));
    }
    if let Some(manager) = request.manager.as_ref() {
        if !canonical_absolute_path(manager) {
            return Err(BuilderError::Invalid(
                "Manager artifact path must be canonical absolute",
            ));
        }
        let manager_metadata = ensure_regular_file(
            manager,
            "inspect Manager artifact",
            "Manager artifact is not an executable regular file",
        )?;
        if manager_metadata.permissions().mode() & 0o111 == 0 {
            return Err(BuilderError::Invalid(
                "Manager artifact is not an executable regular file",
            ));
        }
        if manager_metadata.len() > MANAGER_ARTIFACT_MAX_BYTES {
            return Err(BuilderError::Invalid(
                "Manager artifact exceeds its byte bound",
            ));
        }
    }
    ensure_executable(&request.gzip, "gzip is not an executable regular file")?;
    ensure_executable(
        &request.openssl,
        "OpenSSL is not an executable regular file",
    )?;
    match std::fs::symlink_metadata(&request.output) {
        Ok(_) => return Err(BuilderError::Invalid("output directory already exists")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("inspect output directory", source)),
    }
    let parent = request
        .output
        .parent()
        .ok_or(BuilderError::Invalid("output directory has no parent"))?;
    let metadata = std::fs::symlink_metadata(parent)
        .map_err(|source| io_error("inspect output parent", source))?;
    if !metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "output parent is not a real directory",
        ));
    }
    let canonical = std::fs::canonicalize(parent)
        .map_err(|source| io_error("resolve output parent", source))?;
    if canonical != parent {
        return Err(BuilderError::Invalid("output parent contains a symlink"));
    }
    Ok(())
}

fn spawn_bounded_probe_reader<R>(reader: R) -> std::thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        reader
            .take((MANAGER_ARTIFACT_PROBE_OUTPUT_MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

fn qualify_manager_artifact(path: &Path, staging: &Path) -> Result<(), BuilderError> {
    let mut child = Command::new(path)
        .arg(MANAGER_ARTIFACT_PROBE_ARGUMENT)
        .env_clear()
        .env(MANAGER_ARTIFACT_PROBE_ENV, "1")
        .current_dir(staging)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| io_error("start Manager artifact probe", source))?;
    let stdout = child.stdout.take().ok_or(BuilderError::Tool(
        "Manager artifact probe stdout is unavailable",
    ))?;
    let stderr = child.stderr.take().ok_or(BuilderError::Tool(
        "Manager artifact probe stderr is unavailable",
    ))?;
    let stdout_reader = spawn_bounded_probe_reader(stdout);
    let stderr_reader = spawn_bounded_probe_reader(stderr);
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(MANAGER_ARTIFACT_PROBE_TIMEOUT_SECONDS);
    let mut timed_out = false;
    let status = loop {
        match child
            .try_wait()
            .map_err(|source| io_error("poll Manager artifact probe", source))?
        {
            Some(status) => break Some(status),
            None if std::time::Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(25)),
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| BuilderError::Tool("Manager artifact probe stdout reader failed"))?
        .map_err(|source| io_error("read Manager artifact probe stdout", source))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| BuilderError::Tool("Manager artifact probe stderr reader failed"))?
        .map_err(|source| io_error("read Manager artifact probe stderr", source))?;
    if timed_out {
        return Err(BuilderError::Invalid("Manager artifact probe timed out"));
    }
    if stdout.len() > MANAGER_ARTIFACT_PROBE_OUTPUT_MAX_BYTES
        || stderr.len() > MANAGER_ARTIFACT_PROBE_OUTPUT_MAX_BYTES
    {
        return Err(BuilderError::Invalid(
            "Manager artifact probe output exceeds its byte bound",
        ));
    }
    if !stderr.is_empty() {
        return Err(BuilderError::Invalid(
            "Manager artifact probe wrote to stderr",
        ));
    }
    if !status.is_some_and(|status| status.success()) {
        return Err(BuilderError::Invalid("Manager artifact probe failed"));
    }
    if stdout != MANAGER_ARTIFACT_PROBE_OUTPUT {
        return Err(BuilderError::Invalid(
            "Manager artifact probe output is incompatible",
        ));
    }
    Ok(())
}

fn write_deferred_manager_probe_marker(staging: &Path) -> Result<(), BuilderError> {
    let path = staging.join(MANAGER_ARTIFACT_DEFERRED_MARKER);
    let mut file = create_private_file(&path)?;
    file.write_all(MANAGER_ARTIFACT_DEFERRED_MARKER_BYTES)
        .map_err(|source| io_error("write deferred Manager probe marker", source))?;
    file.sync_all()
        .map_err(|source| io_error("sync deferred Manager probe marker", source))?;
    drop(file);
    set_mode(&path, 0o644, "set deferred Manager probe marker mode")?;
    File::open(&path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync deferred Manager probe marker mode", source))
}

fn validate_fetch_request(request: &FetchRequest) -> Result<(), BuilderError> {
    if !valid_stable_version(&request.version) {
        return Err(BuilderError::Invalid("release version is invalid"));
    }
    for path in [&request.curl, &request.openssl, &request.output] {
        if !canonical_absolute_path(path) {
            return Err(BuilderError::Invalid(
                "fetch paths must be canonical absolute paths",
            ));
        }
    }
    ensure_executable(&request.curl, "curl is not an executable regular file")?;
    ensure_executable(
        &request.openssl,
        "OpenSSL is not an executable regular file",
    )?;
    match std::fs::symlink_metadata(&request.output) {
        Ok(_) => return Err(BuilderError::Invalid("fetch output already exists")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("inspect fetch output", source)),
    }
    let parent = request
        .output
        .parent()
        .ok_or(BuilderError::Invalid("fetch output has no parent"))?;
    let metadata = std::fs::symlink_metadata(parent)
        .map_err(|source| io_error("inspect fetch output parent", source))?;
    if !metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "fetch output parent is not a real directory",
        ));
    }
    let canonical = std::fs::canonicalize(parent)
        .map_err(|source| io_error("resolve fetch output parent", source))?;
    if canonical != parent {
        return Err(BuilderError::Invalid(
            "fetch output parent contains a symlink",
        ));
    }
    Ok(())
}

fn validate_publish_request(request: &PublishRequest) -> Result<(), BuilderError> {
    if !valid_positive_decimal(&request.release_sequence)
        || request.release_sequence.parse::<u64>().is_err()
    {
        return Err(BuilderError::Invalid("release sequence is invalid"));
    }
    if !canonical_absolute_path(&request.generation)
        || !canonical_absolute_path(&request.private_key)
        || !canonical_absolute_path(&request.openssl)
        || !canonical_absolute_path(&request.output)
    {
        return Err(BuilderError::Invalid(
            "publish paths must be canonical absolute paths",
        ));
    }
    let generation_metadata = std::fs::symlink_metadata(&request.generation)
        .map_err(|source| io_error("inspect publication generation", source))?;
    if !generation_metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "publication generation is not a real directory",
        ));
    }
    let generation_canonical = std::fs::canonicalize(&request.generation)
        .map_err(|source| io_error("resolve publication generation", source))?;
    if generation_canonical != request.generation {
        return Err(BuilderError::Invalid(
            "publication generation contains a symlinked path",
        ));
    }
    match std::fs::symlink_metadata(request.generation.join(MANAGER_ARTIFACT_DEFERRED_MARKER)) {
        Ok(_) => {
            return Err(BuilderError::Invalid(
                "Manager artifact probe is still deferred",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("inspect deferred Manager probe marker", source)),
    }
    let private_key = ensure_regular_file(
        &request.private_key,
        "inspect release private key",
        "release private key is not a regular file",
    )?;
    if private_key.len() == 0 || private_key.len() > RELEASE_PRIVATE_KEY_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "release private key exceeds its byte bound",
        ));
    }
    ensure_executable(
        &request.openssl,
        "OpenSSL is not an executable regular file",
    )?;
    match std::fs::symlink_metadata(&request.output) {
        Ok(_) => return Err(BuilderError::Invalid("publication output already exists")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("inspect publication output", source)),
    }
    let parent = request
        .output
        .parent()
        .ok_or(BuilderError::Invalid("publication output has no parent"))?;
    let metadata = std::fs::symlink_metadata(parent)
        .map_err(|source| io_error("inspect publication output parent", source))?;
    if !metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "publication output parent is not a real directory",
        ));
    }
    let canonical = std::fs::canonicalize(parent)
        .map_err(|source| io_error("resolve publication output parent", source))?;
    if canonical != parent {
        return Err(BuilderError::Invalid(
            "publication output parent contains a symlink",
        ));
    }
    Ok(())
}

fn openssl_sha256(openssl: &Path, file: &Path) -> Result<String, BuilderError> {
    let output = Command::new(openssl)
        .args(["dgst", "-sha256", "-binary"])
        .arg(file)
        .env_clear()
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|source| io_error("run OpenSSL SHA-256", source))?;
    if !output.status.success() || output.stdout.len() != 32 {
        return Err(BuilderError::Tool("OpenSSL SHA-256 failed"));
    }
    let mut hex = String::with_capacity(64);
    for byte in output.stdout {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing into a String cannot fail");
    }
    Ok(hex)
}

#[derive(Debug)]
struct PublishFile {
    relative_path: &'static str,
    snapshot_path: PathBuf,
    sha256: String,
    mode: u32,
}

#[derive(Debug)]
struct PublishGeneration {
    generation_id: String,
    files: Vec<PublishFile>,
    r10_bridge: bool,
}

fn validate_publish_file_mode(
    metadata: &std::fs::Metadata,
    executable: bool,
    message: &'static str,
) -> Result<u32, BuilderError> {
    let mode = metadata.permissions().mode() & 0o7777;
    if mode & 0o7000 != 0 || mode & 0o400 == 0 || (executable && mode & 0o100 == 0) {
        return Err(BuilderError::Invalid(message));
    }
    Ok(mode)
}

fn validate_publish_browser_layout(root: &Path) -> Result<(), BuilderError> {
    let browser = root.join("browser");
    let browser_metadata = std::fs::symlink_metadata(&browser)
        .map_err(|source| io_error("inspect publication browser helper root", source))?;
    if !browser_metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "publication browser helper root is not a real directory",
        ));
    }
    let mut seen = BTreeSet::new();
    for entry in std::fs::read_dir(&browser)
        .map_err(|source| io_error("read publication browser helper root", source))?
    {
        let entry =
            entry.map_err(|source| io_error("read publication browser helper entry", source))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| BuilderError::Invalid("publication browser helper path is not UTF-8"))?;
        if !matches!(name.as_str(), "open" | "manual") || !seen.insert(name.clone()) {
            return Err(BuilderError::Invalid(
                "publication browser helper layout is unsupported",
            ));
        }
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(|source| io_error("inspect publication browser helper directory", source))?;
        if !metadata.file_type().is_dir() {
            return Err(BuilderError::Invalid(
                "publication browser helper directory is not a real directory",
            ));
        }
        let mut leaf_entries = std::fs::read_dir(entry.path())
            .map_err(|source| io_error("read publication browser helper directory", source))?;
        let helper = leaf_entries
            .next()
            .ok_or(BuilderError::Invalid(
                "publication browser helper is missing",
            ))?
            .map_err(|source| io_error("read publication browser helper", source))?;
        if helper.file_name() != OsStr::new("curl") || leaf_entries.next().is_some() {
            return Err(BuilderError::Invalid(
                "publication browser helper directory is not exact",
            ));
        }
        let helper_metadata = ensure_regular_file(
            &helper.path(),
            "inspect publication browser helper",
            "publication browser helper is not a regular file",
        )?;
        validate_publish_file_mode(
            &helper_metadata,
            true,
            "publication browser helper mode is unsafe",
        )?;
    }
    if seen.len() != 2 {
        return Err(BuilderError::Invalid(
            "publication browser helper layout is incomplete",
        ));
    }
    Ok(())
}

fn validate_publish_r10_bridge_layout(root: &Path) -> Result<(), BuilderError> {
    let helpers = root.join("helpers");
    let metadata = std::fs::symlink_metadata(&helpers)
        .map_err(|source| io_error("inspect R10 bridge helper root", source))?;
    if !metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "R10 bridge helper root is not a real directory",
        ));
    }
    let mut seen = BTreeSet::new();
    for entry in std::fs::read_dir(&helpers)
        .map_err(|source| io_error("read R10 bridge helper directory", source))?
    {
        let entry = entry.map_err(|source| io_error("read R10 bridge helper entry", source))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| BuilderError::Invalid("R10 bridge helper path is not UTF-8"))?;
        if !matches!(name.as_str(), "0" | "1") || !seen.insert(name) {
            return Err(BuilderError::Invalid(
                "R10 bridge helper layout is not exact",
            ));
        }
        let helper_metadata = ensure_regular_file(
            &entry.path(),
            "inspect R10 bridge helper",
            "R10 bridge helper is not a regular file",
        )?;
        validate_publish_file_mode(&helper_metadata, true, "R10 bridge helper mode is unsafe")?;
    }
    if seen.len() != 2 {
        return Err(BuilderError::Invalid(
            "R10 bridge helper layout is incomplete",
        ));
    }
    Ok(())
}

fn validate_publish_generation_layout(root: &Path, r10_bridge: bool) -> Result<(), BuilderError> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(|source| io_error("inspect publication generation", source))?;
    if !metadata.file_type().is_dir() {
        return Err(BuilderError::Invalid(
            "publication generation is not a real directory",
        ));
    }
    let helper_root = if r10_bridge { "helpers" } else { "browser" };
    let required = [
        "generation.meta",
        "runtime",
        "codex-code-mode-host",
        helper_root,
    ];
    let mut seen = BTreeSet::new();
    for entry in
        std::fs::read_dir(root).map_err(|source| io_error("read publication generation", source))?
    {
        let entry =
            entry.map_err(|source| io_error("read publication generation entry", source))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| BuilderError::Invalid("publication generation path is not UTF-8"))?;
        if (!required.contains(&name.as_str()) && !matches!(name.as_str(), "core" | "manager"))
            || !seen.insert(name.clone())
        {
            return Err(BuilderError::Invalid(
                "publication generation layout is unsupported",
            ));
        }
        if name == helper_root {
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|source| io_error("inspect publication helper root", source))?;
            if !metadata.file_type().is_dir() {
                return Err(BuilderError::Invalid(
                    "publication helper root is not a real directory",
                ));
            }
            continue;
        }
        let file_metadata = ensure_regular_file(
            &entry.path(),
            "inspect publication generation file",
            "publication generation contains a non-regular file",
        )?;
        validate_publish_file_mode(
            &file_metadata,
            matches!(
                name.as_str(),
                "core" | "runtime" | "codex-code-mode-host" | "manager"
            ),
            "publication generation file mode is unsafe",
        )?;
    }
    if required.iter().any(|name| !seen.contains(*name)) {
        return Err(BuilderError::Invalid(
            "publication generation layout is incomplete",
        ));
    }
    if r10_bridge {
        validate_publish_r10_bridge_layout(root)
    } else {
        validate_publish_browser_layout(root)
    }
}

fn snapshot_publish_file(
    source: &Path,
    destination: &Path,
    max_bytes: u64,
    openssl: &Path,
    executable: bool,
) -> Result<(String, u32, u64), BuilderError> {
    let source_metadata = ensure_regular_file(
        source,
        "inspect publication source file",
        "publication source file is not a regular file",
    )?;
    let source_file =
        File::open(source).map_err(|source| io_error("open publication source file", source))?;
    let opened_metadata = source_file
        .metadata()
        .map_err(|source| io_error("inspect opened publication source file", source))?;
    if !opened_metadata.file_type().is_file() || opened_metadata.len() > max_bytes {
        return Err(BuilderError::Invalid(
            "publication source file exceeds its byte bound",
        ));
    }
    let mode = validate_publish_file_mode(
        &source_metadata,
        executable,
        "publication source file mode is unsafe",
    )?;
    let mut output = create_private_file(destination)?;
    let mut source_file = source_file;
    let mut bounded = Read::by_ref(&mut source_file).take(max_bytes.saturating_add(1));
    let copied = io::copy(&mut bounded, &mut output)
        .map_err(|source| io_error("snapshot publication source file", source))?;
    if copied > max_bytes || copied != opened_metadata.len() {
        return Err(BuilderError::Invalid(
            "publication source file changed during snapshot",
        ));
    }
    let final_source_metadata = source_file
        .metadata()
        .map_err(|source| io_error("inspect publication source after snapshot", source))?;
    if final_source_metadata.len() != copied {
        return Err(BuilderError::Invalid(
            "publication source file changed during snapshot",
        ));
    }
    output
        .sync_all()
        .map_err(|source| io_error("sync publication source snapshot", source))?;
    drop(output);
    set_mode(destination, mode, "set publication source snapshot mode")?;
    File::open(destination)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync publication source snapshot mode", source))?;
    let sha256 = openssl_sha256(openssl, destination)?;
    Ok((sha256, mode, copied))
}

fn publish_descriptor_field<'a>(
    lines: &mut std::str::Lines<'a>,
    expected: &'static str,
) -> Result<&'a str, BuilderError> {
    let line = lines
        .next()
        .ok_or(BuilderError::Invalid("generation descriptor is incomplete"))?;
    let Some((name, value)) = line.split_once('\t') else {
        return Err(BuilderError::Invalid(
            "generation descriptor field is malformed",
        ));
    };
    if name != expected || !valid_line_value(value, TEXT_VALUE_MAX_BYTES) {
        return Err(BuilderError::Invalid(
            "generation descriptor field is invalid",
        ));
    }
    Ok(value)
}

fn validate_publish_patch_report(
    value: &str,
    source_digest: &str,
    runtime_digest: &str,
    code_mode_host_digest: &str,
) -> Result<(), BuilderError> {
    let mut fields = value.split(';');
    if fields.next() != Some(PATCH_POLICY_ID) {
        return Err(BuilderError::Invalid(
            "generation patch policy report is invalid",
        ));
    }
    let archive_digest = fields
        .next()
        .and_then(|field| field.strip_prefix("archive_sha256="));
    let raw_runtime_digest = fields
        .next()
        .and_then(|field| field.strip_prefix("raw_runtime_sha256="));
    let adapted_runtime_digest = fields
        .next()
        .and_then(|field| field.strip_prefix("runtime_sha256="));
    let host_digest = fields
        .next()
        .and_then(|field| field.strip_prefix("code_mode_host_sha256="));
    let source_counts = fields.next();
    let changed_bytes = fields.next();
    if fields.next().is_some()
        || archive_digest != Some(source_digest)
        || raw_runtime_digest.is_none_or(|digest| !valid_lower_sha256(digest))
        || adapted_runtime_digest != Some(runtime_digest)
        || host_digest != Some(code_mode_host_digest)
        || source_counts != Some("source_counts=2,1,1,1")
        || changed_bytes != Some("changed_bytes=54")
    {
        return Err(BuilderError::Invalid(
            "generation patch policy report is invalid",
        ));
    }
    Ok(())
}

fn publish_descriptor_helper<'a>(
    lines: &mut std::str::Lines<'a>,
    expected_identity: &'static str,
) -> Result<&'a str, BuilderError> {
    let line = lines.next().ok_or(BuilderError::Invalid(
        "generation descriptor helper is missing",
    ))?;
    let mut fields = line.split('\t');
    if fields.next() != Some("helper") || fields.next() != Some(expected_identity) {
        return Err(BuilderError::Invalid(
            "generation descriptor helper identity is invalid",
        ));
    }
    let digest = fields.next().ok_or(BuilderError::Invalid(
        "generation descriptor helper digest is missing",
    ))?;
    if fields.next().is_some() || !valid_lower_sha256(digest) {
        return Err(BuilderError::Invalid(
            "generation descriptor helper digest is invalid",
        ));
    }
    Ok(digest)
}

fn validate_publish_generation_descriptor(
    descriptor_path: &Path,
    runtime_path: &Path,
    code_mode_host_path: &Path,
    browser_helper_paths: (&Path, &Path),
    core_manager_paths: (Option<&Path>, Option<&Path>),
    r10_bridge: bool,
    openssl: &Path,
) -> Result<String, BuilderError> {
    let (browser_open_helper_path, browser_manual_helper_path) = browser_helper_paths;
    let (core_path, manager_path) = core_manager_paths;
    let metadata = ensure_regular_file(
        descriptor_path,
        "inspect publication generation descriptor",
        "publication generation descriptor is not a regular file",
    )?;
    if metadata.len() > GENERATION_DESCRIPTOR_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "publication generation descriptor exceeds its byte bound",
        ));
    }
    let bytes = std::fs::read(descriptor_path)
        .map_err(|source| io_error("read publication generation descriptor", source))?;
    if !bytes.ends_with(b"\n") || bytes.contains(&b'\r') {
        return Err(BuilderError::Invalid(
            "publication generation descriptor line ending is invalid",
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| BuilderError::Invalid("publication generation descriptor is not UTF-8"))?;
    let mut lines = text.lines();
    if lines.next() != Some(GENERATION_FORMAT) {
        return Err(BuilderError::Invalid(
            "publication generation descriptor format is unsupported",
        ));
    }
    let generation_id = publish_descriptor_field(&mut lines, "generation_id")?;
    if !valid_publish_generation_id(generation_id) {
        return Err(BuilderError::Invalid(
            "publication generation identity is not a safe URL component",
        ));
    }
    let package_identity = publish_descriptor_field(&mut lines, "upstream_package_identity")?;
    let package_version = publish_descriptor_field(&mut lines, "upstream_package_version")?;
    if package_identity != PACKAGE_IDENTITY || !valid_stable_version(package_version) {
        return Err(BuilderError::Invalid(
            "publication generation upstream binding is invalid",
        ));
    }
    let source_digest = publish_descriptor_field(&mut lines, "source_artifact_digest")?;
    let expected_platform = publish_descriptor_field(&mut lines, "expected_platform")?;
    let expected_architecture = publish_descriptor_field(&mut lines, "expected_architecture")?;
    if !valid_lower_sha256(source_digest)
        || expected_platform != "android"
        || expected_architecture != "aarch64"
    {
        return Err(BuilderError::Invalid(
            "publication generation platform binding is invalid",
        ));
    }
    if publish_descriptor_field(&mut lines, "patch_policy_id")? != PATCH_POLICY_ID {
        return Err(BuilderError::Invalid(
            "publication generation patch policy is invalid",
        ));
    }
    let patch_report = publish_descriptor_field(&mut lines, "patch_report")?;
    let runtime_digest = publish_descriptor_field(&mut lines, "runtime_digest")?;
    let core_artifact_digest = publish_descriptor_field(&mut lines, "core_artifact_digest")?;
    let manager_artifact_digest = publish_descriptor_field(&mut lines, "manager_artifact_digest")?;
    if !valid_lower_sha256(runtime_digest) || !valid_lower_sha256(core_artifact_digest) {
        return Err(BuilderError::Invalid(
            "publication generation artifact binding is invalid",
        ));
    }
    if manager_artifact_digest == "-" {
        if manager_path.is_some() {
            return Err(BuilderError::Invalid(
                "publication Manager is present but its descriptor binding is absent",
            ));
        }
    } else if !valid_lower_sha256(manager_artifact_digest)
        || manager_path.is_none()
        || core_path.is_none()
    {
        return Err(BuilderError::Invalid(
            "publication generation Manager binding requires Core",
        ));
    }
    if manager_path.is_some() && core_path.is_none() {
        return Err(BuilderError::Invalid(
            "publication Manager requires a Core artifact",
        ));
    }
    if publish_descriptor_field(&mut lines, "core_api_identity")? != CORE_API_IDENTITY
        || publish_descriptor_field(&mut lines, "persistent_schema_identity")?
            != PERSISTENT_SCHEMA_IDENTITY
        || publish_descriptor_field(&mut lines, "qualification")? != "qualified"
    {
        return Err(BuilderError::Invalid(
            "publication generation qualification binding is invalid",
        ));
    }
    let creation_metadata = publish_descriptor_field(&mut lines, "creation_metadata")?;
    let upstream_doctor = publish_descriptor_field(&mut lines, "upstream_doctor")?;
    let transition_doctor =
        creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA && upstream_doctor == "unsupported";
    if (creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA) != r10_bridge
        || (upstream_doctor != "supported" && !transition_doctor)
        || publish_descriptor_field(&mut lines, "helper_count")? != "2"
    {
        return Err(BuilderError::Invalid(
            "publication generation browser helper layout binding is invalid",
        ));
    }
    let browser_open_helper_digest =
        publish_descriptor_helper(&mut lines, TERMUX_BROWSER_OPEN_HELPER_IDENTITY)?;
    let browser_manual_helper_digest =
        publish_descriptor_helper(&mut lines, TERMUX_BROWSER_MANUAL_HELPER_IDENTITY)?;
    if lines.next().is_some() {
        return Err(BuilderError::Invalid(
            "publication generation qualification binding is invalid",
        ));
    }
    let actual_runtime_digest = openssl_sha256(openssl, runtime_path)?;
    let actual_host_digest = openssl_sha256(openssl, code_mode_host_path)?;
    if actual_runtime_digest != runtime_digest {
        return Err(BuilderError::Invalid(
            "publication runtime digest does not match its descriptor",
        ));
    }
    for (path, expected_digest) in [
        (browser_open_helper_path, browser_open_helper_digest),
        (browser_manual_helper_path, browser_manual_helper_digest),
    ] {
        let metadata = ensure_regular_file(
            path,
            "inspect publication browser helper",
            "publication browser helper is not a regular file",
        )?;
        if metadata.permissions().mode() & 0o100 == 0
            || openssl_sha256(openssl, path)? != expected_digest
        {
            return Err(BuilderError::Invalid(
                "publication browser helper does not match its descriptor",
            ));
        }
    }
    if let Some(core_path) = core_path {
        let core_metadata = ensure_regular_file(
            core_path,
            "inspect publication Core artifact",
            "publication Core artifact is not a regular file",
        )?;
        if core_metadata.permissions().mode() & 0o100 == 0
            || openssl_sha256(openssl, core_path)? != core_artifact_digest
        {
            return Err(BuilderError::Invalid(
                "publication Core artifact does not match its descriptor",
            ));
        }
    }
    if let (Some(manager_path), Some(expected_digest)) = (
        manager_path,
        (!manager_artifact_digest.is_empty()).then_some(manager_artifact_digest),
    ) {
        let metadata = ensure_regular_file(
            manager_path,
            "inspect publication Manager",
            "publication Manager is not a regular file",
        )?;
        if metadata.permissions().mode() & 0o100 == 0
            || openssl_sha256(openssl, manager_path)? != expected_digest
        {
            return Err(BuilderError::Invalid(
                "publication Manager does not match its descriptor",
            ));
        }
    }
    validate_publish_patch_report(
        patch_report,
        source_digest,
        runtime_digest,
        &actual_host_digest,
    )?;
    Ok(generation_id.to_owned())
}

fn snapshot_publish_generation(
    source_root: &Path,
    staging: &Path,
    openssl: &Path,
) -> Result<PublishGeneration, BuilderError> {
    let r10_bridge = std::fs::symlink_metadata(source_root.join("helpers")).is_ok();
    validate_publish_generation_layout(source_root, r10_bridge)?;
    let source_snapshot_root = staging.join(".generation-source");
    create_private_dir(&source_snapshot_root)?;
    if r10_bridge {
        create_private_dir(&source_snapshot_root.join("helpers"))?;
    } else {
        let browser_snapshot_root = source_snapshot_root.join("browser");
        create_private_dir(&browser_snapshot_root)?;
        create_private_dir(&browser_snapshot_root.join("open"))?;
        create_private_dir(&browser_snapshot_root.join("manual"))?;
    }
    let (browser_open_relative, browser_manual_relative) =
        browser_helper_relative_paths(r10_bridge);
    let mut total_size = 0u64;
    let mut files = Vec::with_capacity(7);
    let mut entries = vec![
        ("generation.meta", GENERATION_DESCRIPTOR_MAX_BYTES, false),
        ("runtime", RELEASE_FILE_MAX_BYTES, true),
        ("codex-code-mode-host", RELEASE_FILE_MAX_BYTES, true),
        (browser_open_relative, BROWSER_HELPER_MAX_BYTES, true),
        (browser_manual_relative, BROWSER_HELPER_MAX_BYTES, true),
    ];
    if std::fs::symlink_metadata(source_root.join("core")).is_ok() {
        entries.push(("core", CORE_ARTIFACT_MAX_BYTES, true));
    }
    if std::fs::symlink_metadata(source_root.join("manager")).is_ok() {
        entries.push(("manager", RELEASE_FILE_MAX_BYTES, true));
    }
    for (relative_path, max_bytes, executable) in entries {
        let destination = source_snapshot_root.join(relative_path);
        let (sha256, mode, size) = snapshot_publish_file(
            &source_root.join(relative_path),
            &destination,
            max_bytes,
            openssl,
            executable,
        )?;
        total_size = total_size
            .checked_add(size)
            .filter(|total| *total <= RELEASE_TOTAL_FILE_MAX_BYTES)
            .ok_or(BuilderError::Invalid(
                "publication generation exceeds its total byte bound",
            ))?;
        files.push(PublishFile {
            relative_path,
            snapshot_path: destination,
            sha256,
            mode,
        });
    }
    validate_publish_generation_layout(&source_snapshot_root, r10_bridge)?;
    let manager_path = source_snapshot_root.join("manager");
    let manager_path = std::fs::symlink_metadata(&manager_path)
        .ok()
        .map(|_| manager_path);
    let core_path = source_snapshot_root.join("core");
    let core_path = std::fs::symlink_metadata(&core_path)
        .ok()
        .map(|_| core_path);
    let generation_id = validate_publish_generation_descriptor(
        &source_snapshot_root.join("generation.meta"),
        &source_snapshot_root.join("runtime"),
        &source_snapshot_root.join("codex-code-mode-host"),
        (
            &source_snapshot_root.join(browser_open_relative),
            &source_snapshot_root.join(browser_manual_relative),
        ),
        (core_path.as_deref(), manager_path.as_deref()),
        r10_bridge,
        openssl,
    )?;
    files.sort_by_key(|file| file.relative_path);
    Ok(PublishGeneration {
        generation_id,
        files,
        r10_bridge,
    })
}

fn openssl_public_key(openssl: &Path, private_key: &Path) -> Result<[u8; 32], BuilderError> {
    let output = Command::new(openssl)
        .args(["pkey", "-in"])
        .arg(private_key)
        .args(["-pubout", "-outform", "DER"])
        .env_clear()
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|source| io_error("derive release public key", source))?;
    const PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    if !output.status.success()
        || output.stdout.len() != 44
        || output.stdout[..PREFIX.len()] != PREFIX
    {
        return Err(BuilderError::Tool(
            "release private key is not an Ed25519 private key",
        ));
    }
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&output.stdout[PREFIX.len()..]);
    Ok(public_key)
}

fn public_key_hex(public_key: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in public_key {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing into a String cannot fail");
    }
    hex
}

fn write_published_file(
    path: &Path,
    bytes: &[u8],
    mode: u32,
    operation: &'static str,
) -> Result<(), BuilderError> {
    let mut file = create_private_file(path)?;
    file.write_all(bytes)
        .map_err(|source| io_error(operation, source))?;
    drop(file);
    set_mode(path, mode, operation)?;
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error(operation, source))
}

fn sign_published_file(
    openssl: &Path,
    private_key: &Path,
    input: &Path,
    signature: &Path,
) -> Result<(), BuilderError> {
    let placeholder = create_private_file(signature)?;
    drop(placeholder);
    let status = Command::new(openssl)
        .args(["pkeyutl", "-sign", "-rawin", "-inkey"])
        .arg(private_key)
        .arg("-in")
        .arg(input)
        .arg("-out")
        .arg(signature)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|source| io_error("sign publication file", source))?;
    if !status.success() {
        return Err(BuilderError::Tool("OpenSSL release signature failed"));
    }
    let metadata = ensure_regular_file(
        signature,
        "inspect publication signature",
        "publication signature is not a regular file",
    )?;
    if metadata.len() != RELEASE_SIGNATURE_BYTES {
        return Err(BuilderError::Tool(
            "OpenSSL release signature has invalid size",
        ));
    }
    set_mode(signature, 0o644, "set publication signature mode")?;
    File::open(signature)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync publication signature", source))
}

fn release_manifest_bytes(
    generation_id: &str,
    release_sequence: &str,
    public_key: &[u8; 32],
    files: &[PublishFile],
) -> Result<Vec<u8>, BuilderError> {
    use std::fmt::Write as _;

    let format = if files.iter().any(|file| file.relative_path == "core") {
        RELEASE_FORMAT_V4
    } else {
        RELEASE_FORMAT_V3
    };
    let mut manifest = format!(
        concat!(
            "{}\n",
            "generation_id\t{}\n",
            "release_sequence\t{}\n",
            "channel\t{}\n",
            "expected_platform\tandroid\n",
            "expected_architecture\taarch64\n",
            "core_api_identity\t{}\n",
            "persistent_schema_identity\t{}\n",
            "release_public_key\t{}\n",
            "file_count\t{}\n",
        ),
        format,
        generation_id,
        release_sequence,
        RELEASE_CHANNEL,
        CORE_API_IDENTITY,
        PERSISTENT_SCHEMA_IDENTITY,
        public_key_hex(public_key),
        files.len(),
    );
    for file in files {
        writeln!(
            &mut manifest,
            "file\t{}\t{}\t{:04o}",
            file.relative_path, file.sha256, file.mode
        )
        .expect("writing into String cannot fail");
    }
    if manifest.len() as u64 > RELEASE_MANIFEST_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "release manifest exceeds its byte bound",
        ));
    }
    Ok(manifest.into_bytes())
}

fn update_index_bytes(generation_id: &str, release_base: &str) -> Vec<u8> {
    format!(
        "{RELEASE_INDEX_FORMAT}\nchannel\t{RELEASE_CHANNEL}\ngeneration_id\t{generation_id}\nrelease_base\t{release_base}\n"
    )
    .into_bytes()
}

fn publish(request: &PublishRequest) -> Result<String, BuilderError> {
    validate_publish_request(request)?;
    let staging = create_staging(&request.output)?;
    let result = (|| {
        let generation =
            snapshot_publish_generation(&request.generation, &staging, &request.openssl)?;
        if !valid_publish_release_base(&request.release_base, &generation.generation_id) {
            return Err(BuilderError::Invalid(
                "release base is not canonical or does not match generation identity",
            ));
        }
        let public_key = openssl_public_key(&request.openssl, &request.private_key)?;
        let releases = staging.join("releases");
        let release_dir = releases.join(&generation.generation_id);
        create_private_dir(&releases)?;
        create_private_dir(&release_dir)?;
        let helper_directories = if generation.r10_bridge {
            let release_helpers = release_dir.join("helpers");
            create_private_dir(&release_helpers)?;
            vec![release_helpers]
        } else {
            let release_browser = release_dir.join("browser");
            let release_browser_open = release_browser.join("open");
            let release_browser_manual = release_browser.join("manual");
            create_private_dir(&release_browser)?;
            create_private_dir(&release_browser_open)?;
            create_private_dir(&release_browser_manual)?;
            vec![
                release_browser_open,
                release_browser_manual,
                release_browser,
            ]
        };
        for file in &generation.files {
            rename_noreplace(&file.snapshot_path, &release_dir.join(file.relative_path))?;
        }
        std::fs::remove_dir_all(staging.join(".generation-source"))
            .map_err(|source| io_error("remove publication source staging", source))?;

        for directory in &helper_directories {
            set_mode(directory, 0o755, "set release helper directory mode")?;
            sync_directory(directory, "sync release helper directory")?;
        }

        let manifest = release_manifest_bytes(
            &generation.generation_id,
            &request.release_sequence,
            &public_key,
            &generation.files,
        )?;
        let manifest_path = release_dir.join("release.manifest");
        write_published_file(&manifest_path, &manifest, 0o644, "write release manifest")?;
        sign_published_file(
            &request.openssl,
            &request.private_key,
            &manifest_path,
            &release_dir.join("release.sig"),
        )?;

        let index = update_index_bytes(&generation.generation_id, &request.release_base);
        let index_path = staging.join("update-index-v1");
        write_published_file(&index_path, &index, 0o644, "write update index")?;
        sign_published_file(
            &request.openssl,
            &request.private_key,
            &index_path,
            &staging.join("update-index-v1.sig"),
        )?;

        set_mode(&release_dir, 0o755, "set release directory mode")?;
        sync_directory(&release_dir, "sync published release directory")?;
        set_mode(&releases, 0o755, "set releases directory mode")?;
        sync_directory(&releases, "sync published releases directory")?;
        set_mode(&staging, 0o755, "set publication root mode")?;
        sync_directory(&staging, "sync complete signed publication")?;
        rename_noreplace(&staging, &request.output)?;
        sync_directory(
            request
                .output
                .parent()
                .ok_or(BuilderError::Invalid("publication output has no parent"))?,
            "sync published output parent",
        )?;
        Ok(generation.generation_id)
    })();
    match (result, cleanup_staging(&staging)) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(generation_id), Ok(())) => Ok(generation_id),
    }
}

fn snapshot_core_artifact(request: &BuildRequest, staging: &Path) -> Result<String, BuilderError> {
    let source =
        File::open(&request.core).map_err(|source| io_error("open Core artifact", source))?;
    let metadata = source
        .metadata()
        .map_err(|source| io_error("inspect opened Core artifact", source))?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(BuilderError::Invalid(
            "opened Core artifact is not an executable regular file",
        ));
    }
    if metadata.len() > CORE_ARTIFACT_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "opened Core artifact exceeds its byte bound",
        ));
    }

    let snapshot_path = staging.join(".core-artifact");
    let mut snapshot = create_private_file(&snapshot_path)?;
    let mut bounded = source.take(CORE_ARTIFACT_MAX_BYTES.saturating_add(1));
    let copied = io::copy(&mut bounded, &mut snapshot)
        .map_err(|source| io_error("snapshot Core artifact", source))?;
    if copied > CORE_ARTIFACT_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "Core artifact exceeds its byte bound",
        ));
    }
    snapshot
        .sync_all()
        .map_err(|source| io_error("sync Core artifact snapshot", source))?;
    drop(snapshot);

    validate_android_aarch64_core_elf(&snapshot_path)?;
    let sha256 = openssl_sha256(&request.openssl, &snapshot_path)?;
    Ok(sha256)
}

fn snapshot_manager_artifact(
    request: &BuildRequest,
    staging: &Path,
) -> Result<Option<String>, BuilderError> {
    let Some(source_path) = request.manager.as_ref() else {
        return Ok(None);
    };
    let source =
        File::open(source_path).map_err(|source| io_error("open Manager artifact", source))?;
    let metadata = source
        .metadata()
        .map_err(|source| io_error("inspect opened Manager artifact", source))?;
    if !metadata.file_type().is_file()
        || metadata.permissions().mode() & 0o111 == 0
        || metadata.len() > MANAGER_ARTIFACT_MAX_BYTES
    {
        return Err(BuilderError::Invalid(
            "opened Manager artifact is outside its byte or type bound",
        ));
    }

    let snapshot_path = staging.join(".manager-artifact");
    let mut snapshot = create_private_file(&snapshot_path)?;
    let mut bounded = source.take(MANAGER_ARTIFACT_MAX_BYTES.saturating_add(1));
    let copied = io::copy(&mut bounded, &mut snapshot)
        .map_err(|source| io_error("snapshot Manager artifact", source))?;
    if copied > MANAGER_ARTIFACT_MAX_BYTES || copied != metadata.len() {
        return Err(BuilderError::Invalid(
            "Manager artifact changed or exceeds its byte bound",
        ));
    }
    snapshot
        .sync_all()
        .map_err(|source| io_error("sync Manager artifact snapshot", source))?;
    drop(snapshot);
    set_mode(&snapshot_path, 0o755, "set Manager artifact snapshot mode")?;
    File::open(&snapshot_path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync Manager artifact snapshot mode", source))?;
    let sha256 = openssl_sha256(&request.openssl, &snapshot_path)?;
    Ok(Some(sha256))
}

fn create_staging(output: &Path) -> Result<PathBuf, BuilderError> {
    let parent = output
        .parent()
        .ok_or(BuilderError::Invalid("output directory has no parent"))?;
    let sequence = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(
        ".codex-release-builder-{}-{sequence}",
        std::process::id()
    ));
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(&staging)
        .map_err(|source| io_error("create private builder staging", source))?;
    Ok(staging)
}

fn create_private_dir(path: &Path) -> Result<(), BuilderError> {
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(|source| io_error("create private selected directory", source))
}

fn create_private_file(path: &Path) -> Result<File, BuilderError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|source| io_error("create private selected file", source))
}

fn snapshot_archive(request: &BuildRequest, staging: &Path) -> Result<PathBuf, BuilderError> {
    let mut source =
        File::open(&request.archive).map_err(|source| io_error("open upstream archive", source))?;
    let source_metadata = source
        .metadata()
        .map_err(|source| io_error("inspect opened upstream archive", source))?;
    if !source_metadata.file_type().is_file() || source_metadata.len() > ARCHIVE_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "opened upstream archive is outside its byte or type bound",
        ));
    }

    let snapshot_path = staging.join(".source-archive");
    let mut snapshot = create_private_file(&snapshot_path)?;
    let mut child = Command::new(&request.openssl)
        .args(["dgst", "-sha256", "-binary"])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|source| io_error("start OpenSSL archive SHA-256", source))?;
    let mut digest_input = child
        .stdin
        .take()
        .ok_or(BuilderError::Tool("OpenSSL digest input is unavailable"))?;
    let copy_result = (|| {
        let mut total = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = source
                .read(&mut buffer)
                .map_err(|source| io_error("read upstream archive", source))?;
            if read == 0 {
                break;
            }
            total = total
                .checked_add(read as u64)
                .filter(|total| *total <= ARCHIVE_MAX_BYTES)
                .ok_or(BuilderError::Invalid(
                    "upstream archive exceeds its byte bound",
                ))?;
            snapshot
                .write_all(&buffer[..read])
                .map_err(|source| io_error("write upstream archive snapshot", source))?;
            digest_input
                .write_all(&buffer[..read])
                .map_err(|source| io_error("hash upstream archive snapshot", source))?;
        }
        snapshot
            .sync_all()
            .map_err(|source| io_error("sync upstream archive snapshot", source))
    })();
    drop(digest_input);
    let output = child
        .wait_with_output()
        .map_err(|source| io_error("wait for OpenSSL archive SHA-256", source))?;
    copy_result?;
    if !output.status.success() || output.stdout.len() != 32 {
        return Err(BuilderError::Tool("OpenSSL archive SHA-256 failed"));
    }
    let mut actual = String::with_capacity(64);
    for byte in output.stdout {
        use std::fmt::Write as _;
        write!(&mut actual, "{byte:02x}").expect("writing into a String cannot fail");
    }
    if actual != request.archive_sha256 {
        return Err(BuilderError::ArchiveDigestMismatch);
    }
    Ok(snapshot_path)
}

fn create_fetch_staging(output: &Path) -> Result<PathBuf, BuilderError> {
    let parent = output
        .parent()
        .ok_or(BuilderError::Invalid("fetch output has no parent"))?;
    let sequence = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(
        ".codex-release-fetch-{}-{sequence}",
        std::process::id()
    ));
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(&staging)
        .map_err(|source| io_error("create private fetch staging", source))?;
    Ok(staging)
}

fn cleanup_fetch_staging(staging: &Path) -> Result<(), BuilderError> {
    match std::fs::symlink_metadata(staging) {
        Ok(_) => std::fs::remove_dir_all(staging)
            .map_err(|source| io_error("remove private fetch staging", source)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("inspect private fetch staging", source)),
    }
}

fn fetch(request: &FetchRequest) -> Result<String, BuilderError> {
    validate_fetch_request(request)?;
    let staging = create_fetch_staging(&request.output)?;
    let result = (|| {
        let archive = staging.join(UPSTREAM_ARCHIVE_NAME);
        let output = create_private_file(&archive)?;
        let child_output = output
            .try_clone()
            .map_err(|source| io_error("duplicate fetch archive output", source))?;
        let url = official_archive_url(&request.version);
        let status = Command::new(&request.curl)
            .args([
                "--disable",
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--connect-timeout",
                RELEASE_CONNECT_TIMEOUT_SECONDS,
                "--max-time",
                RELEASE_TRANSFER_TIMEOUT_SECONDS,
                "--max-filesize",
            ])
            .arg(ARCHIVE_MAX_BYTES.to_string())
            .args(["--url", url.as_str()])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::from(child_output))
            .stderr(Stdio::null())
            .status()
            .map_err(|source| io_error("start official upstream archive download", source))?;
        if !status.success() {
            return Err(BuilderError::Tool(
                "official upstream archive download failed",
            ));
        }
        output
            .sync_all()
            .map_err(|source| io_error("sync downloaded upstream archive", source))?;
        let metadata = output
            .metadata()
            .map_err(|source| io_error("inspect downloaded upstream archive", source))?;
        if !metadata.file_type().is_file() {
            return Err(BuilderError::Invalid(
                "downloaded upstream archive is not a regular file",
            ));
        }
        if metadata.len() == 0 {
            return Err(BuilderError::Invalid(
                "downloaded upstream archive is empty",
            ));
        }
        if metadata.len() > ARCHIVE_MAX_BYTES {
            return Err(BuilderError::Invalid(
                "downloaded upstream archive exceeds its byte bound",
            ));
        }
        drop(output);
        let digest = openssl_sha256(&request.openssl, &archive)?;
        rename_noreplace(&archive, &request.output)?;
        sync_directory(
            request
                .output
                .parent()
                .ok_or(BuilderError::Invalid("fetch output has no parent"))?,
            "sync fetched archive parent",
        )?;
        Ok(digest)
    })();
    match (result, cleanup_fetch_staging(&staging)) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(digest), Ok(())) => Ok(digest),
    }
}

#[derive(Debug)]
struct ArchiveSelection {
    raw_runtime: PathBuf,
    code_mode_host: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Regular,
    Directory,
    Pax,
}

#[derive(Debug)]
struct TarHeader {
    path: String,
    size: u64,
    kind: EntryKind,
}

fn parse_octal(field: &[u8]) -> Result<u64, BuilderError> {
    if field.first().is_some_and(|byte| byte & 0x80 != 0) {
        return Err(BuilderError::Archive(
            "base-256 tar numbers are unsupported",
        ));
    }
    let mut value = 0u64;
    let mut saw_digit = false;
    let mut ended = false;
    for byte in field {
        match *byte {
            b'0'..=b'7' if !ended => {
                saw_digit = true;
                value = value
                    .checked_mul(8)
                    .and_then(|value| value.checked_add(u64::from(*byte - b'0')))
                    .ok_or(BuilderError::Archive("tar number overflows"))?;
            }
            0 | b' ' => {
                if saw_digit {
                    ended = true;
                }
            }
            _ => return Err(BuilderError::Archive("tar number is malformed")),
        }
    }
    if saw_digit {
        Ok(value)
    } else {
        Err(BuilderError::Archive("tar number is empty"))
    }
}

fn header_text(field: &[u8]) -> Result<&[u8], BuilderError> {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    if field[end..].iter().any(|byte| *byte != 0) {
        return Err(BuilderError::Archive("tar text field has nonzero suffix"));
    }
    Ok(&field[..end])
}

fn valid_archive_path(path: &str, directory: bool) -> bool {
    if path.is_empty()
        || path.len() > PATH_MAX_BYTES
        || path.starts_with('/')
        || path.contains('\\')
        || path.bytes().any(|byte| byte.is_ascii_control())
    {
        return false;
    }
    let body = if directory {
        let Some(body) = path.strip_suffix('/') else {
            return false;
        };
        body
    } else {
        if path.ends_with('/') {
            return false;
        }
        path
    };
    !body.is_empty()
        && body
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn parse_tar_header(block: &[u8; TAR_BLOCK_BYTES]) -> Result<TarHeader, BuilderError> {
    let expected_checksum = parse_octal(&block[148..156])?;
    let actual_checksum: u64 = block
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            }
        })
        .sum();
    if expected_checksum != actual_checksum {
        return Err(BuilderError::Archive("tar header checksum is invalid"));
    }
    if &block[257..263] != b"ustar\0" || &block[263..265] != b"00" {
        return Err(BuilderError::Archive("tar header is not POSIX ustar"));
    }
    if block[345..500].iter().any(|byte| *byte != 0) {
        return Err(BuilderError::Archive("tar path prefixes are unsupported"));
    }
    if block[157..257].iter().any(|byte| *byte != 0) {
        return Err(BuilderError::Archive("tar link metadata is unsupported"));
    }
    parse_octal(&block[100..108])?;
    parse_octal(&block[108..116])?;
    parse_octal(&block[116..124])?;
    let size = parse_octal(&block[124..136])?;
    parse_octal(&block[136..148])?;
    let path = std::str::from_utf8(header_text(&block[..100])?)
        .map_err(|_| BuilderError::Archive("tar path is not UTF-8"))?
        .to_owned();
    let kind = match block[156] {
        0 | b'0' => EntryKind::Regular,
        b'5' => EntryKind::Directory,
        b'x' => EntryKind::Pax,
        _ => return Err(BuilderError::Archive("tar entry type is unsupported")),
    };
    match kind {
        EntryKind::Pax if path != "././@PaxHeader" => {
            return Err(BuilderError::Archive("PAX header name is unsupported"))
        }
        EntryKind::Pax => {}
        EntryKind::Regular if !valid_archive_path(&path, false) => {
            return Err(BuilderError::Archive("regular-file path is not canonical"))
        }
        EntryKind::Directory if !valid_archive_path(&path, true) || size != 0 => {
            return Err(BuilderError::Archive("directory entry is malformed"))
        }
        _ => {}
    }
    Ok(TarHeader { path, size, kind })
}

fn read_block<R: Read>(reader: &mut R) -> Result<Option<[u8; TAR_BLOCK_BYTES]>, BuilderError> {
    let mut block = [0u8; TAR_BLOCK_BYTES];
    let mut offset = 0;
    while offset < block.len() {
        match reader.read(&mut block[offset..]) {
            Ok(0) if offset == 0 => return Ok(None),
            Ok(0) => return Err(BuilderError::Archive("tar stream ends in a partial block")),
            Ok(read) => offset += read,
            Err(source) => return Err(io_error("read decompressed archive", source)),
        }
    }
    Ok(Some(block))
}

fn read_payload<R: Read>(
    reader: &mut R,
    size: u64,
    mut output: Option<&mut File>,
    capture: bool,
) -> Result<Vec<u8>, BuilderError> {
    let mut captured = if capture {
        Vec::with_capacity(usize::try_from(size).unwrap_or(0))
    } else {
        Vec::new()
    };
    let mut remaining = size;
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        let limit =
            usize::try_from(remaining.min(buffer.len() as u64)).expect("bounded chunk fits usize");
        let read = reader
            .read(&mut buffer[..limit])
            .map_err(|source| io_error("read archive payload", source))?;
        if read == 0 {
            return Err(BuilderError::Archive("archive payload is truncated"));
        }
        if let Some(file) = output.as_deref_mut() {
            file.write_all(&buffer[..read])
                .map_err(|source| io_error("write selected archive payload", source))?;
        }
        if capture {
            captured.extend_from_slice(&buffer[..read]);
        }
        remaining -= read as u64;
    }
    let padding = (TAR_BLOCK_BYTES as u64 - size % TAR_BLOCK_BYTES as u64) % TAR_BLOCK_BYTES as u64;
    if padding != 0 {
        let padding = usize::try_from(padding).expect("tar padding fits usize");
        reader
            .read_exact(&mut buffer[..padding])
            .map_err(|source| io_error("read archive padding", source))?;
        if buffer[..padding].iter().any(|byte| *byte != 0) {
            return Err(BuilderError::Archive("archive payload padding is nonzero"));
        }
    }
    Ok(captured)
}

fn valid_pax_mtime(value: &str) -> bool {
    let mut decimal = false;
    let mut before = 0usize;
    let mut after = 0usize;
    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' if decimal => after += 1,
            b'0'..=b'9' => before += 1,
            b'.' if !decimal && before != 0 => decimal = true,
            _ => return false,
        }
    }
    before != 0 && (!decimal || after != 0)
}

fn parse_pax_payload(bytes: &[u8]) -> Result<(), BuilderError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| BuilderError::Archive("PAX payload is not UTF-8"))?;
    let Some(space) = text.find(' ') else {
        return Err(BuilderError::Archive("PAX record is malformed"));
    };
    let length = text[..space]
        .parse::<usize>()
        .map_err(|_| BuilderError::Archive("PAX record length is invalid"))?;
    if length != bytes.len() || text.as_bytes().last() != Some(&b'\n') {
        return Err(BuilderError::Archive("PAX record length is invalid"));
    }
    let record = &text[space + 1..text.len() - 1];
    let Some((key, value)) = record.split_once('=') else {
        return Err(BuilderError::Archive("PAX record is malformed"));
    };
    if key != "mtime" || !valid_pax_mtime(value) {
        return Err(BuilderError::Archive("PAX semantics are unsupported"));
    }
    Ok(())
}

fn package_json_error() -> BuilderError {
    BuilderError::Archive("codex-package.json is incompatible")
}

struct JsonCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> JsonCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.offset)
            .is_some_and(|byte| matches!(*byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.offset += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        self.skip_whitespace();
        if self.bytes.get(self.offset) == Some(&expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn require(&mut self, expected: u8) -> Result<(), BuilderError> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(package_json_error())
        }
    }

    fn parse_hex_quad(&mut self) -> Result<u16, BuilderError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let byte = *self.bytes.get(self.offset).ok_or_else(package_json_error)?;
            self.offset += 1;
            let digit = match byte {
                b'0'..=b'9' => u16::from(byte - b'0'),
                b'a'..=b'f' => u16::from(byte - b'a' + 10),
                b'A'..=b'F' => u16::from(byte - b'A' + 10),
                _ => return Err(package_json_error()),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn parse_string(&mut self) -> Result<String, BuilderError> {
        self.skip_whitespace();
        if self.bytes.get(self.offset) != Some(&b'"') {
            return Err(package_json_error());
        }
        self.offset += 1;
        let mut output = Vec::new();
        loop {
            let byte = *self.bytes.get(self.offset).ok_or_else(package_json_error)?;
            self.offset += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(output).map_err(|_| package_json_error());
                }
                b'\\' => {
                    let escaped = *self.bytes.get(self.offset).ok_or_else(package_json_error)?;
                    self.offset += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => output.push(escaped),
                        b'b' => output.push(8),
                        b'f' => output.push(12),
                        b'n' => output.push(b'\n'),
                        b'r' => output.push(b'\r'),
                        b't' => output.push(b'\t'),
                        b'u' => {
                            let first = self.parse_hex_quad()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.bytes.get(self.offset) != Some(&b'\\')
                                    || self.bytes.get(self.offset + 1) != Some(&b'u')
                                {
                                    return Err(package_json_error());
                                }
                                self.offset += 2;
                                let second = self.parse_hex_quad()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(package_json_error());
                                }
                                0x10000
                                    + ((u32::from(first) - 0xD800) << 10)
                                    + (u32::from(second) - 0xDC00)
                            } else {
                                if (0xDC00..=0xDFFF).contains(&first) {
                                    return Err(package_json_error());
                                }
                                u32::from(first)
                            };
                            let character =
                                char::from_u32(scalar).ok_or_else(package_json_error)?;
                            let mut encoded = [0u8; 4];
                            output
                                .extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                        }
                        _ => return Err(package_json_error()),
                    }
                }
                0..=0x1f => return Err(package_json_error()),
                _ => output.push(byte),
            }
        }
    }

    fn parse_unsigned_integer(&mut self) -> Result<u64, BuilderError> {
        self.skip_whitespace();
        let start = self.offset;
        match self.bytes.get(self.offset).copied() {
            Some(b'0') => {
                self.offset += 1;
                if self
                    .bytes
                    .get(self.offset)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    return Err(package_json_error());
                }
            }
            Some(b'1'..=b'9') => {
                self.offset += 1;
                while self
                    .bytes
                    .get(self.offset)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    self.offset += 1;
                }
            }
            _ => return Err(package_json_error()),
        }
        if self
            .bytes
            .get(self.offset)
            .is_some_and(|byte| matches!(*byte, b'.' | b'e' | b'E'))
        {
            return Err(package_json_error());
        }
        std::str::from_utf8(&self.bytes[start..self.offset])
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(package_json_error)
    }

    fn skip_number(&mut self) -> Result<(), BuilderError> {
        self.skip_whitespace();
        if self.bytes.get(self.offset) == Some(&b'-') {
            self.offset += 1;
        }
        match self.bytes.get(self.offset).copied() {
            Some(b'0') => {
                self.offset += 1;
                if self
                    .bytes
                    .get(self.offset)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    return Err(package_json_error());
                }
            }
            Some(b'1'..=b'9') => {
                self.offset += 1;
                while self
                    .bytes
                    .get(self.offset)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    self.offset += 1;
                }
            }
            _ => return Err(package_json_error()),
        }
        if self.bytes.get(self.offset) == Some(&b'.') {
            self.offset += 1;
            let fraction = self.offset;
            while self
                .bytes
                .get(self.offset)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.offset += 1;
            }
            if self.offset == fraction {
                return Err(package_json_error());
            }
        }
        if self
            .bytes
            .get(self.offset)
            .is_some_and(|byte| matches!(*byte, b'e' | b'E'))
        {
            self.offset += 1;
            if self
                .bytes
                .get(self.offset)
                .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
            {
                self.offset += 1;
            }
            let exponent = self.offset;
            while self
                .bytes
                .get(self.offset)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.offset += 1;
            }
            if self.offset == exponent {
                return Err(package_json_error());
            }
        }
        Ok(())
    }

    fn skip_literal(&mut self, literal: &[u8]) -> Result<(), BuilderError> {
        self.skip_whitespace();
        if self
            .bytes
            .get(self.offset..self.offset.saturating_add(literal.len()))
            == Some(literal)
        {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(package_json_error())
        }
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), BuilderError> {
        if depth > JSON_DEPTH_MAX {
            return Err(package_json_error());
        }
        self.skip_whitespace();
        match self.bytes.get(self.offset).copied() {
            Some(b'"') => {
                self.parse_string()?;
                Ok(())
            }
            Some(b'{') => self.skip_object(depth + 1),
            Some(b'[') => self.skip_array(depth + 1),
            Some(b't') => self.skip_literal(b"true"),
            Some(b'f') => self.skip_literal(b"false"),
            Some(b'n') => self.skip_literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.skip_number(),
            _ => Err(package_json_error()),
        }
    }

    fn skip_array(&mut self, depth: usize) -> Result<(), BuilderError> {
        self.require(b'[')?;
        if self.consume(b']') {
            return Ok(());
        }
        loop {
            self.skip_value(depth)?;
            if self.consume(b']') {
                return Ok(());
            }
            self.require(b',')?;
        }
    }

    fn skip_object(&mut self, depth: usize) -> Result<(), BuilderError> {
        self.require(b'{')?;
        let mut names = BTreeSet::new();
        if self.consume(b'}') {
            return Ok(());
        }
        loop {
            let name = self.parse_string()?;
            if !names.insert(name) {
                return Err(package_json_error());
            }
            self.require(b':')?;
            self.skip_value(depth)?;
            if self.consume(b'}') {
                return Ok(());
            }
            self.require(b',')?;
        }
    }

    fn finished(&mut self) -> bool {
        self.skip_whitespace();
        self.offset == self.bytes.len()
    }
}

fn validate_package_json(bytes: &[u8], version: &str) -> Result<(), BuilderError> {
    if bytes.len() as u64 > PACKAGE_JSON_MAX_BYTES {
        return Err(package_json_error());
    }

    let mut cursor = JsonCursor::new(bytes);
    cursor.require(b'{')?;
    let mut names = BTreeSet::new();
    let mut layout_version = None;
    let mut package_version = None;
    let mut target = None;
    let mut variant = None;
    let mut entrypoint = None;
    let mut resources_dir = None;
    let mut path_dir = None;

    if !cursor.consume(b'}') {
        loop {
            let name = cursor.parse_string()?;
            if !names.insert(name.clone()) {
                return Err(package_json_error());
            }
            cursor.require(b':')?;
            match name.as_str() {
                "layoutVersion" => layout_version = Some(cursor.parse_unsigned_integer()?),
                "version" => package_version = Some(cursor.parse_string()?),
                "target" => target = Some(cursor.parse_string()?),
                "variant" => variant = Some(cursor.parse_string()?),
                "entrypoint" => entrypoint = Some(cursor.parse_string()?),
                "resourcesDir" => resources_dir = Some(cursor.parse_string()?),
                "pathDir" => path_dir = Some(cursor.parse_string()?),
                _ => cursor.skip_value(1)?,
            }
            if cursor.consume(b'}') {
                break;
            }
            cursor.require(b',')?;
        }
    }

    if !cursor.finished()
        || layout_version != Some(1)
        || package_version.as_deref() != Some(version)
        || target.as_deref() != Some("aarch64-unknown-linux-musl")
        || variant.as_deref() != Some("codex")
        || entrypoint.as_deref() != Some("bin/codex")
        || resources_dir.as_deref() != Some("codex-resources")
        || path_dir.as_deref() != Some("codex-path")
    {
        return Err(package_json_error());
    }
    Ok(())
}

#[cfg(test)]
fn expected_package_json(version: &str) -> Vec<u8> {
    format!(
        concat!(
            "{{\n",
            "  \"layoutVersion\": 1,\n",
            "  \"version\": \"{}\",\n",
            "  \"target\": \"aarch64-unknown-linux-musl\",\n",
            "  \"variant\": \"codex\",\n",
            "  \"entrypoint\": \"bin/codex\",\n",
            "  \"resourcesDir\": \"codex-resources\",\n",
            "  \"pathDir\": \"codex-path\"\n",
            "}}\n"
        ),
        version
    )
    .into_bytes()
}

fn little_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn little_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn little_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn validate_static_aarch64_elf(path: &Path) -> Result<(), BuilderError> {
    let mut file = File::open(path).map_err(|source| io_error("open selected ELF", source))?;
    let file_len = file
        .metadata()
        .map_err(|source| io_error("inspect selected ELF", source))?
        .len();
    let mut header = [0u8; 64];
    file.read_exact(&mut header)
        .map_err(|source| io_error("read selected ELF header", source))?;
    if &header[..4] != b"\x7fELF"
        || header[4] != 2
        || header[5] != 1
        || header[6] != 1
        || little_u16(&header[16..18]) != 2
        || little_u16(&header[18..20]) != 183
        || little_u32(&header[20..24]) != 1
        || little_u16(&header[52..54]) != 64
        || little_u16(&header[54..56]) != 56
    {
        return Err(BuilderError::Archive(
            "selected executable is not a supported static AArch64 ELF",
        ));
    }
    let program_offset = little_u64(&header[32..40]);
    let program_count = u64::from(little_u16(&header[56..58]));
    if program_count == 0 {
        return Err(BuilderError::Archive("selected ELF has no program headers"));
    }
    let program_bytes = program_count
        .checked_mul(56)
        .and_then(|length| program_offset.checked_add(length))
        .filter(|end| *end <= file_len)
        .ok_or(BuilderError::Archive(
            "selected ELF program headers are malformed",
        ))?;
    let _ = program_bytes;
    let mut program_header = [0u8; 56];
    for index in 0..program_count {
        file.seek(SeekFrom::Start(program_offset + index * 56))
            .map_err(|source| io_error("seek selected ELF program header", source))?;
        file.read_exact(&mut program_header)
            .map_err(|source| io_error("read selected ELF program header", source))?;
        if little_u32(&program_header[..4]) == 3 {
            return Err(BuilderError::Archive(
                "selected executable has a PT_INTERP program header",
            ));
        }
    }
    Ok(())
}

fn validate_android_aarch64_core_elf(path: &Path) -> Result<(), BuilderError> {
    let mut file = File::open(path).map_err(|source| io_error("open Core ELF", source))?;
    let file_len = file
        .metadata()
        .map_err(|source| io_error("inspect Core ELF", source))?
        .len();
    let mut header = [0u8; 64];
    file.read_exact(&mut header)
        .map_err(|source| io_error("read Core ELF header", source))?;
    if &header[..4] != b"\x7fELF"
        || header[4] != 2
        || header[5] != 1
        || header[6] != 1
        || little_u16(&header[16..18]) != 3
        || little_u16(&header[18..20]) != 183
        || little_u32(&header[20..24]) != 1
        || little_u16(&header[52..54]) != 64
        || little_u16(&header[54..56]) != 56
    {
        return Err(BuilderError::Invalid(
            "Core artifact is not a supported Android AArch64 PIE ELF",
        ));
    }
    let program_offset = little_u64(&header[32..40]);
    let program_count = u64::from(little_u16(&header[56..58]));
    if program_count == 0 {
        return Err(BuilderError::Invalid("Core ELF has no program headers"));
    }
    program_count
        .checked_mul(56)
        .and_then(|length| program_offset.checked_add(length))
        .filter(|end| *end <= file_len)
        .ok_or(BuilderError::Invalid(
            "Core ELF program headers are malformed",
        ))?;

    let mut interpreter = None;
    let mut program_header = [0u8; 56];
    for index in 0..program_count {
        file.seek(SeekFrom::Start(program_offset + index * 56))
            .map_err(|source| io_error("seek Core ELF program header", source))?;
        file.read_exact(&mut program_header)
            .map_err(|source| io_error("read Core ELF program header", source))?;
        if little_u32(&program_header[..4]) != 3 {
            continue;
        }
        if interpreter.is_some() {
            return Err(BuilderError::Invalid(
                "Core ELF has multiple PT_INTERP program headers",
            ));
        }
        let offset = little_u64(&program_header[8..16]);
        let size = little_u64(&program_header[32..40]);
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= file_len)
            .ok_or(BuilderError::Invalid("Core ELF interpreter is malformed"))?;
        let size = usize::try_from(size)
            .map_err(|_| BuilderError::Invalid("Core ELF interpreter is malformed"))?;
        let mut value = vec![0u8; size];
        file.seek(SeekFrom::Start(offset))
            .map_err(|source| io_error("seek Core ELF interpreter", source))?;
        file.read_exact(&mut value)
            .map_err(|source| io_error("read Core ELF interpreter", source))?;
        let _ = end;
        interpreter = Some(value);
    }
    if interpreter.as_deref() != Some(ANDROID_AARCH64_INTERPRETER) {
        return Err(BuilderError::Invalid(
            "Core artifact does not use the Android AArch64 dynamic linker",
        ));
    }
    Ok(())
}

fn validate_android_aarch64_manager_elf(path: &Path) -> Result<(), BuilderError> {
    let mut file = File::open(path).map_err(|source| io_error("open Manager ELF", source))?;
    let file_len = file
        .metadata()
        .map_err(|source| io_error("inspect Manager ELF", source))?
        .len();
    let mut header = [0u8; 64];
    file.read_exact(&mut header)
        .map_err(|source| io_error("read Manager ELF header", source))?;
    if &header[..4] != b"\x7fELF"
        || header[4] != 2
        || header[5] != 1
        || header[6] != 1
        || little_u16(&header[16..18]) != 3
        || little_u16(&header[18..20]) != 183
        || little_u32(&header[20..24]) != 1
        || little_u16(&header[52..54]) != 64
        || little_u16(&header[54..56]) != 56
    {
        return Err(BuilderError::Invalid(
            "Manager artifact is not a supported Android AArch64 PIE ELF",
        ));
    }
    let program_offset = little_u64(&header[32..40]);
    let program_count = u64::from(little_u16(&header[56..58]));
    if program_count == 0 {
        return Err(BuilderError::Invalid("Manager ELF has no program headers"));
    }
    program_count
        .checked_mul(56)
        .and_then(|length| program_offset.checked_add(length))
        .filter(|end| *end <= file_len)
        .ok_or(BuilderError::Invalid(
            "Manager ELF program headers are malformed",
        ))?;

    let mut interpreter = None;
    let mut program_header = [0u8; 56];
    for index in 0..program_count {
        file.seek(SeekFrom::Start(program_offset + index * 56))
            .map_err(|source| io_error("seek Manager ELF program header", source))?;
        file.read_exact(&mut program_header)
            .map_err(|source| io_error("read Manager ELF program header", source))?;
        if little_u32(&program_header[..4]) != 3 {
            continue;
        }
        if interpreter.is_some() {
            return Err(BuilderError::Invalid(
                "Manager ELF has multiple PT_INTERP program headers",
            ));
        }
        let offset = little_u64(&program_header[8..16]);
        let size = little_u64(&program_header[32..40]);
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= file_len)
            .ok_or(BuilderError::Invalid(
                "Manager ELF interpreter is malformed",
            ))?;
        let size = usize::try_from(size)
            .map_err(|_| BuilderError::Invalid("Manager ELF interpreter is malformed"))?;
        let mut value = vec![0u8; size];
        file.seek(SeekFrom::Start(offset))
            .map_err(|source| io_error("seek Manager ELF interpreter", source))?;
        file.read_exact(&mut value)
            .map_err(|source| io_error("read Manager ELF interpreter", source))?;
        let _ = end;
        interpreter = Some(value);
    }
    if interpreter.as_deref() != Some(ANDROID_AARCH64_INTERPRETER) {
        return Err(BuilderError::Invalid(
            "Manager artifact does not use the Android AArch64 dynamic linker",
        ));
    }
    Ok(())
}

fn parse_archive<R: Read>(
    reader: &mut R,
    staging: &Path,
    version: &str,
) -> Result<ArchiveSelection, BuilderError> {
    let raw_runtime = staging.join(".raw-runtime");
    let code_mode_host = staging.join("codex-code-mode-host");
    let mut seen = BTreeSet::new();
    let mut logical_entries = 0usize;
    let mut total_payload = 0u64;
    let mut pending_pax = false;
    let mut package_json = None;

    loop {
        let block =
            read_block(reader)?.ok_or(BuilderError::Archive("tar stream has no end marker"))?;
        if block.iter().all(|byte| *byte == 0) {
            if pending_pax {
                return Err(BuilderError::Archive("PAX header has no following entry"));
            }
            let second = read_block(reader)?
                .ok_or(BuilderError::Archive("tar stream has only one zero block"))?;
            if second.iter().any(|byte| *byte != 0) {
                return Err(BuilderError::Archive("tar stream has only one zero block"));
            }
            while let Some(trailing) = read_block(reader)? {
                if trailing.iter().any(|byte| *byte != 0) {
                    return Err(BuilderError::Archive(
                        "tar stream has nonzero trailing content",
                    ));
                }
            }
            break;
        }

        let header = parse_tar_header(&block)?;
        if header.kind == EntryKind::Pax {
            if pending_pax || header.size == 0 || header.size > PAX_PAYLOAD_MAX_BYTES {
                return Err(BuilderError::Archive("PAX header is malformed"));
            }
            let payload = read_payload(reader, header.size, None, true)?;
            parse_pax_payload(&payload)?;
            pending_pax = true;
            continue;
        }

        logical_entries += 1;
        if logical_entries > LOGICAL_ENTRY_MAX {
            return Err(BuilderError::Archive(
                "archive entry count exceeds its bound",
            ));
        }
        if REQUIRED_ARCHIVE_FILES.contains(&header.path.as_str())
            && header.kind != EntryKind::Regular
        {
            return Err(BuilderError::Archive(
                "required archive entry is not a regular file",
            ));
        }
        if !seen.insert(header.path.clone()) {
            return Err(BuilderError::Archive("archive path is duplicated"));
        }
        pending_pax = false;

        if header.kind == EntryKind::Directory {
            continue;
        }
        if header.path == "codex-package.json" && header.size > PACKAGE_JSON_MAX_BYTES {
            return Err(package_json_error());
        }
        if header.size > ENTRY_MAX_BYTES {
            return Err(BuilderError::Archive("archive file exceeds its byte bound"));
        }
        total_payload = total_payload
            .checked_add(header.size)
            .filter(|total| *total <= PAYLOAD_MAX_BYTES)
            .ok_or(BuilderError::Archive(
                "archive payload exceeds its byte bound",
            ))?;

        match header.path.as_str() {
            "bin/codex" => {
                let mut file = create_private_file(&raw_runtime)?;
                read_payload(reader, header.size, Some(&mut file), false)?;
                file.sync_all()
                    .map_err(|source| io_error("sync selected raw runtime", source))?;
            }
            "bin/codex-code-mode-host" => {
                let mut file = create_private_file(&code_mode_host)?;
                read_payload(reader, header.size, Some(&mut file), false)?;
                file.sync_all()
                    .map_err(|source| io_error("sync selected code-mode host", source))?;
            }
            "codex-package.json" => {
                package_json = Some(read_payload(reader, header.size, None, true)?);
            }
            _ => {
                read_payload(reader, header.size, None, false)?;
            }
        }
    }

    if REQUIRED_ARCHIVE_FILES
        .iter()
        .any(|path| !seen.contains(*path))
    {
        return Err(BuilderError::Archive("archive layout is incomplete"));
    }
    validate_package_json(
        package_json
            .as_deref()
            .ok_or(BuilderError::Archive("archive layout is incomplete"))?,
        version,
    )?;
    validate_static_aarch64_elf(&raw_runtime)?;
    validate_static_aarch64_elf(&code_mode_host)?;
    Ok(ArchiveSelection {
        raw_runtime,
        code_mode_host,
    })
}

fn select_archive(
    request: &BuildRequest,
    archive: &Path,
    staging: &Path,
) -> Result<ArchiveSelection, BuilderError> {
    let mut child = Command::new(&request.gzip)
        .args(["-dc", "--"])
        .arg(archive)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|source| io_error("start gzip", source))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or(BuilderError::Tool("gzip stdout is unavailable"))?;
    let selected = parse_archive(&mut stdout, staging, &request.version);
    drop(stdout);
    if selected.is_err() {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|source| io_error("wait for gzip", source))?;
    match selected {
        Err(error) => Err(error),
        Ok(_) if !status.success() => Err(BuilderError::Tool("gzip decompression failed")),
        Ok(selected) => Ok(selected),
    }
}

fn occurrence_offsets(bytes: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > bytes.len() {
        return Vec::new();
    }
    (0..=bytes.len() - pattern.len())
        .filter(|offset| bytes[*offset..].starts_with(pattern))
        .collect()
}

#[derive(Debug)]
struct AdaptedGeneration {
    raw_runtime_sha256: String,
    runtime_sha256: String,
    code_mode_host_sha256: String,
    browser_open_helper_sha256: String,
    browser_manual_helper_sha256: String,
    changed_bytes: usize,
}

fn set_mode(path: &Path, mode: u32, operation: &'static str) -> Result<(), BuilderError> {
    let mut permissions = std::fs::symlink_metadata(path)
        .map_err(|source| io_error(operation, source))?
        .permissions();
    permissions.set_mode(mode);
    std::fs::set_permissions(path, permissions).map_err(|source| io_error(operation, source))
}

fn sync_directory(path: &Path, operation: &'static str) -> Result<(), BuilderError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io_error(operation, source))
}

fn write_browser_helper(
    path: &Path,
    contents: &[u8],
    openssl: &Path,
) -> Result<String, BuilderError> {
    if contents.len() as u64 > BROWSER_HELPER_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "browser helper exceeds its byte bound",
        ));
    }
    let mut file = create_private_file(path)?;
    file.write_all(contents)
        .map_err(|source| io_error("write browser helper", source))?;
    file.sync_all()
        .map_err(|source| io_error("sync browser helper", source))?;
    drop(file);
    set_mode(path, 0o755, "set browser helper mode")?;
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync browser helper after final mode", source))?;
    openssl_sha256(openssl, path)
}

fn browser_helper_relative_paths(r10_bridge: bool) -> (&'static str, &'static str) {
    if r10_bridge {
        ("helpers/0", "helpers/1")
    } else {
        ("browser/open/curl", "browser/manual/curl")
    }
}

fn create_browser_helpers(
    staging: &Path,
    openssl: &Path,
    r10_bridge: bool,
) -> Result<(String, String), BuilderError> {
    let (open_relative, manual_relative) = browser_helper_relative_paths(r10_bridge);
    if r10_bridge {
        create_private_dir(&staging.join("helpers"))?;
    } else {
        let browser = staging.join("browser");
        create_private_dir(&browser)?;
        create_private_dir(&browser.join("open"))?;
        create_private_dir(&browser.join("manual"))?;
    }
    let open_sha256 = write_browser_helper(
        &staging.join(open_relative),
        TERMUX_BROWSER_OPEN_HELPER,
        openssl,
    )?;
    let manual_sha256 = write_browser_helper(
        &staging.join(manual_relative),
        TERMUX_BROWSER_MANUAL_HELPER,
        openssl,
    )?;
    if r10_bridge {
        sync_directory(&staging.join("helpers"), "sync R10 bridge helper directory")?;
    } else {
        sync_directory(
            &staging.join("browser/open"),
            "sync browser open helper directory",
        )?;
        sync_directory(
            &staging.join("browser/manual"),
            "sync browser manual helper directory",
        )?;
        sync_directory(&staging.join("browser"), "sync browser helper root")?;
    }
    Ok((open_sha256, manual_sha256))
}

fn adapt_selected_runtime(
    request: &BuildRequest,
    staging: &Path,
    selected: ArchiveSelection,
) -> Result<AdaptedGeneration, BuilderError> {
    let raw_runtime_sha256 = openssl_sha256(&request.openssl, &selected.raw_runtime)?;
    let mut runtime = std::fs::read(&selected.raw_runtime)
        .map_err(|source| io_error("read selected raw runtime", source))?;
    let mut selected_offsets = Vec::new();
    for (source, replacement, expected) in PATCHES {
        if source.len() != replacement.len() {
            return Err(BuilderError::Invalid(
                "patch policy lengths are inconsistent",
            ));
        }
        let source_offsets = occurrence_offsets(&runtime, source);
        if source_offsets.len() != expected || !occurrence_offsets(&runtime, replacement).is_empty()
        {
            return Err(BuilderError::Archive(
                "runtime patch source occurrences do not match policy",
            ));
        }
        for offset in source_offsets {
            selected_offsets.push((offset, source, replacement));
        }
    }
    selected_offsets.sort_by_key(|(offset, _, _)| *offset);
    for pair in selected_offsets.windows(2) {
        if pair[0].0 + pair[0].1.len() > pair[1].0 {
            return Err(BuilderError::Archive("runtime patch positions overlap"));
        }
    }
    let mut changed_bytes = 0usize;
    for (offset, source, replacement) in &selected_offsets {
        changed_bytes += source
            .iter()
            .zip(replacement.iter())
            .filter(|(before, after)| before != after)
            .count();
        runtime[*offset..*offset + source.len()].copy_from_slice(replacement);
    }
    for (source, replacement, expected) in PATCHES {
        if !occurrence_offsets(&runtime, source).is_empty()
            || occurrence_offsets(&runtime, replacement).len() != expected
        {
            return Err(BuilderError::Archive(
                "adapted runtime does not match patch policy",
            ));
        }
    }
    if changed_bytes != 54 {
        return Err(BuilderError::Invalid(
            "patch policy changed-byte count is inconsistent",
        ));
    }

    let runtime_path = staging.join("runtime");
    let mut runtime_file = create_private_file(&runtime_path)?;
    runtime_file
        .write_all(&runtime)
        .map_err(|source| io_error("write adapted runtime", source))?;
    runtime_file
        .sync_all()
        .map_err(|source| io_error("sync adapted runtime", source))?;
    drop(runtime_file);
    set_mode(&runtime_path, 0o755, "set adapted runtime mode")?;
    File::open(&runtime_path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync adapted runtime after final mode", source))?;
    set_mode(&selected.code_mode_host, 0o755, "set code-mode-host mode")?;
    File::open(&selected.code_mode_host)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync code-mode host", source))?;

    let runtime_sha256 = openssl_sha256(&request.openssl, &runtime_path)?;
    let code_mode_host_sha256 = openssl_sha256(&request.openssl, &selected.code_mode_host)?;
    let (browser_open_helper_sha256, browser_manual_helper_sha256) = create_browser_helpers(
        staging,
        &request.openssl,
        request.creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA,
    )?;
    std::fs::remove_file(&selected.raw_runtime)
        .map_err(|source| io_error("remove selected raw runtime", source))?;

    Ok(AdaptedGeneration {
        raw_runtime_sha256,
        runtime_sha256,
        code_mode_host_sha256,
        browser_open_helper_sha256,
        browser_manual_helper_sha256,
        changed_bytes,
    })
}

fn write_generation_descriptor(
    request: &BuildRequest,
    staging: &Path,
    adapted: &AdaptedGeneration,
    core_sha256: &str,
    manager_sha256: Option<&str>,
) -> Result<(), BuilderError> {
    let patch_report = format!(
        "{PATCH_POLICY_ID};archive_sha256={};raw_runtime_sha256={};runtime_sha256={};code_mode_host_sha256={};source_counts=2,1,1,1;changed_bytes={}",
        request.archive_sha256,
        adapted.raw_runtime_sha256,
        adapted.runtime_sha256,
        adapted.code_mode_host_sha256,
        adapted.changed_bytes
    );
    let upstream_doctor = if request.legacy_activation_doctor_unsupported {
        "unsupported"
    } else {
        "supported"
    };
    let descriptor = format!(
        concat!(
            "{}\n",
            "generation_id\t{}\n",
            "upstream_package_identity\t{}\n",
            "upstream_package_version\t{}\n",
            "source_artifact_digest\t{}\n",
            "expected_platform\tandroid\n",
            "expected_architecture\taarch64\n",
            "patch_policy_id\t{}\n",
            "patch_report\t{}\n",
            "runtime_digest\t{}\n",
            "core_artifact_digest\t{}\n",
            "manager_artifact_digest\t{}\n",
            "core_api_identity\t{}\n",
            "persistent_schema_identity\t{}\n",
            "qualification\tqualified\n",
            "creation_metadata\t{}\n",
            "upstream_doctor\t{}\n",
            "helper_count\t2\n",
            "helper\t{}\t{}\n",
            "helper\t{}\t{}\n"
        ),
        GENERATION_FORMAT,
        request.generation_id,
        PACKAGE_IDENTITY,
        request.version,
        request.archive_sha256,
        PATCH_POLICY_ID,
        patch_report,
        adapted.runtime_sha256,
        core_sha256,
        manager_sha256.unwrap_or("-"),
        CORE_API_IDENTITY,
        PERSISTENT_SCHEMA_IDENTITY,
        request.creation_metadata,
        upstream_doctor,
        TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
        adapted.browser_open_helper_sha256,
        TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
        adapted.browser_manual_helper_sha256
    );
    let path = staging.join("generation.meta");
    let mut file = create_private_file(&path)?;
    file.write_all(descriptor.as_bytes())
        .map_err(|source| io_error("write generation descriptor", source))?;
    file.sync_all()
        .map_err(|source| io_error("sync generation descriptor", source))?;
    drop(file);
    set_mode(&path, 0o644, "set generation descriptor mode")?;
    File::open(&path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync generation descriptor after final mode", source))?;
    Ok(())
}

fn rename_noreplace(source: &Path, destination: &Path) -> Result<(), BuilderError> {
    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
    #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
    unsafe extern "C" {
        fn renameat2(
            olddirfd: i32,
            oldpath: *const std::ffi::c_char,
            newdirfd: i32,
            newpath: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| BuilderError::Invalid("staging path contains NUL"))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| BuilderError::Invalid("output path contains NUL"))?;
    // Android API 24 does not export the renameat2 libc symbol, although the arm64
    // kernel ABI provides the syscall. Preserve atomic no-replace publication through
    // bionic's syscall wrapper without raising the Android API level.
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    let result = unsafe {
        unsafe extern "C" {
            fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
        }
        const SYS_RENAMEAT2_AARCH64: std::ffi::c_long = 276;
        syscall(
            SYS_RENAMEAT2_AARCH64,
            AT_FDCWD as std::ffi::c_long,
            source.as_ptr() as std::ffi::c_long,
            AT_FDCWD as std::ffi::c_long,
            destination.as_ptr() as std::ffi::c_long,
            RENAME_NOREPLACE as std::ffi::c_long,
        ) as i32
    };
    #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
    // SAFETY: both C strings are NUL-terminated and valid for the duration of the call.
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            source.as_ptr(),
            AT_FDCWD,
            destination.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io_error(
            "publish complete unsigned generation",
            io::Error::last_os_error(),
        ))
    }
}

fn complete_and_publish(
    request: &BuildRequest,
    staging: &Path,
    selected: ArchiveSelection,
    core_sha256: &str,
    manager_sha256: Option<&str>,
) -> Result<(), BuilderError> {
    let adapted = adapt_selected_runtime(request, staging, selected)?;
    rename_noreplace(&staging.join(".core-artifact"), &staging.join("core"))?;
    set_mode(&staging.join("core"), 0o755, "set generation Core mode")?;
    File::open(staging.join("core"))
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync generation Core", source))?;
    if manager_sha256.is_some() {
        let manager_source = staging.join(".manager-artifact");
        let manager_destination = staging.join("manager");
        rename_noreplace(&manager_source, &manager_destination)?;
        set_mode(&manager_destination, 0o755, "set generation Manager mode")?;
        File::open(&manager_destination)
            .and_then(|file| file.sync_all())
            .map_err(|source| io_error("sync generation Manager", source))?;
    }
    write_generation_descriptor(request, staging, &adapted, core_sha256, manager_sha256)?;
    sync_directory(staging, "sync complete unsigned generation")?;
    rename_noreplace(staging, &request.output)?;
    sync_directory(
        request
            .output
            .parent()
            .ok_or(BuilderError::Invalid("output directory has no parent"))?,
        "sync published output parent",
    )?;
    Ok(())
}

fn cleanup_staging(staging: &Path) -> Result<(), BuilderError> {
    match std::fs::symlink_metadata(staging) {
        Ok(_) => std::fs::remove_dir_all(staging)
            .map_err(|source| io_error("remove private builder staging", source)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("inspect private builder staging", source)),
    }
}

fn build(request: &BuildRequest) -> Result<(), BuilderError> {
    validate_request(request)?;
    let staging = create_staging(&request.output)?;
    let result = (|| {
        let core_sha256 = snapshot_core_artifact(request, &staging)?;
        let manager_sha256 = snapshot_manager_artifact(request, &staging)?;
        if manager_sha256.is_some() {
            let manager_snapshot = staging.join(".manager-artifact");
            if request.defer_manager_probe {
                validate_android_aarch64_manager_elf(&manager_snapshot)?;
                write_deferred_manager_probe_marker(&staging)?;
            } else {
                qualify_manager_artifact(&manager_snapshot, &staging)?;
            }
        }
        let archive = snapshot_archive(request, &staging)?;
        let selected = select_archive(request, &archive, &staging)?;
        std::fs::remove_file(&archive)
            .map_err(|source| io_error("remove upstream archive snapshot", source))?;
        complete_and_publish(
            request,
            &staging,
            selected,
            &core_sha256,
            manager_sha256.as_deref(),
        )
    })();
    match (result, cleanup_staging(&staging)) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

/// Runs the bounded official-archive acquisition without going through the
/// human-facing command dispatcher. Core uses this for its local update path.
pub fn fetch_archive(
    version: &str,
    curl: &Path,
    openssl: &Path,
    output: &Path,
) -> Result<String, String> {
    fetch(&FetchRequest {
        version: version.to_owned(),
        curl: curl.to_owned(),
        openssl: openssl.to_owned(),
        output: output.to_owned(),
    })
    .map_err(|error| error.to_string())
}

/// Runs the bounded upstream adaptation into one absent unsigned generation.
#[allow(clippy::too_many_arguments)]
pub fn build_generation(
    version: &str,
    archive: &Path,
    archive_sha256: &str,
    generation_id: &str,
    core: &Path,
    creation_metadata: &str,
    gzip: &Path,
    openssl: &Path,
    output: &Path,
) -> Result<(), String> {
    build_generation_with_manager(
        version,
        archive,
        archive_sha256,
        generation_id,
        core,
        None,
        creation_metadata,
        gzip,
        openssl,
        output,
    )
}

/// Runs the bounded upstream adaptation with an optional qualified Manager
/// artifact carried into the new generation.
#[allow(clippy::too_many_arguments)]
pub fn build_generation_with_manager(
    version: &str,
    archive: &Path,
    archive_sha256: &str,
    generation_id: &str,
    core: &Path,
    manager: Option<&Path>,
    creation_metadata: &str,
    gzip: &Path,
    openssl: &Path,
    output: &Path,
) -> Result<(), String> {
    build(&BuildRequest {
        version: version.to_owned(),
        archive: archive.to_owned(),
        archive_sha256: archive_sha256.to_owned(),
        generation_id: generation_id.to_owned(),
        core: core.to_owned(),
        manager: manager.map(Path::to_owned),
        defer_manager_probe: false,
        legacy_activation_doctor_unsupported: false,
        creation_metadata: creation_metadata.to_owned(),
        gzip: gzip.to_owned(),
        openssl: openssl.to_owned(),
        output: output.to_owned(),
    })
    .map_err(|error| error.to_string())
}

/// Signs one qualified generation into a complete local publication tree.
pub fn publish_generation(
    generation: &Path,
    release_sequence: &str,
    release_base: &str,
    private_key: &Path,
    openssl: &Path,
    output: &Path,
) -> Result<String, String> {
    publish(&PublishRequest {
        generation: generation.to_owned(),
        release_sequence: release_sequence.to_owned(),
        release_base: release_base.to_owned(),
        private_key: private_key.to_owned(),
        openssl: openssl.to_owned(),
        output: output.to_owned(),
    })
    .map_err(|error| error.to_string())
}

pub fn run_from_args<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match args.first().and_then(|argument| argument.to_str()) {
        Some("fetch") => {
            let request = match parse_fetch_request(args) {
                Ok(request) => request,
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    return 2;
                }
            };
            match fetch(&request) {
                Ok(digest) => {
                    println!("archive_sha256\t{digest}");
                    0
                }
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    1
                }
            }
        }
        Some("build") => {
            let request = match parse_request(args) {
                Ok(request) => request,
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    return 2;
                }
            };
            match build(&request) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    1
                }
            }
        }
        Some("publish") => {
            let request = match parse_publish_request(args) {
                Ok(request) => request,
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    return 2;
                }
            };
            match publish(&request) {
                Ok(generation_id) => {
                    println!("generation_id\t{generation_id}");
                    0
                }
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    1
                }
            }
        }
        _ => {
            eprintln!("codex-release-builder: {USAGE}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Clone)]
    struct TestEntry {
        path: String,
        kind: u8,
        data: Vec<u8>,
        pax_key: Option<&'static str>,
        declared_size: Option<u64>,
    }

    impl TestEntry {
        fn directory(path: &str) -> Self {
            Self {
                path: path.to_owned(),
                kind: b'5',
                data: Vec::new(),
                pax_key: Some("mtime"),
                declared_size: None,
            }
        }

        fn file(path: &str, data: Vec<u8>) -> Self {
            Self {
                path: path.to_owned(),
                kind: b'0',
                data,
                pax_key: Some("mtime"),
                declared_size: None,
            }
        }
    }

    struct Fixture {
        root: PathBuf,
        request: BuildRequest,
        raw_runtime: Vec<u8>,
        code_mode_host: Vec<u8>,
    }

    impl Fixture {
        fn remove(self) {
            std::fs::remove_dir_all(self.root).unwrap();
        }
    }

    fn test_root(label: &str) -> PathBuf {
        let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-release-builder-{}-{sequence}-{label}",
            std::process::id()
        ));
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(&root).unwrap();
        root
    }

    fn find_tool(name: &str) -> PathBuf {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .map(|directory| directory.join(name))
            .find(|path| {
                std::fs::metadata(path).is_ok_and(|metadata| {
                    metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0
                })
            })
            .unwrap_or_else(|| panic!("required test tool is unavailable: {name}"))
    }

    fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }

    fn write_fetch_curl(path: &Path, log: &Path, source: &Path, exit_code: i32) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = find_tool("sh");
        let cat = find_tool("cat");
        let shell = shell.to_str().expect("test shell path must be UTF-8");
        let cat = cat.to_str().expect("test cat path must be UTF-8");
        let log = shell_quote(log.to_str().expect("test log path must be UTF-8"));
        let source = shell_quote(source.to_str().expect("test source path must be UTF-8"));
        std::fs::write(
            path,
            format!(
                r#"#!{shell}
if [ "${{HOME+x}}" = x ] || [ "${{CURL_HOME+x}}" = x ] || [ "${{HTTP_PROXY+x}}" = x ] || [ "${{HTTPS_PROXY+x}}" = x ]; then
  exit 97
fi
: > {log}
for argument in "$@"; do
  printf '%s\n' "$argument" >> {log}
done
if [ {exit_code} -ne 0 ]; then
  exit {exit_code}
fi
{cat} {source}
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    fn no_fetch_staging(root: &Path) -> bool {
        std::fs::read_dir(root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-release-fetch-")
        })
    }

    fn fake_elf(interpreter: bool) -> Vec<u8> {
        let mut bytes = vec![0u8; 120];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&183u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        bytes[64..68].copy_from_slice(&(if interpreter { 3u32 } else { 1u32 }).to_le_bytes());
        bytes
    }

    fn fake_core_elf(interpreter: &[u8]) -> Vec<u8> {
        let interpreter_offset = 120usize;
        let mut bytes = vec![0u8; interpreter_offset + interpreter.len()];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&3u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&183u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        bytes[64..68].copy_from_slice(&3u32.to_le_bytes());
        bytes[72..80].copy_from_slice(&(interpreter_offset as u64).to_le_bytes());
        bytes[96..104].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[104..112].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[interpreter_offset..].copy_from_slice(interpreter);
        bytes
    }

    fn fake_runtime() -> Vec<u8> {
        let mut bytes = fake_elf(false);
        for value in [
            "/etc/resolv.conf",
            "/etc/codex/managed_config.toml",
            "/etc/codex/config.toml",
            "/etc/codex/requirements.toml",
            "/etc/resolv.conf",
        ] {
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    fn happy_entries(version: &str) -> Vec<TestEntry> {
        let runtime = fake_runtime();
        let host = fake_elf(false);
        vec![
            TestEntry::directory("bin/"),
            TestEntry::file("bin/codex", runtime),
            TestEntry::file("bin/codex-code-mode-host", host),
            TestEntry::file("codex-package.json", expected_package_json(version)),
            TestEntry::directory("codex-path/"),
            TestEntry::file("codex-path/rg", b"unused-rg".to_vec()),
            TestEntry::directory("codex-resources/"),
            TestEntry::file("codex-resources/bwrap", b"unused-bwrap".to_vec()),
            TestEntry::directory("codex-resources/zsh/"),
            TestEntry::directory("codex-resources/zsh/bin/"),
            TestEntry::file("codex-resources/zsh/bin/zsh", b"unused-zsh".to_vec()),
        ]
    }

    fn write_octal(field: &mut [u8], value: u64) {
        field.fill(b'0');
        let value = format!("{value:o}");
        assert!(value.len() < field.len());
        let start = field.len() - 1 - value.len();
        field[start..start + value.len()].copy_from_slice(value.as_bytes());
        field[field.len() - 1] = 0;
    }

    fn tar_header(path: &str, kind: u8, size: u64) -> [u8; TAR_BLOCK_BYTES] {
        assert!(path.len() <= 100);
        let mut header = [0u8; TAR_BLOCK_BYTES];
        header[..path.len()].copy_from_slice(path.as_bytes());
        write_octal(
            &mut header[100..108],
            if kind == b'5' { 0o755 } else { 0o644 },
        );
        write_octal(&mut header[108..116], 1001);
        write_octal(&mut header[116..124], 1001);
        write_octal(&mut header[124..136], size);
        write_octal(&mut header[136..148], 1_787_793_845);
        header[148..156].fill(b' ');
        header[156] = kind;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[265..271].copy_from_slice(b"runner");
        header[297..303].copy_from_slice(b"runner");
        let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
        let checksum = format!("{checksum:06o}");
        header[148..154].copy_from_slice(checksum.as_bytes());
        header[154] = 0;
        header[155] = b' ';
        header
    }

    fn pax_record(key: &str) -> Vec<u8> {
        let body = format!("{key}=1787793845.2340834\n");
        let mut length = body.len() + 2;
        loop {
            let next = body.len() + length.to_string().len() + 1;
            if next == length {
                break;
            }
            length = next;
        }
        format!("{length} {body}").into_bytes()
    }

    fn append_payload(tar: &mut Vec<u8>, data: &[u8]) {
        tar.extend_from_slice(data);
        let padding = (TAR_BLOCK_BYTES - data.len() % TAR_BLOCK_BYTES) % TAR_BLOCK_BYTES;
        tar.resize(tar.len() + padding, 0);
    }

    fn make_tar(entries: &[TestEntry], corrupt_header: bool) -> Vec<u8> {
        let mut tar = Vec::new();
        for entry in entries {
            if let Some(key) = entry.pax_key {
                let pax = pax_record(key);
                tar.extend_from_slice(&tar_header("././@PaxHeader", b'x', pax.len() as u64));
                append_payload(&mut tar, &pax);
            }
            tar.extend_from_slice(&tar_header(
                &entry.path,
                entry.kind,
                entry.declared_size.unwrap_or(entry.data.len() as u64),
            ));
            append_payload(&mut tar, &entry.data);
        }
        tar.resize(tar.len() + TAR_BLOCK_BYTES * 2, 0);
        if corrupt_header {
            tar[TAR_BLOCK_BYTES * 2] ^= 1;
        }
        tar
    }

    fn gzip_tar(gzip: &Path, tar: &[u8], archive: &Path) {
        let output = File::create(archive).unwrap();
        let mut child = Command::new(gzip)
            .args(["-cn"])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(output)
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(tar).unwrap();
        assert!(child.wait().unwrap().success());
    }

    fn write_manager_probe(path: &Path, body: &str) {
        let shell = find_tool("sh");
        let script = format!("#!{}\n{}", shell.display(), body);
        std::fs::write(path, script).unwrap();
        set_mode(path, 0o755, "set test Manager probe mode").unwrap();
    }

    fn write_valid_manager_probe(path: &Path) {
        write_manager_probe(
            path,
            "if [ \"$#\" -ne 1 ] || [ \"$1\" != \"--artifact-probe\" ]; then exit 9; fi\n\
             printf '%s\\n' 'codex-manager-artifact-v1' 'core_api=codex-manager-core-v1'\n",
        );
    }

    fn fixture(label: &str, entries: Vec<TestEntry>, corrupt_header: bool) -> Fixture {
        let root = test_root(label);
        let gzip = find_tool("gzip");
        let openssl = find_tool("openssl");
        let archive = root.join("codex-package-aarch64-unknown-linux-musl.tar.gz");
        gzip_tar(&gzip, &make_tar(&entries, corrupt_header), &archive);
        let core = root.join("codex-core");
        std::fs::write(&core, fake_core_elf(ANDROID_AARCH64_INTERPRETER)).unwrap();
        set_mode(&core, 0o700, "set test Core mode").unwrap();
        let archive_sha256 = openssl_sha256(&openssl, &archive).unwrap();
        let raw_runtime = entries
            .iter()
            .find(|entry| entry.path == "bin/codex")
            .unwrap()
            .data
            .clone();
        let code_mode_host = entries
            .iter()
            .find(|entry| entry.path == "bin/codex-code-mode-host")
            .unwrap()
            .data
            .clone();
        Fixture {
            request: BuildRequest {
                version: "0.150.1".to_owned(),
                archive,
                archive_sha256,
                generation_id: "test-generation".to_owned(),
                core,
                manager: None,
                defer_manager_probe: false,
                legacy_activation_doctor_unsupported: false,
                creation_metadata: "test-fixture".to_owned(),
                gzip,
                openssl,
                output: root.join("unsigned-generation"),
            },
            root,
            raw_runtime,
            code_mode_host,
        }
    }

    fn request_args(request: &BuildRequest) -> Vec<OsString> {
        let mut args = vec![
            "build".into(),
            "--version".into(),
            request.version.clone().into(),
            "--archive".into(),
            request.archive.as_os_str().to_owned(),
            "--archive-sha256".into(),
            request.archive_sha256.clone().into(),
            "--generation-id".into(),
            request.generation_id.clone().into(),
            "--core".into(),
            request.core.as_os_str().to_owned(),
            "--creation-metadata".into(),
            request.creation_metadata.clone().into(),
            "--gzip".into(),
            request.gzip.as_os_str().to_owned(),
            "--openssl".into(),
            request.openssl.as_os_str().to_owned(),
            "--output".into(),
            request.output.as_os_str().to_owned(),
        ];
        if let Some(manager) = request.manager.as_ref() {
            args.splice(
                11..11,
                [OsString::from("--manager"), manager.as_os_str().to_owned()],
            );
        }
        if request.defer_manager_probe {
            args.push(OsString::from("--defer-manager-probe"));
        }
        if request.legacy_activation_doctor_unsupported {
            args.push(OsString::from("--legacy-activation-doctor-unsupported"));
        }
        args
    }

    fn fetch_args(request: &FetchRequest) -> Vec<OsString> {
        vec![
            "fetch".into(),
            "--version".into(),
            request.version.clone().into(),
            "--curl".into(),
            request.curl.as_os_str().to_owned(),
            "--openssl".into(),
            request.openssl.as_os_str().to_owned(),
            "--output".into(),
            request.output.as_os_str().to_owned(),
        ]
    }

    fn publish_args(request: &PublishRequest) -> Vec<OsString> {
        vec![
            "publish".into(),
            "--generation".into(),
            request.generation.as_os_str().to_owned(),
            "--release-sequence".into(),
            request.release_sequence.clone().into(),
            "--release-base".into(),
            request.release_base.clone().into(),
            "--private-key".into(),
            request.private_key.as_os_str().to_owned(),
            "--openssl".into(),
            request.openssl.as_os_str().to_owned(),
            "--output".into(),
            request.output.as_os_str().to_owned(),
        ]
    }

    fn generate_publish_key(openssl: &Path, private_key: &Path) {
        let generated = Command::new(openssl)
            .args(["genpkey", "-algorithm", "ED25519", "-out"])
            .arg(private_key)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(generated.success(), "generate test release key");
        set_mode(private_key, 0o600, "set test release key mode").unwrap();
    }

    fn verify_publish_signature(
        openssl: &Path,
        private_key: &Path,
        input: &Path,
        signature: &Path,
        public_key: &Path,
    ) {
        let exported = Command::new(openssl)
            .args(["pkey", "-in"])
            .arg(private_key)
            .args(["-pubout", "-out"])
            .arg(public_key)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(exported.success(), "export test release public key");
        let verified = Command::new(openssl)
            .args(["pkeyutl", "-verify", "-rawin", "-pubin", "-inkey"])
            .arg(public_key)
            .arg("-in")
            .arg(input)
            .arg("-sigfile")
            .arg(signature)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(verified.success(), "verify publication signature");
    }

    fn no_builder_staging(root: &Path) -> bool {
        std::fs::read_dir(root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-release-builder-")
        })
    }

    fn replace_first(bytes: &mut [u8], source: &[u8], replacement: &[u8]) {
        assert_eq!(source.len(), replacement.len());
        let offset = occurrence_offsets(bytes, source)[0];
        bytes[offset..offset + source.len()].copy_from_slice(replacement);
    }

    #[test]
    fn test_m2_b6_slice1_request_boundary_is_strict() {
        let grammar_fixture = fixture("request-grammar", happy_entries("0.150.1"), false);
        for mutation in ["missing-value", "duplicate", "unknown"] {
            let mut args = request_args(&grammar_fixture.request);
            match mutation {
                "missing-value" => {
                    args.pop();
                }
                "duplicate" => {
                    args.extend([OsString::from("--version"), OsString::from("0.150.1")]);
                }
                "unknown" => {
                    args.extend([OsString::from("--channel"), OsString::from("stable")]);
                }
                _ => unreachable!(),
            }
            assert!(matches!(parse_request(args), Err(BuilderError::Usage)));
        }
        let mut duplicate_defer = request_args(&grammar_fixture.request);
        duplicate_defer.extend([
            OsString::from("--defer-manager-probe"),
            OsString::from("--defer-manager-probe"),
        ]);
        assert!(matches!(
            parse_request(duplicate_defer),
            Err(BuilderError::Usage)
        ));
        grammar_fixture.remove();

        for case in [
            "version",
            "digest-format",
            "generation-id",
            "metadata",
            "relative-path",
            "non-executable-tool",
        ] {
            let mut case_fixture = fixture(case, happy_entries("0.150.1"), false);
            match case {
                "version" => case_fixture.request.version = "0.150.1-beta.1".to_owned(),
                "digest-format" => {
                    case_fixture.request.archive_sha256 =
                        case_fixture.request.archive_sha256.to_uppercase()
                }
                "generation-id" => case_fixture.request.generation_id = "../escape".to_owned(),
                "metadata" => case_fixture.request.creation_metadata = "line\nbreak".to_owned(),
                "relative-path" => case_fixture.request.output = PathBuf::from("relative-output"),
                "non-executable-tool" => {
                    case_fixture.request.gzip = case_fixture.request.archive.clone()
                }
                _ => unreachable!(),
            }
            assert!(
                validate_request(&case_fixture.request).is_err(),
                "case {case} passed request validation"
            );
            assert!(!case_fixture.request.output.exists());
            case_fixture.remove();
        }
    }

    #[test]
    fn test_r5_official_fetch_uses_pinned_source_and_fail_closed_output() {
        let fixture = fixture("official-fetch", happy_entries("0.153.3"), false);
        let curl = fixture.root.join("bin/fetch-curl");
        let log = fixture.root.join("curl-arguments");
        write_fetch_curl(&curl, &log, &fixture.request.archive, 0);
        let output = fixture.root.join("downloaded-archive.tar.gz");
        let request = FetchRequest {
            version: "0.153.3".to_owned(),
            curl: curl.clone(),
            openssl: fixture.request.openssl.clone(),
            output: output.clone(),
        };

        let valid_args = fetch_args(&request);
        assert!(parse_fetch_request(valid_args.clone()).is_ok());
        for mutation in ["missing-value", "duplicate", "unknown"] {
            let mut args = valid_args.clone();
            match mutation {
                "missing-value" => {
                    args.pop();
                }
                "duplicate" => {
                    args.extend([OsString::from("--version"), OsString::from("0.153.3")]);
                }
                "unknown" => {
                    args.extend([OsString::from("--channel"), OsString::from("stable")]);
                }
                _ => unreachable!(),
            }
            assert!(matches!(
                parse_fetch_request(args),
                Err(BuilderError::Usage)
            ));
        }

        let expected_digest = openssl_sha256(&request.openssl, &fixture.request.archive).unwrap();
        assert_eq!(run_from_args(valid_args), 0);
        assert_eq!(
            std::fs::read(&output).unwrap(),
            std::fs::read(&fixture.request.archive).unwrap()
        );
        assert_eq!(
            openssl_sha256(&request.openssl, &output).unwrap(),
            expected_digest
        );
        let generated = fixture.root.join("generation-from-official");
        let build_request = BuildRequest {
            version: request.version.clone(),
            archive: output.clone(),
            archive_sha256: expected_digest.clone(),
            generation_id: "official-fetch-generation".to_owned(),
            core: fixture.request.core.clone(),
            manager: None,
            defer_manager_probe: false,
            legacy_activation_doctor_unsupported: false,
            creation_metadata: "r5-fetch-test".to_owned(),
            gzip: fixture.request.gzip.clone(),
            openssl: request.openssl.clone(),
            output: generated.clone(),
        };
        assert_eq!(run_from_args(request_args(&build_request)), 0);
        assert!(generated.join("runtime").is_file());
        assert!(generated.join("codex-code-mode-host").is_file());
        assert_eq!(
            std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(no_fetch_staging(&fixture.root));

        let arguments: Vec<_> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            arguments,
            vec![
                "--disable",
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--connect-timeout",
                RELEASE_CONNECT_TIMEOUT_SECONDS,
                "--max-time",
                RELEASE_TRANSFER_TIMEOUT_SECONDS,
                "--max-filesize",
                &ARCHIVE_MAX_BYTES.to_string(),
                "--url",
                official_archive_url("0.153.3").as_str(),
            ]
        );

        let collision = fixture.root.join("collision.tar.gz");
        std::fs::write(&collision, b"preserve").unwrap();
        let collision_request = FetchRequest {
            output: collision.clone(),
            ..request
        };
        assert!(fetch(&collision_request).is_err());
        assert_eq!(std::fs::read(&collision).unwrap(), b"preserve");
        assert!(no_fetch_staging(&fixture.root));

        let failure_curl = fixture.root.join("bin/failure-curl");
        let failure_log = fixture.root.join("failure-arguments");
        write_fetch_curl(&failure_curl, &failure_log, &fixture.request.archive, 22);
        let failed_output = fixture.root.join("failed-archive.tar.gz");
        let failed_request = FetchRequest {
            version: "0.153.3".to_owned(),
            curl: failure_curl,
            openssl: fixture.request.openssl.clone(),
            output: failed_output.clone(),
        };
        assert!(fetch(&failed_request).is_err());
        assert!(!failed_output.exists());
        assert!(no_fetch_staging(&fixture.root));
        fixture.remove();
    }

    #[test]
    fn test_m2_b6_slice1_real_builder_reaches_validated_selected_archive() {
        let fixture = fixture("slice1-happy", happy_entries("0.150.1"), false);
        validate_request(&fixture.request).unwrap();
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);

        let selected_root = fixture.root.join("selected");
        create_private_dir(&selected_root).unwrap();
        let archive = snapshot_archive(&fixture.request, &selected_root).unwrap();
        std::fs::write(
            &fixture.request.archive,
            b"changed after the pinned snapshot",
        )
        .unwrap();
        let selected = select_archive(&fixture.request, &archive, &selected_root).unwrap();
        std::fs::remove_file(archive).unwrap();
        assert_eq!(
            std::fs::read(&selected.raw_runtime).unwrap(),
            fixture.raw_runtime
        );
        assert_eq!(
            std::fs::read(&selected.code_mode_host).unwrap(),
            fixture.code_mode_host
        );
        let mut top_level: Vec<_> = std::fs::read_dir(&selected_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        top_level.sort();
        assert_eq!(
            top_level,
            vec![
                OsString::from(".raw-runtime"),
                OsString::from("codex-code-mode-host")
            ]
        );
        std::fs::remove_dir_all(&selected_root).unwrap();

        assert!(fixture.request.output.is_dir());
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();
    }

    #[test]
    fn test_rald7_layout_v1_resource_evolution_is_semantic() {
        let mut entries = happy_entries("0.155.0");
        entries[3].data = br#"{
  "pathDir": "codex-path",
  "variant": "codex",
  "extension": {"voice": true, "levels": [1, 2, 3]},
  "entrypoint": "bin/codex",
  "layoutVersion": 1,
  "resourcesDir": "codex-resources",
  "target": "aarch64-unknown-linux-musl",
  "version": "0.155.0"
}
"#
        .to_vec();
        entries.push(TestEntry::directory("codex-resources/voice/"));
        entries.push(TestEntry::directory("codex-resources/voice/bin/"));
        entries.push(TestEntry::file(
            "codex-resources/voice/bin/codex-voice-host",
            b"unselected-voice-host".to_vec(),
        ));
        entries.push(TestEntry::file(
            "codex-resources/voice/manifest.json",
            br#"{"version":1}"#.to_vec(),
        ));
        entries.push(TestEntry::file(
            "future-unselected-resource",
            b"ignored-by-termux-generation".to_vec(),
        ));

        let mut fixture = fixture("rald7-layout-v1-evolution", entries, false);
        fixture.request.version = "0.155.0".to_owned();
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        assert_eq!(
            std::fs::read(fixture.request.output.join("runtime"))
                .unwrap()
                .len(),
            fixture.raw_runtime.len()
        );
        assert_eq!(
            std::fs::read(fixture.request.output.join("codex-code-mode-host")).unwrap(),
            fixture.code_mode_host
        );
        assert!(!fixture.request.output.join("codex-resources").exists());
        assert!(!fixture
            .request
            .output
            .join("future-unselected-resource")
            .exists());
        fixture.remove();
    }

    #[test]
    fn test_m2_b6_slice1_archive_rejection_matrix_is_fail_closed() {
        for case in [
            "digest",
            "duplicate",
            "traversal",
            "symlink",
            "unknown-pax",
            "metadata",
            "layout-version",
            "duplicate-metadata",
            "interp",
            "oversized",
            "checksum",
        ] {
            let mut entries = happy_entries("0.150.1");
            let corrupt_header = case == "checksum";
            match case {
                "duplicate" => entries.push(entries[1].clone()),
                "traversal" => entries[5].path = "../escaped".to_owned(),
                "symlink" => entries[5].kind = b'2',
                "unknown-pax" => entries[0].pax_key = Some("path"),
                "metadata" => entries[3].data = b"{}\n".to_vec(),
                "layout-version" => {
                    entries[3].data = expected_package_json("0.150.1");
                    let text = String::from_utf8(entries[3].data.clone()).unwrap();
                    entries[3].data = text
                        .replacen("\"layoutVersion\": 1", "\"layoutVersion\": 2", 1)
                        .into_bytes();
                }
                "duplicate-metadata" => {
                    entries[3].data = br#"{
  "layoutVersion": 1,
  "layoutVersion": 1,
  "version": "0.150.1",
  "target": "aarch64-unknown-linux-musl",
  "variant": "codex",
  "entrypoint": "bin/codex",
  "resourcesDir": "codex-resources",
  "pathDir": "codex-path"
}
"#
                    .to_vec();
                }
                "interp" => entries[2].data = fake_elf(true),
                "oversized" => entries[5].declared_size = Some(ENTRY_MAX_BYTES + 1),
                "digest" | "checksum" => {}
                _ => unreachable!(),
            }
            let mut fixture = fixture(case, entries, corrupt_header);
            if case == "digest" {
                fixture.request.archive_sha256 = "0".repeat(64);
            }
            let error = build(&fixture.request).unwrap_err();
            assert!(!error.to_string().is_empty(), "case {case} had no error");
            assert!(
                !fixture.request.output.exists(),
                "case {case} published output"
            );
            assert!(
                !fixture.root.join("escaped").exists(),
                "case {case} escaped"
            );
            assert!(no_builder_staging(&fixture.root));
            fixture.remove();
        }
    }

    #[test]
    fn test_m2_b6_slice2_exact_adaptation_and_complete_publication() {
        let fixture = fixture("slice2-happy", happy_entries("0.150.1"), false);
        let raw_runtime_sha256 = openssl_sha256(&fixture.request.openssl, &{
            let path = fixture.root.join("raw-runtime-for-hash");
            std::fs::write(&path, &fixture.raw_runtime).unwrap();
            path
        })
        .unwrap();
        let core_sha256 = openssl_sha256(&fixture.request.openssl, &fixture.request.core).unwrap();

        assert_eq!(run_from_args(request_args(&fixture.request)), 0);

        let runtime_path = fixture.request.output.join("runtime");
        let host_path = fixture.request.output.join("codex-code-mode-host");
        let descriptor_path = fixture.request.output.join("generation.meta");
        let runtime = std::fs::read(&runtime_path).unwrap();
        assert_eq!(runtime.len(), fixture.raw_runtime.len());
        assert_eq!(
            runtime
                .iter()
                .zip(&fixture.raw_runtime)
                .filter(|(after, before)| after != before)
                .count(),
            54
        );
        for (source, replacement, expected) in PATCHES {
            assert_eq!(occurrence_offsets(&runtime, source).len(), 0);
            assert_eq!(occurrence_offsets(&runtime, replacement).len(), expected);
        }
        assert_eq!(std::fs::read(&host_path).unwrap(), fixture.code_mode_host);
        assert_eq!(
            std::fs::symlink_metadata(&runtime_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            std::fs::symlink_metadata(&host_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            std::fs::symlink_metadata(&descriptor_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );

        let runtime_sha256 = openssl_sha256(&fixture.request.openssl, &runtime_path).unwrap();
        let host_sha256 = openssl_sha256(&fixture.request.openssl, &host_path).unwrap();
        let browser_open_path = fixture.request.output.join("browser/open/curl");
        let browser_manual_path = fixture.request.output.join("browser/manual/curl");
        let browser_open_sha256 =
            openssl_sha256(&fixture.request.openssl, &browser_open_path).unwrap();
        let browser_manual_sha256 =
            openssl_sha256(&fixture.request.openssl, &browser_manual_path).unwrap();
        assert_eq!(
            std::fs::read(&browser_open_path).unwrap(),
            TERMUX_BROWSER_OPEN_HELPER
        );
        assert_eq!(
            std::fs::read(&browser_manual_path).unwrap(),
            TERMUX_BROWSER_MANUAL_HELPER
        );
        for helper in [&browser_open_path, &browser_manual_path] {
            assert_eq!(
                std::fs::symlink_metadata(helper)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
        }
        let expected_descriptor = format!(
            concat!(
                "codex-local-generation-v2\n",
                "generation_id\ttest-generation\n",
                "upstream_package_identity\topenai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz\n",
                "upstream_package_version\t0.150.1\n",
                "source_artifact_digest\t{}\n",
                "expected_platform\tandroid\n",
                "expected_architecture\taarch64\n",
                "patch_policy_id\ttermux-fd-remap-v1\n",
                "patch_report\ttermux-fd-remap-v1;archive_sha256={};raw_runtime_sha256={};runtime_sha256={};code_mode_host_sha256={};source_counts=2,1,1,1;changed_bytes=54\n",
                "runtime_digest\t{}\n",
                "core_artifact_digest\t{}\n",
                "manager_artifact_digest\t-\n",
                "core_api_identity\tcore-api-v1\n",
                "persistent_schema_identity\tschema-v1\n",
                "qualification\tqualified\n",
                "creation_metadata\ttest-fixture\n",
                "upstream_doctor\tsupported\n",
                "helper_count\t2\n",
                "helper\ttermux-browser-open-v1\t{}\n",
                "helper\ttermux-browser-manual-v1\t{}\n"
            ),
            fixture.request.archive_sha256,
            fixture.request.archive_sha256,
            raw_runtime_sha256,
            runtime_sha256,
            host_sha256,
            runtime_sha256,
            core_sha256,
            browser_open_sha256,
            browser_manual_sha256
        );
        assert_eq!(
            std::fs::read_to_string(&descriptor_path).unwrap(),
            expected_descriptor
        );

        let mut top_level: Vec<_> = std::fs::read_dir(&fixture.request.output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        top_level.sort();
        assert_eq!(
            top_level,
            vec![
                OsString::from("browser"),
                OsString::from("codex-code-mode-host"),
                OsString::from("core"),
                OsString::from("generation.meta"),
                OsString::from("runtime")
            ]
        );
        assert!(!fixture.request.output.join("compat").exists());

        let descriptor_before_retry = std::fs::read(&descriptor_path).unwrap();
        assert_eq!(run_from_args(request_args(&fixture.request)), 1);
        assert_eq!(
            std::fs::read(&descriptor_path).unwrap(),
            descriptor_before_retry
        );
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();
    }

    #[test]
    fn test_mgr1_slice1_build_carries_optional_manager_artifact_and_digest() {
        let mut fixture = fixture("mgr1-manager-build", happy_entries("0.150.1"), false);
        let manager = fixture.root.join("manager-source");
        write_valid_manager_probe(&manager);
        fixture.request.manager = Some(manager.clone());

        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        let output_manager = fixture.request.output.join("manager");
        assert_eq!(
            std::fs::read(&output_manager).unwrap(),
            std::fs::read(&manager).unwrap()
        );
        assert_eq!(
            std::fs::symlink_metadata(&output_manager)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        let manager_digest = openssl_sha256(&fixture.request.openssl, &output_manager).unwrap();
        let descriptor =
            std::fs::read_to_string(fixture.request.output.join("generation.meta")).unwrap();
        assert!(descriptor.contains(&format!("manager_artifact_digest\t{manager_digest}\n")));
        let mut top_level: Vec<_> = std::fs::read_dir(&fixture.request.output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        top_level.sort();
        assert_eq!(
            top_level,
            vec![
                OsString::from("browser"),
                OsString::from("codex-code-mode-host"),
                OsString::from("core"),
                OsString::from("generation.meta"),
                OsString::from("manager"),
                OsString::from("runtime")
            ]
        );
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();
    }

    #[test]
    fn test_mgr1_slice2_publish_includes_manager_in_signed_inventory() {
        let mut fixture = fixture("mgr1-manager-publish", happy_entries("0.150.1"), false);
        let manager = fixture.root.join("manager-source");
        write_valid_manager_probe(&manager);
        fixture.request.manager = Some(manager);
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);

        let private_key = fixture.root.join("release-key.pem");
        generate_publish_key(&fixture.request.openssl, &private_key);
        let publication = fixture.root.join("publication");
        let request = PublishRequest {
            generation: fixture.request.output.clone(),
            release_sequence: "1".to_owned(),
            release_base: "https://example.test/releases/test-generation/".to_owned(),
            private_key,
            openssl: fixture.request.openssl.clone(),
            output: publication.clone(),
        };
        assert_eq!(publish(&request).unwrap(), "test-generation");
        let release = publication.join("releases/test-generation");
        assert!(release.join("manager").is_file());
        let manifest = std::fs::read_to_string(release.join("release.manifest")).unwrap();
        assert!(manifest.contains("file\tmanager\t"));
        assert!(manifest.contains("file_count\t7\n"));
        fixture.remove();
    }

    #[test]
    fn test_rald3_deferred_manager_probe_is_unsigned_and_publish_blocking() {
        let mut fixture = fixture("rald3-deferred-manager", happy_entries("0.150.1"), false);
        let manager = fixture.root.join("manager-source");
        std::fs::write(&manager, fake_core_elf(ANDROID_AARCH64_INTERPRETER)).unwrap();
        set_mode(&manager, 0o755, "set deferred test Manager mode").unwrap();
        fixture.request.manager = Some(manager.clone());
        fixture.request.defer_manager_probe = true;

        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        let marker = fixture
            .request
            .output
            .join(MANAGER_ARTIFACT_DEFERRED_MARKER);
        assert_eq!(
            std::fs::read(&marker).unwrap(),
            MANAGER_ARTIFACT_DEFERRED_MARKER_BYTES
        );
        assert_eq!(
            std::fs::symlink_metadata(&marker)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );
        assert_eq!(
            std::fs::read(fixture.request.output.join("manager")).unwrap(),
            std::fs::read(&manager).unwrap()
        );

        let private_key = fixture.root.join("release-key.pem");
        generate_publish_key(&fixture.request.openssl, &private_key);
        let publication = fixture.root.join("publication");
        let request = PublishRequest {
            generation: fixture.request.output.clone(),
            release_sequence: "1".to_owned(),
            release_base: "https://example.test/releases/test-generation/".to_owned(),
            private_key,
            openssl: fixture.request.openssl.clone(),
            output: publication.clone(),
        };
        let error = publish(&request).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Manager artifact probe is still deferred"
        );
        assert!(!publication.exists());
        fixture.remove();
    }

    #[test]
    fn test_rald3_deferred_manager_probe_requires_manager_and_android_elf() {
        let mut missing = fixture("rald3-deferred-missing", happy_entries("0.150.1"), false);
        missing.request.defer_manager_probe = true;
        let error = build(&missing.request).unwrap_err();
        assert_eq!(
            error.to_string(),
            "deferred Manager artifact probe requires a Manager artifact"
        );
        assert!(!missing.request.output.exists());
        missing.remove();

        let mut invalid = fixture("rald3-deferred-invalid", happy_entries("0.150.1"), false);
        let manager = invalid.root.join("manager-source");
        write_valid_manager_probe(&manager);
        invalid.request.manager = Some(manager);
        invalid.request.defer_manager_probe = true;
        let error = build(&invalid.request).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Manager artifact is not a supported Android AArch64 PIE ELF"),
            "unexpected error: {error}"
        );
        assert!(!invalid.request.output.exists());
        assert!(no_builder_staging(&invalid.root));
        invalid.remove();
    }

    #[test]
    fn test_mgr5_manager_artifact_probe_is_bounded_and_fail_closed() {
        let cases = [
            (
                "probe-output",
                "printf '%s\\n' 'wrong-manager-artifact'\n",
            ),
            (
                "probe-stderr",
                "printf 'probe-noise\\n' >&2\nprintf '%s\\n' 'codex-manager-artifact-v1' 'core_api=codex-manager-core-v1'\n",
            ),
            (
                "probe-status",
                "printf '%s\\n' 'codex-manager-artifact-v1' 'core_api=codex-manager-core-v1'\nexit 7\n",
            ),
            (
                "probe-oversized",
                "i=0\nwhile [ \"$i\" -lt 513 ]; do printf x; i=$((i + 1)); done\n",
            ),
        ];
        for (label, body) in cases {
            let mut fixture = fixture(label, happy_entries("0.150.1"), false);
            let manager = fixture.root.join("manager-source");
            write_manager_probe(&manager, body);
            fixture.request.manager = Some(manager);
            let error = build(&fixture.request).unwrap_err();
            assert!(
                error.to_string().contains("Manager artifact probe"),
                "unexpected {label} error: {error}"
            );
            assert!(!fixture.request.output.exists());
            assert!(no_builder_staging(&fixture.root));
            fixture.remove();
        }

        let mut fixture = fixture("probe-timeout", happy_entries("0.150.1"), false);
        let manager = fixture.root.join("manager-source");
        write_manager_probe(&manager, "while :; do :; done\n");
        fixture.request.manager = Some(manager);
        let error = build(&fixture.request).unwrap_err();
        assert!(error.to_string().contains("probe timed out"));
        assert!(!fixture.request.output.exists());
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();
    }

    #[test]
    fn test_m2_b8_slice1_core_artifact_identity_and_digest_are_exact() {
        let fixture = fixture("b8-core-exact", happy_entries("0.150.1"), false);
        let expected_core = std::fs::read(&fixture.request.core).unwrap();
        let expected_sha256 =
            openssl_sha256(&fixture.request.openssl, &fixture.request.core).unwrap();

        let staging = fixture.root.join("core-snapshot-stage");
        create_private_dir(&staging).unwrap();
        let selected_sha256 = snapshot_core_artifact(&fixture.request, &staging).unwrap();
        assert_eq!(selected_sha256, expected_sha256);
        assert_eq!(
            std::fs::read(staging.join(".core-artifact")).unwrap(),
            expected_core
        );
        assert_eq!(
            std::fs::symlink_metadata(staging.join(".core-artifact"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        std::fs::write(&fixture.request.core, b"mutated after selection").unwrap();
        assert_eq!(selected_sha256, expected_sha256);
        assert_ne!(
            openssl_sha256(&fixture.request.openssl, &fixture.request.core).unwrap(),
            selected_sha256
        );
        std::fs::write(&fixture.request.core, expected_core).unwrap();
        set_mode(&fixture.request.core, 0o700, "restore test Core mode").unwrap();
        std::fs::remove_file(staging.join(".core-artifact")).unwrap();
        std::fs::remove_dir(&staging).unwrap();

        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        let descriptor =
            std::fs::read_to_string(fixture.request.output.join("generation.meta")).unwrap();
        assert!(descriptor.contains(&format!("core_artifact_digest\t{expected_sha256}\n")));
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();
    }

    #[test]
    fn test_m2_b8_slice1_core_artifact_rejection_matrix_is_fail_closed() {
        for case in [
            "not-executable",
            "not-elf",
            "wrong-type",
            "wrong-machine",
            "no-interp",
            "wrong-interp",
            "oversized",
        ] {
            let fixture = fixture(case, happy_entries("0.150.1"), false);
            match case {
                "not-executable" => {
                    set_mode(&fixture.request.core, 0o600, "clear test Core execute mode").unwrap()
                }
                "not-elf" => std::fs::write(&fixture.request.core, b"not an ELF").unwrap(),
                "wrong-type" => {
                    let mut core = fake_core_elf(ANDROID_AARCH64_INTERPRETER);
                    core[16..18].copy_from_slice(&2u16.to_le_bytes());
                    std::fs::write(&fixture.request.core, core).unwrap();
                }
                "wrong-machine" => {
                    let mut core = fake_core_elf(ANDROID_AARCH64_INTERPRETER);
                    core[18..20].copy_from_slice(&62u16.to_le_bytes());
                    std::fs::write(&fixture.request.core, core).unwrap();
                }
                "no-interp" => {
                    let mut core = fake_core_elf(ANDROID_AARCH64_INTERPRETER);
                    core[64..68].copy_from_slice(&1u32.to_le_bytes());
                    std::fs::write(&fixture.request.core, core).unwrap();
                }
                "wrong-interp" => std::fs::write(
                    &fixture.request.core,
                    fake_core_elf(b"/lib/ld-linux-aarch64.so.1\0"),
                )
                .unwrap(),
                "oversized" => {
                    let core = OpenOptions::new()
                        .write(true)
                        .open(&fixture.request.core)
                        .unwrap();
                    core.set_len(CORE_ARTIFACT_MAX_BYTES + 1).unwrap();
                }
                _ => unreachable!(),
            }
            if case != "not-executable" {
                set_mode(&fixture.request.core, 0o700, "set mutated test Core mode").unwrap();
            }
            assert!(build(&fixture.request).is_err(), "case {case} succeeded");
            assert!(
                !fixture.request.output.exists(),
                "case {case} published output"
            );
            assert!(no_builder_staging(&fixture.root));
            fixture.remove();
        }
    }

    #[test]
    fn test_rald45_transition_flag_is_exact_and_legacy_bounded() {
        let mut fixture = fixture("rald45-transition-flag", happy_entries("0.150.1"), false);
        fixture.request.creation_metadata = R10_BROWSER_HELPER_BRIDGE_METADATA.to_owned();
        fixture.request.legacy_activation_doctor_unsupported = true;
        let parsed = parse_request(request_args(&fixture.request)).unwrap();
        assert!(parsed.legacy_activation_doctor_unsupported);
        assert_eq!(parsed.creation_metadata, R10_BROWSER_HELPER_BRIDGE_METADATA);

        fixture.request.creation_metadata = "test-fixture".to_owned();
        assert!(matches!(
            validate_request(&fixture.request),
            Err(BuilderError::Invalid(
                "legacy activation doctor transition requires the exact R10 helper layout"
            ))
        ));
    }

    #[test]
    fn test_tc_live_bridge_build_and_publish_are_marker_bound_and_r10_readable() {
        {
            let mut fixture = fixture("tc-live-bridge", happy_entries("0.150.1"), false);
            fixture.request.creation_metadata = R10_BROWSER_HELPER_BRIDGE_METADATA.to_owned();
            assert_eq!(run_from_args(request_args(&fixture.request)), 0);
            let open_helper = fixture.request.output.join("helpers/0");
            let manual_helper = fixture.request.output.join("helpers/1");
            assert_eq!(
                std::fs::read(&open_helper).unwrap(),
                TERMUX_BROWSER_OPEN_HELPER
            );
            assert_eq!(
                std::fs::read(&manual_helper).unwrap(),
                TERMUX_BROWSER_MANUAL_HELPER
            );
            assert!(!fixture.request.output.join("browser").exists());
            let descriptor =
                std::fs::read_to_string(fixture.request.output.join("generation.meta")).unwrap();
            assert!(descriptor.contains(&format!(
                "creation_metadata\t{R10_BROWSER_HELPER_BRIDGE_METADATA}\n"
            )));
            assert!(
                descriptor.contains(&format!("helper\t{TERMUX_BROWSER_OPEN_HELPER_IDENTITY}\t"))
            );
            assert!(descriptor.contains(&format!(
                "helper\t{TERMUX_BROWSER_MANUAL_HELPER_IDENTITY}\t"
            )));

            let private_key = fixture.root.join("bridge-release-key.pem");
            generate_publish_key(&fixture.request.openssl, &private_key);
            let request = PublishRequest {
                generation: fixture.request.output.clone(),
                release_sequence: "8".to_owned(),
                release_base: "https://example.test/releases/test-generation/".to_owned(),
                private_key: private_key.clone(),
                openssl: fixture.request.openssl.clone(),
                output: fixture.root.join("bridge-publication"),
            };
            assert_eq!(publish(&request).unwrap(), "test-generation");
            let release = request.output.join("releases/test-generation");
            let manifest = std::fs::read_to_string(release.join("release.manifest")).unwrap();
            assert!(manifest.contains("file\thelpers/0\t"));
            assert!(manifest.contains("file\thelpers/1\t"));
            assert!(!manifest.contains("file\tbrowser/"));
            assert_eq!(
                std::fs::read(release.join("helpers/0")).unwrap(),
                TERMUX_BROWSER_OPEN_HELPER
            );
            assert_eq!(
                std::fs::read(release.join("helpers/1")).unwrap(),
                TERMUX_BROWSER_MANUAL_HELPER
            );
            fixture.remove();
        }

        let mut mismatch = fixture("tc-live-bridge-mismatch", happy_entries("0.150.1"), false);
        mismatch.request.creation_metadata = R10_BROWSER_HELPER_BRIDGE_METADATA.to_owned();
        assert_eq!(run_from_args(request_args(&mismatch.request)), 0);
        let descriptor_path = mismatch.request.output.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap().replace(
            &format!("creation_metadata\t{R10_BROWSER_HELPER_BRIDGE_METADATA}\n"),
            "creation_metadata\ttest-fixture\n",
        );
        std::fs::write(&descriptor_path, descriptor).unwrap();
        let mismatch_key = mismatch.root.join("mismatch-release-key.pem");
        generate_publish_key(&mismatch.request.openssl, &mismatch_key);
        let mismatch_request = PublishRequest {
            generation: mismatch.request.output.clone(),
            release_sequence: "8".to_owned(),
            release_base: "https://example.test/releases/test-generation/".to_owned(),
            private_key: mismatch_key,
            openssl: mismatch.request.openssl.clone(),
            output: mismatch.root.join("mismatch-publication"),
        };
        let error = publish(&mismatch_request).unwrap_err();
        assert!(
            error.to_string().contains("browser helper layout binding"),
            "unexpected mismatch error: {error}"
        );
        assert!(!mismatch_request.output.exists());
        mismatch.remove();
    }

    #[test]
    fn test_tc2_browser_helpers_are_exact_single_arg_and_fail_closed() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = fixture("tc2-browser-helper", happy_entries("0.150.1"), false);
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        let open_helper = fixture.request.output.join("browser/open/curl");
        let manual_helper = fixture.request.output.join("browser/manual/curl");
        assert_eq!(
            std::fs::read(&open_helper).unwrap(),
            TERMUX_BROWSER_OPEN_HELPER
        );
        assert_eq!(
            std::fs::read(&manual_helper).unwrap(),
            TERMUX_BROWSER_MANUAL_HELPER
        );

        let prefix = fixture.root.join("prefix");
        let bin = prefix.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let opener = bin.join("termux-open-url");
        let log = fixture.root.join("opener-log");
        let shell = find_tool("sh");
        std::fs::write(
            &opener,
            format!(
                "#!{}\nprintf '%s\\n' \"$#\" \"$1\" > \"$CODEX_TC2_LOG\"\n",
                shell.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755)).unwrap();

        let valid = "https://example.test/oauth/callback?x=$(id)&semi=one;two#fragment";
        let output = Command::new(&shell)
            .arg(&open_helper)
            .arg(valid)
            .env_clear()
            .env("PREFIX", &prefix)
            .env("CODEX_TERMUX_URL_OPENER", &opener)
            .env("CODEX_TC2_LOG", &log)
            .output()
            .unwrap();
        assert!(output.status.success(), "stderr={:?}", output.stderr);
        assert_eq!(
            std::fs::read_to_string(&log).unwrap(),
            format!("1\n{valid}\n")
        );

        let manual = Command::new(&shell)
            .arg(&manual_helper)
            .arg(valid)
            .env_clear()
            .output()
            .unwrap();
        assert!(manual.status.success());
        assert!(String::from_utf8_lossy(&manual.stderr).contains(valid));

        for invalid in [
            "file:///etc/passwd",
            "https:///missing-host",
            "https://?missing-host",
            "https://:443/missing-host",
            "https://example.test/has space",
            "https://example.test/line\nbreak",
            "https://example.test/back\\slash",
        ] {
            let _ = std::fs::remove_file(&log);
            let blocked = Command::new(&shell)
                .arg(&open_helper)
                .arg(invalid)
                .env_clear()
                .env("PREFIX", &prefix)
                .env("CODEX_TERMUX_URL_OPENER", &opener)
                .env("CODEX_TC2_LOG", &log)
                .output()
                .unwrap();
            assert_eq!(blocked.status.code(), Some(64), "invalid={invalid:?}");
            assert!(!log.exists(), "invalid URL reached opener: {invalid:?}");
            let fallback = Command::new(&shell)
                .arg(&manual_helper)
                .arg(invalid)
                .env_clear()
                .output()
                .unwrap();
            assert!(fallback.status.success());
            let stderr = String::from_utf8_lossy(&fallback.stderr);
            assert!(stderr.contains("Browser launch blocked"));
            assert!(!stderr.contains(invalid));
        }

        for args in [vec![], vec!["https://example.test", "https://other.test"]] {
            let status = Command::new(&shell)
                .arg(&open_helper)
                .args(args)
                .env_clear()
                .env("PREFIX", &prefix)
                .env("CODEX_TERMUX_URL_OPENER", &opener)
                .env("CODEX_TC2_LOG", &log)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(64));
        }

        let unqualified = Command::new(&shell)
            .arg(&open_helper)
            .arg(valid)
            .env_clear()
            .env("PREFIX", &prefix)
            .env("CODEX_TERMUX_URL_OPENER", prefix.join("bin/not-the-opener"))
            .output()
            .unwrap();
        assert_eq!(unqualified.status.code(), Some(64));

        std::fs::remove_file(&opener).unwrap();
        let unavailable = Command::new(&shell)
            .arg(&open_helper)
            .arg(valid)
            .env_clear()
            .env("PREFIX", &prefix)
            .env("CODEX_TERMUX_URL_OPENER", &opener)
            .output()
            .unwrap();
        assert_eq!(unavailable.status.code(), Some(127));
        let fallback = Command::new(&shell)
            .arg(&manual_helper)
            .arg(valid)
            .env_clear()
            .output()
            .unwrap();
        assert!(fallback.status.success());
        assert!(String::from_utf8_lossy(&fallback.stderr).contains(valid));

        fixture.remove();
    }

    #[test]
    fn test_tc3_upstream_rg_is_explicitly_not_published_as_termux_helper() {
        let fixture = fixture(
            "tc3-upstream-rg-disposition",
            happy_entries("0.150.1"),
            false,
        );
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        assert!(!fixture.request.output.join("rg").exists());
        let descriptor =
            std::fs::read_to_string(fixture.request.output.join("generation.meta")).unwrap();
        assert!(!descriptor.contains("termux-rg"));
        assert_eq!(
            descriptor
                .lines()
                .find_map(|line| line.strip_prefix("helper_count\t")),
            Some("2")
        );
        fixture.remove();
    }

    #[test]
    fn test_m2_b6_slice2_patch_and_publication_fail_closed_matrix() {
        for case in ["missing-source", "extra-source", "prepatched-source"] {
            let mut entries = happy_entries("0.150.1");
            let runtime = &mut entries
                .iter_mut()
                .find(|entry| entry.path == "bin/codex")
                .unwrap()
                .data;
            match case {
                "missing-source" => replace_first(runtime, PATCHES[0].0, b"XXXXXXXXXXXXXXXX"),
                "extra-source" => runtime.extend_from_slice(b"/etc/resolv.conf"),
                "prepatched-source" => replace_first(runtime, PATCHES[0].0, PATCHES[0].1),
                _ => unreachable!(),
            }
            let fixture = fixture(case, entries, false);
            assert!(build(&fixture.request).is_err(), "case {case} succeeded");
            assert!(
                !fixture.request.output.exists(),
                "case {case} published output"
            );
            assert!(no_builder_staging(&fixture.root));
            fixture.remove();
        }

        let fixture = fixture("output-exists", happy_entries("0.150.1"), false);
        std::fs::create_dir(&fixture.request.output).unwrap();
        let sentinel = fixture.request.output.join("sentinel");
        std::fs::write(&sentinel, b"preserve").unwrap();
        assert!(build(&fixture.request).is_err());
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();

        let root = test_root("rename-noreplace");
        let source = root.join("source");
        let destination = root.join("destination");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(source.join("source-marker"), b"source").unwrap();
        std::fs::write(destination.join("destination-marker"), b"destination").unwrap();
        assert!(rename_noreplace(&source, &destination).is_err());
        assert_eq!(
            std::fs::read(source.join("source-marker")).unwrap(),
            b"source"
        );
        assert_eq!(
            std::fs::read(destination.join("destination-marker")).unwrap(),
            b"destination"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_r6_publish_emits_core_compatible_signed_tree_and_fails_closed() {
        let fixture = fixture("r6-publish", happy_entries("0.150.1"), false);
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        let private_key = fixture.root.join("release-private.pem");
        let public_key = fixture.root.join("release-public.pem");
        generate_publish_key(&fixture.request.openssl, &private_key);
        let request = PublishRequest {
            generation: fixture.request.output.clone(),
            release_sequence: "7".to_owned(),
            release_base: "https://releases.example.invalid/codex/releases/test-generation/"
                .to_owned(),
            private_key: private_key.clone(),
            openssl: fixture.request.openssl.clone(),
            output: fixture.root.join("publication"),
        };

        let mut missing_value = publish_args(&request);
        missing_value.pop();
        assert_eq!(run_from_args(missing_value), 2);
        let mut unknown_flag = publish_args(&request);
        unknown_flag.extend([OsString::from("--channel"), OsString::from("stable")]);
        assert_eq!(run_from_args(unknown_flag), 2);

        assert_eq!(run_from_args(publish_args(&request)), 0);
        let publication = &request.output;
        let release = publication.join("releases/test-generation");
        let runtime_digest =
            openssl_sha256(&fixture.request.openssl, &release.join("runtime")).unwrap();
        let host_digest = openssl_sha256(
            &fixture.request.openssl,
            &release.join("codex-code-mode-host"),
        )
        .unwrap();
        let descriptor_digest =
            openssl_sha256(&fixture.request.openssl, &release.join("generation.meta")).unwrap();
        let browser_manual_digest = openssl_sha256(
            &fixture.request.openssl,
            &release.join("browser/manual/curl"),
        )
        .unwrap();
        let browser_open_digest =
            openssl_sha256(&fixture.request.openssl, &release.join("browser/open/curl")).unwrap();
        let expected_manifest = format!(
            concat!(
                "codex-release-v4\n",
                "generation_id\ttest-generation\n",
                "release_sequence\t7\n",
                "channel\tstable\n",
                "expected_platform\tandroid\n",
                "expected_architecture\taarch64\n",
                "core_api_identity\tcore-api-v1\n",
                "persistent_schema_identity\tschema-v1\n",
                "release_public_key\t{}\n",
                "file_count\t6\n",
                "file\tbrowser/manual/curl\t{}\t0755\n",
                "file\tbrowser/open/curl\t{}\t0755\n",
                "file\tcodex-code-mode-host\t{}\t0755\n",
                "file\tcore\t{}\t0755\n",
                "file\tgeneration.meta\t{}\t0644\n",
                "file\truntime\t{}\t0755\n"
            ),
            public_key_hex(&openssl_public_key(&fixture.request.openssl, &private_key).unwrap()),
            browser_manual_digest,
            browser_open_digest,
            host_digest,
            openssl_sha256(&fixture.request.openssl, &release.join("core")).unwrap(),
            descriptor_digest,
            runtime_digest,
        );
        assert_eq!(
            std::fs::read_to_string(release.join("release.manifest")).unwrap(),
            expected_manifest
        );
        assert_eq!(
            std::fs::read_to_string(publication.join("update-index-v1")).unwrap(),
            "codex-update-index-v1\nchannel\tstable\ngeneration_id\ttest-generation\nrelease_base\thttps://releases.example.invalid/codex/releases/test-generation/\n"
        );
        assert!(!release.join("release-authority.sig").exists());
        verify_publish_signature(
            &fixture.request.openssl,
            &private_key,
            &release.join("release.manifest"),
            &release.join("release.sig"),
            &public_key,
        );
        verify_publish_signature(
            &fixture.request.openssl,
            &private_key,
            &publication.join("update-index-v1"),
            &publication.join("update-index-v1.sig"),
            &public_key,
        );
        assert_eq!(
            std::fs::read(release.join("runtime")).unwrap(),
            std::fs::read(fixture.request.output.join("runtime")).unwrap()
        );
        assert_eq!(
            std::fs::read(release.join("codex-code-mode-host")).unwrap(),
            std::fs::read(fixture.request.output.join("codex-code-mode-host")).unwrap()
        );
        assert!(!publication.join("release-private.pem").exists());
        assert!(no_builder_staging(&fixture.root));

        for (label, mutation) in [
            ("bad-base", "base"),
            ("bad-sequence", "sequence"),
            ("missing-key", "key"),
        ] {
            let mut invalid = request.clone();
            invalid.output = fixture.root.join(format!("publication-{label}"));
            match mutation {
                "base" => invalid.release_base = "http://example.invalid/test-generation/".into(),
                "sequence" => invalid.release_sequence = "0".into(),
                "key" => invalid.private_key = fixture.root.join("missing-key.pem"),
                _ => unreachable!(),
            }
            assert_eq!(run_from_args(publish_args(&invalid)), 1, "case {label}");
            assert!(!invalid.output.exists(), "case {label} published output");
            assert!(no_builder_staging(&fixture.root));
        }

        let collision_output = fixture.root.join("publication-collision");
        create_private_dir(&collision_output).unwrap();
        let sentinel = collision_output.join("sentinel");
        std::fs::write(&sentinel, b"preserve").unwrap();
        let collision_request = PublishRequest {
            output: collision_output,
            ..request.clone()
        };
        assert_eq!(run_from_args(publish_args(&collision_request)), 1);
        assert_eq!(std::fs::read(sentinel).unwrap(), b"preserve");
        assert!(no_builder_staging(&fixture.root));

        let symlink_output = fixture.root.join("publication-symlink");
        let host = fixture.request.output.join("codex-code-mode-host");
        std::fs::remove_file(&host).unwrap();
        std::os::unix::fs::symlink(fixture.request.output.join("runtime"), &host).unwrap();
        let symlink_request = PublishRequest {
            output: symlink_output.clone(),
            ..collision_request
        };
        assert_eq!(run_from_args(publish_args(&symlink_request)), 1);
        assert!(!symlink_output.exists());
        assert!(no_builder_staging(&fixture.root));
        fixture.remove();
    }
}
