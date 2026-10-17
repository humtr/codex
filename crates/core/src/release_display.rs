//! Optional signed release labels. Never update eligibility or runtime authority.
use super::*;
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
struct Display {
    date: String,
    base: Option<u64>,
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
    {
        return false;
    }
    let year = value[..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<usize>().unwrap_or(0);
    let day = value[8..].parse::<u32>().unwrap_or(0);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        0,
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && month > 0 && month <= 12 && day > 0 && day <= days[month]
}

fn fields(
    cursor: &mut JsonCursor<'_>,
    mut read: impl FnMut(&str, &mut JsonCursor<'_>) -> Result<(), LocalProductError>,
) -> Result<(), LocalProductError> {
    cursor.expect(b'{')?;
    let mut seen = BTreeSet::new();
    loop {
        let key = cursor.string()?;
        if !seen.insert(key.clone()) || seen.len() > 16 {
            return cursor.error();
        }
        cursor.expect(b':')?;
        read(&key, cursor)?;
        cursor.skip_whitespace();
        match cursor.bytes.get(cursor.index) {
            Some(b',') => cursor.index += 1,
            Some(b'}') => {
                cursor.index += 1;
                return Ok(());
            }
            _ => return cursor.error(),
        }
    }
}

fn decode(bytes: &[u8], manifest: &LocalReleaseManifest, digest: &str) -> Option<Display> {
    if bytes.len() > 4096 {
        return None;
    }
    let mut cursor = JsonCursor::new(bytes);
    let (mut schema, mut generation, mut manifest_digest, mut display) = (None, None, None, None);
    fields(&mut cursor, |key, cursor| {
        match key {
            "schema" => schema = Some(cursor.string()?),
            "generation_id" => generation = Some(cursor.string()?),
            "manifest_sha256" => manifest_digest = Some(cursor.string()?),
            "display" => {
                let (mut date, mut base_generation, mut base_sequence) = (None, None, None);
                fields(cursor, |key, cursor| {
                    match key {
                        "release_date" => date = Some(cursor.string()?),
                        "base_generation" => {
                            cursor.skip_whitespace();
                            base_generation =
                                Some(if cursor.bytes.get(cursor.index) == Some(&b'n') {
                                    cursor.literal(b"null")?;
                                    None
                                } else {
                                    Some(cursor.string()?)
                                });
                        }
                        "base_sequence" => {
                            cursor.skip_whitespace();
                            base_sequence =
                                Some(if cursor.bytes.get(cursor.index) == Some(&b'n') {
                                    cursor.literal(b"null")?;
                                    None
                                } else {
                                    let start = cursor.index;
                                    cursor.skip_number()?;
                                    Some(
                                        std::str::from_utf8(&cursor.bytes[start..cursor.index])
                                            .ok()
                                            .and_then(|text| text.parse::<u64>().ok())
                                            .ok_or(LocalProductError::Release(
                                                "invalid display base sequence",
                                            ))?,
                                    )
                                });
                        }
                        _ => return cursor.error(),
                    }
                    Ok(())
                })?;
                let date = date
                    .filter(|date| valid_date(date))
                    .ok_or(LocalProductError::Release("invalid display date"))?;
                let base_generation =
                    base_generation.ok_or(LocalProductError::Release("missing display base"))?;
                let base_sequence =
                    base_sequence.ok_or(LocalProductError::Release("missing display sequence"))?;
                match (&base_generation, base_sequence, is_full(manifest)) {
                    (None, None, false) => {}
                    (Some(id), Some(sequence), true)
                        if sequence > 0
                            && sequence < manifest.release_sequence
                            && id.starts_with("local-base-")
                            && m2_generation_state::validate_generation_identity(
                                id,
                                "display base",
                            )
                            .is_ok() => {}
                    _ => return cursor.error(),
                }
                display = Some(Display {
                    date,
                    base: base_sequence,
                });
            }
            _ => cursor.skip_value(0)?,
        }
        Ok(())
    })
    .ok()?;
    cursor.skip_whitespace();
    if cursor.index != bytes.len()
        || schema.as_deref() != Some("codex-component-input-v1")
        || generation.as_deref() != Some(&manifest.generation_id)
        || manifest_digest.as_deref() != Some(digest)
    {
        return None;
    }
    display
}

fn is_full(manifest: &LocalReleaseManifest) -> bool {
    manifest
        .files
        .iter()
        .any(|file| file.relative_path == "manager")
}

fn label(manifest: &LocalReleaseManifest) -> String {
    format!(
        "{} #{}",
        if is_full(manifest) { "Full" } else { "Base" },
        manifest.release_sequence
    )
}

fn render(
    current: &LocalReleaseManifest,
    target: &LocalReleaseManifest,
    display: Option<&Display>,
) -> String {
    let mut text = format!("Termux release: {}", label(current));
    if current.generation_id != target.generation_id {
        text.push_str(&format!(" -> {}", label(target)));
    }
    if is_full(target) {
        match display.and_then(|display| display.base) {
            Some(base) => text.push_str(&format!(" · Base #{base}")),
            None => text.push_str(" · Base: unavailable"),
        }
    }
    text.push_str(" · Release date: ");
    match display {
        Some(display) => text.push_str(&format!("{} (UTC)", display.date)),
        None => text.push_str("unavailable"),
    }
    text
}

pub(super) fn line(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    manifest_path: &std::path::Path,
    current: &LocalReleaseManifest,
    target: &LocalReleaseManifest,
) -> String {
    let sequence = REMOTE_ACQUISITION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        ".release-display-{}-{sequence}",
        std::process::id()
    ));
    let display = if create_remote_acquisition_root(&root).is_ok() {
        let display = (|| {
            let mut bytes = 0;
            for (name, limit) in [("component-input-v1", 4096), ("component-input-v1.sig", 64)] {
                fetch_remote_resource_with_timeout(
                    roots,
                    base,
                    name,
                    &root.join(name),
                    limit,
                    &mut bytes,
                    "2",
                )
                .ok()?;
            }
            let payload = root.join("component-input-v1");
            verify_release_signature_with_key(
                &roots.openssl,
                target.release_public_key,
                &payload,
                &root.join("component-input-v1.sig"),
            )
            .ok()?;
            let digest = openssl_sha256(&roots.openssl, manifest_path).ok()?;
            let data = read_bounded_regular_file(
                &payload,
                4096,
                "read release display",
                LocalProductError::Release("release display exceeds bound"),
                LocalProductError::Release("release display is not a regular file"),
            )
            .ok()?;
            decode(&data, target, &digest)
        })();
        let _ = std::fs::remove_dir_all(&root);
        display
    } else {
        None
    };
    render(current, target, display.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(full: bool) -> LocalReleaseManifest {
        LocalReleaseManifest {
            format: LocalReleaseFormat::CoordinatedV4,
            generation_id: "local-display-test".into(),
            release_sequence: 59,
            channel: "stable".into(),
            expected_platform: "android".into(),
            expected_architecture: "aarch64".into(),
            core_api_identity: "core-api-v1".into(),
            persistent_schema_identity: "schema-v1".into(),
            release_public_key: ReleasePublicKey([1; 32]),
            files: if full {
                vec![ReleaseFileEntry {
                    relative_path: "manager".into(),
                    sha256: "a".repeat(64),
                    mode: 0o755,
                }]
            } else {
                vec![]
            },
        }
    }

    fn payload(display: &str) -> Vec<u8> {
        format!(r#"{{"schema":"codex-component-input-v1","generation_id":"local-display-test","manifest_sha256":"{}","display":{display}}}"#, "a".repeat(64)).into_bytes()
    }

    #[test]
    fn metadata_requires_exact_identity_valid_date_and_plausible_base() {
        let full = manifest(true);
        let digest = "a".repeat(64);
        let good = r#"{"release_date":"2026-10-17","base_generation":"local-base-owned","base_sequence":58}"#;
        let info = decode(&payload(good), &full, &digest).unwrap();
        assert_eq!(
            info,
            Display {
                date: "2026-10-17".into(),
                base: Some(58)
            }
        );
        for bad in [
            good.replace("2026-10-17", "2026-02-30"),
            good.replace("2026-10-17", "2026-10-17\\nforged"),
            good.replace("58", "59"),
            good.replace("58", "0"),
            good.replace("58", "5.8"),
            good.replace("local-base-owned", "foreign"),
            good.replace(
                "\"base_sequence\":58",
                "\"base_sequence\":58,\"base_sequence\":1",
            ),
            good.replace("58", "null"),
        ] {
            assert_eq!(decode(&payload(&bad), &full, &digest), None, "{bad}");
        }
        assert_eq!(decode(&payload(good), &full, &"b".repeat(64)), None);
        let changed = String::from_utf8(payload(good))
            .unwrap()
            .replace("local-display-test", "other");
        assert_eq!(decode(changed.as_bytes(), &full, &digest), None);
        let base = r#"{"release_date":"2024-02-29","base_generation":null,"base_sequence":null}"#;
        assert!(decode(&payload(base), &manifest(false), &digest).is_some());
        assert!(decode(&payload(base), &full, &digest).is_none());
        assert!(!valid_date("2025-02-29"));
        assert!(decode(&vec![b' '; 4097], &full, &digest).is_none());
    }

    #[test]
    fn labels_distinguish_same_upstream_releases_without_inventing_legacy_dates() {
        let target = manifest(true);
        let mut previous = target.clone();
        previous.release_sequence = 57;
        previous.generation_id = "older".into();
        let info = Display {
            date: "2026-10-17".into(),
            base: Some(58),
        };
        assert_eq!(
            render(&previous, &target, Some(&info)),
            "Termux release: Full #57 -> Full #59 · Base #58 · Release date: 2026-10-17 (UTC)"
        );
        assert_eq!(
            render(&target, &target, None),
            "Termux release: Full #59 · Base: unavailable · Release date: unavailable"
        );
        assert_eq!(
            render(
                &manifest(false),
                &manifest(false),
                Some(&Display {
                    date: info.date,
                    base: None
                })
            ),
            "Termux release: Base #59 · Release date: 2026-10-17 (UTC)"
        );
        let legacy=br#"{"schema":"codex-component-input-v1","generation_id":"local-display-test","manifest_sha256":"old"}"#;
        assert!(decode(legacy, &target, "old").is_none());
    }
}
