//! Optional signed component revisions. They never authorize payload admission.
use super::*;
use std::collections::BTreeMap;

pub(super) const LIMIT: u64 = 16 * 1024;
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Component {
    pub revision: u64,
    pub inputs: String,
    pub artifact: String,
    pub interface: String,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Metadata {
    pub components: Option<BTreeMap<String, Component>>,
    pub composition: Option<u64>,
}
fn number(cursor: &mut JsonCursor<'_>) -> Result<u64, LocalProductError> {
    cursor.skip_whitespace();
    let start = cursor.index;
    cursor.skip_number()?;
    std::str::from_utf8(&cursor.bytes[start..cursor.index])
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .ok_or(LocalProductError::Release("invalid component revision"))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn decode(
    bytes: &[u8],
    manifest: &LocalReleaseManifest,
    manifest_digest: &str,
) -> Option<Metadata> {
    if bytes.len() > LIMIT as usize {
        return None;
    }
    let mut cursor = JsonCursor::new(bytes);
    let (mut schema, mut generation, mut binding, mut components, mut composition) =
        (None, None, None, None, None);
    release_display::fields(&mut cursor, |key, cursor| {
        match key {
            "schema" => schema = Some(cursor.string()?),
            "generation_id" => generation = Some(cursor.string()?),
            "manifest_sha256" => binding = Some(cursor.string()?),
            "composition_revision" => composition = Some(number(cursor)?),
            "components" => {
                let mut entries = BTreeMap::new();
                release_display::fields(cursor, |name, cursor| {
                    let path = match name {
                        "core" => "core",
                        "runtime" => "runtime",
                        "manager" => "manager",
                        "tui" => "helpers/2",
                        _ => return cursor.error(),
                    };
                    let (mut revision, mut inputs, mut artifact, mut interface) =
                        (None, None, None, None);
                    release_display::fields(cursor, |key, cursor| {
                        match key {
                            "revision" => revision = Some(number(cursor)?),
                            "inputs_sha256" => inputs = Some(cursor.string()?),
                            "artifact_sha256" => artifact = Some(cursor.string()?),
                            "interface" => interface = Some(cursor.string()?),
                            _ => return cursor.error(),
                        };
                        Ok(())
                    })?;
                    let error = || LocalProductError::Release("invalid component metadata");
                    let inputs = inputs.filter(|v| digest(v)).ok_or_else(error)?;
                    let artifact = artifact.filter(|v| digest(v)).ok_or_else(error)?;
                    let interface = interface
                        .filter(|v| {
                            !v.is_empty()
                                && v.len() <= 128
                                && v.bytes().all(|b| b.is_ascii_graphic())
                        })
                        .ok_or_else(error)?;
                    if !manifest
                        .files
                        .iter()
                        .any(|f| f.relative_path == path && f.sha256 == artifact)
                    {
                        return Err(error());
                    }
                    entries.insert(
                        name.to_owned(),
                        Component {
                            revision: revision.ok_or_else(error)?,
                            inputs,
                            artifact,
                            interface,
                        },
                    );
                    Ok(())
                })?;
                if !entries.contains_key("core")
                    || !entries.contains_key("runtime")
                    || entries.contains_key("manager") != entries.contains_key("tui")
                    || manifest.files.iter().any(|f| f.relative_path == "manager")
                        != entries.contains_key("manager")
                    || manifest
                        .files
                        .iter()
                        .any(|f| f.relative_path == "helpers/2")
                        != entries.contains_key("tui")
                {
                    return cursor.error();
                }
                components = Some(entries);
            }
            _ => cursor.skip_value(0)?,
        };
        Ok(())
    })
    .ok()?;
    cursor.skip_whitespace();
    if cursor.index != bytes.len()
        || schema.as_deref() != Some("codex-component-input-v1")
        || generation.as_deref() != Some(&manifest.generation_id)
        || binding.as_deref() != Some(manifest_digest)
        || components.is_some() != composition.is_some()
    {
        return None;
    }
    Some(Metadata {
        components,
        composition,
    })
}
pub(super) fn read(
    roots: &LocalCoreRoots,
    directory: &std::path::Path,
    manifest: &LocalReleaseManifest,
) -> Option<Metadata> {
    let payload = directory.join("component-input-v1");
    let bytes = read_bounded_regular_file(
        &payload,
        LIMIT as usize,
        "read component metadata",
        LocalProductError::Release("component metadata exceeds bound"),
        LocalProductError::Release("component metadata is not regular"),
    )
    .ok()?;
    verify_release_signature_with_key(
        &roots.openssl,
        manifest.release_public_key,
        &payload,
        &directory.join("component-input-v1.sig"),
    )
    .ok()?;
    let binding = openssl_sha256(&roots.openssl, &directory.join("release.manifest")).ok()?;
    decode(&bytes, manifest, &binding)
}
pub(super) fn fetch(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    directory: &std::path::Path,
    manifest: &LocalReleaseManifest,
) {
    let mut bytes = 0;
    let fetched = [
        ("component-input-v1", LIMIT),
        ("component-input-v1.sig", 64),
    ]
    .into_iter()
    .all(|(name, limit)| {
        fetch_remote_resource_with_timeout(
            roots,
            base,
            name,
            &directory.join(name),
            limit,
            &mut bytes,
            "2",
        )
        .is_ok()
    });
    if !fetched || read(roots, directory, manifest).is_none() {
        // Only this acquisition's optional files; legacy releases remain usable.
        for name in ["component-input-v1", "component-input-v1.sig"] {
            let _ = std::fs::remove_file(directory.join(name));
        }
    }
}
impl Metadata {
    pub(super) fn revisions_json(&self) -> String {
        self.components.as_ref().map(|v| format!("{{{}}}",v.iter().map(|(name,c)| format!("\"{}\":{{\"revision\":{},\"inputs_sha256\":\"{}\",\"artifact_sha256\":\"{}\",\"interface\":\"{}\"}}",name,c.revision,c.inputs,c.artifact,json_escape(&c.interface))).collect::<Vec<_>>().join(","))).unwrap_or_else(||"null".into())
    }
}
