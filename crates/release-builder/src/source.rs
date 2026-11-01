//! Common source preparation for device builds and remote release executors.
//! Work happens only in an explicitly supplied disposable official checkout;
//! applying a patch does not grant execution or publication authority.

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
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

fn replace(text: &str, old: &str, new: &str, where_: &str) -> Result<String> {
    let count = text.matches(old).count();
    if count != 1 {
        return Err(format!("{where_}: expected one {old:?}, found {count}"));
    }
    Ok(text.replacen(old, new, 1))
}

fn function(
    text: &str,
    name: &str,
    where_: &str,
    transform: impl FnOnce(&str, &str) -> Result<String>,
) -> Result<String> {
    // Official rustfmt output ends the function at its declaration indentation.
    // Match the declaration, never a release-specific line or byte offset.
    let mut declarations = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len() - trimmed.len();
        let mut declaration = trimmed;
        if let Some(tail) = declaration.strip_prefix("pub(") {
            declaration = tail.split_once(") ").map_or("", |(_, tail)| tail);
        } else if let Some(tail) = declaration.strip_prefix("pub ") {
            declaration = tail;
        }
        declaration = declaration.strip_prefix("async ").unwrap_or(declaration);
        if let Some(tail) = declaration
            .strip_prefix("fn ")
            .and_then(|t| t.strip_prefix(name))
        {
            if !tail.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
                declarations.push((offset, indent, line.len()));
            }
        }
        offset += line.len();
    }
    let location = format!("{where_}::{name}");
    if declarations.len() != 1 {
        return Err(format!(
            "{location}: expected one function, found {}",
            declarations.len()
        ));
    }
    let (start, indent, length) = declarations[0];
    let closing = format!("{}}}", " ".repeat(indent));
    let mut stop = start + length;
    for line in text[stop..].split_inclusive('\n') {
        stop += line.len();
        if line.strip_suffix('\n').unwrap_or(line) == closing {
            let updated = transform(&text[start..stop], &location)?;
            return Ok(format!("{}{}{}", &text[..start], updated, &text[stop..]));
        }
    }
    Err(format!("{location}: cannot locate closing brace"))
}

fn replacements(text: &str, pairs: &[(&str, &str)], where_: &str) -> Result<String> {
    let mut text = text.to_owned();
    for (old, new) in pairs {
        text = replace(&text, old, new, where_)?;
    }
    Ok(text)
}

struct Plan<'a> {
    source: &'a Path,
    changes: BTreeMap<PathBuf, String>,
}

impl Plan<'_> {
    fn edit(
        &mut self,
        path: &str,
        name: &str,
        transform: impl FnOnce(&str, &str) -> Result<String>,
    ) -> Result<()> {
        let path = self.source.join("codex-rs").join(path);
        let text = match self.changes.get(&path) {
            Some(text) => text.clone(),
            None => read(&path)?,
        };
        let changed = function(&text, name, &path.display().to_string(), transform)?;
        self.changes.insert(path, changed);
        Ok(())
    }
}

