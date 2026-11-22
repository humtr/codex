use crate::error::LocalProductError;
use crate::generation::{
    load_activated_generation, probe_release_candidate, valid_sha256_hex,
    verify_generation_asset_digests, LoadedLocalGeneration, LocalWrapperRoots,
};
use crate::json::json_escape;
use crate::maintenance;
use crate::runtime_env::{capture_termux_process_env, prepare_wrapper_config, resolve_codex_home};
use crate::state::generation_state;
use std::ffi::OsString;
use std::fs;

fn print_wrapper_usage() {
    println!(
        "usage: codex wrapper <status [--verify [--wrapper-sha256 SHA256] [--upstream-version VER]] [--json] | --help>"
    );
    println!("  status [--verify] [--json]  show or qualify active generation and wrapper paths");
    println!("  --help                      show this help");
}

struct WrapperStatusOptions {
    json: bool,
    verify: bool,
    expected_wrapper_sha256: Option<String>,
    expected_upstream_version: Option<String>,
}

pub(super) fn run_wrapper_command(args: Vec<OsString>) -> i32 {
    let mut iter = args.into_iter();
    let Some(subcommand) = iter.next() else {
        print_wrapper_usage();
        return 0;
    };
    let rest: Vec<OsString> = iter.collect();
    match subcommand.to_str() {
        Some("-h" | "--help" | "help") if rest.is_empty() => {
            print_wrapper_usage();
            0
        }
        Some("status") => {
            let mut options = WrapperStatusOptions {
                json: false,
                verify: false,
                expected_wrapper_sha256: None,
                expected_upstream_version: None,
            };
            let mut index = 0usize;
            while index < rest.len() {
                let arg = &rest[index];
                match arg.to_str() {
                    Some("--json") if !options.json => {
                        options.json = true;
                        index += 1;
                    }
                    Some("--verify") if !options.verify => {
                        options.verify = true;
                        index += 1;
                    }
                    Some("--wrapper-sha256") if options.expected_wrapper_sha256.is_none() => {
                        let Some(val) = rest.get(index + 1).and_then(|v| v.to_str()) else {
                            eprintln!(
                                "codex wrapper status: --wrapper-sha256 requires a SHA-256 hex value"
                            );
                            return 2;
                        };
                        if !valid_sha256_hex(val) {
                            eprintln!(
                                "codex wrapper status: --wrapper-sha256 must be 64 lowercase hex characters"
                            );
                            return 2;
                        }
                        options.expected_wrapper_sha256 = Some(val.to_owned());
                        index += 2;
                    }
                    Some("--upstream-version") if options.expected_upstream_version.is_none() => {
                        let Some(val) = rest.get(index + 1).and_then(|v| v.to_str()) else {
                            eprintln!(
                                "codex wrapper status: --upstream-version requires a version value"
                            );
                            return 2;
                        };
                        if !codex_release_builder::valid_stable_version(val) {
                            eprintln!(
                                "codex wrapper status: --upstream-version must be MAJOR.MINOR.PATCH"
                            );
                            return 2;
                        }
                        options.expected_upstream_version = Some(val.to_owned());
                        index += 2;
                    }
                    Some("-h" | "--help") => {
                        print_wrapper_usage();
                        return 0;
                    }
                    _ => {
                        eprintln!("codex wrapper status: unexpected argument {:?}", arg);
                        return 2;
                    }
                }
            }
            if !options.verify
                && (options.expected_wrapper_sha256.is_some()
                    || options.expected_upstream_version.is_some())
            {
                eprintln!(
                    "codex wrapper status: --wrapper-sha256 and --upstream-version require --verify"
                );
                return 2;
            }
            run_wrapper_status(options)
        }
        _ => {
            eprintln!("codex wrapper: unknown subcommand {:?}", subcommand);
            print_wrapper_usage();
            2
        }
    }
}

