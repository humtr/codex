//! Offline Core-owned installed inventory. No Manager, Provider or network call.
use super::*;

const USAGE: &str = "usage: codex termux version [--details | --json]";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Format {
    Human,
    Details,
    Json,
}

pub(super) fn parse(args: &[OsString]) -> Result<Format, &'static str> {
    match args {
        [] => Ok(Format::Human),
        [arg] if arg == "--details" => Ok(Format::Details),
        [arg] if arg == "--json" => Ok(Format::Json),
        _ => Err(USAGE),
    }
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}
fn optional(value: Option<&str>) -> String {
    value.map(quoted).unwrap_or_else(|| "null".into())
}

pub(super) fn read(roots: &LocalCoreRoots, format: Format) -> Result<String, LocalProductError> {
    let paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let state = m2_generation_state::read_pointer_state(&paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    let (release, loaded) = verify_installed_local_release(
        roots,
        &state.current,
        state.current_key,
        "version generation descriptor differs from selected generation",
    )?;
    let metadata =
        component_metadata::read(roots, &roots.generation_root.join(&state.current), &release);
    let revisions = metadata
        .as_ref()
        .map(|m| m.revisions_json())
        .unwrap_or_else(|| "null".into());
    let composition = metadata
        .as_ref()
        .and_then(|m| m.composition)
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".into());
    // The signed inventory determines the edition, even if an optional file
    // has become unavailable. Never relabel a damaged Full as Core.
    let edition = if release
        .files
        .iter()
        .any(|f| f.relative_path == "manager" || f.relative_path == "helpers/2")
    {
        "Full"
    } else {
        "Core"
    };
    let manager = if loaded.manager_path.is_some() {
        "available"
    } else if loaded.manifest.manager_artifact_digest.is_some() {
        "unavailable"
    } else {
        "not_installed"
    };
    let tui = if loaded.tui_available {
        "available"
    } else if tui::index(&loaded.manifest)?.is_some() {
        "unavailable"
    } else {
        "not_installed"
    };
    let frontend_digest =
        tui::index(&loaded.manifest)?.map(|i| loaded.manifest.helper_digests[i].digest.as_str());
    let (provider_status, provider_json) = match provider::inventory(roots) {
        Ok(Some(v)) => ("verified", format!("{{\"status\":\"verified\",\"generation_id\":{},\"release_sequence\":{},\"manifest_sha256\":{},\"previous_generation\":{},\"readiness\":\"not_queried\"}}", quoted(&v.generation),v.sequence,quoted(&v.manifest_digest),optional(v.previous.as_deref()))),
        Ok(None) => ("not_selected", "{\"status\":\"not_selected\",\"readiness\":\"not_queried\"}".to_owned()),
        Err(_) => ("unavailable", "{\"status\":\"unavailable\",\"reason\":\"package_verification_failed\",\"readiness\":\"not_queried\"}".to_owned()),
    };
    if format == Format::Json {
        return Ok(format!(concat!(
            "{{\"schema\":\"codex-termux-version-v1\",\"upstream_version\":{},",
            "\"edition\":{},\"generation_id\":{},\"release_sequence\":{},",
            "\"signature\":\"verified\",\"core\":{{\"sha256\":{},\"api\":{}}},",
            "\"runtime_bundle\":{{\"runtime_sha256\":{},\"source_sha256\":{},\"platform\":{},\"architecture\":{}}},",
            "\"manager\":{{\"status\":{},\"sha256\":{}}},\"tui\":{{\"status\":{},\"sha256\":{}}},",
            "\"provider\":{},\"component_revisions\":{},\"composition_revision\":{},",
            "\"previous_generation\":{}}}\n"),
            quoted(&loaded.manifest.upstream_package_version), quoted(edition), quoted(&state.current), release.release_sequence,
            quoted(&loaded.manifest.core_artifact_digest), quoted(&loaded.manifest.core_api_identity),
            quoted(&loaded.manifest.runtime_digest), quoted(&loaded.manifest.source_artifact_digest),
            quoted(&loaded.manifest.expected_platform), quoted(&loaded.manifest.expected_architecture),
            quoted(manager), optional(loaded.manifest.manager_artifact_digest.as_deref()), quoted(tui), optional(frontend_digest), provider_json, revisions, composition, optional(state.previous.as_deref())));
    }
    let mut output = format!("Codex {}\nTermux release: {edition} #{}\nManager: {manager}\nEnhanced TUI: {tui}\nChatGPT Provider package: {provider_status}\n",
        loaded.manifest.upstream_package_version, release.release_sequence);
    if format == Format::Details {
        use std::fmt::Write as _;
        if let Some(entries) = metadata.as_ref().and_then(|m| m.components.as_ref()) {
            for (name, c) in entries {
                writeln!(
                    output,
                    "{name}: r{} · input {} · interface {}",
                    c.revision, c.inputs, c.interface
                )
                .expect("String write");
            }
            writeln!(output, "Composition revision: r{composition}").expect("String write");
        } else {
            output.push_str("Component revision: unavailable (legacy or unverified metadata)\n");
        }

        writeln!(
            output,
            "Manager SHA256: {}\nEnhanced TUI SHA256: {}",
            loaded
                .manifest
                .manager_artifact_digest
                .as_deref()
                .unwrap_or("not installed"),
            frontend_digest.unwrap_or("not installed")
        )
        .expect("String write");
        writeln!(output, "Generation: {}\nSignature: verified\nCore API: {}\nCore SHA256: {}\nRuntime SHA256: {}\nSource archive SHA256: {}\nPlatform: {}/{}\nPrevious generation: {}\nProvider: independent selection; service/authentication readiness not queried",
            state.current, loaded.manifest.core_api_identity, loaded.manifest.core_artifact_digest,
            loaded.manifest.runtime_digest, loaded.manifest.source_artifact_digest,
            loaded.manifest.expected_platform, loaded.manifest.expected_architecture,
            state.previous.as_deref().unwrap_or("none")).expect("String write");
    }
    Ok(output)
}

pub(super) fn run(roots: &LocalCoreRoots, args: &[OsString]) -> i32 {
    let format = match parse(args) {
        Ok(v) => v,
        Err(message) => {
            eprintln!("{message}");
            return 2;
        }
    };
    match read(roots, format) {
        Ok(output) => {
            print!("{output}");
            0
        }
        Err(error) => {
            eprintln!("codex termux version: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_are_bounded_and_upstream_version_is_not_reinterpreted() {
        assert_eq!(parse(&[]), Ok(Format::Human));
        assert_eq!(parse(&["--details".into()]), Ok(Format::Details));
        assert_eq!(parse(&["--json".into()]), Ok(Format::Json));
        for args in [
            vec!["--version".into()],
            vec!["--json".into(), "--details".into()],
            vec!["--json".into(), "--json".into()],
        ] {
            assert_eq!(parse(&args), Err(USAGE));
        }
        assert!(matches!(
            plan_public_dispatch(["--version"]).unwrap(),
            PublicDispatchRoute::Upstream(_)
        ));
        assert!(matches!(
            plan_public_dispatch(["-V"]).unwrap(),
            PublicDispatchRoute::Upstream(_)
        ));
    }
}
