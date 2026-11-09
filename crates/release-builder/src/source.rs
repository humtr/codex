//! Common source preparation for device builds and remote release executors.
//! Work happens only in an explicitly supplied disposable official checkout;
//! applying a patch does not grant execution or publication authority.

mod patch;
mod syntax;

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const FULL_ACCESS: &str = "BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS";
const WORKSPACE: &str = "BUILT_IN_PERMISSION_PROFILE_WORKSPACE";
const DESCRIPTION: &str = "Run without a sandbox; the selected reviewer handles approval requests required by command rules and policy.";
const REVIEW_DESCRIPTION: &str = "Auto-review approval requests without a sandbox.";

type Result<T> = std::result::Result<T, String>;

fn read(path: &Path) -> Result<String> {
    if !fs::symlink_metadata(path)
        .map_err(|error| format!("{}: {error}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "{}: expected a regular source file",
            path.display()
        ));
    }
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn toml(text: &str) -> Result<toml::Value> {
    toml::from_str(text).map_err(|error| format!("source TOML: {error}"))
}

fn git(source: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(args)
        .output()
        .map_err(|error| format!("official source Git: {error}"))?;
    if !output.status.success() {
        return Err(format!("official source Git failed: {}", output.status));
    }
    Ok(output.stdout)
}

pub fn native_patch_base() -> Result<String> {
    patch::base(include_bytes!("native_route.patch"))
}

fn replace(text: &str, old: &str, new: &str, where_: &str) -> Result<String> {
    let count = text.matches(old).count();
    if count != 1 {
        return Err(format!("{where_}: expected one {old:?}, found {count}"));
    }
    Ok(text.replacen(old, new, 1))
}

fn rust_sources(directory: &Path, result: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            rust_sources(&entry.path(), result)?;
        } else if kind.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        {
            result.push(entry.path());
        }
    }
    Ok(())
}

struct Plan<'a> {
    source: &'a Path,
    changes: BTreeMap<PathBuf, String>,
}

