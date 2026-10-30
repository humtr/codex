//! Resolve the fixed public logical origin to immutable GitHub Release assets.
//! The signed inventory remains the authority for content and installation paths.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const LOGICAL_ORIGIN: &str = "https://codex-releases.humtr.workers.dev/";
const GITHUB_RELEASES: &str = "https://github.com/humtr/codex/releases/download/";
const MAP_NAME: &str = "asset-map-v1.json";
const MAP_MAX_BYTES: usize = 16 * 1024;
const CONTROLS: &[&str] = &[
    "release.manifest",
    "release.sig",
    "update-index-v1",
    "update-index-v1.sig",
    "update-public-key.pem",
    "compat/download-size-v1",
    "compat/download-size-v1.sig",
    "component-input-v1",
    "component-input-v1.sig",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    generation: String,
    asset: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AssetMap {
    schema: String,
    generation_id: String,
    base_generation: Option<String>,
    manifest_sha256: String,
    files: BTreeMap<String, Target>,
}

fn invalid_map() -> LocalProductError {
    LocalProductError::Remote("GitHub Release asset map is invalid")
}

fn object(
    cursor: &mut JsonCursor<'_>,
    mut field: impl FnMut(&mut JsonCursor<'_>, String) -> Result<(), LocalProductError>,
) -> Result<(), LocalProductError> {
    cursor.expect(b'{')?;
    cursor.skip_whitespace();
    if cursor.bytes.get(cursor.index) == Some(&b'}') {
        cursor.index += 1;
        return Ok(());
    }
    loop {
        let key = cursor.string()?;
        cursor.expect(b':')?;
        field(cursor, key)?;
        cursor.skip_whitespace();
        match cursor.bytes.get(cursor.index) {
            Some(b',') => cursor.index += 1,
            Some(b'}') => {
                cursor.index += 1;
                return Ok(());
            }
            _ => return Err(invalid_map()),
        }
    }
}

fn target(cursor: &mut JsonCursor<'_>) -> Result<Target, LocalProductError> {
    let mut generation = None;
    let mut asset = None;
    object(cursor, |cursor, key| {
        match key.as_str() {
            "generation" if generation.is_none() => generation = Some(cursor.string()?),
            "asset" if asset.is_none() => asset = Some(cursor.string()?),
            _ => return Err(invalid_map()),
        }
        Ok(())
    })?;
    Ok(Target {
        generation: generation.ok_or_else(invalid_map)?,
        asset: asset.ok_or_else(invalid_map)?,
    })
}

fn decode_map(bytes: &[u8]) -> Result<AssetMap, LocalProductError> {
    let mut cursor = JsonCursor::new(bytes);
    let (mut schema, mut generation, mut base, mut digest, mut files) =
        (None, None, None, None, None);
    object(&mut cursor, |cursor, key| {
        match key.as_str() {
            "schema" if schema.is_none() => schema = Some(cursor.string()?),
            "generation_id" if generation.is_none() => generation = Some(cursor.string()?),
            "manifest_sha256" if digest.is_none() => digest = Some(cursor.string()?),
            "base_generation" if base.is_none() => {
                cursor.skip_whitespace();
                base = Some(if cursor.bytes.get(cursor.index) == Some(&b'n') {
                    cursor.literal(b"null")?;
                    None
                } else {
                    Some(cursor.string()?)
                });
            }
            "files" if files.is_none() => {
                let mut routes = BTreeMap::new();
                object(cursor, |cursor, path| {
                    if routes.insert(path, target(cursor)?).is_some() {
                        return Err(invalid_map());
                    }
                    Ok(())
                })?;
                files = Some(routes);
            }
            _ => return Err(invalid_map()),
        }
        Ok(())
    })?;
    cursor.skip_whitespace();
    if cursor.index != bytes.len() {
        return Err(invalid_map());
    }
    Ok(AssetMap {
        schema: schema.ok_or_else(invalid_map)?,
        generation_id: generation.ok_or_else(invalid_map)?,
        base_generation: base.ok_or_else(invalid_map)?,
        manifest_sha256: digest.ok_or_else(invalid_map)?,
        files: files.ok_or_else(invalid_map)?,
    })
}

fn valid_generation(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("local-") else {
        return false;
    };
    !suffix.is_empty()
        && suffix.len() <= 127
        && (suffix.as_bytes()[0].is_ascii_lowercase() || suffix.as_bytes()[0].is_ascii_digit())
        && suffix.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-')
        })
        && (value.starts_with("local-relay-hosted-")
            || value.starts_with("local-base-relay-hosted-"))
}

pub(super) fn mapped_generation(base: &str) -> Option<&str> {
    let generation = base.strip_prefix(LOGICAL_ORIGIN)?.strip_suffix('/')?;
    valid_generation(generation).then_some(generation)
}

