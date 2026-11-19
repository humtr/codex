use crate::cli::{PublicDispatchRoute, UpstreamArgScan};
use crate::error::LocalProductError;
use crate::generation::{
    load_activated_generation, probe_release_candidate, valid_sha256_hex,
    verify_generation_asset_digests, LoadedLocalGeneration, LocalWrapperRoots,
};
use crate::json::json_escape;
use crate::runtime_env::{capture_termux_process_env, prepare_wrapper_config, resolve_codex_home};
use crate::state::generation_state;
use crate::{execute_activated_route, extensions, maintenance, system_config};
use std::ffi::OsString;
use std::fs;
use std::io;

fn print_wrapper_usage() {
    println!("usage: codex wrapper <status [--verify [--wrapper-sha256 SHA256] [--upstream-version VER]] [--json] | setup [show|--bare|--enable MODULES|--disable MODULES|--signed-update true|false] | --bare [--] [UPSTREAM_ARGS...] | --with MODULES [--] [UPSTREAM_ARGS...] | --help>");
    println!("  status [--verify] [--json]  show or qualify active generation, paths, and opt-in modules");
    println!(
        "  setup                       read or change the persistent optional-module selection"
    );
    println!(
        "  --bare                      launch upstream without optional modules for this invocation"
    );
    println!(
        "  --with MODULES              opt in only for this invocation (unavailable modules fail closed)"
    );
    println!("  --help                      show this help");
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SetupOp<'a> {
    Bare,
    Enable(&'a str),
    Disable(&'a str),
    SignedUpdate(bool),
}

enum ParsedSetupAction<'a> {
    Help,
    Show,
    Update(Vec<SetupOp<'a>>),
}

fn parse_setup_bool(value: &str) -> Result<bool, ()> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(()),
    }
}

fn parse_setup_words<'a>(words: &[&'a str]) -> Result<ParsedSetupAction<'a>, &'static str> {
    match words {
        [] | ["show"] => return Ok(ParsedSetupAction::Show),
        ["-h" | "--help"] => return Ok(ParsedSetupAction::Help),
        _ => {}
    }
    let mut ops = Vec::new();
    let mut index = 0usize;
    while index < words.len() {
        let word = words[index];
        if word == "--bare" {
            ops.push(SetupOp::Bare);
            index += 1;
        } else if let Some(modules) = word.strip_prefix("--enable=") {
            ops.push(SetupOp::Enable(modules));
            index += 1;
        } else if word == "--enable" {
            let Some(&modules) = words.get(index + 1) else {
                return Err("expected module list after --enable");
            };
            ops.push(SetupOp::Enable(modules));
            index += 2;
        } else if let Some(modules) = word.strip_prefix("--disable=") {
            ops.push(SetupOp::Disable(modules));
            index += 1;
        } else if word == "--disable" {
            let Some(&modules) = words.get(index + 1) else {
                return Err("expected module list after --disable");
            };
            ops.push(SetupOp::Disable(modules));
            index += 2;
        } else if let Some(raw) = word.strip_prefix("--signed-update=") {
            let val = parse_setup_bool(raw)
                .map_err(|()| "expected true or false after --signed-update=")?;
            ops.push(SetupOp::SignedUpdate(val));
            index += 1;
        } else if word == "--signed-update" {
            let Some(&raw) = words.get(index + 1) else {
                return Err("expected true or false after --signed-update");
            };
            let val = parse_setup_bool(raw)
                .map_err(|()| "expected true or false after --signed-update")?;
            ops.push(SetupOp::SignedUpdate(val));
            index += 2;
        } else {
            return Err("invalid setup arguments");
        }
    }
    Ok(ParsedSetupAction::Update(ops))
}

