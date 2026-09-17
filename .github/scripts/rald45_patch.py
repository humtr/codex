#!/usr/bin/env python3
from pathlib import Path
import re


def once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


core_path = Path("crates/core/src/main.rs")
core = core_path.read_text()

probe_fn = '''#[cfg(unix)]
fn probe_qualified_upstream_doctor<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
) -> Result<UpstreamDoctorStatus, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    Ok(capture_qualified_upstream_doctor(
        assets,
        process_env,
        cert_file,
        cert_dir,
        resolver_path,
        config_dir,
        DoctorCaptureOptions {
            json: false,
            use_color: false,
            force_color: false,
        },
    )?
    .status)
}

'''
core = once(core, probe_fn, "", "remove activation-only doctor probe helper")

execute_marker = '#[cfg(unix)]\nfn execute_activated_route(\n'
public_helper = '''#[cfg(unix)]
fn public_doctor_capability(
    creation_metadata: &str,
    declared: UpstreamDoctorCapability,
) -> UpstreamDoctorCapability {
    if creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA
        && declared == UpstreamDoctorCapability::Unsupported
    {
        // RALD-4.5 transition generations preserve the exact R10 layout marker so
        // the public stable Core can stage them, while `unsupported` tells only
        // that legacy activation gate not to couple activation to user/provider
        // health. The corrected public doctor must still execute upstream doctor.
        UpstreamDoctorCapability::Supported
    } else {
        declared
    }
}

'''
core = once(core, execute_marker, public_helper + execute_marker, "public doctor transition interpretation")
core = once(
    core,
    '            doctor_capability: loaded.doctor_capability,\n',
    '            doctor_capability: public_doctor_capability(\n'
    '                &loaded.manifest.creation_metadata,\n'
    '                loaded.doctor_capability,\n'
    '            ),\n',
    "public doctor capability use",
)

doctor_gate = '''        if loaded.doctor_capability == UpstreamDoctorCapability::Supported
            && probe_qualified_upstream_doctor(
                assets,
                process_env,
                roots.cert_file.as_os_str(),
                Some(roots.cert_dir.as_os_str()),
                &roots.resolver_path,
                &roots.config_dir,
            )
            .map_err(|_| LocalProductError::CandidateProbe("candidate doctor probe failed"))?
                != UpstreamDoctorStatus::Healthy
        {
            return Err(LocalProductError::CandidateProbe(
                "candidate doctor probe was unhealthy",
            ));
        }
'''
core = once(core, doctor_gate, "", "remove activation doctor-health gate")
core = once(
    core,
    'fn test_m2_b4_activation_version_and_doctor_probe_failures_preserve_old_current() {',
    'fn test_m2_b4_activation_version_probe_failure_blocks_but_doctor_health_does_not() {',
    "rename activation probe contract test",
)
doctor_test = '''        b4_assert_public_update_rejected(
            &doctor_failure,
            &home,
            &prefix,
            &tmp,
            b"candidate doctor probe was unhealthy",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
'''
doctor_test_new = '''        b4_assert_public_update_activated(
            &doctor_failure,
            &home,
            &prefix,
            &tmp,
            "doctor-failure",
        );
        let state_after_doctor_health = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state_after_doctor_health.current, "doctor-failure");
        assert_eq!(
            state_after_doctor_health.previous.as_deref(),
            Some("probe-current")
        );
'''
core = once(core, doctor_test, doctor_test_new, "doctor health is not activation integrity")

test_marker = '''    #[cfg(unix)]
    #[test]
    fn test_m2_b4_activation_version_probe_failure_blocks_but_doctor_health_does_not() {
'''
transition_test = '''    #[cfg(unix)]
    #[test]
    fn test_rald45_transition_signal_preserves_public_doctor_semantics() {
        assert_eq!(
            public_doctor_capability(
                R10_BROWSER_HELPER_BRIDGE_METADATA,
                UpstreamDoctorCapability::Unsupported,
            ),
            UpstreamDoctorCapability::Supported
        );
        assert_eq!(
            public_doctor_capability("test-fixture", UpstreamDoctorCapability::Unsupported),
            UpstreamDoctorCapability::Unsupported
        );
        assert_eq!(
            public_doctor_capability(
                R10_BROWSER_HELPER_BRIDGE_METADATA,
                UpstreamDoctorCapability::Supported,
            ),
            UpstreamDoctorCapability::Supported
        );
    }

'''
core = once(core, test_marker, transition_test + test_marker, "transition public-doctor test")
core = once(
    core,
    '        b4_write_probe_runtime(&bad_probe_release, 0, 1);\n',
    '        b4_write_probe_runtime(&bad_probe_release, 1, 0);\n',
    "rollback guard keeps exact version probe failure",
)
core_path.write_text(core)

