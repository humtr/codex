use crate::error::LocalProductError;
use crate::fs_util::read_bounded_regular_file;
use crate::generation::{valid_sha256_hex, LocalWrapperRoots};
use crate::json::JsonParser;
use crate::remote_download::{self, create_remote_output, REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS};
use std::path::Path;

pub(super) const DEFAULT_UPSTREAM_LATEST_URL: &str =
    "https://api.github.com/repos/openai/codex/releases/latest";
pub(super) const UPSTREAM_RELEASE_METADATA_BASE: &str =
    "https://releases.openai.com/codex/releases";
pub(super) const UPSTREAM_PACKAGE_ASSET: &str = "codex-package-aarch64-unknown-linux-musl.tar.gz";
pub(super) const UPSTREAM_RELEASE_METADATA_MAX_BYTES: usize = 1024 * 1024;
pub(super) const REMOTE_RELEASE_URL_MAX_BYTES: usize = 4096;
pub(super) const UPDATE_VERSION_ENV: &str = "CODEX_TERMUX_UPDATE_VERSION";
pub(super) fn valid_update_version(value: &str) -> bool {
    codex_release_builder::valid_stable_version(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OfficialReleaseMetadata {
    pub(super) version: String,
    pub(super) package_sha256: String,
}

fn malformed() -> LocalProductError {
    LocalProductError::LocalUpdate("upstream release metadata is malformed")
}

fn parse_assets(
    cursor: &mut JsonParser<'_>,
    wanted_asset: &str,
) -> Result<Option<String>, LocalProductError> {
    cursor.expect(b'[').map_err(|_| malformed())?;
    if cursor.consume(b']') {
        return Ok(None);
    }
    let mut wanted_digest = None;
    loop {
        cursor.expect(b'{').map_err(|_| malformed())?;
        let mut name = None;
        let mut digest = None;
        let mut digest_seen = false;
        if !cursor.consume(b'}') {
            loop {
                let key = cursor.string().map_err(|_| malformed())?;
                cursor.expect(b':').map_err(|_| malformed())?;
                match key.as_str() {
                    "name" => {
                        if name.is_some() {
                            return Err(malformed());
                        }
                        name = Some(cursor.string().map_err(|_| malformed())?);
                    }
                    "digest" => {
                        if digest_seen {
                            return Err(malformed());
                        }
                        digest_seen = true;
                        if cursor.peek() == Some(b'n') {
                            cursor.literal(b"null").map_err(|_| malformed())?;
                        } else {
                            digest = Some(cursor.string().map_err(|_| malformed())?);
                        }
                    }
                    _ => cursor.skip_value().map_err(|_| malformed())?,
                }
                if cursor.consume(b'}') {
                    break;
                }
                cursor.expect(b',').map_err(|_| malformed())?;
            }
        }
        if name.as_deref() == Some(wanted_asset) {
            if wanted_digest.is_some() {
                return Err(malformed());
            }
            let digest = digest.ok_or(LocalProductError::LocalUpdate(
                "upstream release metadata is missing the package digest",
            ))?;
            wanted_digest = Some(digest);
        }
        if cursor.consume(b']') {
            return Ok(wanted_digest);
        }
        cursor.expect(b',').map_err(|_| malformed())?;
    }
}

pub(super) fn parse_official_release_metadata(
    bytes: &[u8],
    requested_version: Option<&str>,
) -> Result<OfficialReleaseMetadata, LocalProductError> {
    let mut cursor = JsonParser::new(bytes);
    cursor.expect(b'{').map_err(|_| malformed())?;
    let mut tag_name = None;
    let mut package_digest = None;
    let mut assets_seen = false;
    let mut release_flags_seen = 0u8;
    if cursor.peek() == Some(b'}') {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata is incomplete",
        ));
    }
    loop {
        let key = cursor.string().map_err(|_| malformed())?;
        cursor.expect(b':').map_err(|_| malformed())?;
        match key.as_str() {
            "draft" | "prerelease" => {
                let bit = if key == "draft" { 1 } else { 2 };
                if release_flags_seen & bit != 0 {
                    return Err(malformed());
                }
                release_flags_seen |= bit;
                cursor.literal(b"false").map_err(|_| malformed())?;
            }
            "tag_name" => {
                if tag_name.is_some() {
                    return Err(malformed());
                }
                tag_name = Some(cursor.string().map_err(|_| malformed())?);
            }
            "assets" => {
                if assets_seen {
                    return Err(malformed());
                }
                assets_seen = true;
                package_digest = parse_assets(&mut cursor, UPSTREAM_PACKAGE_ASSET)?;
            }
            _ => cursor.skip_value().map_err(|_| malformed())?,
        }
        if cursor.consume(b'}') {
            break;
        }
        cursor.expect(b',').map_err(|_| malformed())?;
    }
    if cursor.peek().is_some() {
        return Err(malformed());
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

pub(super) fn validate_https_url(value: &str) -> Result<(), LocalProductError> {
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

pub(super) fn resolve_official_release_metadata(
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
    let output = create_remote_output(&path)?;
    remote_download::fetch(
        roots,
        url,
        &output,
        UPSTREAM_RELEASE_METADATA_MAX_BYTES as u64,
        remote_download::TransferTimeout::Fixed(REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS),
        0,
        None,
    )?;
    let bytes = read_bounded_regular_file(
        &path,
        UPSTREAM_RELEASE_METADATA_MAX_BYTES,
        "read upstream release metadata",
        LocalProductError::LocalUpdate("upstream release metadata exceeds its byte bound"),
        LocalProductError::LocalUpdate("upstream release metadata is not a regular file"),
    )?;
    parse_official_release_metadata(&bytes, requested_version)
}

pub(super) fn version_order(a: &str, b: &str) -> Result<std::cmp::Ordering, LocalProductError> {
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

pub(super) fn reject_changed_archive(
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
