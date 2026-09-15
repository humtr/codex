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
old = '''fn publish_descriptor_field<'a>(
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
'''
new = '''fn publish_descriptor_field_with_bound<'a>(
    lines: &mut std::str::Lines<'a>,
    expected: &'static str,
    max_bytes: usize,
) -> Result<&'a str, BuilderError> {
    let line = lines
        .next()
        .ok_or(BuilderError::Invalid("generation descriptor is incomplete"))?;
    let Some((name, value)) = line.split_once('\t') else {
        return Err(BuilderError::Invalid(
            "generation descriptor field is malformed",
        ));
    };
    if name != expected || !valid_line_value(value, max_bytes) {
        return Err(BuilderError::Invalid(
            "generation descriptor field is invalid",
        ));
    }
    Ok(value)
}

fn publish_descriptor_field<'a>(
    lines: &mut std::str::Lines<'a>,
    expected: &'static str,
) -> Result<&'a str, BuilderError> {
    publish_descriptor_field_with_bound(lines, expected, TEXT_VALUE_MAX_BYTES)
}
'''
b = replace_once(b, old, new, "add bounded publish descriptor field parser")
b = replace_once(
    b,
    '    let creation_metadata = publish_descriptor_field(&mut lines, "creation_metadata")?;',
    '    let creation_metadata = publish_descriptor_field_with_bound(\n        &mut lines,\n        "creation_metadata",\n        CREATION_METADATA_MAX_BYTES,\n    )?;',
    "use creation-metadata publish bound",
)

test = r'''

    #[test]
    fn test_rald1_creation_metadata_bound_roundtrips_build_and_publish() {
        let mut fixture = fixture("rald1-creation-metadata", happy_entries("0.150.1"), false);
        fixture.request.creation_metadata = "m".repeat(1024);
        assert_eq!(run_from_args(request_args(&fixture.request)), 0);
        let descriptor = std::fs::read_to_string(fixture.request.output.join("generation.meta"))
            .unwrap();
        assert!(descriptor.contains(&format!(
            "creation_metadata\t{}\n",
            fixture.request.creation_metadata
        )));

        let private_key = fixture.root.join("release-private.pem");
        generate_publish_key(&fixture.request.openssl, &private_key);
        let publish_request = PublishRequest {
            generation: fixture.request.output.clone(),
            release_sequence: "7".to_owned(),
            release_base: "https://example.test/releases/test-generation/".to_owned(),
            private_key,
            openssl: fixture.request.openssl.clone(),
            output: fixture.root.join("publication"),
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
            fixture.request.creation_metadata
        )));

        let mut too_large = fixture("rald1-creation-metadata-too-large", happy_entries("0.150.1"), false);
        too_large.request.creation_metadata = "m".repeat(CREATION_METADATA_MAX_BYTES + 1);
        assert!(build(&too_large.request).is_err());
        assert!(!too_large.request.output.exists());
        too_large.remove();
        fixture.remove();
    }
'''
last = b.rfind("\n}")
if last < 0:
    raise SystemExit("release-builder tests module closing brace missing")
b = b[:last] + test + b[last:]
builder.write_text(b)
