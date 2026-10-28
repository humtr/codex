//! Optional signed release labels. Never update eligibility or runtime authority.
use super::*;
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
struct Display {
    date: String,
    base: Option<u64>,
    base_generation: Option<String>,
}

fn valid_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit())
    {
        return false;
    }
    let year = value[..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..].parse::<u32>().unwrap_or(0);
    year > 0 && day > 0 && month_days(year, month).is_some_and(|days| day <= days)
}

/// Public release annotations; signed indexes and manifests still select versions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Publication {
    pub(super) at: Option<String>,
    body: String,
}

fn month_days(year: u32, month: u32) -> Option<u32> {
    match month {
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(
            if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) {
                29
            } else {
                28
            },
        ),
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        _ => None,
    }
}

// KST is the requested fixed UTC+09:00 presentation, not a device clock change.
pub(super) fn kst(value: &str) -> Option<String> {
    let b = value.as_bytes();
    if b.len() != 20
        || !value.is_ascii()
        || !valid_date(&value[..10])
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
        || !b[11..19]
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 2 | 5) || c.is_ascii_digit())
    {
        return None;
    }
    let mut year = value[..4].parse::<u32>().ok()?;
    let mut month = value[5..7].parse::<u32>().ok()?;
    let mut day = value[8..10].parse::<u32>().ok()?;
    let hour = value[11..13].parse::<u32>().ok()?;
    let minute = value[14..16].parse::<u32>().ok()?;
    let second = value[17..19].parse::<u32>().ok()?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let shifted = hour + 9;
    if shifted >= 24 {
        day += 1;
        if day > month_days(year, month)? {
            day = 1;
            month += 1;
            if month == 13 {
                month = 1;
                year += 1;
            }
        }
    }
    (year <= 9999).then(|| {
        format!(
            "{year:04}-{month:02}-{day:02} {:02}:{minute:02}:{second:02} KST (UTC+09:00)",
            shifted % 24
        )
    })
}

pub(super) fn publication(bytes: &[u8], tag: &str) -> Option<Publication> {
    if bytes.len() > UPSTREAM_RELEASE_METADATA_MAX_BYTES {
        return None;
    }
    let mut cursor = JsonCursor::new(bytes);
    let (mut actual_tag, mut at, mut body, mut draft) = (None, None, String::new(), false);
    fields(&mut cursor, |key, cursor| {
        match key {
            "tag_name" => actual_tag = Some(cursor.string()?),
            "published_at" | "body" => {
                cursor.skip_whitespace();
                if cursor.bytes.get(cursor.index) == Some(&b'"') {
                    let value = cursor.string()?;
                    if key == "published_at" {
                        at = kst(&value);
                    } else {
                        body = value;
                    }
                } else {
                    cursor.skip_value(0)?;
                }
            }
            "draft" => {
                cursor.skip_whitespace();
                draft = cursor.bytes.get(cursor.index) == Some(&b't');
                cursor.skip_value(0)?;
            }
            _ => cursor.skip_value(0)?,
        }
        Ok(())
    })
    .ok()?;
    cursor.skip_whitespace();
    (cursor.index == bytes.len() && actual_tag.as_deref() == Some(tag) && !draft)
        .then_some(Publication { at, body })
}

impl Publication {
    fn changes(&self, section: &str) -> Vec<String> {
        let mut active = section == "Upstream";
        let mut fenced = false;
        let mut notes = Vec::new();
        for line in self.body.lines() {
            let line = line.trim();
            if line.starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            if let Some(heading) = line.strip_prefix("## ").filter(|_| section != "Upstream") {
                active = heading.trim() == section;
                continue;
            }
            if !active {
                continue;
            }
            let Some(note) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) else {
                continue;
            };
            if note.is_empty() || note.chars().any(char::is_control) {
                continue;
            }
            let mut brief: String = note.chars().take(180).collect();
            if note.chars().count() > 180 {
                brief.push('…');
            }
            notes.push(brief);
            if notes.len() == 3 {
                break;
            }
        }
        notes
    }
}

fn public_origin(base: &str, tag: &str) -> bool {
    base == format!("https://humtr.github.io/codex/{tag}/")
        || base == format!("https://github.com/humtr/codex/releases/download/{tag}/")
}

fn public_release(roots: &LocalCoreRoots, tag: &str) -> Option<Publication> {
    let sequence = REMOTE_ACQUISITION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        ".release-publication-{}-{sequence}",
        std::process::id()
    ));
    create_remote_acquisition_root(&root).ok()?;
    let info = (|| {
        let path = root.join("release.json");
        let output = create_remote_output(&path).ok()?;
        fetch_remote_file_with_timeout(
            roots,
            &format!("https://api.github.com/repos/humtr/codex/releases/tags/{tag}"),
            &output,
            UPSTREAM_RELEASE_METADATA_MAX_BYTES as u64,
            "2",
        )
        .ok()?;
        let bytes = read_bounded_regular_file(
            &path,
            UPSTREAM_RELEASE_METADATA_MAX_BYTES,
            "read release publication",
            LocalProductError::Release("release publication exceeds bound"),
            LocalProductError::Release("release publication is not a regular file"),
        )
        .ok()?;
        publication(&bytes, tag)
    })();
    let _ = std::fs::remove_dir_all(root);
    info
}