fn run_wrapper_setup(args: Vec<OsString>) -> i32 {
    let words = args
        .iter()
        .map(|value| value.to_str())
        .collect::<Option<Vec<_>>>();
    let Some(words) = words else {
        eprintln!("codex wrapper setup: arguments must be UTF-8");
        return 2;
    };
    let action = match parse_setup_words(&words) {
        Ok(ParsedSetupAction::Help) => {
            print_wrapper_usage();
            return 0;
        }
        Ok(action) => action,
        Err(message) => {
            eprintln!("codex wrapper setup: {message}");
            print_wrapper_usage();
            return 2;
        }
    };
    let roots = match LocalWrapperRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            eprintln!("codex wrapper setup: {err}");
            return 1;
        }
    };
    let is_show = matches!(action, ParsedSetupAction::Show);
    let result = match action {
        ParsedSetupAction::Help => unreachable!(),
        ParsedSetupAction::Show => extensions::WrapperSettings::read(&roots.state_root),
        ParsedSetupAction::Update(ops) => {
            let allow_malformed = ops == [SetupOp::Bare];
            let updated = extensions::WrapperSettings::update(
                &roots.state_root,
                allow_malformed,
                |settings| {
                    for op in ops {
                        match op {
                            SetupOp::Bare => settings.clear_addons(),
                            SetupOp::Enable(modules) => settings.set_modules(modules, true)?,
                            SetupOp::Disable(modules) => settings.set_modules(modules, false)?,
                            SetupOp::SignedUpdate(val) => settings.signed_update = val,
                        }
                    }
                    Ok(())
                },
            );
            if updated.is_ok() {
                let _ = system_config::prune(&roots.config_dir);
            }
            updated
        }
    };
    match result {
        Ok(settings) => {
            println!(
                "config: {}",
                roots.state_root.join("wrapper.toml").display()
            );
            println!("addons: {}", settings.addons_label());
            println!("signed_update: {}", settings.signed_update);
            0
        }
        Err(err) if is_show => {
            println!(
                "config: {}",
                roots.state_root.join("wrapper.toml").display()
            );
            println!("addons: invalid");
            println!("signed_update: unknown");
            eprintln!("codex wrapper setup: invalid wrapper.toml: {err}");
            0
        }
        Err(err) => {
            eprintln!("codex wrapper setup: {err}");
            if err.kind() == io::ErrorKind::InvalidData {
                2
            } else {
                1
            }
        }
    }
}