fn flat_asset(path: &str) -> Option<&str> {
    match path {
        "helpers/0" => Some("helper-0"),
        "helpers/1" => Some("helper-1"),
        "helpers/2" => Some("helper-2"),
        "compat/download-size-v1" => Some("download-size-v1"),
        "compat/download-size-v1.sig" => Some("download-size-v1.sig"),
        "update-index-v1" => Some("candidate-update-index-v1"),
        "update-index-v1.sig" => Some("candidate-update-index-v1.sig"),
        "codex-code-mode-host"
        | "core"
        | "generation.meta"
        | "manager"
        | "runtime"
        | "release.manifest"
        | "release.sig"
        | "update-public-key.pem"
        | "component-input-v1"
        | "component-input-v1.sig" => Some(path),
        _ => None,
    }
}

fn parse_map(
    bytes: &[u8],
    generation: &str,
    manifest: &LocalReleaseManifest,
    digest: &str,
) -> Result<AssetMap, LocalProductError> {
    if bytes.len() > MAP_MAX_BYTES {
        return Err(invalid_map());
    }
    let map = decode_map(bytes).map_err(|_| invalid_map())?;
    if map.schema != "codex-asset-map-v1"
        || map.generation_id != generation
        || manifest.generation_id != generation
        || map.manifest_sha256 != digest
    {
        return Err(invalid_map());
    }
    if generation.starts_with("local-base-") {
        if map.base_generation.is_some() {
            return Err(invalid_map());
        }
    } else if !map
        .base_generation
        .as_deref()
        .is_some_and(|base| base.starts_with("local-base-relay-hosted-") && valid_generation(base))
    {
        return Err(invalid_map());
    }
    let mut expected: BTreeSet<&str> = CONTROLS.iter().copied().collect();
    for file in &manifest.files {
        if flat_asset(&file.relative_path).is_none() || !expected.insert(&file.relative_path) {
            return Err(invalid_map());
        }
    }
    if map.files.len() != expected.len()
        || !map
            .files
            .keys()
            .all(|path| expected.contains(path.as_str()))
    {
        return Err(invalid_map());
    }
    for (path, target) in &map.files {
        if flat_asset(path) != Some(target.asset.as_str())
            || !valid_generation(&target.generation)
            || ((CONTROLS.contains(&path.as_str()) || path == "generation.meta")
                && target.generation != generation)
        {
            return Err(invalid_map());
        }
    }
    Ok(map)
}

pub(super) fn resource_url(
    generation: &str,
    map: Option<&AssetMap>,
    path: &str,
) -> Result<String, LocalProductError> {
    let target = if let Some(map) = map {
        map.files
            .get(path)
            .map(|target| (target.generation.as_str(), target.asset.as_str()))
    } else {
        match path {
            "release.manifest"
            | "release.sig"
            | "release-authority.sig"
            | MAP_NAME
            | "component-input-v1"
            | "component-input-v1.sig" => Some((generation, path)),
            _ => None,
        }
    };
    let (physical_generation, asset) = target.ok_or(LocalProductError::Remote(
        "GitHub Release asset route is unavailable",
    ))?;
    Ok(format!("{GITHUB_RELEASES}{physical_generation}/{asset}"))
}