fn changes(text: &mut String, info: Option<&Publication>, section: &str, url: &str) {
    let notes = info.map(|info| info.changes(section)).unwrap_or_default();
    text.push_str(&format!("\n{section} changes:"));
    if notes.is_empty() {
        text.push_str(" unavailable");
    } else {
        for note in notes {
            text.push_str(&format!("\n  - {note}"));
        }
        text.push_str(&format!("\n  Full notes: {url}"));
    }
}

pub(super) fn provider_details(
    roots: &LocalCoreRoots,
    base: &str,
    generation: &str,
    sequence: u64,
    current: Option<u64>,
) -> Option<String> {
    if !public_origin(base, generation) {
        return None;
    }
    let info = public_release(roots, generation);
    let versions = match current {
        Some(selected) if selected == sequence => format!("#{sequence} (latest)"),
        Some(selected) => format!("#{selected} -> latest #{sequence} (separate Provider update)"),
        None => format!("release #{sequence}"),
    };
    let mut text = format!(
        "Provider chatgpt: {versions} · Published: {}",
        info.as_ref()
            .and_then(|v| v.at.as_deref())
            .unwrap_or("unavailable")
    );
    changes(
        &mut text,
        info.as_ref(),
        "Provider",
        &format!("https://github.com/humtr/codex/releases/tag/{generation}"),
    );
    Some(text)
}

pub(super) fn upstream(version: &str, info: Option<&Publication>) -> String {
    let mut text = format!(
        "Latest upstream Codex: {version} · Published: {}",
        info.and_then(|v| v.at.as_deref()).unwrap_or("unavailable")
    );
    changes(
        &mut text,
        info,
        "Upstream",
        &format!("https://github.com/openai/codex/releases/tag/rust-v{version}"),
    );
    text
}

