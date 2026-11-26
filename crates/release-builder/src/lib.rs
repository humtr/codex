#![cfg(unix)]

mod runtime_patch;
mod sha256;

pub use runtime_patch::PATCH_POLICY_ID;
pub use sha256::{sha256_bytes, sha256_file};

use sha256::Sha256;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

pub const ARCHIVE_MAX_BYTES: u64 = 512 * 1024 * 1024;

pub fn binary_adaptation_available(version: &str) -> bool {
    valid_stable_version(version)
}
const ENTRY_MAX_BYTES: u64 = 384 * 1024 * 1024;
const PAYLOAD_MAX_BYTES: u64 = 768 * 1024 * 1024;
// Bound the complete decompressed tar stream, including headers, padding and
// trailing zero blocks (not just extracted file payloads).
const DECOMPRESSED_ARCHIVE_MAX_BYTES: u64 = 1024 * 1024 * 1024;
const WRAPPER_ARTIFACT_MAX_BYTES: u64 = 64 * 1024 * 1024;
const PATH_MAX_BYTES: usize = 256;
const LOGICAL_ENTRY_MAX: usize = 1024;
const PAX_PAYLOAD_MAX_BYTES: u64 = 4096;
const PACKAGE_JSON_MAX_BYTES: u64 = 64 * 1024;
const JSON_DEPTH_MAX: usize = 16;
const GENERATION_ID_MAX_BYTES: usize = 512;
const TEXT_VALUE_MAX_BYTES: usize = 512;
const TAR_BLOCK_BYTES: usize = 512;
const GENERATION_FORMAT: &str = "codex-local-generation";
const TERMUX_RG_WRAPPER_SCRIPT: &[u8] = b"#!/system/bin/sh\nif [ -n \"${PREFIX:-}\" ] && [ -x \"${PREFIX}/bin/rg\" ]; then\n  exec \"${PREFIX}/bin/rg\" \"$@\"\nfi\nexec /data/data/com.termux/files/usr/bin/rg \"$@\"\n";
const WRAPPER_API_IDENTITY: &str = "wrapper-api";
const PERSISTENT_SCHEMA_IDENTITY: &str = "schema";
const PACKAGE_IDENTITY: &str = "openai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz";
pub const RELEASE_TRANSFER_TIMEOUT_SECONDS: &str = "300";
const BROWSER_HELPER_MAX_BYTES: u64 = 4096;
// webbrowser 1.2.2 treats a command whose basename is `curl` as a text browser,
// so it waits for the helper exit status before trying the next BROWSER entry.
// Keep the signed helper basename `curl` unless that dependency contract is requalified.
const TERMUX_BROWSER_OPEN_HELPER_IDENTITY: &str = "termux-browser-open";
const TERMUX_BROWSER_MANUAL_HELPER_IDENTITY: &str = "termux-browser-manual";
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

const FD_REMAP_RULES: [(&[u8], &[u8]); 4] = [
    (b"/etc/resolv.conf", b"/proc/self/fd/33"),
    (b"/etc/codex/config.toml", b"/dev/fd/34/config.toml"),
    (
        b"/etc/codex/requirements.toml",
        b"/dev/fd/34/requirements.toml",
    ),
    (
        b"/etc/codex/managed_config.toml",
        b"/dev/fd/34/managed_config.toml",
    ),
];

const REQUIRED_ARCHIVE_FILES: [&str; 2] = ["bin/codex", "codex-package.json"];

const USAGE: &str = concat!(
    "usage: codex-release-builder build --version <MAJOR.MINOR.PATCH> ",
    "--archive <ABSOLUTE_FILE> --archive-sha256 <LOWERCASE_SHA256> ",
    "--generation-id <ID> --wrapper <ABSOLUTE_FILE> ",
    "--creation-metadata <VALUE> ",
    "--gzip <ABSOLUTE_EXECUTABLE> [--openssl <PATH>] ",
    "--output <ABSENT_ABSOLUTE_DIRECTORY>"
);

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum BuilderError {
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
pub struct GenerationRequest {
    pub version: String,
    pub archive: PathBuf,
    pub archive_sha256: String,
    pub generation_id: String,
    pub wrapper: PathBuf,
    pub creation_metadata: String,
    pub gzip: PathBuf,
    /// Accepted for compatibility with existing callers.
    pub openssl: Option<PathBuf>,
    pub output: PathBuf,
}

#[derive(Default)]
struct RequestFields {
    version: Option<String>,
    archive: Option<PathBuf>,
    archive_sha256: Option<String>,
    generation_id: Option<String>,
    wrapper: Option<PathBuf>,
    creation_metadata: Option<String>,
    gzip: Option<PathBuf>,
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

fn parse_request<I, S>(args: I) -> Result<GenerationRequest, BuilderError>
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
        let value = args.next().ok_or(BuilderError::Usage)?;
        match flag.to_str() {
            Some("--version") => set_once(&mut fields.version, text_value(value)?)?,
            Some("--archive") => set_once(&mut fields.archive, PathBuf::from(value))?,
            Some("--archive-sha256") => set_once(&mut fields.archive_sha256, text_value(value)?)?,
            Some("--generation-id") => set_once(&mut fields.generation_id, text_value(value)?)?,
            Some("--wrapper") => set_once(&mut fields.wrapper, PathBuf::from(value))?,
            Some("--creation-metadata") => {
                set_once(&mut fields.creation_metadata, text_value(value)?)?
            }
            Some("--gzip") => set_once(&mut fields.gzip, PathBuf::from(value))?,
            Some("--openssl") => set_once(&mut fields.openssl, PathBuf::from(value))?,
            Some("--output") => set_once(&mut fields.output, PathBuf::from(value))?,
            _ => return Err(BuilderError::Usage),
        }
    }
    Ok(GenerationRequest {
        version: fields.version.ok_or(BuilderError::Usage)?,
        archive: fields.archive.ok_or(BuilderError::Usage)?,
        archive_sha256: fields.archive_sha256.ok_or(BuilderError::Usage)?,
        generation_id: fields.generation_id.ok_or(BuilderError::Usage)?,
        wrapper: fields.wrapper.ok_or(BuilderError::Usage)?,
        creation_metadata: fields.creation_metadata.ok_or(BuilderError::Usage)?,
        gzip: fields.gzip.ok_or(BuilderError::Usage)?,
        openssl: fields.openssl,
        output: fields.output.ok_or(BuilderError::Usage)?,
    })
}

pub fn valid_stable_version(value: &str) -> bool {
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

fn validate_request(request: &GenerationRequest) -> Result<(), BuilderError> {
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
    for path in [
        &request.archive,
        &request.wrapper,
        &request.gzip,
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
    let wrapper = ensure_regular_file(
        &request.wrapper,
        "inspect wrapper artifact",
        "wrapper artifact is not an executable regular file",
    )?;
    if wrapper.permissions().mode() & 0o111 == 0 {
        return Err(BuilderError::Invalid(
            "wrapper artifact is not an executable regular file",
        ));
    }
    if wrapper.len() > WRAPPER_ARTIFACT_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "wrapper artifact exceeds its byte bound",
        ));
    }
    ensure_executable(&request.gzip, "gzip is not an executable regular file")?;
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

fn hash_file(path: &Path) -> Result<String, BuilderError> {
    sha256_file(path).map_err(|source| io_error("hash generation input or asset", source))
}

fn create_dir_mode(path: &Path, mode: u32, operation: &'static str) -> Result<(), BuilderError> {
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(mode);
    builder
        .create(path)
        .map_err(|source| io_error(operation, source))
}

fn ensure_dir_chain_0755(root: &Path, relative_dir: &str) -> Result<(), BuilderError> {
    let mut current = root.to_path_buf();
    for component in relative_dir.split('/').filter(|part| !part.is_empty()) {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                if !meta.file_type().is_dir() {
                    return Err(BuilderError::Invalid(
                        "generation parent component is not a real directory",
                    ));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                create_dir_mode(&current, 0o755, "create generation subdirectory")?;
            }
            Err(source) => return Err(io_error("inspect generation subdirectory", source)),
        }
    }
    Ok(())
}

fn valid_package_bin_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.bytes().any(|b| b.is_ascii_control())
}

fn snapshot_wrapper_artifact(
    request: &GenerationRequest,
    staging: &Path,
) -> Result<String, BuilderError> {
    let source =
        File::open(&request.wrapper).map_err(|source| io_error("open wrapper artifact", source))?;
    let metadata = source
        .metadata()
        .map_err(|source| io_error("inspect opened wrapper artifact", source))?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(BuilderError::Invalid(
            "opened wrapper artifact is not an executable regular file",
        ));
    }
    if metadata.len() > WRAPPER_ARTIFACT_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "opened wrapper artifact exceeds its byte bound",
        ));
    }

    let snapshot_path = staging.join(".wrapper-artifact");
    let mut snapshot = create_private_file(&snapshot_path)?;
    let mut bounded = source.take(WRAPPER_ARTIFACT_MAX_BYTES.saturating_add(1));
    let copied = io::copy(&mut bounded, &mut snapshot)
        .map_err(|source| io_error("snapshot wrapper artifact", source))?;
    if copied > WRAPPER_ARTIFACT_MAX_BYTES {
        return Err(BuilderError::Invalid(
            "wrapper artifact exceeds its byte bound",
        ));
    }
    snapshot
        .sync_all()
        .map_err(|source| io_error("sync wrapper artifact snapshot", source))?;
    drop(snapshot);

    validate_android_aarch64_wrapper_elf(&snapshot_path)?;
    hash_file(&snapshot_path)
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