fn verify_active_generation_snapshot(
    roots: &LocalWrapperRoots,
    gen: &LoadedLocalGeneration,
    options: &WrapperStatusOptions,
) -> Result<(), String> {
    let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root)
        .map_err(|err| err.to_string())?;
    for pending in [
        &state_paths.activation_journal,
        &state_paths.activation_journal_temp,
        &state_paths.activation_state_temp,
    ] {
        if fs::symlink_metadata(pending).is_ok() {
            return Err(format!(
                "activation state has unsettled transaction artifact '{}'",
                pending.display()
            ));
        }
    }
    if let Some(expected_sha) = options.expected_wrapper_sha256.as_deref() {
        if gen.manifest.wrapper_artifact_digest != expected_sha {
            return Err(format!(
                "generation wrapper_artifact_digest '{}' does not match expected '{expected_sha}'",
                gen.manifest.wrapper_artifact_digest
            ));
        }
    }
    if let Some(expected_ver) = options.expected_upstream_version.as_deref() {
        if gen.manifest.upstream_package_version != expected_ver {
            return Err(format!(
                "generation upstream_package_version '{}' does not match requested '{expected_ver}'",
                gen.manifest.upstream_package_version
            ));
        }
    }
    if gen.manifest.patch_policy_id != codex_release_builder::PATCH_POLICY_ID {
        return Err(format!(
            "generation patch_policy_id '{}' does not match required '{}'",
            gen.manifest.patch_policy_id,
            codex_release_builder::PATCH_POLICY_ID
        ));
    }
    verify_generation_asset_digests(gen).map_err(|err| err.to_string())?;
    prepare_wrapper_config(&roots.config_dir)
        .map_err(|err| format!("wrapper config preparation failed: {err}"))?;
    let env = capture_termux_process_env();
    probe_release_candidate(gen, roots, &env).map_err(|err| err.to_string())
}