impl Plan<'_> {
    fn operation(&mut self, operation: syntax::Operation) -> Result<()> {
        let workspace = self.source.join("codex-rs");
        let mut candidates = Vec::new();
        manifests(&workspace, &workspace, &mut candidates)?;
        let mut packages = Vec::new();
        for path in candidates {
            let value = toml(&read(&path)?)?;
            if value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(toml::Value::as_str)
                == Some(operation.package())
            {
                packages.push(path);
            }
        }
        if packages.len() != 1 {
            return Err(format!(
                "{operation:?}: expected one {} package, found {}",
                operation.package(),
                packages.len()
            ));
        }
        let package = packages[0].parent().unwrap();
        let mut files = Vec::new();
        rust_sources(&package.join("src"), &mut files)?;
        if package.join("build.rs").is_file() {
            files.push(package.join("build.rs"));
        }
        let sources = files
            .into_iter()
            .map(|path| {
                let text = match self.changes.get(&path) {
                    Some(text) => text.clone(),
                    None => read(&path)?,
                };
                Ok((path, text))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut resolvers = BTreeSet::new();
        if matches!(operation, syntax::Operation::ForegroundProfile) {
            for (_, text) in &sources {
                if text.contains("LoaderOverrides") {
                    resolvers.extend(syntax::profile_resolvers(text)?);
                }
            }
        }
        let mut selected = Vec::new();
        for (path, text) in sources {
            if !text.contains(operation.anchor()) {
                continue;
            }
            if let Some(updated) = operation
                .adapt(&text, &resolvers)
                .map_err(|error| format!("{}: {error}", path.display()))?
            {
                selected.push((path, updated));
            }
        }
        if selected.len() != 1 {
            return Err(format!(
                "{operation:?}: expected one production consumer, found {}",
                selected.len()
            ));
        }
        let (path, text) = selected.pop().unwrap();
        self.changes.insert(path, text);
        Ok(())
    }
}

fn plan(source: &Path, upstream_tui: bool) -> Result<BTreeMap<PathBuf, String>> {
    let mut plan = Plan {
        source,
        changes: BTreeMap::new(),
    };
    for operation in [
        syntax::Operation::ForegroundProfile,
        syntax::Operation::ProfileAdmission,
        syntax::Operation::Protoc,
        syntax::Operation::SocketRoot,
        syntax::Operation::ApprovalPreset,
    ] {
        plan.operation(operation)?;
    }
    if upstream_tui {
        for operation in [
            syntax::Operation::AutoReviewMode,
            syntax::Operation::ResumePermissions,
            syntax::Operation::PermissionShortcut,
            syntax::Operation::PermissionMatch,
            syntax::Operation::PermissionSelection,
            syntax::Operation::PermissionDescription,
            syntax::Operation::PermissionMenu,
            syntax::Operation::AutoReviewDescription,
        ] {
            plan.operation(operation)?;
        }
    }
    for text in plan.changes.values_mut() {
        let imported = format!("use codex_protocol::models::{WORKSPACE};\n");
        if text.matches(WORKSPACE).count() == 1 && text.contains(&imported) {
            *text = text.replacen(&imported, "", 1);
        }
    }
    Ok(plan.changes)
}

/// Preflight the entire adaptation before changing any file.
pub fn prepare(source: &Path, version: &str, revision: &str, upstream_tui: bool) -> Result<Value> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("expected exact official source commit".into());
    }
    if git(source, &["rev-parse", "HEAD"])? != format!("{revision}\n").as_bytes() {
        return Err("official source commit differs from release input".into());
    }
    if !git(source, &["status", "--porcelain"])?.is_empty() {
        return Err("runtime source preparation requires a pristine disposable checkout".into());
    }
    let manifest = toml(&read(&source.join("codex-rs/Cargo.toml"))?)?;
    if !super::valid_stable_version(version)
        || manifest
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(toml::Value::as_str)
            != Some(version)
    {
        return Err("official workspace version differs from release input".into());
    }
    let changes = plan(source, upstream_tui)?;
    let merged = patch::rebase(source, include_bytes!("native_route.patch"), &changes)?;
    patch::apply(source, &merged)?;
    Ok(json!({"version": version, "source": revision, "changed_files": merged.changed_files}))
}

fn manifests(root: &Path, directory: &Path, result: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if directory == root && entry.file_name() == "target" {
            continue;
        }
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            manifests(root, &path, result)?;
        } else if entry.file_name() == "Cargo.toml" {
            result.push(path);
        }
    }
    Ok(())
}

