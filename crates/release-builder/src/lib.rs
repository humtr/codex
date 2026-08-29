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
const PATH_MAX_BYTES: usize = 256;
const LOGICAL_ENTRY_MAX: usize = 32;
const PAX_PAYLOAD_MAX_BYTES: u64 = 512;
const GENERATION_ID_MAX_BYTES: usize = 512;
const TEXT_VALUE_MAX_BYTES: usize = 512;
const TAR_BLOCK_BYTES: usize = 512;
const GENERATION_FORMAT: &str = "codex-local-generation-v1";
const CORE_API_IDENTITY: &str = "core-api-v1";
const PERSISTENT_SCHEMA_IDENTITY: &str = "schema-v1";
const PACKAGE_IDENTITY: &str = "openai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz";
const PATCH_POLICY_ID: &str = "termux-fd-remap-v1";

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

const EXPECTED_DIRECTORIES: [&str; 5] = [
    "bin/",
    "codex-path/",
    "codex-resources/",
    "codex-resources/zsh/",
    "codex-resources/zsh/bin/",
];
const EXPECTED_FILES: [&str; 6] = [
    "bin/codex",
    "bin/codex-code-mode-host",
    "codex-package.json",
    "codex-path/rg",
    "codex-resources/bwrap",
    "codex-resources/zsh/bin/zsh",
];

const USAGE: &str = "usage: codex-release-builder build --version <MAJOR.MINOR.PATCH> --archive <ABSOLUTE_FILE> --archive-sha256 <LOWERCASE_SHA256> --generation-id <ID> --core <ABSOLUTE_FILE> --creation-metadata <VALUE> --gzip <ABSOLUTE_EXECUTABLE> --openssl <ABSOLUTE_EXECUTABLE> --output <ABSENT_ABSOLUTE_DIRECTORY>";

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
    creation_metadata: String,
    gzip: PathBuf,
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
        let value = args.next().ok_or(BuilderError::Usage)?;
        match flag.to_str() {
            Some("--version") => set_once(&mut fields.version, text_value(value)?)?,
            Some("--archive") => set_once(&mut fields.archive, PathBuf::from(value))?,
            Some("--archive-sha256") => set_once(&mut fields.archive_sha256, text_value(value)?)?,
            Some("--generation-id") => set_once(&mut fields.generation_id, text_value(value)?)?,
            Some("--core") => set_once(&mut fields.core, PathBuf::from(value))?,
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
        creation_metadata: fields.creation_metadata.ok_or(BuilderError::Usage)?,
        gzip: fields.gzip.ok_or(BuilderError::Usage)?,
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

fn validate_request(request: &BuildRequest) -> Result<(), BuilderError> {
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
    ensure_regular_file(
        &request.core,
        "inspect Core artifact",
        "Core artifact is not a regular file",
    )?;
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

fn expected_kind(path: &str) -> Option<EntryKind> {
    if EXPECTED_DIRECTORIES.contains(&path) {
        Some(EntryKind::Directory)
    } else if EXPECTED_FILES.contains(&path) {
        Some(EntryKind::Regular)
    } else {
        None
    }
}

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

fn parse_archive<R: Read>(
    reader: &mut R,
    staging: &Path,
    version: &str,
) -> Result<ArchiveSelection, BuilderError> {
    let compat = staging.join("compat");
    create_private_dir(&compat)?;
    let raw_runtime = staging.join(".raw-runtime");
    let code_mode_host = compat.join("codex-code-mode-host");
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
        if expected_kind(&header.path) != Some(header.kind) {
            return Err(BuilderError::Archive("archive layout is unsupported"));
        }
        if !seen.insert(header.path.clone()) {
            return Err(BuilderError::Archive("archive path is duplicated"));
        }
        pending_pax = false;

        if header.kind == EntryKind::Directory {
            continue;
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

    let expected: BTreeSet<String> = EXPECTED_DIRECTORIES
        .iter()
        .chain(EXPECTED_FILES.iter())
        .map(|path| (*path).to_owned())
        .collect();
    if seen != expected {
        return Err(BuilderError::Archive("archive layout is incomplete"));
    }
    if package_json.as_deref() != Some(expected_package_json(version).as_slice()) {
        return Err(BuilderError::Archive("codex-package.json is incompatible"));
    }
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
    core_sha256: String,
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
    set_mode(&selected.code_mode_host, 0o755, "set code-mode-host mode")?;
    File::open(&selected.code_mode_host)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error("sync code-mode host", source))?;

    let runtime_sha256 = openssl_sha256(&request.openssl, &runtime_path)?;
    let code_mode_host_sha256 = openssl_sha256(&request.openssl, &selected.code_mode_host)?;
    let core_sha256 = openssl_sha256(&request.openssl, &request.core)?;
    std::fs::remove_file(&selected.raw_runtime)
        .map_err(|source| io_error("remove selected raw runtime", source))?;

    Ok(AdaptedGeneration {
        raw_runtime_sha256,
        runtime_sha256,
        code_mode_host_sha256,
        core_sha256,
        changed_bytes,
    })
}

fn write_generation_descriptor(
    request: &BuildRequest,
    staging: &Path,
    adapted: &AdaptedGeneration,
) -> Result<(), BuilderError> {
    let patch_report = format!(
        "{PATCH_POLICY_ID};archive_sha256={};raw_runtime_sha256={};runtime_sha256={};code_mode_host_sha256={};source_counts=2,1,1,1;changed_bytes={}",
        request.archive_sha256,
        adapted.raw_runtime_sha256,
        adapted.runtime_sha256,
        adapted.code_mode_host_sha256,
        adapted.changed_bytes
    );
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
            "manager_artifact_digest\t-\n",
            "core_api_identity\t{}\n",
            "persistent_schema_identity\t{}\n",
            "qualification\tqualified\n",
            "creation_metadata\t{}\n",
            "upstream_doctor\tsupported\n",
            "helper_count\t0\n"
        ),
        GENERATION_FORMAT,
        request.generation_id,
        PACKAGE_IDENTITY,
        request.version,
        request.archive_sha256,
        PATCH_POLICY_ID,
        patch_report,
        adapted.runtime_sha256,
        adapted.core_sha256,
        CORE_API_IDENTITY,
        PERSISTENT_SCHEMA_IDENTITY,
        request.creation_metadata
    );
    let path = staging.join("generation.meta");
    let mut file = create_private_file(&path)?;
    file.write_all(descriptor.as_bytes())
        .map_err(|source| io_error("write generation descriptor", source))?;
    file.sync_all()
        .map_err(|source| io_error("sync generation descriptor", source))?;
    drop(file);
    set_mode(&path, 0o644, "set generation descriptor mode")?;
    Ok(())
}