fn run_wrapper_status(options: WrapperStatusOptions) -> i32 {
    let roots = match LocalWrapperRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            eprintln!("codex wrapper status: {err}");
            return 1;
        }
    };
    let codex_home = resolve_codex_home();
    let _lease = match maintenance::launch_lease(&roots.generation_root) {
        Ok(lease) => lease,
        Err(err) => {
            eprintln!("codex wrapper status: cannot lease installed generation: {err}");
            return 1;
        }
    };
    let loaded = load_activated_generation(&roots);
    let verification_result = match (&loaded, options.verify) {
        (Ok((_, gen)), true) => Some(verify_active_generation_snapshot(&roots, gen, &options)),
        (Err(err), true) => Some(Err(err.to_string())),
        (_, false) => None,
    };
    let verified_ok = verification_result.as_ref().is_none_or(|res| res.is_ok());

    if options.json {
        let verify_json = match &verification_result {
            None => String::new(),
            Some(Ok(())) => ",\"qualified\":true".to_owned(),
            Some(Err(reason)) => format!(
                ",\"qualified\":false,\"verification_error\":\"{}\"",
                json_escape(reason)
            ),
        };
        let body = match &loaded {
            Ok((pointer, gen)) => {
                let prev_json = pointer
                    .previous
                    .as_deref()
                    .map(|p| format!("\"{}\"", json_escape(p)))
                    .unwrap_or_else(|| "null".to_owned());
                let codex_home_json = codex_home
                    .as_ref()
                    .map(|p| format!("\"{}\"", json_escape(&p.to_string_lossy())))
                    .unwrap_or_else(|| "null".to_owned());
                format!(
                    "{{\"status\":\"active\",\"generation_id\":\"{}\",\"previous_generation_id\":{},\"upstream_version\":\"{}\",\"layout\":\"{}\",\"runtime_path\":\"{}\",\"has_code_mode_host\":{},\"patch_policy_id\":\"{}\",\"patch_report\":\"{}\",\"creation_metadata\":\"{}\",\"daemon_passthrough\":\"supported\"{verify_json},\"generation_root\":\"{}\",\"state_root\":\"{}\",\"config_dir\":\"{}\",\"codex_home\":{}}}\n",
                    json_escape(&gen.generation_id),
                    prev_json,
                    json_escape(&gen.manifest.upstream_package_version),
                    json_escape(gen.generation_layout.as_str()),
                    json_escape(&gen.runtime_path.to_string_lossy()),
                    gen.code_mode_host_path.is_some(),
                    json_escape(&gen.manifest.patch_policy_id),
                    json_escape(&gen.manifest.patch_report),
                    json_escape(&gen.manifest.creation_metadata),
                    json_escape(&roots.generation_root.to_string_lossy()),
                    json_escape(&roots.state_root.to_string_lossy()),
                    json_escape(&roots.config_dir.to_string_lossy()),
                    codex_home_json,
                )
            }
            Err(LocalProductError::NoCurrentGeneration) => {
                let codex_home_json = codex_home
                    .as_ref()
                    .map(|p| format!("\"{}\"", json_escape(&p.to_string_lossy())))
                    .unwrap_or_else(|| "null".to_owned());
                format!(
                    "{{\"status\":\"uninitialized\",\"generation_id\":null,\"previous_generation_id\":null,\"daemon_passthrough\":\"supported\"{verify_json},\"generation_root\":\"{}\",\"state_root\":\"{}\",\"config_dir\":\"{}\",\"codex_home\":{}}}\n",
                    json_escape(&roots.generation_root.to_string_lossy()),
                    json_escape(&roots.state_root.to_string_lossy()),
                    json_escape(&roots.config_dir.to_string_lossy()),
                    codex_home_json,
                )
            }
            Err(err) => {
                eprintln!("codex wrapper status: {err}");
                return 1;
            }
        };
        print!("{body}");
        return if loaded.is_ok() && verified_ok { 0 } else { 1 };
    }

    match loaded {
        Ok((pointer, gen)) => {
            println!("status: active");
            println!("generation_id: {}", gen.generation_id);
            println!(
                "previous_generation_id: {}",
                pointer.previous.as_deref().unwrap_or("-")
            );
            println!(
                "upstream_version: {}",
                gen.manifest.upstream_package_version
            );
            println!("layout: {}", gen.generation_layout.as_str());
            println!("runtime_path: {}", gen.runtime_path.display());
            println!("has_code_mode_host: {}", gen.code_mode_host_path.is_some());
            println!("patch_policy_id: {}", gen.manifest.patch_policy_id);
            println!("patch_report: {}", gen.manifest.patch_report);
            println!("creation_metadata: {}", gen.manifest.creation_metadata);
            println!("daemon_passthrough: supported");
            if let Some(ref res) = verification_result {
                match res {
                    Ok(()) => println!("qualified: true"),
                    Err(reason) => {
                        println!("qualified: false");
                        println!("verification_error: {reason}");
                    }
                }
            }
            println!("generation_root: {}", roots.generation_root.display());
            println!("state_root: {}", roots.state_root.display());
            println!("config_dir: {}", roots.config_dir.display());
            println!(
                "codex_home: {}",
                codex_home
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "-".to_owned())
            );
            if verified_ok {
                0
            } else {
                1
            }
        }
        Err(LocalProductError::NoCurrentGeneration) => {
            println!("status: uninitialized");
            println!("generation_root: {}", roots.generation_root.display());
            println!("state_root: {}", roots.state_root.display());
            println!("config_dir: {}", roots.config_dir.display());
            if let Some(Err(reason)) = verification_result {
                println!("qualified: false");
                println!("verification_error: {reason}");
            }
            println!(
                "Run 'codex update' or 'codex update --path <FILE_OR_DIR>' to install a generation."
            );
            1
        }
        Err(err) => {
            eprintln!("codex wrapper status: {err}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::run_public_main;
    use crate::tests::os_vec;

    #[test]
    fn wrapper_cli_rejects_retired_or_unknown_subcommands() {
        assert_eq!(run_public_main(os_vec(&["wrapper", "--help"])), 0);
        assert_eq!(run_public_main(os_vec(&["wrapper", "status", "--help"])), 0);
        for retired in [
            &["wrapper", "setup"][..],
            &["wrapper", "setup", "show"],
            &["wrapper", "--bare"],
            &["wrapper", "--with", "manager"],
            &["wrapper", "unknown"],
        ] {
            assert_eq!(run_public_main(os_vec(retired)), 2, "{retired:?}");
        }
    }
}