fn plan(source: &Path) -> Result<BTreeMap<PathBuf, String>> {
    let mut plan = Plan {
        source,
        changes: BTreeMap::new(),
    };
    plan.edit("cli/src/main.rs", "cli_main", |text, where_| {
        replacements(text, &[
            ("Some(Subcommand::AppServer(app_server_cli)) => {", "Some(Subcommand::AppServer(app_server_cli)) => {\n            let loader_overrides = loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?;"),
            ("root_config_overrides,\n                    LoaderOverrides::default(),\n                    strict_config,\n                    startup_config,", "root_config_overrides,\n                    loader_overrides,\n                    strict_config,\n                    startup_config,"),
            ("root_config_overrides,\n                        LoaderOverrides::default(),\n                        strict_config,\n                        analytics_default_enabled,", "root_config_overrides,\n                        loader_overrides,\n                        strict_config,\n                        analytics_default_enabled,"),
        ], where_)
    })?;
    plan.edit("cli/src/main.rs", "profile_v2_for_subcommand", |text, where_| {
        let text = replace(text, "    match subcommand {", "    match subcommand {\n        Subcommand::AppServer(server) if server.subcommand.is_none() => Ok(Some(profile_v2)),", where_)?;
        replace(&text, "and `codex debug prompt-input`.\"", "`codex debug prompt-input`, and foreground `codex app-server`.\"", where_)
    })?;
    plan.edit("code-mode-protocol/build.rs", "main", |text, where_| {
        replace(
            text,
            "    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);",
            "    config.protoc_executable(match std::env::var_os(\"PROTOC\") {\n        Some(path) => PathBuf::from(path),\n        None => protoc_bin_vendored::protoc_bin_path()?,\n    });\n    println!(\"cargo:rerun-if-env-changed=PROTOC\");",
            where_,
        )
    })?;
    plan.edit(
        "uds/src/daemon_directory.rs",
        "shared_daemon_socket_directory",
        |text, where_| {
            replacements(
                text,
                &[
                    (
                        "fs::canonicalize(\"/tmp\")?",
                        "fs::canonicalize(\"/proc/self/fd/35\")?",
                    ),
                    ("format!(\"codex-daemon-{uid}\")", "format!(\"{uid}\")"),
                ],
                where_,
            )
        },
    )?;
    plan.edit("utils/approval-presets/src/lib.rs", "builtin_approval_presets", |text, where_| {
        replacements(text, &[
            (WORKSPACE, FULL_ACCESS),
            ("PermissionProfile::workspace_write()", "PermissionProfile::Disabled"),
                    ("Codex can read and edit files in the current workspace, and run commands. Approval is required to access the internet or edit other files. (Identical to Agent mode)", DESCRIPTION),
        ], where_)
    })?;
    plan.edit("tui/src/app.rs", "auto_review_mode", |text, where_| {
        replace(
            text,
            WORKSPACE,
            &format!("codex_protocol::models::{FULL_ACCESS}"),
            where_,
        )
    })?;
    plan.edit(
        "tui/src/app_server_session.rs",
        "thread_resume_params_from_config",
        |text, where_| {
            replace(
                text,
                "thread_params_mode == ThreadParamsMode::Remote || !permission_overrides.profile",
                "thread_params_mode == ThreadParamsMode::Remote",
                where_,
            )
        },
    )?;
    plan.edit(
        "tui/src/chatwidget/permission_shortcuts.rs",
        "handle_permission_shortcut",
        |text, where_| {
            replace(
                text,
                "!matches!(preset.id, \"read-only\" | \"auto\")",
                "preset.id != \"auto\"",
                where_,
            )
        },
    )?;
    plan.edit(
        "tui/src/chatwidget/permission_popups.rs",
        "preset_matches_current",
        |text, where_| {
            replace(
                text,
                r#""auto" => {
                let file_system_policy = current_permission_profile.file_system_sandbox_policy();
                matches!(
                    current_permission_profile,
                    PermissionProfile::Managed { .. }
                ) && file_system_policy.can_write_local_path_with_cwd(cwd, cwd)
                    && !file_system_policy.has_full_disk_write_access()
            }"#,
                "\"auto\" => matches!(current_permission_profile, PermissionProfile::Disabled),",
                where_,
            )
        },
    )?;
    plan.edit(
        "tui/src/chatwidget/permissions_menu.rs",
        "try_select_permission_profile",
        |text, where_| {
            replace(
                text,
                &format!("\":ask-for-approval\" | \":approve-for-me\" => {WORKSPACE}"),
                &format!("\":ask-for-approval\" | \":approve-for-me\" => {FULL_ACCESS}"),
                where_,
            )
        },
    )?;
    plan.edit("tui/src/chatwidget/permissions_menu.rs", "permission_preset_description", |text, where_| {
        replace(text, "Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace", DESCRIPTION, where_)
    })?;
    plan.edit("tui/src/chatwidget/permissions_menu.rs", "open_permission_profiles_popup", |text, where_| {
        let binding = r#"        let Some(read_only) = presets.iter().find(|preset| preset.id == "read-only") else {
            self.add_error_message(
                "Internal error: missing the 'read-only' approval preset.".to_string(),
            );
            return;
        };
"#;
        let item = r#"        items.push(self.builtin_permission_mode_selection_item(
            &discovery,
            read_only,
            ":read-only",
            permission_preset_description(read_only).to_string(),
            AskForApproval::from(read_only.approval),
            ApprovalsReviewer::User,
        ));