pub(super) fn resolve(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    acquisition_root: &std::path::Path,
    manifest: &LocalReleaseManifest,
    acquired_bytes: &mut u64,
) -> Result<RemoteReleaseBase, LocalProductError> {
    let Some(generation) = mapped_generation(&base.value) else {
        return Ok(base.clone());
    };
    let path = acquisition_root.join(".asset-map-v1.json");
    fetch_remote_control_resource(
        roots,
        base,
        MAP_NAME,
        &path,
        MAP_MAX_BYTES as u64,
        acquired_bytes,
    )?;
    let bytes = read_bounded_regular_file(
        &path,
        MAP_MAX_BYTES,
        "read GitHub Release asset map",
        LocalProductError::Remote("GitHub Release asset map exceeds its byte bound"),
        LocalProductError::Remote("GitHub Release asset map is not a regular file"),
    )?;
    let digest = openssl_sha256(&roots.openssl, &acquisition_root.join("release.manifest"))?;
    let map = parse_map(&bytes, generation, manifest, &digest)?;
    std::fs::remove_file(&path).map_err(|source| LocalProductError::Io {
        operation: "remove GitHub Release asset map",
        source,
    })?;
    let mut resolved = base.clone();
    resolved.routes = Some(map);
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = "local-relay-hosted-test-s20";
    const BASE: &str = "local-base-relay-hosted-test-s10";

    fn fixture() -> (LocalReleaseManifest, String, String) {
        let digest = "a".repeat(64);
        let manifest = LocalReleaseManifest {
            format: LocalReleaseFormat::CoordinatedV4,
            generation_id: FULL.into(),
            release_sequence: 20,
            channel: "stable".into(),
            expected_platform: "android".into(),
            expected_architecture: "aarch64".into(),
            core_api_identity: "core-api-v1".into(),
            persistent_schema_identity: "schema-v1".into(),
            release_public_key: ReleasePublicKey([0; 32]),
            files: ["core", "generation.meta", "helpers/0", "runtime"]
                .into_iter()
                .map(|path| ReleaseFileEntry {
                    relative_path: path.into(),
                    sha256: "b".repeat(64),
                    mode: 0o755,
                })
                .collect(),
        };
        let map = r#"{
            "schema":"codex-asset-map-v1", "generation_id":"@FULL@",
            "base_generation":"@BASE@", "manifest_sha256":"@DIGEST@",
            "files":{
                "core":{"generation":"@BASE@","asset":"core"},
                "generation.meta":{"generation":"@FULL@","asset":"generation.meta"},
                "helpers/0":{"generation":"@BASE@","asset":"helper-0"},
                "runtime":{"generation":"@BASE@","asset":"runtime"},
                "release.manifest":{"generation":"@FULL@","asset":"release.manifest"},
                "release.sig":{"generation":"@FULL@","asset":"release.sig"},
                "update-index-v1":{"generation":"@FULL@","asset":"candidate-update-index-v1"},
                "update-index-v1.sig":{"generation":"@FULL@","asset":"candidate-update-index-v1.sig"},
                "update-public-key.pem":{"generation":"@FULL@","asset":"update-public-key.pem"},
                "compat/download-size-v1":{"generation":"@FULL@","asset":"download-size-v1"},
                "compat/download-size-v1.sig":{"generation":"@FULL@","asset":"download-size-v1.sig"},
                "component-input-v1":{"generation":"@FULL@","asset":"component-input-v1"},
                "component-input-v1.sig":{"generation":"@FULL@","asset":"component-input-v1.sig"}
            }
        }"#.replace("@FULL@", FULL).replace("@BASE@", BASE).replace("@DIGEST@", &digest);
        (manifest, map, digest)
    }

    #[test]
    fn signed_logical_inventory_resolves_earlier_flat_assets_and_preserves_custom_origins() {
        let (manifest, map, digest) = fixture();
        let map = parse_map(map.as_bytes(), FULL, &manifest, &digest).unwrap();
        assert_eq!(
            resource_url(FULL, Some(&map), "helpers/0").unwrap(),
            format!("https://github.com/humtr/codex/releases/download/{BASE}/helper-0")
        );
        assert_eq!(
            resource_url(FULL, Some(&map), "generation.meta").unwrap(),
            format!("https://github.com/humtr/codex/releases/download/{FULL}/generation.meta")
        );
        assert_eq!(
            resource_url(FULL, None, "release.manifest").unwrap(),
            format!("https://github.com/humtr/codex/releases/download/{FULL}/release.manifest")
        );
        assert!(resource_url(FULL, None, "helpers/0").is_err());
        let custom =
            RemoteReleaseBase::parse(OsStr::new("https://private.example/owned/")).unwrap();
        assert_eq!(
            custom.resource_url("helpers/0").unwrap(),
            "https://private.example/owned/helpers/0"
        );
    }

    #[test]
    fn malformed_foreign_unbound_or_ambiguous_maps_are_rejected() {
        let (manifest, map, digest) = fixture();
        let changes = [
            (
                format!("\"manifest_sha256\":\"{digest}\""),
                format!("\"manifest_sha256\":\"{}\"", "c".repeat(64)),
            ),
            (
                format!("\"generation_id\":\"{FULL}\""),
                format!("\"generation_id\":\"{BASE}\""),
            ),
            (
                format!("\"base_generation\":\"{BASE}\""),
                "\"base_generation\":\"local-base-relay-hosted-../escape\"".into(),
            ),
            (
                "\"asset\":\"helper-0\"".into(),
                "\"asset\":\"../helper-0\"".into(),
            ),
            (
                format!("\"helpers/0\":{{\"generation\":\"{BASE}\""),
                "\"helpers/0\":{\"generation\":\"foreign/repository\"".into(),
            ),
            (
                format!("\"release.sig\":{{\"generation\":\"{FULL}\""),
                format!("\"release.sig\":{{\"generation\":\"{BASE}\""),
            ),
            (
                format!("\"runtime\":{{\"generation\":\"{BASE}\",\"asset\":\"runtime\"}},"),
                "".into(),
            ),
            (
                "\"files\":{".into(),
                format!("\"files\":{{\"extra\":{{\"generation\":\"{FULL}\",\"asset\":\"extra\"}},"),
            ),
            (format!("\"base_generation\":\"{BASE}\","), "".into()),
        ];
        for (original, replacement) in changes {
            assert!(
                map.contains(&original),
                "missing fixture mutation: {original}"
            );
            let item = map.replacen(&original, &replacement, 1);
            assert!(
                parse_map(item.as_bytes(), FULL, &manifest, &digest).is_err(),
                "{item}"
            );
        }
        let duplicate = map.replacen("\"files\":{", "\"files\":{\"core\":{\"asset\":\"core\",\"generation\":\"local-relay-hosted-test-s20\"},", 1);
        assert!(parse_map(duplicate.as_bytes(), FULL, &manifest, &digest).is_err());
        assert!(parse_map(&vec![b' '; MAP_MAX_BYTES + 1], FULL, &manifest, &digest).is_err());
    }
}