fn snapshot_archive(request: &GenerationRequest, staging: &Path) -> Result<PathBuf, BuilderError> {
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
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(io_error("read upstream archive", err)),
        };
        total += read as u64;
        if total > ARCHIVE_MAX_BYTES {
            return Err(BuilderError::Invalid(
                "upstream archive exceeds its byte bound",
            ));
        }
        snapshot
            .write_all(&buffer[..read])
            .map_err(|source| io_error("write upstream archive snapshot", source))?;
        digest.update(&buffer[..read]);
    }
    snapshot
        .sync_all()
        .map_err(|source| io_error("sync upstream archive snapshot", source))?;
    if digest.finish() != request.archive_sha256 {
        return Err(BuilderError::ArchiveDigestMismatch);
    }
    Ok(snapshot_path)
}

#[derive(Debug)]
struct ArchiveSelection {
    raw_runtime: PathBuf,
    extra_bins: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Regular,
    Directory,
    Pax,
    GnuLongName,
}

#[derive(Debug)]
struct TarHeader<PathName> {
    path: PathName,
    size: u64,
    mode: u64,
    kind: EntryKind,
}

fn parse_octal(field: &[u8]) -> Result<u64, BuilderError> {
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

fn parse_tar_number(field: &[u8]) -> Result<i128, BuilderError> {
    let Some((&first, rest)) = field.split_first() else {
        return Err(BuilderError::Archive("tar number is empty"));
    };
    if first & 0x80 == 0 {
        return parse_octal(field).map(i128::from);
    }
    // The top bit marks a binary number. The other bits are signed big endian.
    let leading = i128::from(first & 0x7f) - if first & 0x40 != 0 { 0x80 } else { 0 };
    Ok(rest
        .iter()
        .fold(leading, |value, byte| (value << 8) | i128::from(*byte)))
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

fn valid_archive_path(path: &[u8], directory: bool) -> bool {
    if path.len() > PATH_MAX_BYTES || path.contains(&b'\\') || path.iter().any(u8::is_ascii_control)
    {
        return false;
    }
    let body = if directory {
        path.strip_suffix(b"/").unwrap_or(path)
    } else {
        path
    };
    body.split(|&byte| byte == b'/')
        .all(|component| !matches!(component, b"" | b"." | b".."))
}

fn parse_tar_header(block: &[u8; TAR_BLOCK_BYTES]) -> Result<TarHeader<Vec<u8>>, BuilderError> {
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
    let prefix = match &block[257..265] {
        b"ustar\x0000" => header_text(&block[345..500])?,
        b"ustar  \0" => b"",
        _ => return Err(BuilderError::Archive("tar header is not USTAR or GNU tar")),
    };
    if block[157..257].iter().any(|byte| *byte != 0) {
        return Err(BuilderError::Archive("tar link metadata is unsupported"));
    }
    let mode = parse_octal(&block[100..108])?;
    parse_tar_number(&block[108..116])?;
    parse_tar_number(&block[116..124])?;
    let size = u64::try_from(parse_tar_number(&block[124..136])?)
        .map_err(|_| BuilderError::Archive("tar size is out of range"))?;
    parse_octal(&block[136..148])?;
    let name = header_text(&block[..100])?;
    if name.is_empty() {
        return Err(BuilderError::Archive("tar header name is empty"));
    }
    let path = if prefix.is_empty() {
        name.to_vec()
    } else {
        [prefix, b"/", name].concat()
    };
    let kind = match block[156] {
        0 | b'0' => EntryKind::Regular,
        b'5' => EntryKind::Directory,
        b'x' => EntryKind::Pax,
        b'L' => EntryKind::GnuLongName,
        _ => return Err(BuilderError::Archive("tar entry type is unsupported")),
    };
    Ok(TarHeader {
        path,
        size,
        mode,
        kind,
    })
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
    for byte in value.strip_prefix('-').unwrap_or(value).bytes() {
        match byte {
            b'0'..=b'9' if decimal => after += 1,
            b'0'..=b'9' => before += 1,
            b'.' if !decimal && before != 0 => decimal = true,
            _ => return false,
        }
    }
    before != 0 && (!decimal || after != 0)
}

#[derive(Default)]
struct PaxOverrides {
    path: Option<String>,
    size: Option<u64>,
}

fn parse_pax_payload(mut bytes: &[u8]) -> Result<PaxOverrides, BuilderError> {
    let mut overrides = PaxOverrides::default();
    let mut saw_mtime = false;
    let malformed = || BuilderError::Archive("PAX record is malformed");
    while !bytes.is_empty() {
        let space = bytes
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or_else(malformed)?;
        let length = std::str::from_utf8(&bytes[..space])
            .ok()
            .filter(|digits| digits.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|digits| digits.parse::<usize>().ok())
            .filter(|&length| length > space + 1 && length <= bytes.len())
            .ok_or(BuilderError::Archive("PAX record length is invalid"))?;
        let (record, rest) = bytes.split_at(length);
        bytes = rest;
        let record = record[space + 1..]
            .strip_suffix(b"\n")
            .ok_or_else(malformed)?;
        let record = std::str::from_utf8(record)
            .map_err(|_| BuilderError::Archive("PAX record is not UTF-8"))?;
        let (key, value) = record.split_once('=').ok_or_else(malformed)?;
        if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(malformed());
        }
        match key {
            "path" if overrides.path.is_none() => overrides.path = Some(value.to_owned()),
            "size"
                if overrides.size.is_none() && value.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                overrides.size = Some(value.parse().map_err(|_| malformed())?);
            }
            "mtime" if !saw_mtime && valid_pax_mtime(value) => saw_mtime = true,
            "path" | "size" | "mtime" => return Err(malformed()),
            // These can change how the next entry's bytes or type are interpreted.
            "linkpath" | "SCHILY.realsize" | "SCHILY.filetype" => {
                return Err(BuilderError::Archive(
                    "PAX link or sparse metadata is unsupported",
                ));
            }
            key if key.starts_with("GNU.sparse.") => {
                return Err(BuilderError::Archive("PAX sparse metadata is unsupported"));
            }
            _ => {}
        }
    }
    Ok(overrides)
}

// Resolve local extension headers before either the version scan or extraction
// sees the next logical entry. Only its resolved path and size reach callers.
fn read_tar_entry<R: Read>(reader: &mut R) -> Result<Option<TarHeader<String>>, BuilderError> {
    let mut pax = None;
    let mut long_name = None;
    loop {
        let block =
            read_block(reader)?.ok_or(BuilderError::Archive("tar stream has no end marker"))?;
        if block.iter().all(|byte| *byte == 0) {
            if pax.is_some() || long_name.is_some() {
                return Err(BuilderError::Archive(
                    "tar extension has no following entry",
                ));
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
            return Ok(None);
        }

        let header = parse_tar_header(&block)?;
        match header.kind {
            EntryKind::Pax => {
                if pax.is_some() || header.size == 0 || header.size > PAX_PAYLOAD_MAX_BYTES {
                    return Err(BuilderError::Archive("PAX header is malformed"));
                }
                let mut name = header.path.as_slice();
                while let Some(trimmed) = name.strip_prefix(b"./") {
                    name = trimmed;
                }
                if !valid_archive_path(name, false) {
                    return Err(BuilderError::Archive("PAX header name is not canonical"));
                }
                let payload = read_payload(reader, header.size, None, true)?;
                pax = Some(parse_pax_payload(&payload)?);
            }
            EntryKind::GnuLongName => {
                if long_name.is_some()
                    || header.size == 0
                    || header.size > PATH_MAX_BYTES as u64 + 1
                {
                    return Err(BuilderError::Archive("GNU long-name header is malformed"));
                }
                if header.path != b"././@LongLink" && !valid_archive_path(&header.path, false) {
                    return Err(BuilderError::Archive(
                        "GNU long-name header name is not canonical",
                    ));
                }
                let payload = read_payload(reader, header.size, None, true)?;
                if payload.last() != Some(&0) {
                    return Err(BuilderError::Archive("GNU long name is not NUL-terminated"));
                }
                long_name = Some(
                    std::str::from_utf8(header_text(&payload)?)
                        .map_err(|_| BuilderError::Archive("GNU long name is not UTF-8"))?
                        .to_owned(),
                );
            }
            EntryKind::Regular | EntryKind::Directory => {
                let directory = header.kind == EntryKind::Directory;
                let overrides = pax.unwrap_or_default();
                // GNU and PAX can truncate a placeholder in the middle of a
                // UTF-8 character. The raw name still cannot contain traversal.
                let has_override = long_name.is_some() || overrides.path.is_some();
                if !valid_archive_path(&header.path, directory || has_override) {
                    return Err(BuilderError::Archive("tar header path is not canonical"));
                }
                if long_name.is_some() && overrides.path.is_some() {
                    return Err(BuilderError::Archive(
                        "tar extensions specify conflicting paths",
                    ));
                }
                let mut path = match overrides.path.or(long_name) {
                    Some(path) => path,
                    None => String::from_utf8(header.path)
                        .map_err(|_| BuilderError::Archive("tar path is not UTF-8"))?,
                };
                let size = overrides.size.unwrap_or(header.size);
                if !valid_archive_path(path.as_bytes(), directory) {
                    return Err(BuilderError::Archive("tar entry path is not canonical"));
                }
                if directory && size != 0 {
                    return Err(BuilderError::Archive("directory entry is malformed"));
                }
                if directory && path.ends_with('/') {
                    path.pop();
                }
                return Ok(Some(TarHeader {
                    path,
                    size,
                    mode: header.mode,
                    kind: header.kind,
                }));
            }
        }
    }
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

fn parse_package_json_version(
    bytes: &[u8],
    expected_version: Option<&str>,
) -> Result<String, BuilderError> {
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

    let Some(version) = package_version.filter(|value| valid_stable_version(value)) else {
        return Err(package_json_error());
    };
    if !cursor.finished()
        || layout_version != Some(1)
        || expected_version.is_some_and(|expected| version != expected)
        || target.as_deref() != Some("aarch64-unknown-linux-musl")
        || variant.as_deref() != Some("codex")
        || entrypoint.as_deref() != Some("bin/codex")
        || resources_dir.as_deref() != Some("codex-resources")
        || path_dir.as_deref() != Some("codex-path")
    {
        return Err(package_json_error());
    }
    Ok(version)
}

fn validate_package_json(bytes: &[u8], version: &str) -> Result<(), BuilderError> {
    parse_package_json_version(bytes, Some(version)).map(|_| ())
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
        || !matches!(little_u16(&header[16..18]), 2 | 3)
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

fn validate_android_aarch64_wrapper_elf(path: &Path) -> Result<(), BuilderError> {
    let mut file = File::open(path).map_err(|source| io_error("open wrapper ELF", source))?;
    let file_len = file
        .metadata()
        .map_err(|source| io_error("inspect wrapper ELF", source))?
        .len();
    let mut header = [0u8; 64];
    file.read_exact(&mut header)
        .map_err(|source| io_error("read wrapper ELF header", source))?;
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
            "wrapper artifact is not a supported Android AArch64 PIE ELF",
        ));
    }
    let program_offset = little_u64(&header[32..40]);
    let program_count = u64::from(little_u16(&header[56..58]));
    if program_count == 0 {
        return Err(BuilderError::Invalid("wrapper ELF has no program headers"));
    }
    program_count
        .checked_mul(56)
        .and_then(|length| program_offset.checked_add(length))
        .filter(|end| *end <= file_len)
        .ok_or(BuilderError::Invalid(
            "wrapper ELF program headers are malformed",
        ))?;

    let mut interpreter = None;
    let mut program_header = [0u8; 56];
    for index in 0..program_count {
        file.seek(SeekFrom::Start(program_offset + index * 56))
            .map_err(|source| io_error("seek wrapper ELF program header", source))?;
        file.read_exact(&mut program_header)
            .map_err(|source| io_error("read wrapper ELF program header", source))?;
        if little_u32(&program_header[..4]) != 3 {
            continue;
        }
        if interpreter.is_some() {
            return Err(BuilderError::Invalid(
                "wrapper ELF has multiple PT_INTERP program headers",
            ));
        }
        let offset = little_u64(&program_header[8..16]);
        let size = little_u64(&program_header[32..40]);
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= file_len)
            .ok_or(BuilderError::Invalid(
                "wrapper ELF interpreter is malformed",
            ))?;
        let size = usize::try_from(size)
            .map_err(|_| BuilderError::Invalid("wrapper ELF interpreter is malformed"))?;
        let mut value = vec![0u8; size];
        file.seek(SeekFrom::Start(offset))
            .map_err(|source| io_error("seek wrapper ELF interpreter", source))?;
        file.read_exact(&mut value)
            .map_err(|source| io_error("read wrapper ELF interpreter", source))?;
        let _ = end;
        interpreter = Some(value);
    }
    if interpreter.as_deref() != Some(ANDROID_AARCH64_INTERPRETER) {
        return Err(BuilderError::Invalid(
            "wrapper artifact does not use the Android AArch64 dynamic linker",
        ));
    }
    Ok(())
}