fn fields(
    cursor: &mut JsonCursor<'_>,
    mut read: impl FnMut(&str, &mut JsonCursor<'_>) -> Result<(), LocalProductError>,
) -> Result<(), LocalProductError> {
    cursor.expect(b'{')?;
    let mut seen = BTreeSet::new();
    loop {
        let key = cursor.string()?;
        if !seen.insert(key.clone()) {
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
                    base_generation,
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

fn label_of(manifest: &LocalReleaseManifest) -> String {
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
    publication: Option<&Publication>,
    core_publication: Option<&Publication>,
) -> String {
    let mut text = format!("Termux release: {}", label_of(current));
    if current.generation_id != target.generation_id {
        text.push_str(&format!(" -> {}", label_of(target)));
    }
    if is_full(target) {
        match display.and_then(|display| display.base) {
            Some(base) => text.push_str(&format!(" · Base #{base}")),
            None => text.push_str(" · Base: unavailable"),
        }
    }
    if let Some(at) = publication.and_then(|info| info.at.as_ref()) {
        text.push_str(&format!(" · Published: {at}"));
    } else {
        text.push_str(" · Release date: ");
        match display {
            Some(display) => text.push_str(&format!("{} (UTC)", display.date)),
            None => text.push_str("unavailable"),
        }
    }
    if publication.is_some() || core_publication.is_some() {
        let url = format!(
            "https://github.com/humtr/codex/releases/tag/{}",
            target.generation_id
        );
        let core_url = display
            .and_then(|d| d.base_generation.as_deref())
            .map(|generation| format!("https://github.com/humtr/codex/releases/tag/{generation}"))
            .unwrap_or_else(|| url.clone());
        if is_full(target) {
            let component = display
                .and_then(|d| d.base)
                .map(|sequence| format!("Base #{sequence}"))
                .unwrap_or_else(|| format!("{} package", label_of(target)));
            text.push_str(&format!(
                "\nCore ({component}) · Published: {}",
                core_publication
                    .and_then(|v| v.at.as_deref())
                    .unwrap_or("unavailable")
            ));
        }
        changes(&mut text, core_publication, "Core", &core_url);
        for (file, label, section) in [
            ("manager", "Manager", "Manager"),
            ("helpers/2", "Enhanced TUI", "TUI"),
        ] {
            if target.files.iter().any(|entry| entry.relative_path == file) {
                text.push_str(&format!(
                    "\n{label} ({} package) · Published: {}",
                    label_of(target),
                    publication
                        .and_then(|info| info.at.as_deref())
                        .unwrap_or("unavailable")
                ));
                changes(&mut text, publication, section, &url);
            }
        }
    }
    text
}

pub(super) fn line(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    manifest_path: &std::path::Path,
    current: &LocalReleaseManifest,
    target: &LocalReleaseManifest,
    provider_key: ReleasePublicKey,
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
    if !public_origin(&base.value, &target.generation_id) {
        return render(current, target, display.as_ref(), None, None);
    }
    let info = public_release(roots, &target.generation_id);
    let base_info = display
        .as_ref()
        .and_then(|d| d.base_generation.as_deref())
        .and_then(|generation| public_release(roots, generation));
    let core_info = if display
        .as_ref()
        .is_some_and(|d| d.base_generation.is_some())
    {
        base_info.as_ref()
    } else {
        info.as_ref()
    };
    let mut text = render(current, target, display.as_ref(), info.as_ref(), core_info);
    match provider::update_versions(roots, provider_key) {
        Ok(Some((selected, latest))) => {
            if let Some(details) = provider_details(
                roots,
                &latest.release_base(),
                &latest.generation,
                latest.sequence,
                Some(selected.sequence),
            ) {
                text.push('\n');
                text.push_str(&details);
            }
        }
        Ok(None) => (),
        Err(error) => text.push_str(&format!(
            "\nProvider chatgpt: latest information unavailable: {error}"
        )),
    }
    text
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
    fn publication_times_use_kst_calendar_boundaries_without_inventing_precision() {
        assert_eq!(
            kst("2026-10-28T17:04:05Z").as_deref(),
            Some("2026-10-29 02:04:05 KST (UTC+09:00)")
        );
        assert_eq!(
            kst("2024-02-29T23:59:59Z").as_deref(),
            Some("2024-03-01 08:59:59 KST (UTC+09:00)")
        );
        assert_eq!(
            kst("2026-12-31T16:00:00Z").as_deref(),
            Some("2027-01-01 01:00:00 KST (UTC+09:00)")
        );
        for bad in [
            "2026-10-28",
            "2026-02-30T10:00:00Z",
            "2026-10-28T24:00:00Z",
            "2026-10-28T00:60:00Z",
            "2026-10-28T00:00:60Z",
            "9999-12-31T23:00:00Z",
            "aaaaaaaaa한12345678",
            "2026-10-28T10:00:00X",
        ] {
            assert_eq!(kst(bad), None, "{bad}");
        }
    }

    #[test]
    fn public_annotations_bind_exact_tag_preserve_scope_and_reject_terminal_controls() {
        let data = br###"{"tag_name":"local-display-test","published_at":"2026-10-28T17:04:05Z","draft":false,"body":"## Core\n- New update display.\n- Invalid \u001b[2J injection.\n## Provider\n- Provider-only change.\n"}"###;
        let info = publication(data, "local-display-test").unwrap();
        assert_eq!(info.changes("Core"), ["New update display."]);
        assert_eq!(info.changes("Provider"), ["Provider-only change."]);
        assert!(publication(data, "wrong-tag").is_none());
        assert!(publication(
            &String::from_utf8(data.to_vec())
                .unwrap()
                .replace("false", "true")
                .into_bytes(),
            "local-display-test"
        )
        .is_none());
        let missing = publication(
            br#"{"tag_name":"old","body":null,"published_at":null}"#,
            "old",
        )
        .unwrap();
        assert_eq!(missing, Publication::default());
        let rendered = render(
            &manifest(false),
            &manifest(true),
            None,
            Some(&info),
            Some(&info),
        );
        assert!(rendered.contains("2026-10-29 02:04:05 KST (UTC+09:00)"));
        assert!(rendered.contains("Core changes:\n  - New update display."));
        assert!(rendered.contains("Manager (Full #59 package)"));
        assert!(!rendered.contains("Provider-only"));
        // GitHub release objects have more keys than the small signed sidecar.
        let extras = (0..30)
            .map(|i| format!("\"extra{i}\":null,"))
            .collect::<String>();
        let complete = format!("{{{extras}\"tag_name\":\"old\",\"published_at\":null}}");
        assert!(publication(complete.as_bytes(), "old").is_some());
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
                base: Some(58),
                base_generation: Some("local-base-owned".into())
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
            base_generation: Some("local-base-owned".into()),
        };
        assert_eq!(
            render(&previous, &target, Some(&info), None, None),
            "Termux release: Full #57 -> Full #59 · Base #58 · Release date: 2026-10-17 (UTC)"
        );
        assert_eq!(
            render(&target, &target, None, None, None),
            "Termux release: Full #59 · Base: unavailable · Release date: unavailable"
        );
        assert_eq!(
            render(
                &manifest(false),
                &manifest(false),
                Some(&Display {
                    date: info.date,
                    base: None,
                    base_generation: None
                }),
                None,
                None
            ),
            "Termux release: Base #59 · Release date: 2026-10-17 (UTC)"
        );
        let legacy=br#"{"schema":"codex-component-input-v1","generation_id":"local-display-test","manifest_sha256":"old"}"#;
        assert!(decode(legacy, &target, "old").is_none());
    }
}