fn normalized(source: &Path, before: &str) -> Result<(String, usize, usize)> {
    let manifest = toml(&read(&source.join("Cargo.toml"))?)?;
    let workspace = manifest
        .get("workspace")
        .ok_or("missing source workspace")?;
    let version = workspace
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or("missing workspace release version")?;
    if !super::valid_stable_version(version) {
        return Err("unexpected upstream release version".into());
    }
    for member in workspace
        .get("members")
        .and_then(toml::Value::as_array)
        .ok_or("missing workspace members")?
    {
        let pattern = source.join(member.as_str().ok_or("invalid workspace member")?);
        let mut paths = glob::glob(pattern.to_str().ok_or("non-UTF-8 workspace pattern")?)
            .map_err(|error| error.to_string())?
            .peekable();
        if paths.peek().is_none() {
            return Err("missing workspace member".into());
        }
        for path in paths {
            read(&path.map_err(|error| error.to_string())?.join("Cargo.toml"))?;
        }
    }
    let mut paths = Vec::new();
    manifests(source, source, &mut paths)?;
    let mut names = BTreeSet::new();
    for path in paths {
        let value = toml(&read(&path)?)?;
        if value.get("package").and_then(|p| p.get("version"))
            == Some(&toml::Value::Table(toml::map::Map::from_iter([(
                "workspace".into(),
                toml::Value::Boolean(true),
            )])))
        {
            let name = value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(toml::Value::as_str)
                .ok_or("missing workspace package name")?;
            if !names.insert(name.to_owned()) {
                return Err("duplicate workspace package".into());
            }
        }
    }
    let old = toml(before)?;
    let packages = old
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("missing lock package inventory")?;
    let selected: BTreeSet<_> = packages
        .iter()
        .filter(|p| p.get("source").is_none())
        .filter_map(|p| p.get("name").and_then(toml::Value::as_str))
        .filter(|name| names.contains(*name))
        .map(str::to_owned)
        .collect();
    if selected != names {
        return Err("workspace lock inventory mismatch".into());
    }
    let mut expected = old.clone();
    for package in expected["package"].as_array_mut().unwrap() {
        if package.get("source").is_some() {
            continue;
        }
        if names.contains(
            package
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or("invalid lock package name")?,
        ) {
            if package.get("version").and_then(toml::Value::as_str) != Some("0.0.0") {
                return Err("unexpected workspace lock version".into());
            }
            package["version"] = toml::Value::String(version.into());
        }
        if let Some(dependencies) = package.get_mut("dependencies") {
            for dependency in dependencies
                .as_array_mut()
                .ok_or("invalid lock dependency array")?
            {
                let value = dependency.as_str().ok_or("invalid lock dependency")?;
                if let Some(name) = value
                    .strip_suffix(" 0.0.0")
                    .filter(|name| names.contains(*name))
                {
                    *dependency = toml::Value::String(format!("{name} {version}"));
                }
            }
        }
    }
    let mut after = String::new();
    let mut changed = 0;
    for (index, block) in before.split("[[package]]\n").enumerate() {
        if index == 0 {
            after.push_str(block);
            continue;
        }
        let mut block = format!("[[package]]\n{block}");
        let value = toml(&block)?;
        let package = &value["package"][0];
        if package.get("source").is_none() {
            if names.contains(package["name"].as_str().ok_or("invalid lock name")?) {
                block = replace(
                    &block,
                    "\nversion = \"0.0.0\"\n",
                    &format!("\nversion = \"{version}\"\n"),
                    "workspace lock version",
                )?;
                changed += 1;
            }
            for name in &names {
                block = block.replace(
                    &format!("\"{name} 0.0.0\""),
                    &format!("\"{name} {version}\""),
                );
            }
        }
        after.push_str(&block);
    }
    if toml(&after)? != expected || changed != names.len() {
        return Err("normalization changed unrelated lock data".into());
    }
    Ok((
        after,
        changed,
        packages
            .iter()
            .filter(|p| p.get("source").is_some())
            .count(),
    ))
}

pub fn normalize_lock(source: &Path, openssl: &Path) -> Result<Value> {
    if !super::canonical_absolute_path(openssl) {
        return Err("source audit requires an absolute OpenSSL executable".into());
    }
    super::ensure_executable(openssl, "OpenSSL is not an executable regular file")
        .map_err(|error| error.to_string())?;
    let path = source.join("Cargo.lock");
    let before = read(&path)?;
    let (after, changed, external) = normalized(source, &before)?;
    let original = super::openssl_sha256(openssl, &path).map_err(|error| error.to_string())?;
    fs::write(&path, after).map_err(|error| error.to_string())?;
    let normalized = super::openssl_sha256(openssl, &path).map_err(|error| error.to_string())?;
    Ok(
        json!({"original_lock_sha256": original, "normalized_lock_sha256": normalized,
        "workspace_versions_changed": changed, "external_packages_unchanged": external}),
    )
}