"#;
        let text = replace(text, binding, "", where_)?;
        let text = replace(&text, item, "", where_)?;
        if text.matches("\":workspace\"").count() != 2 {
            return Err(format!("{where_}: expected Ask and Auto workspace selections"));
        }
        Ok(text.replace("\":workspace\"", FULL_ACCESS))
    })?;
    let widget = source.join("codex-rs/tui/src/chatwidget.rs");
    plan.changes.insert(widget.clone(), replace(&read(&widget)?,
        "const AUTO_REVIEW_DESCRIPTION: &str = \"Only ask for actions detected as potentially unsafe\";",
        &format!("const AUTO_REVIEW_DESCRIPTION: &str = \"{REVIEW_DESCRIPTION}\";"),
        "tui/src/chatwidget.rs::AUTO_REVIEW_DESCRIPTION")?);
    for text in plan.changes.values_mut() {
        let imported = format!("use codex_protocol::models::{WORKSPACE};\n");
        if text.matches(WORKSPACE).count() == 1 && text.contains(&imported) {
            *text = text.replacen(&imported, "", 1);
        }
    }
    Ok(plan.changes)
}

/// Preflight the entire adaptation before changing any file.
pub fn prepare(source: &Path, version: &str, revision: &str) -> Result<Value> {
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
    let changes = plan(source)?;
    let files: Vec<_> = changes
        .keys()
        .map(|path| path.strip_prefix(source).unwrap())
        .collect();
    for (path, text) in &changes {
        fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(json!({"version": version, "source": revision, "changed_files": files}))
}

pub fn preserve_times(source: &Path) -> Result<()> {
    let epoch = String::from_utf8(git(source, &["show", "-s", "--format=%ct", "HEAD"])?)
        .map_err(|error| error.to_string())?
        .trim()
        .parse::<libc::time_t>()
        .map_err(|error| error.to_string())?;
    let changed = git(source, &["diff", "--name-only", "-z", "HEAD"])?;
    let changed: BTreeSet<_> = changed.split(|b| *b == 0).collect();
    let files = git(source, &["ls-files", "-z", "--", "codex-rs"])?;
    let times = [libc::timespec {
        tv_sec: epoch,
        tv_nsec: 0,
    }; 2];
    for file in files
        .split(|b| *b == 0)
        .filter(|file| !file.is_empty() && !changed.contains(file))
    {
        let path = source.join(std::ffi::OsStr::from_bytes(file));
        let path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|error| error.to_string())?;
        // Preserve symlink timestamps rather than following them outside the checkout.
        if unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path.as_ptr(),
                times.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(format!(
                "preserve pristine source time: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
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
    let expected: &[&str] = match action {
        "prepare-source" => &["--source", "--version", "--source-sha"],
        "normalize-lock" => &["--source", "--openssl", "--audit"],
        "preserve-source-times" => &["--source"],
        _ => return Err("unsupported source operation".into()),
    };
    if fields.len() != expected.len() || expected.iter().any(|flag| !fields.contains_key(flag)) {
        return Err(format!("{action} requires {}", expected.join(" ")));
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
                    .ok_or("invalid source revision")?
            )?
        ),
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
        "preserve-source-times" => preserve_times(source)?,
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

    #[test]
    fn relocation_only_changes_the_selected_function() {
        let source = "fn earlier() {\n    old_expression();\n}\n\npub(crate) async fn selected(\n) {\n    old_expression();\n}\n\nfn later() {\n    old_expression();\n}\n";
        let changed = function(source, "selected", "owned.rs", |text, where_| {
            replace(text, "old_expression()", "new_expression()", where_)
        })
        .unwrap();
        assert_eq!(
            changed,
            source.replacen(
                "    old_expression();\n}\n\nfn later",
                "    new_expression();\n}\n\nfn later",
                1
            )
        );
    }

    #[test]
    fn ambiguous_or_changed_source_names_the_responsible_function() {
        for source in [
            "fn other() {\n}\n",
            "fn selected() {\n}\nfn selected() {\n}\n",
            "fn selected() {\n    changed_expression();\n}\n",
        ] {
            let error = function(source, "selected", "owned.rs", |text, where_| {
                replace(text, "old_expression()", "new_expression()", where_)
            })
            .unwrap_err();
            assert!(error.contains("owned.rs::selected"), "{error}");
        }
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