fn run_bare_upstream(args: Vec<OsString>) -> i32 {
    let scan = UpstreamArgScan::new(&args);
    if let Err(err) = scan.reject_bare_or_with_updater() {
        eprintln!("codex wrapper --bare: {err}");
        return 2;
    }
    let roots = match LocalWrapperRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            eprintln!("codex wrapper --bare: {err}");
            return 1;
        }
    };
    let env = capture_termux_process_env();
    if let Err(err) =
        execute_activated_route(PublicDispatchRoute::Upstream(args), &roots, &env, &scan)
    {
        eprintln!("codex wrapper --bare: {err}");
        return 1;
    }
    0
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
        Some("setup") => run_wrapper_setup(rest),
        Some("--bare") => {
            let mut args = rest;
            if args.first().is_some_and(|value| value == "--") {
                args.remove(0);
            }
            run_bare_upstream(args)
        }
        Some("--with") => {
            let Some(names) = rest.first().and_then(|value| value.to_str()) else {
                eprintln!("codex wrapper --with: expected module list");
                return 2;
            };
            let mut settings = extensions::WrapperSettings::default();
            if let Err(err) = settings.set_modules(names, true) {
                eprintln!("codex wrapper --with: {err}");
                return 2;
            }
            let mut args = rest[1..].to_vec();
            if args.first().is_some_and(|value| value == "--") {
                args.remove(0);
            }
            if let Err(err) = UpstreamArgScan::new(&args).reject_bare_or_with_updater() {
                eprintln!("codex wrapper --with: {err}");
                return 2;
            }
            eprintln!(
                "codex wrapper --with: optional module(s) {} are not installed",
                settings.addons_label()
            );
            1
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
    let (addons, signed_update, addons_error) =
        match extensions::WrapperSettings::read(&roots.state_root) {
            Ok(settings) => (settings.addons_label(), Some(settings.signed_update), None),
            Err(err) => {
                eprintln!("codex wrapper status: warning: invalid wrapper.toml: {err}");
                ("invalid".to_owned(), None, Some(err.to_string()))
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
        let addons_err_json = addons_error
            .as_deref()
            .map(|msg| format!(",\"addons_error\":\"{}\"", json_escape(msg)))
            .unwrap_or_default();
        let signed_json = signed_update
            .map(|b| b.to_string())
            .unwrap_or_else(|| "null".to_owned());
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
                    "{{\"status\":\"active\",\"generation_id\":\"{}\",\"previous_generation_id\":{},\"upstream_version\":\"{}\",\"layout\":\"{}\",\"runtime_path\":\"{}\",\"has_code_mode_host\":{},\"patch_policy_id\":\"{}\",\"patch_report\":\"{}\",\"creation_metadata\":\"{}\",\"daemon_passthrough\":\"supported\",\"addons\":\"{}\"{addons_err_json},\"signed_update\":{signed_json}{verify_json},\"generation_root\":\"{}\",\"state_root\":\"{}\",\"config_dir\":\"{}\",\"codex_home\":{}}}\n",
                    json_escape(&gen.generation_id),
                    prev_json,
                    json_escape(&gen.manifest.upstream_package_version),
                    json_escape(gen.generation_layout.as_str()),
                    json_escape(&gen.runtime_path.to_string_lossy()),
                    gen.code_mode_host_path.is_some(),
                    json_escape(&gen.manifest.patch_policy_id),
                    json_escape(&gen.manifest.patch_report),
                    json_escape(&gen.manifest.creation_metadata),
                    json_escape(&addons),
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
                    "{{\"status\":\"uninitialized\",\"generation_id\":null,\"previous_generation_id\":null,\"daemon_passthrough\":\"supported\",\"addons\":\"{}\"{addons_err_json},\"signed_update\":{signed_json}{verify_json},\"generation_root\":\"{}\",\"state_root\":\"{}\",\"config_dir\":\"{}\",\"codex_home\":{}}}\n",
                    json_escape(&addons),
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
            println!("addons: {addons}");
            if let Some(err) = addons_error.as_deref() {
                println!("addons_error: {err}");
            }
            if let Some(signed) = signed_update {
                println!("signed_update: {signed}");
            } else {
                println!("signed_update: unknown");
            }
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
            println!("addons: {addons}");
            if let Some(err) = addons_error.as_deref() {
                println!("addons_error: {err}");
            }
            if let Some(signed) = signed_update {
                println!("signed_update: {signed}");
            } else {
                println!("signed_update: unknown");
            }
            if let Some(Err(reason)) = verification_result {
                println!("qualified: false");
                println!("verification_error: {reason}");
            }
            println!("Run 'codex update' or 'codex update --path <FILE_OR_DIR>' to install a generation.");
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
    use super::LocalWrapperRoots;
    use crate::run_public_main;
    use crate::tests::{
        create_private_test_dirs, os_vec, remove_temp_root, temp_root, EnvRestoreGuard,
    };
    use std::fs;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    #[test]
    fn optional_module_cli_rejects_unknown_or_unavailable_modules() {
        assert_eq!(
            run_public_main(os_vec(&["wrapper", "--with", "manager"])),
            1
        );
        assert_eq!(
            run_public_main(os_vec(&["wrapper", "--with", "tui,provider"])),
            1
        );
        assert_eq!(
            run_public_main(os_vec(&["wrapper", "--with", "not-a-module"])),
            2
        );
        assert_eq!(
            run_public_main(os_vec(&[
                "wrapper",
                "--with",
                "manager",
                "--",
                "--sandbox",
                "read-only"
            ])),
            2
        );
    }

    #[test]
    fn pure_bare_setup_resets_malformed_config_without_following_symlinks() {
        let root = temp_root("setup-bare-recovery");
        let home = root.join("home");
        let prefix = root.join("prefix");
        create_private_test_dirs(&home);
        create_private_test_dirs(&prefix);
        let _env = EnvRestoreGuard::new(&[
            ("HOME", Some(&home)),
            ("PREFIX", Some(&prefix)),
            ("TMPDIR", Some(&root)),
        ]);
        let roots = LocalWrapperRoots::from_environment().unwrap();
        create_private_test_dirs(&roots.state_root);
        let config = roots.state_root.join("wrapper.toml");
        let corrupt = b"manager = true\nsigned_update = true\ninvalid = [";
        fs::write(&config, corrupt).unwrap();
        let inode = fs::metadata(&config).unwrap().ino();
        for flags in [
            &["setup", "--enable", "manager"][..],
            &["setup", "--disable", "manager"],
            &["setup", "--bare", "--signed-update", "true"],
            &["setup", "--bare", "--bare"],
            &["--with", "manager"],
        ] {
            let mut args = os_vec(&["wrapper"]);
            args.extend(os_vec(flags));
            assert_ne!(run_public_main(args), 0, "{flags:?}");
            assert_eq!(fs::read(&config).unwrap(), corrupt);
        }
        assert_eq!(run_public_main(os_vec(&["wrapper", "setup", "--bare"])), 0);
        let settings = crate::extensions::WrapperSettings::read(&roots.state_root).unwrap();
        assert_eq!(settings, crate::extensions::WrapperSettings::default());
        assert_ne!(fs::metadata(&config).unwrap().ino(), inode);

        fs::write(&config, vec![b'x'; 32 * 1024]).unwrap();
        assert_eq!(run_public_main(os_vec(&["wrapper", "setup", "--bare"])), 0);
        assert_eq!(
            crate::extensions::WrapperSettings::read(&roots.state_root).unwrap(),
            crate::extensions::WrapperSettings::default()
        );

        let foreign = root.join("foreign");
        fs::rename(&config, &foreign).unwrap();
        std::os::unix::fs::symlink(&foreign, &config).unwrap();
        assert_eq!(run_public_main(os_vec(&["wrapper", "setup", "--bare"])), 2);
        assert_eq!(fs::read_link(&config).unwrap(), foreign);
        fs::remove_file(&config).unwrap();
        fs::rename(&foreign, &config).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o622)).unwrap();
        assert_eq!(run_public_main(os_vec(&["wrapper", "setup", "--bare"])), 2);
        assert_eq!(fs::metadata(&config).unwrap().mode() & 0o777, 0o622);

        drop(_env);
        remove_temp_root(root);
    }
}