fn execute(args: &[OsString]) -> Result<()> {
    let action = args
        .first()
        .and_then(|a| a.to_str())
        .ok_or("missing source operation")?;
    let mut fields = BTreeMap::new();
    if !(args.len() - 1).is_multiple_of(2) {
        return Err("source operation requires --flag value pairs".into());
    }
    for pair in args[1..].as_chunks::<2>().0 {
        let flag = pair[0].to_str().ok_or("invalid source flag")?;
        if fields.insert(flag, &pair[1]).is_some() {
            return Err(format!("duplicate source flag: {flag}"));
        }
    }
    let upstream_tui = if action == "prepare-source" {
        match fields.remove("--frontend").and_then(|value| value.to_str()) {
            None | Some("upstream") => true,
            Some("provided") => false,
            Some(_) => return Err("source frontend must be upstream or provided".into()),
        }
    } else {
        true
    };
    let owned = if action == "apply-source-patch" {
        match (
            fields.remove("--owned-source"),
            fields.remove("--owned-target"),
        ) {
            (None, None) => None,
            (Some(directory), Some(target)) => Some((Path::new(directory), Path::new(target))),
            _ => return Err("owned sources require both --owned-source and --owned-target".into()),
        }
    } else {
        None
    };
    let expected: &[&str] = match action {
        "prepare-source" => &["--source", "--version", "--source-sha"],
        "normalize-lock" => &["--source", "--openssl", "--audit"],
        "patch-base" => &["--patch"],
        "apply-source-patch" => &["--source", "--patch"],
        _ => return Err("unsupported source operation".into()),
    };
    if fields.len() != expected.len() || expected.iter().any(|flag| !fields.contains_key(flag)) {
        return Err(format!("{action} requires {}", expected.join(" ")));
    }
    if action == "patch-base" {
        println!(
            "{}",
            patch::base(&patch::from_file(Path::new(fields["--patch"]))?)?
        );
        return Ok(());
    }
    let source = Path::new(fields["--source"]);
    if !source.is_absolute() || source.is_symlink() || !source.is_dir() {
        return Err("source must be an absolute real disposable checkout".into());
    }
    match action {
        "prepare-source" => println!(
            "{}",
            prepare(
                source,
                fields["--version"]
                    .to_str()
                    .ok_or("invalid source version")?,
                fields["--source-sha"]
                    .to_str()
                    .ok_or("invalid source revision")?,
                upstream_tui,
            )?
        ),
        "apply-source-patch" => {
            let owned = match owned {
                Some((directory, target)) => patch::owned_sources(source, directory, target)?,
                None => BTreeMap::new(),
            };
            let rebased = patch::rebase_with_owned(
                source,
                &patch::from_file(Path::new(fields["--patch"]))?,
                &BTreeMap::new(),
                &owned,
            )?;
            patch::apply(source, &rebased)?;
        }
        "normalize-lock" => {
            let audit = Path::new(fields["--audit"]);
            if !audit.is_absolute() {
                return Err("source audit requires an absent absolute output".into());
            }
            let mut audit = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(audit)
                .map_err(|error| format!("reserve source audit output: {error}"))?;
            let report = normalize_lock(source, Path::new(fields["--openssl"]))?;
            writeln!(audit, "{report}").map_err(|error| error.to_string())?;
            println!("{report}");
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) fn command(args: &[OsString]) -> i32 {
    match execute(args) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("codex-release-builder: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_fixture(inputs: &BTreeMap<String, String>) -> PathBuf {
        let root = fixture();
        for (relative, text) in inputs {
            let path = root.join("codex-rs").join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        for (directory, name) in [
            ("cli", "codex-cli"),
            ("code-mode-protocol", "codex-code-mode-protocol"),
            ("uds", "codex-uds"),
            ("utils/approval-presets", "codex-utils-approval-presets"),
            ("tui", "codex-tui"),
        ] {
            let directory = root.join("codex-rs").join(directory);
            fs::create_dir_all(directory.join("src")).unwrap();
            fs::write(
                directory.join("Cargo.toml"),
                format!("[package]\nname={name:?}\n"),
            )
            .unwrap();
        }
        root
    }

    #[test]
    fn native_and_provided_frontends_keep_their_permission_contracts() {
        // Unmodified upstream function inputs exercise the real preparation plan.
        let inputs: BTreeMap<String, String> =
            serde_json::from_str(include_str!("fixtures/source-plan.json")).unwrap();
        let root = source_fixture(&inputs);
        let provided = plan(&root, false).unwrap();
        assert!(provided
            .keys()
            .all(|path| !path.starts_with(root.join("codex-rs/tui"))));
        let native = plan(&root, true).unwrap();
        for (path, text) in &provided {
            assert_eq!(native.get(path), Some(text));
        }
        let popup = &native[&root.join("codex-rs/tui/src/chatwidget/permission_popups.rs")];
        assert!(!popup.contains("can_write_local_path_with_cwd"));
        assert!(popup.contains(
            "\"auto\" => { matches!(current_permission_profile, PermissionProfile::Disabled) }"
        ));
        let menu = &native[&root.join("codex-rs/tui/src/chatwidget/permissions_menu.rs")];
        assert!(!menu.contains("let Some(read_only)"));
        assert!(!menu.contains("\":workspace\""));
        assert!(menu.contains(FULL_ACCESS));
        for (relative, text) in &inputs {
            assert_eq!(read(&root.join("codex-rs").join(relative)).unwrap(), *text);
        }
        git(&root, &["init", "-q"]).unwrap();
        git(&root, &["add", "."]).unwrap();
        git(
            &root,
            &[
                "-c",
                "user.name=Source proof",
                "-c",
                "user.email=proof@localhost",
                "commit",
                "-qm",
                "Official inputs",
            ],
        )
        .unwrap();
        for (path, text) in native {
            fs::write(path, text).unwrap();
        }
        git(&root, &["diff", "--check"]).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_operations_follow_private_refactors_and_package_file_moves() {
        use quote::ToTokens;
        let current: BTreeMap<String, String> =
            serde_json::from_str(include_str!("fixtures/source-plan.json")).unwrap();
        let previous: BTreeMap<String, String> =
            serde_json::from_str(include_str!("fixtures/source-plan-166.json")).unwrap();
        for mode in ["current", "previous", "refactored"] {
            let mut inputs = current.clone();
            if mode == "previous" {
                inputs.extend(previous.clone());
            }
            if mode == "refactored" {
                for text in inputs.values_mut() {
                    *text = text
                        .replace("dispatch_cli", "dispatch_foreground")
                        .replace("loader_overrides_for_profile", "resolve_loader")
                        .replace("interactive", "runtime_args")
                        .replace("{uid}", "{effective_uid}")
                        .replace("let uid =", "let effective_uid =");
                    for name in [
                        "auto_review_mode",
                        "thread_resume_params_from_config",
                        "handle_permission_shortcut",
                        "preset_matches_current",
                        "try_select_permission_profile",
                        "permission_preset_description",
                        "open_permission_profiles_popup",
                    ] {
                        *text = text.replace(name, &format!("relocated_{name}"));
                    }
                    *text = syn::parse_file(text)
                        .unwrap()
                        .into_token_stream()
                        .to_string();
                }
            }
            let root = source_fixture(&inputs);
            if mode == "refactored" {
                for relative in [
                    "cli/src/main.rs",
                    "uds/src/daemon_directory.rs",
                    "utils/approval-presets/src/lib.rs",
                ] {
                    let path = root.join("codex-rs").join(relative);
                    fs::rename(&path, path.with_file_name("relocated.rs")).unwrap();
                }
            }
            if mode == "refactored" {
                let cli = root.join("codex-rs/cli/src/relocated.rs");
                let text = read(&cli).unwrap();
                let start = text.find("fn resolve_loader (").unwrap();
                fs::write(cli.with_file_name("profile.rs"), &text[start..]).unwrap();
                fs::write(&cli, &text[..start]).unwrap();
            }
            if mode == "refactored" {
                for relative in inputs.keys().filter(|path| path.starts_with("tui/")) {
                    let path = root.join("codex-rs").join(relative);
                    let moved = path.with_file_name(format!(
                        "moved_{}",
                        path.file_name().unwrap().to_str().unwrap()
                    ));
                    fs::rename(path, moved).unwrap();
                }
            }
            let native = plan(&root, true).unwrap();
            assert!(native.len() > 4, "{mode}");
            let result = plan(&root, false).unwrap();
            assert_eq!(result.len(), 4, "{mode}");
            let all = result.values().cloned().collect::<Vec<_>>().join("\n");
            assert!(all.contains("/proc/self/fd/35"), "{mode}");
            assert!(all.contains("cargo:rerun-if-env-changed=PROTOC"), "{mode}");
            assert!(all.contains(DESCRIPTION), "{mode}");
            let cli = result
                .iter()
                .find(|(path, _)| path.starts_with(root.join("codex-rs/cli")))
                .unwrap()
                .1;
            let compact: String = cli.chars().filter(|c| !c.is_whitespace()).collect();
            let resolver = if mode == "refactored" {
                "resolve_loader(runtime_args.config_profile_v2.as_ref())?"
            } else {
                "loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?"
            };
            assert!(
                compact.contains(&format!("root_config_overrides,{resolver},strict_config")),
                "{mode}"
            );
            assert!(compact.contains("Subcommand::AppServer(server)ifserver.subcommand.is_none()=>Ok(Some(profile_v2))"), "{mode}");
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn changed_contract_and_extra_deleted_binding_consumer_preserve_sources() {
        let baseline: BTreeMap<String, String> =
            serde_json::from_str(include_str!("fixtures/source-plan.json")).unwrap();
        for fault in ["loader", "read-only-consumer"] {
            let mut inputs = baseline.clone();
            if fault == "loader" {
                let cli = inputs.get_mut("cli/src/main.rs").unwrap();
                *cli = cli.replace(
                    "LoaderOverrides::default()",
                    "LoaderOverrides::constrained()",
                );
            } else {
                let menu = inputs
                    .get_mut("tui/src/chatwidget/permissions_menu.rs")
                    .unwrap();
                *menu = menu.replace(
                    "let mut items = vec!",
                    "inspect_extra_consumer(read_only); let mut items = vec!",
                );
            }
            let root = source_fixture(&inputs);
            let error = plan(&root, true).unwrap_err();
            assert!(
                error.contains(if fault == "loader" {
                    "loader contract changed"
                } else {
                    "additional consumers"
                }),
                "{error}"
            );
            for (name, original) in &inputs {
                assert_eq!(read(&root.join("codex-rs").join(name)).unwrap(), *original);
            }
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn test_copies_and_other_transport_callers_are_not_foreground_consumers() {
        let mut inputs: BTreeMap<String, String> =
            serde_json::from_str(include_str!("fixtures/source-plan.json")).unwrap();
        let copy = inputs["cli/src/main.rs"].clone();
        inputs
            .get_mut("cli/src/main.rs")
            .unwrap()
            .push_str(&format!("\n#[cfg(test)] mod fixture_copy {{ {copy} }}"));
        let root = source_fixture(&inputs);
        fs::write(root.join("codex-rs/cli/src/remote.rs"), "fn remote_control() { codex_app_server::run_main_with_transport_options(a, b, LoaderOverrides::default()); }").unwrap();
        let changes = plan(&root, false).unwrap();
        assert_eq!(changes.len(), 4);
        assert!(changes[&root.join("codex-rs/cli/src/main.rs")].contains(&copy));
        fs::remove_dir_all(root).unwrap();
    }

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "source-lock-{}-{}",
            std::process::id(),
            super::super::STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("member")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers=[\"member\"]\n[workspace.package]\nversion=\"0.163.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("member/Cargo.toml"),
            "[package]\nname=\"codex-sample\"\nversion.workspace=true\n",
        )
        .unwrap();
        fs::write(root.join("Cargo.lock"), format!("version = 4\n\n[[package]]\nname = \"codex-sample\"\nversion = \"0.0.0\"\ndependencies = [\"external\"]\n\n[[package]]\nname = \"external\"\nversion = \"1.2.3\"\nsource = \"registry+https://example.invalid/index\"\nchecksum = \"{}\"\n", "a".repeat(64))).unwrap();
        root
    }

    #[test]
    fn normalization_retains_external_bytes_and_includes_nonmember_path_packages() {
        let root = fixture();
        fs::create_dir(root.join("path-dependency")).unwrap();
        fs::write(
            root.join("path-dependency/Cargo.toml"),
            "[package]\nname=\"codex-path\"\nversion.workspace=true\n",
        )
        .unwrap();
        let before = read(&root.join("Cargo.lock"))
            .unwrap()
            .replace("[\"external\"]", "[\"external\", \"codex-path 0.0.0\"]")
            + "\n[[package]]\nname = \"codex-path\"\nversion = \"0.0.0\"\n";
        let (after, changed, external) = normalized(&root, &before).unwrap();
        assert_eq!(
            after,
            before
                .replace("version = \"0.0.0\"", "version = \"0.163.0\"")
                .replace("\"codex-path 0.0.0\"", "\"codex-path 0.163.0\"")
        );
        assert_eq!((changed, external), (2, 1));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_release_member_inventory_or_version_does_not_mutate_lock() {
        for fault in ["release", "member", "inventory", "version"] {
            let root = fixture();
            match fault {
                "release" => fs::write(
                    root.join("Cargo.toml"),
                    read(&root.join("Cargo.toml"))
                        .unwrap()
                        .replace("0.163.0", "0.165.0-preview"),
                )
                .unwrap(),
                "member" => fs::remove_file(root.join("member/Cargo.toml")).unwrap(),
                "inventory" => fs::write(
                    root.join("Cargo.lock"),
                    read(&root.join("Cargo.lock"))
                        .unwrap()
                        .replace("codex-sample", "other"),
                )
                .unwrap(),
                "version" => fs::write(
                    root.join("Cargo.lock"),
                    read(&root.join("Cargo.lock"))
                        .unwrap()
                        .replace("0.0.0", "0.163.0"),
                )
                .unwrap(),
                _ => unreachable!(),
            }
            let before = read(&root.join("Cargo.lock")).unwrap();
            assert!(normalized(&root, &before).is_err(), "{fault}");
            assert_eq!(read(&root.join("Cargo.lock")).unwrap(), before);
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn new_stable_versions_need_no_registration() {
        let root = fixture();
        fs::write(
            root.join("Cargo.toml"),
            read(&root.join("Cargo.toml"))
                .unwrap()
                .replace("0.163.0", "0.999.0"),
        )
        .unwrap();
        let before = read(&root.join("Cargo.lock")).unwrap();
        assert_eq!(
            normalized(&root, &before).unwrap().0,
            before.replace("version = \"0.0.0\"", "version = \"0.999.0\"")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn occupied_audit_output_is_preserved_before_lock_normalization() {
        let root = fixture();
        let audit = root.join("audit.json");
        fs::write(&audit, "existing user output").unwrap();
        let before = read(&root.join("Cargo.lock")).unwrap();
        let openssl = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"))
            .join("bin/openssl");
        let args = vec![
            OsString::from("normalize-lock"),
            OsString::from("--source"),
            root.as_os_str().to_owned(),
            OsString::from("--openssl"),
            openssl.into_os_string(),
            OsString::from("--audit"),
            audit.as_os_str().to_owned(),
        ];
        let error = execute(&args).unwrap_err();
        assert!(error.contains("reserve source audit output"), "{error}");
        assert_eq!(read(&audit).unwrap(), "existing user output");
        assert_eq!(read(&root.join("Cargo.lock")).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }
}
