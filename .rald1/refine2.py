from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


local = Path("crates/core/src/local_derived.rs")
s = local.read_text()
s = replace_once(
    s,
    "            (Ok(()), Ok(public_key)) => public_key,",
    "            (Ok(_), Ok(public_key)) => public_key,",
    "accept release-builder generation id while destroying ephemeral signer",
)
local.write_text(s)

builder = Path("crates/release-builder/src/lib.rs")
b = builder.read_text()
function_start = b.index("fn publish_descriptor_field<'a>(")
next_function = b.index("\nfn validate_publish_patch_report(", function_start)
helper = r'''
fn publish_creation_metadata_field<'a>(
    lines: &mut std::str::Lines<'a>,
) -> Result<&'a str, BuilderError> {
    let line = lines
        .next()
        .ok_or(BuilderError::Invalid("generation descriptor is incomplete"))?;
    let Some((name, value)) = line.split_once('\t') else {
        return Err(BuilderError::Invalid(
            "generation descriptor field is malformed",
        ));
    };
    if name != "creation_metadata" || !valid_line_value(value, CREATION_METADATA_MAX_BYTES) {
        return Err(BuilderError::Invalid(
            "generation descriptor field is invalid",
        ));
    }
    Ok(value)
}

'''
b = b[:next_function] + "\n" + helper + b[next_function + 1 :]
b = replace_once(
    b,
    '    let creation_metadata = publish_descriptor_field(&mut lines, "creation_metadata")?;',
    '    let creation_metadata = publish_creation_metadata_field(&mut lines)?;',
    "use creation-metadata publish bound",
)

test = r'''

    #[test]
    fn test_rald1_creation_metadata_bound_roundtrips_build_and_publish() {
        let mut roundtrip = fixture("rald1-creation-metadata", happy_entries("0.150.1"), false);
        roundtrip.request.creation_metadata = "m".repeat(1024);
        assert_eq!(run_from_args(request_args(&roundtrip.request)), 0);
        let descriptor =
            std::fs::read_to_string(roundtrip.request.output.join("generation.meta")).unwrap();
        assert!(descriptor.contains(&format!(
            "creation_metadata\t{}\n",
            roundtrip.request.creation_metadata
        )));

        let private_key = roundtrip.root.join("release-private.pem");
        generate_publish_key(&roundtrip.request.openssl, &private_key);
        let publish_request = PublishRequest {
            generation: roundtrip.request.output.clone(),
            release_sequence: "7".to_owned(),
            release_base: "https://example.test/releases/test-generation/".to_owned(),
            private_key,
            openssl: roundtrip.request.openssl.clone(),
            output: roundtrip.root.join("publication"),
        };
        assert_eq!(run_from_args(publish_args(&publish_request)), 0);
        let published_descriptor = std::fs::read_to_string(
            publish_request
                .output
                .join("releases/test-generation/generation.meta"),
        )
        .unwrap();
        assert!(published_descriptor.contains(&format!(
            "creation_metadata\t{}\n",
            roundtrip.request.creation_metadata
        )));

        let mut too_large = fixture(
            "rald1-creation-metadata-too-large",
            happy_entries("0.150.1"),
            false,
        );
        too_large.request.creation_metadata = "m".repeat(CREATION_METADATA_MAX_BYTES + 1);
        assert!(build(&too_large.request).is_err());
        assert!(!too_large.request.output.exists());
        too_large.remove();
        roundtrip.remove();
    }
'''
last = b.rfind("\n}")
if last < 0:
    raise SystemExit("release-builder tests module closing brace missing")
b = b[:last] + test + b[last:]
builder.write_text(b)

main = Path("crates/core/src/main.rs")
m = main.read_text()
platform_helper = r'''#[cfg(all(unix, not(test)))]
fn local_release_platform_matches(value: &str) -> bool {
    value == std::env::consts::OS
}

#[cfg(all(unix, test))]
fn local_release_platform_matches(value: &str) -> bool {
    value == std::env::consts::OS
        || (value == "android"
            && std::env::var_os("CODEX_TEST_TERMUX_ANDROID_RELEASE_PLATFORM").as_deref()
                == Some(OsStr::new("1")))
}

'''
marker = "#[cfg(unix)]\nfn validate_local_release_policy(manifest: &LocalReleaseManifest)"
if m.count(marker) != 1:
    raise SystemExit("local release policy marker missing")
m = m.replace(marker, platform_helper + marker, 1)
m = replace_once(
    m,
    "    if manifest.expected_platform != std::env::consts::OS {",
    "    if !local_release_platform_matches(&manifest.expected_platform) {",
    "route release platform through production-exact/test-only matcher",
)

generation_platform_helper = r'''#[cfg(not(test))]
fn generation_manifest_platform_matches(value: &str, requirement: &str) -> bool {
    value == requirement
}

#[cfg(test)]
fn generation_manifest_platform_matches(value: &str, requirement: &str) -> bool {
    value == requirement
        || (requirement == std::env::consts::OS
            && value == "android"
            && std::env::var_os("CODEX_TEST_TERMUX_ANDROID_RELEASE_PLATFORM").as_deref()
                == Some(OsStr::new("1")))
}

'''
marker = "fn qualify_generation_manifest<'a>("
if m.count(marker) != 1:
    raise SystemExit("generation manifest qualifier marker missing")
m = m.replace(marker, generation_platform_helper + marker, 1)
m = replace_once(
    m,
    "    if manifest.expected_platform != requirements.platform {",
    "    if !generation_manifest_platform_matches(&manifest.expected_platform, requirements.platform) {",
    "route generation platform through production-exact/test-only matcher",
)
main.write_text(m)