builder_path = Path("crates/release-builder/src/lib.rs")
builder = builder_path.read_text()
builder = once(
    builder,
    '    "[--defer-manager-probe] --creation-metadata <VALUE> ",\n',
    '    "[--defer-manager-probe] [--legacy-activation-doctor-unsupported] ",\n'
    '    "--creation-metadata <VALUE> ",\n',
    "builder usage transition flag",
)
builder = once(
    builder,
    '    defer_manager_probe: bool,\n    creation_metadata: String,\n',
    '    defer_manager_probe: bool,\n    legacy_activation_doctor_unsupported: bool,\n    creation_metadata: String,\n',
    "BuildRequest transition field",
)
builder = once(
    builder,
    '    defer_manager_probe: bool,\n    creation_metadata: Option<String>,\n',
    '    defer_manager_probe: bool,\n    legacy_activation_doctor_unsupported: bool,\n    creation_metadata: Option<String>,\n',
    "RequestFields transition field",
)
parse_anchor = '''        if flag == OsStr::new("--defer-manager-probe") {
            if fields.defer_manager_probe {
                return Err(BuilderError::Usage);
            }
            fields.defer_manager_probe = true;
            continue;
        }
'''
parse_new = parse_anchor + '''        if flag == OsStr::new("--legacy-activation-doctor-unsupported") {
            if fields.legacy_activation_doctor_unsupported {
                return Err(BuilderError::Usage);
            }
            fields.legacy_activation_doctor_unsupported = true;
            continue;
        }
'''
builder = once(builder, parse_anchor, parse_new, "parse transition flag")
builder = once(
    builder,
    '        defer_manager_probe: fields.defer_manager_probe,\n        creation_metadata: fields.creation_metadata.ok_or(BuilderError::Usage)?,\n',
    '        defer_manager_probe: fields.defer_manager_probe,\n'
    '        legacy_activation_doctor_unsupported: fields.legacy_activation_doctor_unsupported,\n'
    '        creation_metadata: fields.creation_metadata.ok_or(BuilderError::Usage)?,\n',
    "parsed transition field",
)
validate_anchor = '''    if !valid_line_value(&request.creation_metadata, TEXT_VALUE_MAX_BYTES) {
        return Err(BuilderError::Invalid("creation metadata is invalid"));
    }
'''
validate_new = validate_anchor + '''    if request.legacy_activation_doctor_unsupported
        && request.creation_metadata != R10_BROWSER_HELPER_BRIDGE_METADATA
    {
        return Err(BuilderError::Invalid(
            "legacy activation doctor transition requires the exact R10 helper layout",
        ));
    }
'''
builder = once(builder, validate_anchor, validate_new, "validate bounded transition flag")

validator_old = '''    let creation_metadata = publish_descriptor_field(&mut lines, "creation_metadata")?;
    if (creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA) != r10_bridge
        || publish_descriptor_field(&mut lines, "upstream_doctor")? != "supported"
        || publish_descriptor_field(&mut lines, "helper_count")? != "2"
    {
'''
validator_new = '''    let creation_metadata = publish_descriptor_field(&mut lines, "creation_metadata")?;
    let upstream_doctor = publish_descriptor_field(&mut lines, "upstream_doctor")?;
    let transition_doctor = creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA
        && upstream_doctor == "unsupported";
    if (creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA) != r10_bridge
        || (upstream_doctor != "supported" && !transition_doctor)
        || publish_descriptor_field(&mut lines, "helper_count")? != "2"
    {
'''
builder = once(builder, validator_old, validator_new, "publish validator bounded transition")

write_marker = '''    let descriptor = format!(
'''
write_new = '''    let upstream_doctor = if request.legacy_activation_doctor_unsupported {
        "unsupported"
    } else {
        "supported"
    };
    let descriptor = format!(
'''
start = builder.index('fn write_generation_descriptor(')
end = builder.index('\nfn rename_noreplace', start)
descriptor_fn = builder[start:end]
descriptor_fn = once(descriptor_fn, write_marker, write_new, "descriptor transition value")
descriptor_fn = once(
    descriptor_fn,
    'upstream_doctor\\tsupported',
    'upstream_doctor\\t{}',
    "descriptor doctor placeholder",
)
descriptor_fn = once(
    descriptor_fn,
    '        request.creation_metadata,\n        TERMUX_BROWSER_OPEN_HELPER_IDENTITY,\n',
    '        request.creation_metadata,\n        upstream_doctor,\n        TERMUX_BROWSER_OPEN_HELPER_IDENTITY,\n',
    "descriptor doctor argument",
)
builder = builder[:start] + descriptor_fn + builder[end:]

pattern = re.compile(r'(?P<i>[ \t]+)defer_manager_probe: false,\n(?P=i)creation_metadata:')
builder, constructor_count = pattern.subn(
    lambda m: f'{m.group("i")}defer_manager_probe: false,\n{m.group("i")}legacy_activation_doctor_unsupported: false,\n{m.group("i")}creation_metadata:',
    builder,
)
if constructor_count != 3:
    raise SystemExit(f"direct BuildRequest constructors: expected 3, found {constructor_count}")

args_anchor = '''        if request.defer_manager_probe {
            args.push(OsString::from("--defer-manager-probe"));
        }
        args
'''
args_new = '''        if request.defer_manager_probe {
            args.push(OsString::from("--defer-manager-probe"));
        }
        if request.legacy_activation_doctor_unsupported {
            args.push(OsString::from("--legacy-activation-doctor-unsupported"));
        }
        args
'''
builder = once(builder, args_anchor, args_new, "test args preserve transition flag")

builder_test_marker = '''    #[test]
    fn test_tc_live_bridge_build_and_publish_are_marker_bound_and_r10_readable() {
'''
builder_test = '''    #[test]
    fn test_rald45_transition_flag_is_exact_and_legacy_bounded() {
        let mut fixture = Fixture::new("rald45-transition-flag");
        fixture.request.creation_metadata = R10_BROWSER_HELPER_BRIDGE_METADATA.to_owned();
        fixture.request.legacy_activation_doctor_unsupported = true;
        let parsed = parse_request(request_args(&fixture.request)).unwrap();
        assert!(parsed.legacy_activation_doctor_unsupported);
        assert_eq!(parsed.creation_metadata, R10_BROWSER_HELPER_BRIDGE_METADATA);

        fixture.request.creation_metadata = "test-fixture".to_owned();
        assert!(matches!(
            validate_request(&fixture.request),
            Err(BuilderError::Invalid(
                "legacy activation doctor transition requires the exact R10 helper layout"
            ))
        ));
    }

'''
builder = once(builder, builder_test_marker, builder_test + builder_test_marker, "builder transition unit test")
builder_path.write_text(builder)
