use crate::generation_state;
use crate::generation_state::ActivationOperation;
use crate::maintenance;
use crate::remote_download;
use crate::{
    capture_termux_process_env, copy_local_directory_tree, ensure_openssl_available,
    ensure_real_directory_tree, load_local_generation, openssl_sha256, prepare_wrapper_config,
    probe_release_candidate, rename_noreplace, sync_directory, valid_sha256_hex,
    verify_generation_asset_digests, LoadedLocalGeneration, LocalProductError, LocalWrapperRoots,
    ReleaseFileEntry, TermuxProcessEnvSnapshot, DEFAULT_UPSTREAM_LATEST_URL,
    REMOTE_RELEASE_URL_MAX_BYTES, REMOTE_TRANSFER_TIMEOUT_SECONDS, UPSTREAM_PACKAGE_ASSET,
    UPSTREAM_RELEASE_METADATA_BASE, UPSTREAM_RELEASE_METADATA_MAX_BYTES,
};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

const UPDATE_VERSION_ENV: &str = "CODEX_TERMUX_UPDATE_VERSION";
const WRAPPER_CREATION_METADATA: &str = "codex-wrapper";
const UPDATE_USAGE: &str = "usage: codex update [--upstream] [--version <MAJOR.MINOR.PATCH>] | --check [--version <MAJOR.MINOR.PATCH>] [--json] | --path <FILE_OR_DIR> | --rollback | --help";
const UPDATE_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub(super) fn print_update_usage() {
    println!("{UPDATE_USAGE}");
    println!("  default / --upstream      download official Codex package, verify SHA-256, adapt locally for Termux, and activate");
    println!("  --version <M.m.p>         target a specific official release version (with default/--upstream)");
    println!("  --path <FILE_OR_DIR>      adapt an offline official package .tar.gz or stage a prepared generation directory");
    println!("  --rollback                atomically switch back to the previous generation");
    println!("  --check [--json]          inspect exact official release inputs without installing or changing state");
    println!("  --help                    show this help");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateAction {
    Help,
    Upstream { version: Option<String> },
    Check { version: Option<String>, json: bool },
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
    codex_release_builder::valid_stable_version(value)
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
    let mut check = false;
    let mut json = false;
    let mut version: Option<String> = None;
    let mut local_path: Option<PathBuf> = None;
    let mut idx = 0;
    while idx < args.len() {
        let Some(flag) = args[idx].to_str() else {
            return Err(UpdateCliError::Usage(
                "codex update: unsupported non-UTF-8 argument",
            ));
        };
        if flag == "--check" && !check {
            check = true;
            idx += 1;
        } else if flag == "--json" && !json {
            json = true;
            idx += 1;
        } else if flag == "--upstream" {
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
        if upstream_flag || version.is_some() || check || json {
            return Err(UpdateCliError::Usage(
                "codex update: --path cannot be combined with --upstream or --version",
            ));
        }
        return Ok(UpdateAction::Path(path));
    }

    if json && !check {
        return Err(UpdateCliError::Usage(
            "codex update: --json requires --check",
        ));
    }
    if check {
        return Ok(UpdateAction::Check { version, json });
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
    let mut release_flags_seen = 0u8;
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
            "draft" | "prerelease" => {
                let bit = if key == "draft" { 1 } else { 2 };
                if release_flags_seen & bit != 0 {
                    return cursor.error();
                }
                release_flags_seen |= bit;
                cursor.literal(b"false")?;
            }
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
    roots: &LocalWrapperRoots,
    staging_root: &Path,
    cli_version: Option<&str>,
) -> Result<(OfficialReleaseMetadata, String), LocalProductError> {
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
    let fetch = |url: &str, name: &str, expected: Option<&str>| {
        fetch_official_metadata(roots, staging_root, url, name, expected)
    };
    if let Some(version) = requested_version {
        let url = format!("{UPSTREAM_RELEASE_METADATA_BASE}/{version}/release.json");
        return Ok((fetch(&url, "upstream-release.json", Some(version))?, url));
    }
    let latest = fetch(DEFAULT_UPSTREAM_LATEST_URL, "upstream-latest.json", None)?;
    let url = format!(
        "{UPSTREAM_RELEASE_METADATA_BASE}/{}/release.json",
        latest.version
    );
    let pinned = fetch(&url, "upstream-release.json", Some(&latest.version))?;
    if pinned != latest {
        return Err(LocalProductError::LocalUpdate(
            "official latest and exact-version descriptors disagree",
        ));
    }
    Ok((pinned, url))
}

fn fetch_official_metadata(
    roots: &LocalWrapperRoots,
    staging_root: &Path,
    url: &str,
    name: &str,
    requested_version: Option<&str>,
) -> Result<OfficialReleaseMetadata, LocalProductError> {
    validate_https_url(url)?;
    let path = staging_root.join(name);
    let output = crate::create_remote_output(&path)?;
    remote_download::fetch(
        roots,
        url,
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

fn version_order(a: &str, b: &str) -> Result<std::cmp::Ordering, LocalProductError> {
    let tuple = |s: &str| {
        if !valid_update_version(s) {
            return Err(LocalProductError::LocalUpdate(
                "release comparison requires stable versions",
            ));
        }
        s.split('.')
            .map(|n| {
                n.parse::<u64>()
                    .map_err(|_| LocalProductError::LocalUpdate("release version overflow"))
            })
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(tuple(a)?.cmp(&tuple(b)?))
}

fn reject_changed_archive(
    version: &str,
    digest: &str,
    discovered: &OfficialReleaseMetadata,
) -> Result<(), LocalProductError> {
    if version == discovered.version && digest != discovered.package_sha256 {
        return Err(LocalProductError::LocalUpdate(
            "official archive digest changed for an already installed version",
        ));
    }
    Ok(())
}

fn check_release(
    roots: &LocalWrapperRoots,
    version: Option<&str>,
    json: bool,
) -> Result<String, LocalProductError> {
    ensure_openssl_available(&roots.openssl)?;
    let staging = create_staging_root(&std::env::temp_dir(), ".codex-check-")?;
    let result = (|| {
        let (metadata, url) = resolve_official_release_metadata(roots, &staging, version)?;
        let descriptor_sha =
            openssl_sha256(&roots.openssl, &staging.join("upstream-release.json"))?;
        let wrapper_sha = openssl_sha256(&roots.openssl, &running_wrapper_artifact()?)?;
        // Inspection never recovers a journal or takes an update/pruning lease.
        let paths = generation_state::WrapperStatePaths::new(&roots.state_root)
            .map_err(LocalProductError::StateFormat)?;
        let state =
            generation_state::read_pointer_state(&paths).map_err(LocalProductError::State)?;
        let installed = state
            .map(|state| load_local_generation(&roots.generation_root.join(state.current)))
            .transpose()?;
        if let Some(old) = installed.as_ref() {
            reject_changed_archive(
                &old.manifest.upstream_package_version,
                &old.manifest.source_artifact_digest,
                &metadata,
            )?;
        }
        let order = installed
            .as_ref()
            .map(|old| version_order(&metadata.version, &old.manifest.upstream_package_version))
            .transpose()?;
        let status = match installed.as_ref() {
            Some(_) if order.is_some_and(|order| order.is_lt()) => "older",
            Some(old)
                if old.manifest.upstream_package_version == metadata.version
                    && old.manifest.source_artifact_digest == metadata.package_sha256
                    && old.manifest.wrapper_artifact_digest == wrapper_sha =>
            {
                "current"
            }
            _ => "available",
        };
        let archive_url = format!(
            "{UPSTREAM_RELEASE_METADATA_BASE}/{}/{UPSTREAM_PACKAGE_ASSET}",
            metadata.version
        );
        if json {
            let installed_version = installed.as_ref().map_or("null".to_owned(), |old| {
                format!("\"{}\"", old.manifest.upstream_package_version)
            });
            Ok(format!("{{\"schema_version\":1,\"status\":\"{status}\",\"upstream_version\":\"{}\",\"archive_url\":\"{archive_url}\",\"archive_sha256\":\"{}\",\"metadata_url\":\"{url}\",\"metadata_sha256\":\"{descriptor_sha}\",\"wrapper_sha256\":\"{wrapper_sha}\",\"installed_version\":{installed_version}}}\n", metadata.version, metadata.package_sha256))
        } else {
            Ok(format!(
                "Codex {}: {status}\narchive: {archive_url}\nsha256: {}\n",
                metadata.version, metadata.package_sha256
            ))
        }
    })();
    std::fs::remove_dir_all(staging).map_err(|source| LocalProductError::Io {
        operation: "remove release inspection temporary directory",
        source,
    })?;
    result
}

fn update_progress_enabled(stdout_is_terminal: bool, stderr_is_terminal: bool) -> bool {
    stdout_is_terminal && stderr_is_terminal
}

enum UpdateTransientCommand {
    Update { message: String },
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
                        Ok(UpdateTransientCommand::Update { message }) => {
                            current_message = message;
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

    fn fail(&mut self, error: &LocalProductError) -> i32 {
        self.clear_transient();
        eprintln!("codex update: {error}");
        if matches!(
            error,
            LocalProductError::State(generation_state::ActivationTransactionError::CommitSync(_))
        ) {
            3
        } else {
            1
        }
    }
}

impl Drop for UpdatePresentation {
    fn drop(&mut self) {
        self.clear_transient();
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn local_update_generation_id(
    roots: &LocalWrapperRoots,
    version: &str,
    archive_sha256: &str,
) -> String {
    let short: String = archive_sha256.chars().take(6).collect();
    let short = if short.len() == 6 {
        short
    } else {
        "000000".to_owned()
    };
    let base = format!("{version}-{short}");
    if std::fs::symlink_metadata(roots.generation_root.join(&base)).is_err() {
        return base;
    }
    for seq in 2u64.. {
        let candidate = format!("{base}-{seq}");
        if std::fs::symlink_metadata(roots.generation_root.join(&candidate)).is_err() {
            return candidate;
        }
    }
    base
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

fn ensure_writable_roots(roots: &LocalWrapperRoots) -> Result<(), LocalProductError> {
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

fn activation_paths(
    roots: &LocalWrapperRoots,
) -> Result<generation_state::WrapperStatePaths, LocalProductError> {
    generation_state::WrapperStatePaths::new(&roots.state_root)
        .map(|paths| paths.with_generation_root(&roots.generation_root))
        .map_err(LocalProductError::StateFormat)
}

fn activate_generation(
    paths: &generation_state::WrapperStatePaths,
    operation: ActivationOperation,
    before: Option<&generation_state::GenerationPointerState>,
    after: &generation_state::GenerationPointerState,
    lock: &generation_state::ActivationLockGuard,
) -> Result<(), LocalProductError> {
    use generation_state::ActivationTransactionError::{CommitSync, CommittedCleanup, Io};

    let mut io = generation_state::FsActivationIo;
    match generation_state::activate_pointer_state_locked(
        paths, operation, before, after, &mut io, lock,
    ) {
        Ok(()) => Ok(()),
        Err(err @ CommittedCleanup { .. }) => {
            eprintln!("codex update: warning: {err}");
            Ok(())
        }
        Err(err @ Io { .. }) => {
            // A link operation may have changed the filesystem before failing.
            // Repair it from the journal while still holding the writer lock.
            match generation_state::recover_activation_locked(paths, &mut io, lock) {
                Ok((state, _)) if state.as_ref() == Some(after) => {
                    eprintln!("codex update: warning: activation recovered after: {err}");
                    Ok(())
                }
                Ok(_) => Err(LocalProductError::State(err)),
                Err(recovery @ CommitSync(_)) => Err(LocalProductError::State(recovery)),
                Err(recovery) => {
                    eprintln!("codex update: activation recovery is pending: {recovery}");
                    Err(LocalProductError::State(err))
                }
            }
        }
        Err(err) => Err(LocalProductError::State(err)),
    }
}

fn current_loaded_generation(
    roots: &LocalWrapperRoots,
) -> Result<
    (
        Option<generation_state::GenerationPointerState>,
        Option<LoadedLocalGeneration>,
    ),
    LocalProductError,
> {
    let state_paths = activation_paths(roots)?;
    let lock = generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let (before, _) = generation_state::recover_activation_locked(
        &state_paths,
        &mut generation_state::FsActivationIo,
        &lock,
    )
    .map_err(LocalProductError::State)?;
    state_paths
        .read_standalone_current()
        .map_err(LocalProductError::State)?;
    drop(lock);
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
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    before: Option<&generation_state::GenerationPointerState>,
    before_loaded: Option<&LoadedLocalGeneration>,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    let loaded = load_local_generation(built_dir)?;
    let generation_id = loaded.generation_id.clone();
    let version = loaded.manifest.upstream_package_version.clone();

    ensure_writable_roots(roots)?;
    let final_dir = roots.generation_root.join(&generation_id);
    let reuse_existing = match std::fs::symlink_metadata(&final_dir) {
        Ok(meta) if meta.is_dir() => true,
        Ok(_) => return Err(LocalProductError::GenerationCollision),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect destination generation path",
                source,
            })
        }
    };

    let candidate_parent = roots.generation_root.clone();
    let candidate_seq = crate::LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let candidate_dir =
        candidate_parent.join(format!(".candidate-{}-{candidate_seq}", std::process::id()));

    let result = (|| -> Result<ActivatedOutcome, LocalProductError> {
        let staged_loaded = if reuse_existing {
            // An interrupted update can leave its complete generation in the
            // catalog. Reuse an exact, verified admission without rewriting it.
            let existing = load_local_generation(&final_dir)?;
            if existing.generation_id != generation_id
                || existing.generation_layout != loaded.generation_layout
                || existing.manifest != loaded.manifest
            {
                return Err(LocalProductError::GenerationCollision);
            }
            verify_generation_asset_digests(&loaded, &roots.openssl)?;
            existing
        } else {
            copy_local_directory_tree(built_dir, &candidate_dir)?;
            sync_tree_recursive(&candidate_dir)?;
            load_local_generation(&candidate_dir)?
        };

        presentation.transient(&format!(
            "Verifying Codex {version} assets & candidate runtime..."
        ));
        verify_generation_asset_digests(&staged_loaded, &roots.openssl)?;
        prepare_wrapper_config(&roots.config_dir).map_err(|source| LocalProductError::Io {
            operation: "prepare wrapper config.toml",
            source,
        })?;
        probe_release_candidate(&staged_loaded, roots, process_env)?;

        presentation.transient(&format!("Activating Codex {version}..."));
        let state_paths = activation_paths(roots)?;
        generation_state::prepare_wrapper_state_paths(&state_paths)
            .map_err(LocalProductError::State)?;
        let lock = generation_state::acquire_activation_lock(&state_paths)
            .map_err(LocalProductError::State)?;
        let authoritative =
            generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
        if authoritative.as_ref() != before {
            return Err(LocalProductError::State(
                generation_state::ActivationTransactionError::StaleAuthoritativeState,
            ));
        }
        if generation_state::has_pending_activation(&state_paths)
            .map_err(LocalProductError::State)?
        {
            return Err(LocalProductError::State(
                generation_state::ActivationTransactionError::PendingJournal,
            ));
        }
        state_paths
            .read_standalone_current()
            .map_err(LocalProductError::State)?;
        if !reuse_existing {
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
        }
        sync_directory(&roots.generation_root)?;

        if authoritative
            .as_ref()
            .is_some_and(|state| state.current == generation_id)
        {
            let mut io = generation_state::FsActivationIo;
            generation_state::sync_standalone_current(
                &state_paths,
                Some(&generation_id),
                None,
                &mut io,
                &lock,
            )
            .map_err(LocalProductError::State)?;
            return Ok(ActivatedOutcome {
                generation_id,
                previous_version: None,
                current_version: version,
            });
        }
        let after = match authoritative.as_ref() {
            Some(prev) => generation_state::plan_activation_pointer_state(prev, &generation_id),
            None => generation_state::plan_initial_pointer_state(&generation_id),
        }
        .map_err(LocalProductError::StateFormat)?;
        activate_generation(
            &state_paths,
            ActivationOperation::Update,
            authoritative.as_ref(),
            &after,
            &lock,
        )?;

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

fn gzip_tool_path(roots: &LocalWrapperRoots) -> Result<PathBuf, LocalProductError> {
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
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    before: Option<&generation_state::GenerationPointerState>,
    before_loaded: Option<&LoadedLocalGeneration>,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    if !codex_release_builder::binary_adaptation_available(inputs.version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream Codex version format is invalid",
        ));
    }
    let wrapper = running_wrapper_artifact()?;
    let generation_id = local_update_generation_id(roots, inputs.version, inputs.archive_sha256);
    let built_dir = inputs.staging_root.join("built-generation");
    let gzip = gzip_tool_path(roots)?;
    presentation.transient(&format!(
        "Building Termux generation for Codex {}...",
        inputs.version
    ));
    let version_label = inputs.version;
    let mut patch_progress = |stage: &str| {
        presentation.transient(&format!("Codex {version_label}: {stage}"));
    };
    codex_release_builder::build_generation_with_progress(
        &codex_release_builder::GenerationRequest {
            version: inputs.version.to_owned(),
            archive: inputs.archive_path.to_path_buf(),
            archive_sha256: inputs.archive_sha256.to_owned(),
            generation_id,
            wrapper,
            creation_metadata: WRAPPER_CREATION_METADATA.to_owned(),
            gzip,
            openssl: roots.openssl.clone(),
            output: built_dir.clone(),
        },
        &mut patch_progress,
    )
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
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> Result<UpstreamOutcome, LocalProductError> {
    ensure_openssl_available(&roots.openssl)?;
    ensure_writable_roots(roots)?;
    let (before, before_loaded) = current_loaded_generation(roots)?;
    let staging_root = create_staging_root(&roots.state_root, ".local-update-")?;
    let result = (|| -> Result<UpstreamOutcome, LocalProductError> {
        presentation.transient("Checking official Codex release...");
        let (metadata, _) =
            resolve_official_release_metadata(roots, &staging_root, requested_version)?;

        if let Some(current) = before_loaded.as_ref() {
            reject_changed_archive(
                &current.manifest.upstream_package_version,
                &current.manifest.source_artifact_digest,
                &metadata,
            )?;
            if requested_version.is_none()
                && std::env::var_os(UPDATE_VERSION_ENV).is_none()
                && version_order(
                    &metadata.version,
                    &current.manifest.upstream_package_version,
                )?
                .is_lt()
            {
                return Err(LocalProductError::LocalUpdate("official discovery would downgrade the installed version; use an explicit --version if intended"));
            }
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
        let mut last_bytes = u64::MAX;
        let mut last_total: Option<u64> = None;
        let version_label = metadata.version.as_str();
        let mut progress = |bytes: u64, total: Option<u64>| {
            if bytes != last_bytes || total != last_total {
                last_bytes = bytes;
                last_total = total;
                let msg = match total {
                    Some(t) if t > 0 => {
                        let pct = (bytes.saturating_mul(100) / t).min(100);
                        format!(
                            "Downloading Codex {version_label} ({} / {}, {pct}%)...",
                            format_bytes(bytes),
                            format_bytes(t),
                        )
                    }
                    _ => format!(
                        "Downloading Codex {version_label} ({})...",
                        format_bytes(bytes)
                    ),
                };
                presentation.transient(&msg);
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
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
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
    roots: &LocalWrapperRoots,
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
            "local package archive version format is invalid",
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

fn run_rollback(
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    let state_paths = activation_paths(roots)?;
    let lock = generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let (before, committed) = generation_state::recover_activation_locked(
        &state_paths,
        &mut generation_state::FsActivationIo,
        &lock,
    )
    .map_err(LocalProductError::State)?;
    let before = before.ok_or(LocalProductError::NoCurrentGeneration)?;
    state_paths
        .read_standalone_current()
        .map_err(LocalProductError::State)?;
    drop(lock);
    if let Some(journal) = committed {
        if journal.operation == ActivationOperation::Legacy && journal.is_rollback_transition() {
            return Err(LocalProductError::LocalUpdate(
                "recovered an older activation journal with ambiguous operation; inspect codex wrapper status before retrying",
            ));
        }
        if journal.operation == ActivationOperation::Rollback {
            // The previous rollback published its state but stopped before removing
            // the journal. Recovery has finished that operation, so do not swap again.
            let version = |id: &str| {
                load_local_generation(&roots.generation_root.join(id))
                    .map(|loaded| loaded.manifest.upstream_package_version)
                    .unwrap_or_else(|_| id.to_owned())
            };
            return Ok(ActivatedOutcome {
                current_version: version(&before.current),
                previous_version: before.previous.as_deref().map(version),
                generation_id: before.current,
            });
        }
    }
    let after = generation_state::plan_rollback_pointer_state(&before)
        .map_err(LocalProductError::StateFormat)?;

    // Do not let a damaged current generation prevent rolling back to a healthy previous one.
    let previous_version = load_local_generation(&roots.generation_root.join(&before.current))
        .ok()
        .map(|current| current.manifest.upstream_package_version)
        .unwrap_or_else(|| before.current.clone());

    let target_loaded = load_local_generation(&roots.generation_root.join(&after.current))?;
    if target_loaded.generation_id != after.current {
        return Err(LocalProductError::Descriptor(
            "rollback generation descriptor id does not match previous",
        ));
    }
    presentation.transient(&format!(
        "Verifying rollback target {}...",
        target_loaded.generation_id
    ));
    verify_generation_asset_digests(&target_loaded, &roots.openssl)?;
    prepare_wrapper_config(&roots.config_dir).map_err(|source| LocalProductError::Io {
        operation: "prepare wrapper config.toml",
        source,
    })?;
    probe_release_candidate(&target_loaded, roots, process_env)?;

    let lock = generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let authoritative =
        generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
    if authoritative.as_ref() != Some(&before) {
        return Err(LocalProductError::State(
            generation_state::ActivationTransactionError::StaleAuthoritativeState,
        ));
    }
    activate_generation(
        &state_paths,
        ActivationOperation::Rollback,
        Some(&before),
        &after,
        &lock,
    )?;
    Ok(ActivatedOutcome {
        generation_id: after.current,
        previous_version: Some(previous_version),
        current_version: target_loaded.manifest.upstream_package_version,
    })
}

pub(super) fn run_update_action(action: UpdateAction) -> i32 {
    if matches!(action, UpdateAction::Help) {
        print_update_usage();
        return 0;
    }

    let mut presentation = UpdatePresentation::new();
    let roots = match LocalWrapperRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            return presentation.fail(&err);
        }
    };
    if let UpdateAction::Check { version, json } = action {
        return match check_release(&roots, version.as_deref(), json) {
            Ok(output) => {
                print!("{output}");
                0
            }
            Err(err) => presentation.fail(&err),
        };
    }
    let _lease = match maintenance::UpdateLease::acquire(&roots) {
        Ok(lease) => lease,
        Err(err) => {
            return presentation.fail(&err);
        }
    };
    let process_env = capture_termux_process_env();

    match action {
        UpdateAction::Help => 0,
        UpdateAction::Check { .. } => {
            unreachable!("read-only inspection handled before update lease")
        }
        UpdateAction::Rollback => match run_rollback(&roots, &process_env, &mut presentation) {
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
            Err(err) => presentation.fail(&err),
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
                Err(err) => presentation.fail(&err),
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
                Err(err) => presentation.fail(&err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{remove_temp_root, temp_root};
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    #[test]
    fn inspection_cli_is_read_only_and_conflicting_flags_are_rejected() {
        let args = |items: &[&str]| items.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            parse_update_args(&args(&["--check", "--json", "--version", "1.2.3"])).unwrap(),
            UpdateAction::Check {
                version: Some("1.2.3".into()),
                json: true
            }
        );
        for words in [
            vec!["--json"],
            vec!["--check", "--check"],
            vec!["--check", "--rollback"],
            vec!["--check", "--path", "/tmp/archive"],
            vec!["--check", "--json", "--json"],
        ] {
            assert!(parse_update_args(&args(&words)).is_err(), "{words:?}");
        }
        assert!(version_order("1.10.0", "1.9.99").unwrap().is_gt());
        assert!(version_order("1.2", "1.2.3").is_err());
        assert!(version_order("1.0.0", "invalid").is_err());
        let discovered = OfficialReleaseMetadata {
            version: "1.2.3".into(),
            package_sha256: "a".repeat(64),
        };
        assert!(reject_changed_archive("1.2.3", &"b".repeat(64), &discovered).is_err());
        assert!(reject_changed_archive("1.2.3", &"a".repeat(64), &discovered).is_ok());
        assert!(reject_changed_archive("1.2.2", &"b".repeat(64), &discovered).is_ok());
    }

    fn metadata(digest: char) -> String {
        format!("{{\"tag_name\":\"rust-v1.2.3\",\"assets\":[{{\"name\":\"{UPSTREAM_PACKAGE_ASSET}\",\"digest\":\"sha256:{}\"}}]}}", digest.to_string().repeat(64))
    }

    #[test]
    fn official_metadata_rejects_prerelease_draft_and_duplicate_flags() {
        for extra in [
            "\"draft\":true,",
            "\"prerelease\":true,",
            "\"draft\":false,\"draft\":false,",
        ] {
            let raw = metadata('a').replacen('{', &format!("{{{extra}"), 1);
            assert!(
                parse_official_release_metadata(raw.as_bytes(), None).is_err(),
                "{extra}"
            );
        }
    }

    #[test]
    fn inspection_cross_checks_descriptors_without_installed_state_or_artifact_downloads() {
        let root = temp_root("release-check");
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let cat = if Path::new("/system/bin/cat").is_file() {
            "/system/bin/cat"
        } else {
            "/bin/cat"
        };
        let latest = root.join("latest.json");
        let exact = root.join("exact.json");
        fs::write(&latest, metadata('a')).unwrap();
        fs::write(&exact, metadata('a')).unwrap();
        let curl = root.join("curl");
        fs::write(&curl, format!("#!{shell}\nurl=\nwhile [ $# -gt 0 ]; do\n if [ \"$1\" = --url ]; then shift; url=$1; fi\n shift\ndone\ncase \"$url\" in\n *releases/latest) exec {cat} '{}' ;;\n */release.json) exec {cat} '{}' ;;\n '') exit 0 ;;\n *) exit 57 ;;\nesac\n", latest.display(), exact.display())).unwrap();
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o700)).unwrap();
        let prefix = {
            let _env = crate::tests::ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::env::var_os("PREFIX")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/usr"))
        };
        let state = root.join("state");
        let roots = LocalWrapperRoots {
            generation_root: root.join("generations"),
            state_root: state.clone(),
            config_dir: state.join("config"),
            resolver_path: state.join("resolv.conf"),
            openssl: prefix.join("bin/openssl"),
            curl,
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
        };
        let output = check_release(&roots, None, true).unwrap();
        assert!(output.contains("\"upstream_version\":\"1.2.3\""));
        assert!(output.contains("\"installed_version\":null"));
        assert!(!state.exists());
        assert!(!roots.generation_root.exists());
        fs::write(&exact, metadata('b')).unwrap();
        assert!(check_release(&roots, None, true).is_err());
        assert!(!state.exists());
        assert!(!roots.generation_root.exists());
        remove_temp_root(root);
    }

    fn setup_update_test_roots(root: &Path) -> LocalWrapperRoots {
        let state_root = root.join("state");
        let config_dir = state_root.join("config");
        let generation_root = root.join(".codex/packages/standalone/releases");
        crate::tests::create_private_test_dirs(&config_dir);
        fs::create_dir_all(&generation_root).unwrap();
        let resolver_path = root.join("resolv.conf");
        fs::write(&resolver_path, b"nameserver 1.1.1.1\n").unwrap();
        let cert_file = root.join("cert.pem");
        let cert_dir = root.join("certs");
        fs::write(&cert_file, b"").unwrap();
        fs::create_dir_all(&cert_dir).unwrap();
        let openssl = crate::tests::test_openssl_path();
        let curl = openssl.parent().unwrap().join("curl");
        LocalWrapperRoots {
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
    fn install_built_generation_dir_rejects_tampered_runtime_and_helpers() {
        let root = temp_root("p1-01-digest");
        let roots = setup_update_test_roots(&root);
        let env = crate::tests::test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();

        // 1. Tampered bin/codex
        let built_bad_rt = root.join("built-bad-rt");
        crate::tests::write_sample_generation(&built_bad_rt, "1.2.3-aaaaaa", false);
        fs::write(
            built_bad_rt.join("bin/codex"),
            b"#!/bin/sh\necho tampered\n",
        )
        .unwrap();
        fs::set_permissions(
            built_bad_rt.join("bin/codex"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(matches!(
            install_built_generation_dir(
                &built_bad_rt,
                &roots,
                &env,
                None,
                None,
                &mut presentation
            ),
            Err(LocalProductError::Descriptor(_))
        ));
        assert!(!roots.generation_root.join("1.2.3-aaaaaa").exists());

        // 2. Tampered browser helper
        let built_bad_helper = root.join("built-bad-helper");
        crate::tests::write_sample_generation(&built_bad_helper, "1.2.3-bbbbbb", false);
        fs::write(
            built_bad_helper.join("browser/open/curl"),
            b"#!/bin/sh\necho tampered\n",
        )
        .unwrap();
        fs::set_permissions(
            built_bad_helper.join("browser/open/curl"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(matches!(
            install_built_generation_dir(
                &built_bad_helper,
                &roots,
                &env,
                None,
                None,
                &mut presentation
            ),
            Err(LocalProductError::Descriptor(_))
        ));
        assert!(!roots.generation_root.join("1.2.3-bbbbbb").exists());

        // 3. Missing browser helper
        let built_missing_helper = root.join("built-missing-helper");
        crate::tests::write_sample_generation(&built_missing_helper, "1.2.3-cccccc", false);
        fs::remove_file(built_missing_helper.join("browser/manual/curl")).unwrap();
        assert!(install_built_generation_dir(
            &built_missing_helper,
            &roots,
            &env,
            None,
            None,
            &mut presentation
        )
        .is_err());
        assert!(!roots.generation_root.join("1.2.3-cccccc").exists());

        remove_temp_root(root);
    }

    #[test]
    fn rollback_rejects_corrupt_previous_and_recovers_from_damaged_current() {
        let root = temp_root("p1-02-03-rollback");
        let roots = setup_update_test_roots(&root);
        let env = crate::tests::test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();

        let src1 = root.join("src-g1");
        let src2 = root.join("src-g2");
        crate::tests::write_sample_generation(&src1, "1.2.3-111111", false);
        crate::tests::write_sample_generation(&src2, "1.2.4-222222", false);
        install_built_generation_dir(&src1, &roots, &env, None, None, &mut presentation).unwrap();
        let (ptr1, loaded1) = current_loaded_generation(&roots).unwrap();
        install_built_generation_dir(
            &src2,
            &roots,
            &env,
            ptr1.as_ref(),
            loaded1.as_ref(),
            &mut presentation,
        )
        .unwrap();

        let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        let before_bytes = fs::read(&state_paths.activation_state).unwrap();
        let current_link = roots.generation_root.parent().unwrap().join("current");
        assert_eq!(
            fs::read_link(&current_link).unwrap(),
            PathBuf::from("releases/1.2.4-222222")
        );

        // P1-02: Tampered previous target fails before lock/pointer mutation.
        let prev_rt = roots.generation_root.join("1.2.3-111111/bin/codex");
        let orig_prev_rt = fs::read(&prev_rt).unwrap();
        fs::write(&prev_rt, b"#!/bin/sh\necho corrupt\n").unwrap();
        fs::set_permissions(&prev_rt, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(run_rollback(&roots, &env, &mut presentation).is_err());
        assert_eq!(
            fs::read(&state_paths.activation_state).unwrap(),
            before_bytes
        );
        assert_eq!(
            fs::read_link(&current_link).unwrap(),
            PathBuf::from("releases/1.2.4-222222")
        );

        // P1-03: Restore previous and completely damage active `current` -> rollback still recovers.
        fs::write(&prev_rt, orig_prev_rt).unwrap();
        fs::set_permissions(&prev_rt, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_file(roots.generation_root.join("1.2.4-222222/bin/codex")).unwrap();
        fs::write(
            roots.generation_root.join("1.2.4-222222/generation.meta"),
            b"corrupt\n",
        )
        .unwrap();

        let rolled = run_rollback(&roots, &env, &mut presentation).unwrap();
        assert_eq!(rolled.generation_id, "1.2.3-111111");
        assert_eq!(
            fs::read_link(&current_link).unwrap(),
            PathBuf::from("releases/1.2.3-111111")
        );

        remove_temp_root(root);
    }

    #[test]
    fn rollback_distinguishes_reselecting_previous_from_retrying_a_committed_rollback() {
        for operation in [
            ActivationOperation::Update,
            ActivationOperation::Rollback,
            ActivationOperation::Legacy,
        ] {
            let root = temp_root("rollback-journal-operation");
            let roots = setup_update_test_roots(&root);
            let env = crate::tests::test_process_env(&roots);
            let mut presentation = UpdatePresentation::new();
            let first = root.join("first");
            let second = root.join("second");
            let first_id = "1.2.3-111111";
            let second_id = "1.2.4-222222";
            crate::tests::write_sample_generation(&first, first_id, false);
            crate::tests::write_sample_generation(&second, second_id, false);
            for dir in [&first, &second, &first] {
                let (state, loaded) = current_loaded_generation(&roots).unwrap();
                install_built_generation_dir(
                    dir,
                    &roots,
                    &env,
                    state.as_ref(),
                    loaded.as_ref(),
                    &mut presentation,
                )
                .unwrap();
            }

            let paths = activation_paths(&roots).unwrap();
            let after = generation_state::read_pointer_state(&paths)
                .unwrap()
                .unwrap();
            let before = generation_state::plan_rollback_pointer_state(&after).unwrap();
            let journal = generation_state::ActivationJournal {
                operation,
                before: Some(before.clone()),
                after: after.clone(),
            };
            // Both commands can leave this same committed pair after a cleanup failure.
            fs::write(
                &paths.activation_journal,
                generation_state::encode_activation_journal(&journal).unwrap(),
            )
            .unwrap();

            let result = run_rollback(&roots, &env, &mut presentation);
            let expected = match operation {
                ActivationOperation::Update => {
                    assert_eq!(result.unwrap().generation_id, second_id);
                    before
                }
                ActivationOperation::Rollback => {
                    assert_eq!(result.unwrap().generation_id, first_id);
                    after
                }
                ActivationOperation::Legacy => {
                    assert!(result
                        .unwrap_err()
                        .to_string()
                        .contains("ambiguous operation"));
                    after
                }
            };
            assert_eq!(
                paths.read_standalone_current().unwrap(),
                paths.standalone_current_target(&expected.current)
            );
            assert_eq!(
                generation_state::read_pointer_state(&paths).unwrap(),
                Some(expected)
            );
            assert!(!paths.activation_journal.exists());
            remove_temp_root(root);
        }
    }

    #[test]
    fn local_generation_id_uses_version_prefix_and_collision_suffix() {
        let root = temp_root("gen-id-naming");
        let roots = setup_update_test_roots(&root);
        let digest = "abcdef0123456789".repeat(4);

        assert_eq!(
            local_update_generation_id(&roots, "0.169.1", &digest),
            "0.169.1-abcdef"
        );
        fs::create_dir_all(roots.generation_root.join("0.169.1-abcdef")).unwrap();
        assert_eq!(
            local_update_generation_id(&roots, "0.169.1", &digest),
            "0.169.1-abcdef-2"
        );
        fs::create_dir_all(roots.generation_root.join("0.169.1-abcdef-2")).unwrap();
        assert_eq!(
            local_update_generation_id(&roots, "0.169.1", &digest),
            "0.169.1-abcdef-3"
        );

        remove_temp_root(root);
    }
}