fn rename_noreplace(source: &Path, destination: &Path) -> Result<(), BuilderError> {
    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
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
) -> Result<(), BuilderError> {
    let adapted = adapt_selected_runtime(request, staging, selected)?;
    write_generation_descriptor(request, staging, &adapted)?;
    set_mode(
        &staging.join("compat"),
        0o755,
        "set compatibility directory mode",
    )?;
    sync_directory(&staging.join("compat"), "sync compatibility directory")?;
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
        let archive = snapshot_archive(request, &staging)?;
        let selected = select_archive(request, &archive, &staging)?;
        std::fs::remove_file(&archive)
            .map_err(|source| io_error("remove upstream archive snapshot", source))?;
        complete_and_publish(request, &staging, selected)
    })();
    match (result, cleanup_staging(&staging)) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

pub fn run_from_args<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
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
                std::fs::symlink_metadata(path).is_ok_and(|metadata| {
                    metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0
                })
            })
            .unwrap_or_else(|| panic!("required test tool is unavailable: {name}"))
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

    fn fixture(label: &str, entries: Vec<TestEntry>, corrupt_header: bool) -> Fixture {
        let root = test_root(label);
        let gzip = find_tool("gzip");
        let openssl = find_tool("openssl");
        let archive = root.join("codex-package-aarch64-unknown-linux-musl.tar.gz");
        gzip_tar(&gzip, &make_tar(&entries, corrupt_header), &archive);
        let core = root.join("codex-core");
        std::fs::write(&core, b"test Core artifact").unwrap();
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
        vec![
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
        ]
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
                    case_fixture.request.gzip = case_fixture.request.core.clone()
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
            vec![OsString::from(".raw-runtime"), OsString::from("compat")]
        );
        std::fs::remove_dir_all(&selected_root).unwrap();

        assert!(fixture.request.output.is_dir());
        assert!(no_builder_staging(&fixture.root));
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
            "extra-file",
            "metadata",
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
                "extra-file" => entries.push(TestEntry::file("extra", b"extra".to_vec())),
                "metadata" => entries[3].data = b"{}\n".to_vec(),
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
        let host_path = fixture.request.output.join("compat/codex-code-mode-host");
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
        let expected_descriptor = format!(
            concat!(
                "codex-local-generation-v1\n",
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
                "helper_count\t0\n"
            ),
            fixture.request.archive_sha256,
            fixture.request.archive_sha256,
            raw_runtime_sha256,
            runtime_sha256,
            host_sha256,
            runtime_sha256,
            core_sha256
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
                OsString::from("compat"),
                OsString::from("generation.meta"),
                OsString::from("runtime")
            ]
        );
        let compat_entries: Vec<_> = std::fs::read_dir(fixture.request.output.join("compat"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(compat_entries, vec![OsString::from("codex-code-mode-host")]);

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
}