fn write_staged_file(
    destination: &Path,
    bytes: &[u8],
    mode: u32,
    operation: &'static str,
) -> Result<(), BuilderError> {
    let mut file = create_private_file(destination)?;
    file.write_all(bytes)
        .map_err(|source| io_error(operation, source))?;
    file.sync_all()
        .map_err(|source| io_error(operation, source))?;
    drop(file);
    set_mode(destination, mode, operation)?;
    File::open(destination)
        .and_then(|f| f.sync_all())
        .map_err(|source| io_error(operation, source))
}

fn parse_archive<R: Read>(
    reader: &mut R,
    staging: &Path,
    version: &str,
) -> Result<ArchiveSelection, BuilderError> {
    let raw_runtime = staging.join(".raw-runtime");
    ensure_dir_chain_0755(staging, "bin")?;
    let mut extra_bins = Vec::new();
    let mut seen = BTreeSet::new();
    let mut logical_entries = 0usize;
    let mut total_payload = 0u64;
    let mut package_json = None;
    let mut rg_extracted = false;

    while let Some(header) = read_tar_entry(reader)? {
        logical_entries += 1;
        if logical_entries > LOGICAL_ENTRY_MAX {
            return Err(BuilderError::Archive(
                "archive entry count exceeds its bound",
            ));
        }
        if (REQUIRED_ARCHIVE_FILES.contains(&header.path.as_str())
            || header.path == "bin/codex-code-mode-host")
            && header.kind != EntryKind::Regular
        {
            return Err(BuilderError::Archive(
                "required archive entry is not a regular file",
            ));
        }
        if !seen.insert(header.path.clone()) {
            return Err(BuilderError::Archive("archive path is duplicated"));
        }

        if header.kind == EntryKind::Directory {
            let trimmed = header.path.as_str();
            if trimmed == "bin"
                || trimmed == "codex-path"
                || trimmed == "codex-resources"
                || trimmed.starts_with("codex-path/")
                || trimmed.starts_with("codex-resources/")
            {
                ensure_dir_chain_0755(staging, trimmed)?;
            }
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
            "codex-package.json" => {
                package_json = Some(read_payload(reader, header.size, None, true)?);
            }
            rel if rel.starts_with("bin/") => {
                let name = &rel["bin/".len()..];
                if !valid_package_bin_name(name) {
                    return Err(BuilderError::Archive("archive bin entry name is invalid"));
                }
                let dest = staging.join(rel);
                let mut file = create_private_file(&dest)?;
                read_payload(reader, header.size, Some(&mut file), false)?;
                file.sync_all()
                    .map_err(|source| io_error("sync selected bin entry", source))?;
                extra_bins.push(rel.to_owned());
            }
            "codex-path/rg" => {
                ensure_dir_chain_0755(staging, "codex-path")?;
                let dest = staging.join("codex-path/rg");
                let mut file = create_private_file(&dest)?;
                read_payload(reader, header.size, Some(&mut file), false)?;
                file.sync_all()
                    .map_err(|source| io_error("sync staged rg", source))?;
                drop(file);
                if validate_static_aarch64_elf(&dest).is_ok() {
                    set_mode(&dest, 0o755, "set static rg mode")?;
                    File::open(&dest)
                        .and_then(|f| f.sync_all())
                        .map_err(|source| io_error("sync static rg mode", source))?;
                } else {
                    std::fs::remove_file(&dest)
                        .map_err(|source| io_error("remove non-static rg", source))?;
                    write_staged_file(
                        &dest,
                        TERMUX_RG_WRAPPER_SCRIPT,
                        0o755,
                        "write Termux rg wrapper",
                    )?;
                }
                rg_extracted = true;
            }
            rel if rel.starts_with("codex-path/") || rel.starts_with("codex-resources/") => {
                if let Some((parent, _)) = rel.rsplit_once('/') {
                    ensure_dir_chain_0755(staging, parent)?;
                }
                let dest = staging.join(rel);
                let mut file = create_private_file(&dest)?;
                read_payload(reader, header.size, Some(&mut file), false)?;
                file.sync_all()
                    .map_err(|source| io_error("sync companion resource", source))?;
                drop(file);
                let mode = if header.mode & 0o111 != 0 {
                    0o755
                } else {
                    0o644
                };
                set_mode(&dest, mode, "set companion resource mode")?;
                File::open(&dest)
                    .and_then(|f| f.sync_all())
                    .map_err(|source| io_error("sync companion resource mode", source))?;
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
    let pkg_bytes = package_json
        .as_deref()
        .ok_or(BuilderError::Archive("archive layout is incomplete"))?;
    validate_package_json(pkg_bytes, version)?;
    write_staged_file(
        &staging.join("codex-package.json"),
        pkg_bytes,
        0o644,
        "write codex-package.json",
    )?;
    if !rg_extracted {
        ensure_dir_chain_0755(staging, "codex-path")?;
        write_staged_file(
            &staging.join("codex-path/rg"),
            TERMUX_RG_WRAPPER_SCRIPT,
            0o755,
            "write Termux rg wrapper",
        )?;
    }
    validate_static_aarch64_elf(&raw_runtime)?;
    for bin_path in &extra_bins {
        validate_static_aarch64_elf(&staging.join(bin_path))?;
    }
    Ok(ArchiveSelection {
        raw_runtime,
        extra_bins,
    })
}

fn select_archive(
    request: &GenerationRequest,
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
    let mut bounded = stdout.by_ref().take(DECOMPRESSED_ARCHIVE_MAX_BYTES + 1);
    let selected = parse_archive(&mut bounded, staging, &request.version).and_then(|selected| {
        if bounded.limit() == 0 {
            Err(BuilderError::Archive(
                "decompressed archive exceeds its total byte bound",
            ))
        } else {
            Ok(selected)
        }
    });
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
    companion_digests: BTreeMap<String, String>,
    browser_open_helper_sha256: String,
    browser_manual_helper_sha256: String,
    fd_counts: [usize; 4],
    changed_bytes: usize,
    uds_applied: bool,
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

fn write_browser_helper(path: &Path, contents: &[u8]) -> Result<String, BuilderError> {
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
    hash_file(path)
}

fn create_browser_helpers(staging: &Path) -> Result<(String, String), BuilderError> {
    let browser = staging.join("browser");
    create_private_dir(&browser)?;
    create_private_dir(&browser.join("open"))?;
    create_private_dir(&browser.join("manual"))?;
    let open_sha256 = write_browser_helper(
        &staging.join("browser/open/curl"),
        TERMUX_BROWSER_OPEN_HELPER,
    )?;
    let manual_sha256 = write_browser_helper(
        &staging.join("browser/manual/curl"),
        TERMUX_BROWSER_MANUAL_HELPER,
    )?;
    sync_directory(
        &staging.join("browser/open"),
        "sync browser open helper directory",
    )?;
    sync_directory(
        &staging.join("browser/manual"),
        "sync browser manual helper directory",
    )?;
    sync_directory(&staging.join("browser"), "sync browser helper root")?;
    Ok((open_sha256, manual_sha256))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FdRemapSummary {
    counts: [usize; 4],
    changed_bytes: usize,
}

fn apply_fd_path_remaps_with_options(
    runtime: &mut [u8],
    require_main_paths: bool,
) -> Result<FdRemapSummary, BuilderError> {
    let mut counts = [0usize; 4];
    let mut selected_offsets = Vec::new();
    for (index, (source, replacement)) in FD_REMAP_RULES.into_iter().enumerate() {
        if source.len() != replacement.len() {
            return Err(BuilderError::Invalid(
                "patch policy lengths are inconsistent",
            ));
        }
        if !occurrence_offsets(runtime, replacement).is_empty() {
            return Err(BuilderError::Archive(
                "runtime patch replacement is already present",
            ));
        }
        let source_offsets = occurrence_offsets(runtime, source);
        counts[index] = source_offsets.len();
        for offset in source_offsets {
            selected_offsets.push((offset, source, replacement));
        }
    }
    if require_main_paths && (counts[0] == 0 || counts[1] == 0) {
        return Err(BuilderError::Archive(
            "bin/codex must contain both /etc/resolv.conf and /etc/codex/config.toml",
        ));
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
    for (index, (source, replacement)) in FD_REMAP_RULES.into_iter().enumerate() {
        if !occurrence_offsets(runtime, source).is_empty()
            || occurrence_offsets(runtime, replacement).len() != counts[index]
        {
            return Err(BuilderError::Archive(
                "adapted runtime does not match patch policy",
            ));
        }
    }

    Ok(FdRemapSummary {
        counts,
        changed_bytes,
    })
}

#[cfg(test)]
pub(crate) fn apply_fd_path_remaps(runtime: &mut [u8]) -> Result<usize, BuilderError> {
    apply_fd_path_remaps_with_options(runtime, true).map(|summary| summary.changed_bytes)
}

fn adapt_selected_runtime<F>(
    staging: &Path,
    selected: ArchiveSelection,
    progress: &mut F,
) -> Result<AdaptedGeneration, BuilderError>
where
    F: FnMut(&str),
{
    let mut runtime = std::fs::read(&selected.raw_runtime)
        .map_err(|source| io_error("read selected raw runtime", source))?;
    let raw_runtime_sha256 = sha256_bytes(&runtime);
    progress("Patching bin/codex (FD remap: /etc/resolv.conf, /etc/codex)...");
    let fd = apply_fd_path_remaps_with_options(&mut runtime, true)?;
    let mut changed_bytes = fd.changed_bytes;

    let uds_applied = if runtime_patch::contains_daemon_fmt(&runtime) {
        progress("Patching bin/codex (UDS FD 35: /tmp sockets)...");
        changed_bytes +=
            runtime_patch::apply_uds_fd35(&mut runtime).map_err(BuilderError::Archive)?;
        true
    } else if runtime_patch::contains_daemon_prefix_literal(&runtime) {
        return Err(BuilderError::Archive(
            "bin/codex contains codex-daemon- without the supported FD 35 format",
        ));
    } else {
        false
    };

    let runtime_path = staging.join("bin/codex");
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

    let mut companion_digests = BTreeMap::new();
    for relative_path in selected.extra_bins {
        let bin_path = staging.join(&relative_path);
        progress(&format!("Patching companion {relative_path}..."));
        let mut companion = std::fs::read(&bin_path)
            .map_err(|source| io_error("read companion bin artifact", source))?;
        let c_fd = apply_fd_path_remaps_with_options(&mut companion, false)?;
        let c_uds = if runtime_patch::contains_daemon_fmt(&companion) {
            runtime_patch::apply_uds_fd35(&mut companion).map_err(BuilderError::Archive)?
        } else if runtime_patch::contains_daemon_prefix_literal(&companion) {
            return Err(BuilderError::Archive(
                "companion contains codex-daemon- without the supported FD 35 format",
            ));
        } else {
            0
        };
        if c_fd.changed_bytes > 0 || c_uds > 0 {
            let mut out = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&bin_path)
                .map_err(|source| io_error("open adapted companion bin", source))?;
            out.write_all(&companion)
                .map_err(|source| io_error("write adapted companion bin", source))?;
            out.sync_all()
                .map_err(|source| io_error("sync adapted companion bin", source))?;
        }
        set_mode(&bin_path, 0o755, "set companion bin mode")?;
        File::open(&bin_path)
            .and_then(|file| file.sync_all())
            .map_err(|source| io_error("sync companion bin after mode", source))?;
        companion_digests.insert(relative_path, hash_file(&bin_path)?);
    }
    sync_directory(&staging.join("bin"), "sync bin directory")?;

    let runtime_sha256 = hash_file(&runtime_path)?;
    progress("Creating Termux browser helpers...");
    let (browser_open_helper_sha256, browser_manual_helper_sha256) =
        create_browser_helpers(staging)?;
    std::fs::remove_file(&selected.raw_runtime)
        .map_err(|source| io_error("remove selected raw runtime", source))?;

    Ok(AdaptedGeneration {
        raw_runtime_sha256,
        runtime_sha256,
        companion_digests,
        browser_open_helper_sha256,
        browser_manual_helper_sha256,
        fd_counts: fd.counts,
        changed_bytes,
        uds_applied,
    })
}

fn write_generation_descriptor(
    request: &GenerationRequest,
    staging: &Path,
    adapted: &AdaptedGeneration,
    wrapper_sha256: &str,
) -> Result<(), BuilderError> {
    let mut patch_report = format!(
        "{};archive_sha256={};raw_runtime_sha256={};runtime_sha256={};code_mode_host_sha256={};source_counts={},{},{},{};changed_bytes={}",
        runtime_patch::PATCH_POLICY_ID,
        request.archive_sha256,
        adapted.raw_runtime_sha256,
        adapted.runtime_sha256,
        adapted
            .companion_digests
            .get("bin/codex-code-mode-host")
            .map_or("-", String::as_str),
        adapted.fd_counts[0],
        adapted.fd_counts[1],
        adapted.fd_counts[2],
        adapted.fd_counts[3],
        adapted.changed_bytes
    );
    if adapted.uds_applied {
        patch_report.push_str(&format!(";uds_policy={}", runtime_patch::UDS_POLICY_ID));
    }
    let mut descriptor = format!(
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
            "wrapper_artifact_digest\t{}\n",
            "wrapper_api_identity\t{}\n",
            "persistent_schema_identity\t{}\n",
            "qualification\tqualified\n",
            "creation_metadata\t{}\n",
            "helper_count\t2\n",
            "helper\t{}\t{}\n",
            "helper\t{}\t{}\n"
        ),
        GENERATION_FORMAT,
        request.generation_id,
        PACKAGE_IDENTITY,
        request.version,
        request.archive_sha256,
        runtime_patch::PATCH_POLICY_ID,
        patch_report,
        adapted.runtime_sha256,
        wrapper_sha256,
        WRAPPER_API_IDENTITY,
        PERSISTENT_SCHEMA_IDENTITY,
        request.creation_metadata,
        TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
        adapted.browser_open_helper_sha256,
        TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
        adapted.browser_manual_helper_sha256
    );
    for (path, digest) in &adapted.companion_digests {
        use std::fmt::Write as _;
        writeln!(descriptor, "companion\t{path}\t{digest}")
            .expect("writing into a String cannot fail");
    }
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

fn complete_and_publish<F>(
    request: &GenerationRequest,
    staging: &Path,
    selected: ArchiveSelection,
    wrapper_sha256: &str,
    progress: &mut F,
) -> Result<(), BuilderError>
where
    F: FnMut(&str),
{
    let adapted = adapt_selected_runtime(staging, selected, progress)?;
    rename_noreplace(&staging.join(".wrapper-artifact"), &staging.join("wrapper"))?;
    set_mode(
        &staging.join("wrapper"),
        0o755,
        "set generation wrapper mode",
    )?;
    File::open(staging.join("wrapper"))
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync generation wrapper", source))?;
    std::os::unix::fs::symlink("bin/codex", staging.join("codex"))
        .map_err(|source| io_error("create upstream codex entrypoint symlink", source))?;
    progress("Writing generation manifest & syncing files...");
    write_generation_descriptor(request, staging, &adapted, wrapper_sha256)?;
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

/// Assemble one unsigned generation using the same admission path as the CLI.
pub fn build_generation(request: &GenerationRequest) -> Result<(), BuilderError> {
    build_generation_with_progress(request, &mut |_| {})
}

/// Assemble one unsigned generation and emit patching stage progress messages.
pub fn build_generation_with_progress<F>(
    request: &GenerationRequest,
    progress: &mut F,
) -> Result<(), BuilderError>
where
    F: FnMut(&str),
{
    validate_request(request)?;
    let staging = create_staging(&request.output)?;
    let result = (|| {
        progress("Verifying archive & wrapper artifacts...");
        let wrapper_sha256 = snapshot_wrapper_artifact(request, &staging)?;
        let archive = snapshot_archive(request, &staging)?;
        progress("Extracting upstream package archive...");
        let selected = select_archive(request, &archive, &staging)?;
        std::fs::remove_file(&archive)
            .map_err(|source| io_error("remove upstream archive snapshot", source))?;
        complete_and_publish(request, &staging, selected, &wrapper_sha256, progress)
    })();
    match (result, cleanup_staging(&staging)) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn scan_package_archive_version<R: Read>(reader: &mut R) -> Result<String, BuilderError> {
    let mut seen = BTreeSet::new();
    let mut logical_entries = 0usize;
    let mut total_payload = 0u64;
    let mut package_json = None;

    while let Some(header) = read_tar_entry(reader)? {
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
        if header.path == "codex-package.json" {
            package_json = Some(read_payload(reader, header.size, None, true)?);
        } else {
            read_payload(reader, header.size, None, false)?;
        }
    }

    if REQUIRED_ARCHIVE_FILES
        .iter()
        .any(|path| !seen.contains(*path))
    {
        return Err(BuilderError::Archive("archive layout is incomplete"));
    }
    let pkg_bytes = package_json
        .as_deref()
        .ok_or(BuilderError::Archive("archive layout is incomplete"))?;
    parse_package_json_version(pkg_bytes, None)
}

/// Inspects an upstream `.tar.gz` package archive and returns its validated stable version.
pub fn inspect_package_archive_version(
    archive: &Path,
    gzip: &Path,
) -> Result<String, BuilderError> {
    let metadata = ensure_regular_file(
        archive,
        "inspect package archive",
        "package archive must be a regular file",
    )?;
    if metadata.len() == 0 || metadata.len() > ARCHIVE_MAX_BYTES {
        return Err(BuilderError::Invalid("archive exceeds its byte bound"));
    }
    ensure_executable(gzip, "gzip is not an executable regular file")?;
    let mut child = Command::new(gzip)
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
    let mut bounded = stdout.by_ref().take(DECOMPRESSED_ARCHIVE_MAX_BYTES + 1);
    let scanned = scan_package_archive_version(&mut bounded).and_then(|version| {
        if bounded.limit() == 0 {
            Err(BuilderError::Archive(
                "decompressed archive exceeds its total byte bound",
            ))
        } else {
            Ok(version)
        }
    });
    drop(stdout);
    if scanned.is_err() {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|source| io_error("wait for gzip", source))?;
    match scanned {
        Err(error) => Err(error),
        Ok(_) if !status.success() => Err(BuilderError::Tool("gzip decompression failed")),
        Ok(version) => Ok(version),
    }
}

pub fn run_from_args<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match args.first().and_then(|argument| argument.to_str()) {
        Some("build") => {
            let request = match parse_request(args) {
                Ok(request) => request,
                Err(error) => {
                    eprintln!("codex-release-builder: {error}");
                    return 2;
                }
            };
            match build_generation(&request) {
                Ok(()) => 0,
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
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub(crate) fn temp_dir(label: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "codex-release-builder-test-{label}-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn find_tool(name: &str) -> PathBuf {
        let mut candidates = Vec::new();
        if let Some(prefix) = std::env::var_os("PREFIX") {
            candidates.push(PathBuf::from(prefix).join("bin").join(name));
        }
        candidates.push(PathBuf::from("/data/data/com.termux/files/usr/bin").join(name));
        candidates.push(PathBuf::from("/usr/bin").join(name));
        candidates.push(PathBuf::from("/bin").join(name));
        candidates
            .into_iter()
            .find(|path| path.is_file())
            .unwrap_or_else(|| panic!("required test tool '{name}' not found"))
    }

    fn shell_executable() -> &'static str {
        if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        }
    }

    fn write_u16(buf: &mut [u8], off: usize, val: u16) {
        buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
    }

    fn write_u32(buf: &mut [u8], off: usize, val: u32) {
        buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    fn write_u64(buf: &mut [u8], off: usize, val: u64) {
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }

    fn build_wrapper_pie_elf(interp: Option<&[u8]>, e_type: u16) -> Vec<u8> {
        let interp_bytes = interp.unwrap_or(&[]);
        let ph_count: u16 = 1;
        let ph_off = 64usize;
        let payload_off = 128usize;
        let mut buf = vec![0u8; payload_off + interp_bytes.len().max(16)];
        buf[..4].copy_from_slice(b"\x7fELF");
        buf[4] = 2; // ELFCLASS64
        buf[5] = 1; // ELFDATA2LSB
        buf[6] = 1; // EV_CURRENT
        write_u16(&mut buf, 16, e_type);
        write_u16(&mut buf, 18, 183); // EM_AARCH64
        write_u32(&mut buf, 20, 1);
        write_u64(&mut buf, 32, ph_off as u64);
        write_u16(&mut buf, 52, 64);
        write_u16(&mut buf, 54, 56);
        write_u16(&mut buf, 56, ph_count);

        if let Some(bytes) = interp {
            write_u32(&mut buf, ph_off, 3); // PT_INTERP
            write_u64(&mut buf, ph_off + 8, payload_off as u64);
            write_u64(&mut buf, ph_off + 32, bytes.len() as u64);
            write_u64(&mut buf, ph_off + 40, bytes.len() as u64);
            buf[payload_off..payload_off + bytes.len()].copy_from_slice(bytes);
        } else {
            let file_len = buf.len() as u64;
            write_u32(&mut buf, ph_off, 1); // PT_LOAD
            write_u64(&mut buf, ph_off + 8, 0);
            write_u64(&mut buf, ph_off + 32, file_len);
        }
        buf
    }

    fn build_static_aarch64_elf(include_fd_sources: bool, include_daemon_uds: bool) -> Vec<u8> {
        let ph_off = 64usize;
        let rx_va = 0x400_000u64;
        let rx_off = 0usize;
        let rx_sz = 0x2000usize;
        let text_off = 0x1000usize;
        let text_sz = 0x400usize;
        let fini_off = 0x1400usize;
        let fini_sz = 0x20usize;
        let ro_off = 0x2000usize;
        let ro_va = 0x402_000u64;
        let ro_sz = 0x400usize;
        let shstr_off = 0x2400usize;
        let shstr = b"\0.text\0.fini\0.rodata\0.shstrtab\0";
        let sh_off = 0x2800usize;
        let sh_count = 5usize;
        let total = sh_off + sh_count * 64;
        let mut buf = vec![0u8; total];

        buf[..4].copy_from_slice(b"\x7fELF");
        buf[4] = 2;
        buf[5] = 1;
        buf[6] = 1;
        write_u16(&mut buf, 16, 2); // ET_EXEC
        write_u16(&mut buf, 18, 183); // EM_AARCH64
        write_u32(&mut buf, 20, 1);
        write_u64(&mut buf, 32, ph_off as u64);
        write_u64(&mut buf, 40, sh_off as u64);
        write_u16(&mut buf, 52, 64);
        write_u16(&mut buf, 54, 56);
        write_u16(&mut buf, 56, 2);
        write_u16(&mut buf, 58, 64);
        write_u16(&mut buf, 60, sh_count as u16);
        write_u16(&mut buf, 62, 4);

        // PT_LOAD 0: RX covering .text and .fini plus alignment cave before .rodata
        write_u32(&mut buf, ph_off, 1);
        write_u32(&mut buf, ph_off + 4, 5);
        write_u64(&mut buf, ph_off + 8, rx_off as u64);
        write_u64(&mut buf, ph_off + 16, rx_va);
        write_u64(&mut buf, ph_off + 32, rx_sz as u64);
        write_u64(&mut buf, ph_off + 40, rx_sz as u64);

        // PT_LOAD 1: RO covering .rodata
        let ph1 = ph_off + 56;
        write_u32(&mut buf, ph1, 1);
        write_u32(&mut buf, ph1 + 4, 4);
        write_u64(&mut buf, ph1 + 8, ro_off as u64);
        write_u64(&mut buf, ph1 + 16, ro_va);
        write_u64(&mut buf, ph1 + 32, 0x800);
        write_u64(&mut buf, ph1 + 40, 0x800);

        buf[shstr_off..shstr_off + shstr.len()].copy_from_slice(shstr);
        let write_sh =
            |b: &mut [u8], idx: usize, name_off: u32, sh_type: u32, off: usize, sz: usize| {
                let base = sh_off + idx * 64;
                write_u32(b, base, name_off);
                write_u32(b, base + 4, sh_type);
                write_u64(b, base + 24, off as u64);
                write_u64(b, base + 32, sz as u64);
            };
        write_sh(&mut buf, 1, 1, 1, text_off, text_sz);
        write_sh(&mut buf, 2, 7, 1, fini_off, fini_sz);
        write_sh(&mut buf, 3, 13, 1, ro_off, ro_sz);
        write_sh(&mut buf, 4, 21, 3, shstr_off, shstr.len());

        let mut ro_cursor = ro_off;
        if include_daemon_uds {
            let fmt = b"\x0dcodex-daemon-\xc0\0";
            buf[ro_cursor..ro_cursor + fmt.len()].copy_from_slice(fmt);
            ro_cursor += 0x20;

            let site1_off = text_off + 0x80;
            let site1_insns = [
                0x528e_85e8u32, // movz w8, #0x742f ("/t")
                0x9100_63e0u32, // add x0, sp, #0x18
                0x72ae_0da8u32, // movk w8, #0x706d, lsl #16 ("mp")
                0x5280_00a1u32, // mov w1, #5
                0x3900_73ffu32, // strb wzr, [sp, #0x1c]
                0xb900_1be8u32, // str w8, [sp, #0x18]
            ];
            for (i, insn) in site1_insns.into_iter().enumerate() {
                write_u32(&mut buf, site1_off + i * 4, insn);
            }

            let site2_off = text_off + 0x140;
            // PC page is 0x401_000, fmt VA is 0x402_000 => page delta = +1, lo12 = 0.
            write_u32(&mut buf, site2_off, 0xb000_0000);
            write_u32(&mut buf, site2_off + 4, 0x9100_0000);

            crate::runtime_patch::write_synthetic_realpath_fixture(
                &mut buf,
                rx_va,
                text_off,
                Some(text_off + 0x110),
                false,
                false,
                false,
            );
        }

        if include_fd_sources {
            for (source, _) in FD_REMAP_RULES {
                buf[ro_cursor..ro_cursor + source.len()].copy_from_slice(source);
                ro_cursor += source.len() + 1;
            }
        }
        buf
    }

    fn append_tar_entry(tar: &mut Vec<u8>, path: &str, typeflag: u8, mode: u32, body: &[u8]) {
        let mut header = [0u8; 512];
        if path.len() <= 100 {
            header[..path.len()].copy_from_slice(path.as_bytes());
        } else {
            let (prefix, name) = path.rsplit_once('/').unwrap();
            header[..name.len()].copy_from_slice(name.as_bytes());
            header[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
        }
        header[100..108].copy_from_slice(format!("{mode:07o}\0").as_bytes());
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        header[124..136].copy_from_slice(format!("{:011o}\0", body.len()).as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[156] = typeflag;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        set_tar_checksum(&mut header);
        tar.extend_from_slice(&header);
        tar.extend_from_slice(body);
        let pad = (512 - (body.len() % 512)) % 512;
        tar.resize(tar.len() + pad, 0);
    }

    fn set_tar_checksum(header: &mut [u8]) {
        header[148..156].fill(b' ');
        let sum: u32 = header.iter().map(|b| u32::from(*b)).sum();
        header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    }

    fn pax_record(key: &str, value: &str) -> String {
        let record = format!("{key}={value}\n");
        let mut length = record.len() + 2;
        loop {
            let encoded = format!("{length} {record}");
            if encoded.len() == length {
                return encoded;
            }
            length = encoded.len();
        }
    }

    fn write_tar_gz(tar_bytes: &[u8], out_gz: &Path, gzip: &Path) {
        let raw_tar = out_gz.with_extension("tar");
        std::fs::write(&raw_tar, tar_bytes).unwrap();
        let compressed = Command::new(gzip)
            .args(["-cn", "--"])
            .arg(&raw_tar)
            .output()
            .unwrap();
        assert!(compressed.status.success());
        std::fs::write(out_gz, &compressed.stdout).unwrap();
        let _ = std::fs::remove_file(&raw_tar);
    }

    fn package_tar_bytes(entries: &[(&str, u8, u32, &[u8])]) -> Vec<u8> {
        let mut tar = Vec::new();
        for &(path, typeflag, mode, body) in entries {
            append_tar_entry(&mut tar, path, typeflag, mode, body);
        }
        tar.resize(tar.len() + 1024, 0);
        tar
    }

    #[test]
    fn cli_parses_build_and_rejects_invalid_or_retired_subcommands() {
        let digest = "a".repeat(64);
        let args = [
            "build",
            "--version",
            "1.2.3",
            "--archive",
            "/tmp/a.tar.gz",
            "--archive-sha256",
            &digest,
            "--generation-id",
            "gen-1",
            "--wrapper",
            "/tmp/wrapper",
            "--creation-metadata",
            "codex-wrapper",
            "--gzip",
            "/usr/bin/gzip",
            "--output",
            "/tmp/out",
        ];
        let parsed = parse_request(args).unwrap();
        assert_eq!(parsed.version, "1.2.3");
        assert_eq!(parsed.creation_metadata, "codex-wrapper");
        assert_eq!(parsed.openssl, None);
        let legacy =
            parse_request(args.into_iter().chain(["--openssl", "/unused/openssl"])).unwrap();
        assert_eq!(legacy.openssl, Some(PathBuf::from("/unused/openssl")));

        // Duplicate flag rejected.
        assert!(matches!(
            parse_request(["build", "--version", "1.2.3", "--version", "1.2.4",]),
            Err(BuilderError::Usage)
        ));
        // Removed subcommands fail with usage code 2.
        assert_eq!(run_from_args(["prepare", "--version", "1.2.3"]), 2);
        assert_eq!(run_from_args(["fetch"]), 2);
        assert_eq!(run_from_args(["publish"]), 2);
    }

    #[test]
    fn version_and_package_json_validation_are_structural() {
        assert!(binary_adaptation_available("0.1.0"));
        assert!(binary_adaptation_available("12.34.56"));
        assert!(!binary_adaptation_available("01.2.3"));
        assert!(!binary_adaptation_available("1.2"));
        assert!(!binary_adaptation_available("1.2.3-alpha.1"));

        let valid = expected_package_json("1.2.3");
        assert_eq!(parse_package_json_version(&valid, None).unwrap(), "1.2.3");
        assert!(validate_package_json(&valid, "1.2.4").is_err());

        let dup_key = b"{\"layoutVersion\":1,\"version\":\"1.2.3\",\"version\":\"1.2.3\",\"target\":\"aarch64-unknown-linux-musl\",\"variant\":\"codex\",\"entrypoint\":\"bin/codex\",\"resourcesDir\":\"codex-resources\",\"pathDir\":\"codex-path\"}";
        assert!(parse_package_json_version(dup_key, None).is_err());

        let wrong_layout = b"{\"layoutVersion\":2,\"version\":\"1.2.3\",\"target\":\"aarch64-unknown-linux-musl\",\"variant\":\"codex\",\"entrypoint\":\"bin/codex\",\"resourcesDir\":\"codex-resources\",\"pathDir\":\"codex-path\"}";
        assert!(parse_package_json_version(wrong_layout, None).is_err());
    }

    #[test]
    fn elf_validators_enforce_android_wrapper_pie_and_static_upstream_formats() {
        let dir = temp_dir("elf");
        let good_wrapper = dir.join("wrapper-good");
        let wrong_interp = dir.join("wrapper-wrong-interp");
        let exec_wrapper = dir.join("wrapper-et-exec");
        let static_bin = dir.join("upstream-static");
        let static_pie = dir.join("upstream-static-pie");

        std::fs::write(
            &good_wrapper,
            build_wrapper_pie_elf(Some(b"/system/bin/linker64\0"), 3),
        )
        .unwrap();
        std::fs::write(
            &wrong_interp,
            build_wrapper_pie_elf(Some(b"/lib/ld-musl-aarch64.so.1\0"), 3),
        )
        .unwrap();
        std::fs::write(
            &exec_wrapper,
            build_wrapper_pie_elf(Some(b"/system/bin/linker64\0"), 2),
        )
        .unwrap();
        std::fs::write(&static_bin, build_static_aarch64_elf(true, false)).unwrap();

        let mut pie = build_static_aarch64_elf(true, false);
        write_u16(&mut pie, 16, 3);
        std::fs::write(&static_pie, pie).unwrap();

        assert!(validate_android_aarch64_wrapper_elf(&good_wrapper).is_ok());
        assert!(validate_android_aarch64_wrapper_elf(&wrong_interp).is_err());
        assert!(validate_android_aarch64_wrapper_elf(&exec_wrapper).is_err());
        assert!(validate_static_aarch64_elf(&static_bin).is_ok());
        assert!(validate_static_aarch64_elf(&static_pie).is_ok());
        assert!(matches!(
            validate_static_aarch64_elf(&good_wrapper),
            Err(BuilderError::Archive(
                "selected executable has a PT_INTERP program header"
            ))
        ));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn build_generation_remaps_fds_relocates_uds_builds_helpers_and_guards_failures() {
        let dir = temp_dir("build-gen");
        let gzip = find_tool("gzip");
        let wrapper = dir.join("codex-wrapper");
        std::fs::write(
            &wrapper,
            build_wrapper_pie_elf(Some(b"/system/bin/linker64\0"), 3),
        )
        .unwrap();
        set_mode(&wrapper, 0o755, "mode test wrapper").unwrap();

        let pkg_json = expected_package_json("1.2.3");
        let main_elf = build_static_aarch64_elf(true, true);
        let companion_elf = build_static_aarch64_elf(true, false);
        let static_rg = build_static_aarch64_elf(false, false);

        // Case 1: Full package with daemon UDS, companion exec-server, and non-ELF rg wrapper fallback.
        let tar1 = package_tar_bytes(&[
            ("bin/codex", b'0', 0o755, &main_elf),
            ("bin/extra", b'0', 0o755, &static_rg),
            ("bin/codex-exec-server", b'0', 0o755, &companion_elf),
            ("bin/codex-code-mode-host", b'0', 0o755, &companion_elf),
            ("codex-package.json", b'0', 0o644, &pkg_json),
            ("codex-resources/notice.txt", b'0', 0o644, b"ok\n"),
            ("codex-path/rg", b'0', 0o755, b"#!/bin/sh\n"),
        ]);
        let archive1 = dir.join("pkg1.tar.gz");
        write_tar_gz(&tar1, &archive1, &gzip);
        assert_eq!(
            inspect_package_archive_version(&archive1, &gzip).unwrap(),
            "1.2.3"
        );
        let sha1 = sha256_file(&archive1).unwrap();
        let out1 = dir.join("gen-with-uds");
        build_generation(&GenerationRequest {
            version: "1.2.3".to_owned(),
            archive: archive1.clone(),
            archive_sha256: sha1.clone(),
            generation_id: "gen-uds".to_owned(),
            wrapper: wrapper.clone(),
            creation_metadata: "codex-wrapper".to_owned(),
            gzip: gzip.clone(),
            openssl: Some(PathBuf::from("nonexistent/unused-openssl")),
            output: out1.clone(),
        })
        .unwrap();

        let meta1 = std::fs::read_to_string(out1.join("generation.meta")).unwrap();
        assert!(meta1.starts_with("codex-local-generation\n"));
        assert!(meta1.contains("patch_policy_id\ttermux-fd-remap\n"));
        assert!(meta1.contains(";uds_policy=termux-uds-fd35\n"));
        assert!(meta1.contains("wrapper_api_identity\twrapper-api\n"));
        assert!(meta1.contains("persistent_schema_identity\tschema\n"));
        assert!(meta1.contains("helper_count\t2\n"));
        assert!(meta1.contains("helper\ttermux-browser-open\t"));
        assert!(meta1.contains("helper\ttermux-browser-manual\t"));
        assert!(!meta1.contains("manager_artifact_digest"));
        assert!(!meta1.contains("upstream_doctor"));

        let expected_rows: Vec<_> = [
            "bin/codex-code-mode-host",
            "bin/codex-exec-server",
            "bin/extra",
        ]
        .into_iter()
        .map(|name| {
            format!(
                "companion\t{name}\t{}",
                sha256_file(&out1.join(name)).unwrap()
            )
        })
        .collect();
        let tail: Vec<_> = meta1
            .lines()
            .skip_while(|line| !line.starts_with("helper\ttermux-browser-manual\t"))
            .skip(1)
            .collect();
        assert_eq!(tail, expected_rows);
        assert_eq!(std::fs::read(out1.join("bin/extra")).unwrap(), static_rg);

        let patched_main = std::fs::read(out1.join("bin/codex")).unwrap();
        let patched_companion = std::fs::read(out1.join("bin/codex-exec-server")).unwrap();
        assert!(occurrence_offsets(&patched_main, b"/etc/resolv.conf").is_empty());
        assert!(!occurrence_offsets(&patched_main, b"/proc/self/fd/33").is_empty());
        assert!(!occurrence_offsets(&patched_main, b"/proc/self/fd/35\0").is_empty());
        assert!(!occurrence_offsets(&patched_companion, b"/proc/self/fd/33").is_empty());
        assert_eq!(
            std::fs::read(out1.join("codex-path/rg")).unwrap(),
            TERMUX_RG_WRAPPER_SCRIPT
        );

        // Verify browser helpers enforce single HTTP/HTTPS URL contract.
        let mock_prefix = dir.join("prefix");
        std::fs::create_dir_all(mock_prefix.join("bin")).unwrap();
        let captured_url = dir.join("opened-url");
        let opener = mock_prefix.join("bin/termux-open-url");
        std::fs::write(
            &opener,
            format!(
                "#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n",
                captured_url.display()
            ),
        )
        .unwrap();
        set_mode(&opener, 0o755, "mode mock opener").unwrap();

        let sh = shell_executable();
        let open_ok = Command::new(sh)
            .arg(out1.join("browser/open/curl"))
            .arg("https://example.com/auth?code=1")
            .env("PREFIX", &mock_prefix)
            .env("CODEX_TERMUX_URL_OPENER", &opener)
            .status()
            .unwrap();
        assert!(open_ok.success());
        assert_eq!(
            std::fs::read_to_string(&captured_url).unwrap(),
            "https://example.com/auth?code=1"
        );
        let open_bad = Command::new(sh)
            .arg(out1.join("browser/open/curl"))
            .arg("file:///etc/passwd")
            .env("PREFIX", &mock_prefix)
            .env("CODEX_TERMUX_URL_OPENER", &opener)
            .status()
            .unwrap();
        assert_eq!(open_bad.code(), Some(64));

        // Case 2: Runtime without daemon UDS omits uds_policy; static AArch64 rg is preserved.
        let no_uds_elf = build_static_aarch64_elf(true, false);
        let tar2 = package_tar_bytes(&[
            ("bin/codex", b'0', 0o755, &no_uds_elf),
            ("codex-package.json", b'0', 0o644, &pkg_json),
            ("codex-path/rg", b'0', 0o755, &static_rg),
        ]);
        let archive2 = dir.join("pkg2.tar.gz");
        write_tar_gz(&tar2, &archive2, &gzip);
        let sha2 = sha256_file(&archive2).unwrap();
        let out2 = dir.join("gen-no-uds");
        build_generation(&GenerationRequest {
            version: "1.2.3".to_owned(),
            archive: archive2,
            archive_sha256: sha2,
            generation_id: "gen-plain".to_owned(),
            wrapper: wrapper.clone(),
            creation_metadata: "codex-wrapper".to_owned(),
            gzip: gzip.clone(),
            openssl: None,
            output: out2.clone(),
        })
        .unwrap();
        let meta2 = std::fs::read_to_string(out2.join("generation.meta")).unwrap();
        assert!(!meta2.contains("uds_policy="));
        assert_eq!(
            std::fs::read(out2.join("codex-path/rg")).unwrap(),
            static_rg
        );

        // Case 3: Existing output is not overwritten and wrong digest fails cleanly.
        assert!(build_generation(&GenerationRequest {
            version: "1.2.3".to_owned(),
            archive: archive1.clone(),
            archive_sha256: sha1,
            generation_id: "gen-uds".to_owned(),
            wrapper: wrapper.clone(),
            creation_metadata: "codex-wrapper".to_owned(),
            gzip: gzip.clone(),
            openssl: None,
            output: out1.clone(),
        })
        .is_err());

        let out_bad_sha = dir.join("gen-bad-sha");
        assert!(matches!(
            build_generation(&GenerationRequest {
                version: "1.2.3".to_owned(),
                archive: archive1,
                archive_sha256: "b".repeat(64),
                generation_id: "gen-bad".to_owned(),
                wrapper,
                creation_metadata: "codex-wrapper".to_owned(),
                gzip,
                openssl: None,
                output: out_bad_sha.clone(),
            }),
            Err(BuilderError::ArchiveDigestMismatch)
        ));
        assert!(!out_bad_sha.exists());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn build_rejects_uds_layout_drift_in_main_and_companions_without_publication() {
        let dir = temp_dir("uds-layout-drift");
        let gzip = find_tool("gzip");
        let wrapper = dir.join("wrapper");
        std::fs::write(
            &wrapper,
            build_wrapper_pie_elf(Some(b"/system/bin/linker64\0"), 3),
        )
        .unwrap();
        set_mode(&wrapper, 0o755, "mode test wrapper").unwrap();
        let package = expected_package_json("1.2.3");
        let plain = build_static_aarch64_elf(true, false);
        let mut drifted = build_static_aarch64_elf(true, true);
        // Still a valid static executable, but its section table was stripped.
        // The UDS implementation is present and must not be mistaken for absence.
        write_u64(&mut drifted, 40, 0);
        write_u16(&mut drifted, 60, 0);
        write_u16(&mut drifted, 62, 0);
        let mut unsupported_companion = plain.clone();
        unsupported_companion.extend_from_slice(b"codex-daemon-%u\0");
        for (name, main, companion) in [
            ("main", &drifted, &plain),
            ("companion", &plain, &drifted),
            ("unsupported-companion", &plain, &unsupported_companion),
        ] {
            let archive = dir.join(format!("{name}.tar.gz"));
            write_tar_gz(
                &package_tar_bytes(&[
                    ("bin/codex", b'0', 0o755, main),
                    ("bin/codex-exec-server", b'0', 0o755, companion),
                    ("codex-package.json", b'0', 0o644, &package),
                ]),
                &archive,
                &gzip,
            );
            let output = dir.join(format!("gen-{name}"));
            let result = build_generation(&GenerationRequest {
                version: "1.2.3".to_owned(),
                archive_sha256: sha256_file(&archive).unwrap(),
                archive,
                generation_id: format!("gen-{name}"),
                wrapper: wrapper.clone(),
                creation_metadata: "codex-wrapper".to_owned(),
                gzip: gzip.clone(),
                openssl: None,
                output: output.clone(),
            });
            assert!(
                matches!(result, Err(BuilderError::Archive(_))),
                "{name}: {result:?}"
            );
            assert!(!output.exists(), "failed candidate must not be published");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn main_requires_both_fd_literals_and_rejects_unsupported_daemon_prefix() {
        let dir = temp_dir("main-patch-policy");
        let gzip = find_tool("gzip");
        let wrapper = dir.join("wrapper");
        std::fs::write(
            &wrapper,
            build_wrapper_pie_elf(Some(b"/system/bin/linker64\0"), 3),
        )
        .unwrap();
        set_mode(&wrapper, 0o755, "mode test wrapper").unwrap();
        let package = expected_package_json("1.2.3");

        let plain = build_static_aarch64_elf(true, false);
        let mut bad_daemon = plain.clone();
        bad_daemon.extend_from_slice(b"codex-daemon-%u\0");
        let mut cases = vec![(
            "unsupported-daemon",
            bad_daemon,
            "bin/codex contains codex-daemon- without the supported FD 35 format",
        )];
        for (name, index) in [("missing-resolver", 0), ("missing-config", 1)] {
            let mut image = plain.clone();
            let needle = FD_REMAP_RULES[index].0;
            let offsets = occurrence_offsets(&image, needle);
            assert!(!offsets.is_empty());
            for offset in offsets {
                image[offset..offset + needle.len()].fill(0);
            }
            assert!(!occurrence_offsets(&image, FD_REMAP_RULES[1 - index].0).is_empty());
            cases.push((
                name,
                image,
                "bin/codex must contain both /etc/resolv.conf and /etc/codex/config.toml",
            ));
        }

        for (name, image, expected_error) in cases {
            let archive = dir.join(format!("{name}.tar.gz"));
            write_tar_gz(
                &package_tar_bytes(&[
                    ("bin/codex", b'0', 0o755, &image),
                    ("codex-package.json", b'0', 0o644, &package),
                ]),
                &archive,
                &gzip,
            );
            let output = dir.join(name);
            let result = build_generation(&GenerationRequest {
                version: "1.2.3".to_owned(),
                archive_sha256: sha256_file(&archive).unwrap(),
                archive,
                generation_id: name.to_owned(),
                wrapper: wrapper.clone(),
                creation_metadata: "codex-wrapper".to_owned(),
                gzip: gzip.clone(),
                openssl: None,
                output: output.clone(),
            });
            assert!(
                matches!(result, Err(BuilderError::Archive(error)) if error == expected_error),
                "{name}: {result:?}"
            );
            assert!(!output.exists());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tar_extensions_apply_the_same_names_and_sizes_to_inspection_and_extraction() {
        let dir = temp_dir("tar-extended");
        let mut tar = Vec::new();
        let runtime = build_static_aarch64_elf(true, false);
        append_tar_entry(&mut tar, "bin/codex", b'0', 0o755, &runtime);
        let header = &mut tar[..TAR_BLOCK_BYTES];
        header[108..116].copy_from_slice(&[0x80, 0, 0, 0, 1, 0, 0, 0]);
        header[116..124].copy_from_slice(&[0xff; 8]);
        header[124..136].fill(0);
        header[124] = 0x80;
        header[128..136].copy_from_slice(&(runtime.len() as u64).to_be_bytes());
        set_tar_checksum(header);

        let ustar_path = format!("codex-resources/{}/ustar.txt", "u".repeat(95));
        append_tar_entry(&mut tar, &ustar_path, b'0', 0o644, b"ustar");

        let gnu_path = format!("codex-resources/{}é{}", "g".repeat(83), "n".repeat(60));
        let gnu_at = tar.len();
        append_tar_entry(
            &mut tar,
            "././@LongLink",
            b'L',
            0o644,
            format!("{gnu_path}\0").as_bytes(),
        );
        let gnu_target = tar.len();
        append_tar_entry(&mut tar, "placeholder", b'0', 0o644, b"gnu");
        tar[gnu_target..gnu_target + 100].copy_from_slice(&gnu_path.as_bytes()[..100]);
        for at in [gnu_at, gnu_target] {
            let header = &mut tar[at..at + TAR_BLOCK_BYTES];
            header[257..265].copy_from_slice(b"ustar  \0");
            set_tar_checksum(header);
        }

        let package = expected_package_json("1.2.3");
        let mut pax = [
            pax_record("path", "codex-package.json"),
            pax_record("size", &package.len().to_string()),
            pax_record("mtime", "-1.25"),
        ]
        .concat();
        let key = "custom.metadata";
        let record_overhead = 4 + key.len() + 3; // Length, space, '=', newline
        let value_len = PAX_PAYLOAD_MAX_BYTES as usize - pax.len() - record_overhead;
        pax.push_str(&pax_record(key, &"x".repeat(value_len)));
        assert_eq!(pax.len(), 4096);
        append_tar_entry(
            &mut tar,
            "PaxHeaders/safe-name",
            b'x',
            0o644,
            pax.as_bytes(),
        );
        let pax_target = tar.len();
        append_tar_entry(&mut tar, "placeholder.json", b'0', 0o644, &package);
        let header = &mut tar[pax_target..pax_target + TAR_BLOCK_BYTES];
        header[124..136].copy_from_slice(b"00000000000\0");
        set_tar_checksum(header);
        tar.resize(tar.len() + 2 * TAR_BLOCK_BYTES, 0);

        assert_eq!(
            scan_package_archive_version(&mut tar.as_slice()).unwrap(),
            "1.2.3"
        );
        parse_archive(&mut tar.as_slice(), &dir, "1.2.3").unwrap();
        for (path, contents) in [(&ustar_path, &b"ustar"[..]), (&gnu_path, &b"gnu"[..])] {
            assert_eq!(std::fs::read(dir.join(path)).unwrap(), contents);
        }
        assert_eq!(
            std::fs::read(dir.join("codex-package.json")).unwrap(),
            package
        );
        assert!(!dir.join("placeholder.json").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tar_extensions_reject_unsafe_paths_links_and_malformed_records() {
        let long_name = b"codex-resources/../../escape\0";
        let path_escape = pax_record("path", "codex-resources/../escape");
        let absolute_path = pax_record("path", "/escape");
        let linkpath = pax_record("linkpath", "../escape");
        let sparse = pax_record("GNU.sparse.size", "1");
        let duplicate = pax_record("path", "bin/codex").repeat(2);
        let oversized = pax_record(
            "custom.metadata",
            &"x".repeat(PAX_PAYLOAD_MAX_BYTES as usize),
        );
        for (name, kind, payload, target, target_kind) in [
            ("././@LongLink", b'L', &long_name[..], "bin/codex", b'0'),
            (
                "././@LongLink",
                b'L',
                &b"missing-nul"[..],
                "bin/codex",
                b'0',
            ),
            ("../metadata", b'x', &b"10 mtime=0\n"[..], "bin/codex", b'0'),
            ("PaxHeader", b'x', path_escape.as_bytes(), "bin/codex", b'0'),
            (
                "PaxHeader",
                b'x',
                absolute_path.as_bytes(),
                "bin/codex",
                b'0',
            ),
            ("PaxHeader", b'x', linkpath.as_bytes(), "bin/codex", b'0'),
            ("PaxHeader", b'x', sparse.as_bytes(), "bin/codex", b'0'),
            ("PaxHeader", b'x', duplicate.as_bytes(), "bin/codex", b'0'),
            ("PaxHeader", b'x', oversized.as_bytes(), "bin/codex", b'0'),
            ("PaxHeader", b'x', &b"999 mtime=0\n"[..], "bin/codex", b'0'),
            ("PaxHeader", b'x', &b"11 mtime=0\n"[..], "bin/codex", b'2'),
        ] {
            let tar = package_tar_bytes(&[
                (name, kind, 0o644, payload),
                (target, target_kind, 0o755, b""),
            ]);
            assert!(
                read_tar_entry(&mut tar.as_slice()).is_err(),
                "{name} {kind} {payload:?}"
            );
        }
        let prefixed_escape = format!("{}/../escape", "p".repeat(101));
        let tar = package_tar_bytes(&[(&prefixed_escape, b'0', 0o644, b"")]);
        assert!(read_tar_entry(&mut tar.as_slice()).is_err());

        let mut negative = package_tar_bytes(&[("bin/codex", b'0', 0o755, b"")]);
        negative[124..136].fill(0xff);
        set_tar_checksum(&mut negative[..512]);
        assert!(read_tar_entry(&mut negative.as_slice()).is_err());
    }

    #[test]
    fn compressed_archive_bound_is_512_mib() {
        let dir = temp_dir("archive-bound");
        let archive = dir.join("oversized.tar.gz");
        let file = File::create(&archive).unwrap();
        file.set_len(512 * 1024 * 1024 + 1).unwrap();
        assert_eq!(ARCHIVE_MAX_BYTES, 512 * 1024 * 1024);
        assert!(matches!(
            inspect_package_archive_version(&archive, &dir.join("unused-gzip")),
            Err(BuilderError::Invalid("archive exceeds its byte bound"))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tar_logical_entry_count_has_the_1024_entry_bound() {
        let mut tar = package_tar_bytes(&[
            ("bin/codex", b'0', 0o755, b""),
            (
                "codex-package.json",
                b'0',
                0o644,
                &expected_package_json("1.2.3"),
            ),
        ]);
        tar.truncate(tar.len() - 2 * TAR_BLOCK_BYTES);
        for index in 2..LOGICAL_ENTRY_MAX {
            append_tar_entry(&mut tar, &format!("resource-{index}"), b'0', 0o644, b"");
        }
        let end = tar.len();
        tar.resize(end + 2 * TAR_BLOCK_BYTES, 0);
        assert_eq!(
            scan_package_archive_version(&mut tar.as_slice()).unwrap(),
            "1.2.3"
        );
        tar.truncate(end);
        append_tar_entry(&mut tar, "overflow", b'0', 0o644, b"");
        tar.resize(tar.len() + 2 * TAR_BLOCK_BYTES, 0);
        assert!(matches!(
            scan_package_archive_version(&mut tar.as_slice()),
            Err(BuilderError::Archive(
                "archive entry count exceeds its bound"
            ))
        ));
    }

    #[test]
    fn tar_parser_rejects_traversal_pax_abuse_and_missing_required_entries() {
        let dir = temp_dir("tar-errors");
        let gzip = find_tool("gzip");
        let pkg_json = expected_package_json("1.2.3");

        let traversal = package_tar_bytes(&[
            ("../escape", b'0', 0o644, b"x"),
            ("codex-package.json", b'0', 0o644, &pkg_json),
        ]);
        let trav_gz = dir.join("trav.tar.gz");
        write_tar_gz(&traversal, &trav_gz, &gzip);
        assert!(inspect_package_archive_version(&trav_gz, &gzip).is_err());

        let missing_bin = package_tar_bytes(&[("codex-package.json", b'0', 0o644, &pkg_json)]);
        let miss_gz = dir.join("missing-bin.tar.gz");
        write_tar_gz(&missing_bin, &miss_gz, &gzip);
        assert!(inspect_package_archive_version(&miss_gz, &gzip).is_err());

        // PAX extended header followed by regular entry works.
        let pax_payload = b"20 mtime=1700000000\n";
        let mut with_pax = Vec::new();
        append_tar_entry(&mut with_pax, "././@PaxHeader", b'x', 0o644, pax_payload);
        append_tar_entry(&mut with_pax, "bin/codex", b'0', 0o755, b"placeholder");
        append_tar_entry(&mut with_pax, "codex-package.json", b'0', 0o644, &pkg_json);
        with_pax.resize(with_pax.len() + 1024, 0);
        let pax_gz = dir.join("pax.tar.gz");
        write_tar_gz(&with_pax, &pax_gz, &gzip);
        assert_eq!(
            inspect_package_archive_version(&pax_gz, &gzip).unwrap(),
            "1.2.3"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
