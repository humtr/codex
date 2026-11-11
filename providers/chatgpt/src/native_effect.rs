//! Resource claims for Native tools offered through the Provider MCP broker.
//!
//! This guards the broker's *offer-to-result* interval, not a Native work's
//! lifetime. In-process upstream/CLI tools and unmanaged writers do not pass
//! this boundary and cannot be represented as protected by these claims.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

const SNAPSHOT_LIMIT: u64 = 4 * 1024 * 1024;

#[path = "native_compiler.rs"]
mod compiler;
#[cfg(test)]
pub(crate) use compiler::fixture as compiler_fixture;
#[cfg(test)]
pub(crate) use compiler::hook_fixture;
pub(crate) use compiler::Execution;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Effect {
    /// In-process metadata/read tools with no shell or PATH dependency.
    Read,
    Opaque,
    Paths(Vec<FileEffect>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileEffect {
    path: PathBuf,
    entry: String,
    inode: Option<String>,
    base: Option<String>,
    anchors: Vec<String>,
    operation: FileOperation,
    #[serde(default)]
    linked: bool,
    #[serde(default)]
    external_input: bool,
    // Extensible file records remain readable by previous Providers. Only the
    // validated classifier below sets this proof; absent/unknown proof stays
    // readable for original-result settlement but is not a bounded claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope_schema: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum FileOperation {
    Update,
    Add,
    Delete,
    Input,
    Output,
    MetadataInput,
    AbsentInput,
    AbsentDirectory,
}

impl FileOperation {
    fn is_input(self) -> bool {
        matches!(
            self,
            Self::Input | Self::MetadataInput | Self::AbsentInput | Self::AbsentDirectory
        )
    }
}

fn version(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn snapshot(
    root: &Path,
    name: &str,
    op: FileOperation,
    anchors: Vec<String>,
) -> Option<FileEffect> {
    let path_arg = Path::new(name);
    let relative = if path_arg.is_absolute() {
        path_arg.strip_prefix(root).ok()?
    } else {
        path_arg
    };
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(relative);
    let parent = fs::canonicalize(path.parent()?).ok()?;
    if !parent.starts_with(root) {
        return None;
    }
    let parent_meta = fs::metadata(&parent).ok()?;
    if !parent_meta.is_dir() {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    let entry = format!("{}:{}:{}", parent_meta.dev(), parent_meta.ino(), name);
    let (inode, base, linked) = match fs::canonicalize(&path) {
        Ok(real) => {
            if !real.starts_with(root) {
                return None;
            }
            let meta = fs::metadata(&real).ok()?;
            if !meta.is_file() || meta.len() > SNAPSHOT_LIMIT {
                return None;
            }
            let bytes = fs::read(&real).ok()?;
            if bytes.len() as u64 > SNAPSHOT_LIMIT {
                return None;
            }
            if op == FileOperation::Add {
                // The upstream patch implementation's Add must never be used
                // as a blind replacement of a pre-existing path.
                return None;
            }
            (
                Some(format!("{}:{}", meta.dev(), meta.ino())),
                Some(version(&bytes)),
                fs::symlink_metadata(&path).ok()?.file_type().is_symlink(),
            )
        }
        Err(_) => {
            if op != FileOperation::Add || fs::symlink_metadata(&path).is_ok() {
                return None;
            }
            (None, None, false)
        }
    };
    Some(FileEffect {
        path,
        entry,
        inode,
        base,
        anchors,
        operation: op,
        linked,
        external_input: false,
        scope_schema: None,
    })
}

fn collect_anchor(hunk: &[&str]) -> Option<String> {
    let old = hunk
        .iter()
        .filter_map(|line| line.strip_prefix(' ').or_else(|| line.strip_prefix('-')))
        .collect::<Vec<_>>();
    (!old.is_empty()).then(|| format!("{}\n", old.join("\n")))
}

fn parse_patch(input: &str, workspace: &Path) -> Option<Effect> {
    let mut lines = input.lines().peekable();
    if lines.next()? != "*** Begin Patch" {
        return None;
    }
    let mut edits = Vec::new();
    let mut terminated = false;
    while let Some(header) = lines.next() {
        if header == "*** End Patch" {
            terminated = lines.next().is_none();
            break;
        }
        let (name, operation) = if let Some(name) = header.strip_prefix("*** Update File: ") {
            (name, FileOperation::Update)
        } else if let Some(name) = header.strip_prefix("*** Add File: ") {
            (name, FileOperation::Add)
        } else {
            (
                header.strip_prefix("*** Delete File: ")?,
                FileOperation::Delete,
            )
        };
        let mut anchors = Vec::new();
        let mut hunk = Vec::new();
        let mut destination = None;
        let mut has_hunk = false;
        while let Some(&line) = lines.peek() {
            if line.starts_with("*** ") {
                if line.starts_with("*** Move to: ") && operation == FileOperation::Update {
                    destination = Some(lines.next()?.strip_prefix("*** Move to: ")?);
                    continue;
                }
                if line == "*** End of File" {
                    lines.next();
                    continue;
                }
                break;
            }
            let line = lines.next()?;
            if line.starts_with("@@") {
                if !hunk.is_empty() {
                    anchors.push(collect_anchor(&hunk)?);
                    hunk.clear();
                }
                has_hunk = true;
            } else if operation == FileOperation::Update {
                if !has_hunk || !matches!(line.as_bytes().first(), Some(b' ' | b'+' | b'-')) {
                    return None;
                }
                hunk.push(line);
            } else if (operation == FileOperation::Add && !line.starts_with('+'))
                || operation == FileOperation::Delete
            {
                return None;
            }
        }
        if !hunk.is_empty() {
            anchors.push(collect_anchor(&hunk)?);
        }
        if operation == FileOperation::Update && (!has_hunk || anchors.is_empty()) {
            return None;
        }
        let old = snapshot(workspace, name, operation, anchors)?;
        edits.push(old);
        if let Some(dest) = destination {
            edits.push(snapshot(workspace, dest, FileOperation::Add, Vec::new())?);
        }
    }
    (terminated && !edits.is_empty()).then_some(Effect::Paths(edits))
}

/// Candidate command shapes only. Authoritative admission additionally pins
/// the same Native execution dependencies used by bounded compiler writes.
fn is_simple_read_command(input: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input) else {
        return false;
    };
    let Some(arguments) = value.as_object() else {
        return false;
    };
    // Unknown executor parameters can run additional code even for an innocent
    // cmd. A supplied shell must also match the selected Native shell below.
    if arguments.keys().any(|key| {
        !matches!(
            key.as_str(),
            "cmd" | "workdir" | "yield_time_ms" | "max_output_tokens" | "login" | "shell"
        )
    }) {
        return false;
    }
    // The executor may start a login shell by default. Explicit non-login is
    // necessary but not sufficient: BASH_ENV and PATH are checked separately.
    if value.get("login").and_then(serde_json::Value::as_bool) != Some(false) {
        return false;
    }
    let Some(command) = value.get("cmd").and_then(|v| v.as_str()) else {
        return false;
    };
    if matches!(
        command.trim(),
        "pwd"
            | "git rev-parse HEAD"
            | "git rev-parse --show-toplevel"
            | "git branch --show-current"
            | "git --no-pager -c log.showSignature=false log -1 --oneline"
            | "ls"
            | "ls -la"
            | "ls -l"
            | "ls -1"
    ) {
        return true;
    }
    // A single explicit, inert path operand is a real read even when a
    // bounded write to an unrelated resource is still running. No wildcard,
    // flag, command substitution, redirection or shell control is admitted.
    let Some(path) = command.trim().strip_prefix("cat ") else {
        return false;
    };
    !path.is_empty()
        && !path.starts_with('-')
        && !path.starts_with('/')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-'))
        && !path.split('/').any(|part| part == ".." || part.is_empty())
}

/// Empty stdin only observes an existing process. Unknown input fields, input
/// bytes and missing session identity cannot inherit an earlier resource proof.
pub(crate) fn empty_session_poll(tool: &str, input: &str) -> Option<u64> {
    if tool != "functions.write_stdin" {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    let fields = value.as_object()?;
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "session_id" | "chars" | "yield_time_ms" | "max_output_tokens"
        )
    }) || fields
        .get("chars")
        .is_some_and(|chars| chars.as_str() != Some(""))
        || ["yield_time_ms", "max_output_tokens"].iter().any(|key| {
            fields
                .get(*key)
                .is_some_and(|value| value.as_u64().is_none())
        })
    {
        return None;
    }
    fields
        .get("session_id")?
        .as_u64()
        .filter(|session| *session > 0)
}

pub(crate) fn qualified_session_poll(
    tool: &str,
    input: &str,
    workspace: Option<&Path>,
    execution: Option<&Execution>,
) -> Option<u64> {
    let (root, facts) = workspace.zip(execution)?;
    // Native schema 2's exec proof includes write_stdin's tool and permission
    // hook aliases. A later turn must supply its own current proof as well.
    facts
        .permits_bounded_tool("exec_command", root)
        .then(|| empty_session_poll(tool, input))
        .flatten()
}

pub(crate) fn classify(tool: &str, input: &str, workspace: Option<&Path>) -> Effect {
    classify_with_execution(tool, input, workspace, None)
}

/// Authoritative bounded claims require the actual Native step and hook proof.
/// The Provider's own process environment is never an execution attestation.
pub(crate) fn classify_with_execution(
    tool: &str,
    input: &str,
    workspace: Option<&Path>,
    execution: Option<&Execution>,
) -> Effect {
    // Native tool_search has a dedicated payload with no tool hooks. Ordinary
    // Function/Custom/MCP calls do run hooks, even when their main operation is
    // a read or a bounded patch. Missing proof must retain the opaque claim.
    if tool == "tool_search" {
        return Effect::Read;
    }
    let bounded_tool = match tool {
        "functions.exec_command" => "exec_command",
        "functions.apply_patch" | "apply_patch" => "apply_patch",
        "functions.view_image" => "view_image",
        "clock.curr_time" => "clock.curr_time",
        "clock.sleep" => "clock.sleep",
        _ => return Effect::Opaque,
    };
    if !workspace
        .zip(execution)
        .is_some_and(|(root, facts)| facts.permits_bounded_tool(bounded_tool, root))
    {
        return Effect::Opaque;
    }
    let mut effect = match tool {
        "clock.curr_time" | "clock.sleep" | "functions.view_image" => Effect::Read,
        "functions.exec_command" if is_simple_read_command(input) => workspace
            .and_then(|root| compiler::qualified_read(input, root, execution?))
            .unwrap_or(Effect::Opaque),
        "functions.exec_command" => workspace
            .and_then(|root| compiler::classify(input, root, execution?))
            .unwrap_or(Effect::Opaque),
        "functions.apply_patch" | "apply_patch" => workspace
            .and_then(|root| parse_patch(input, root))
            .unwrap_or(Effect::Opaque),
        _ => Effect::Opaque,
    };
    if let Effect::Paths(files) = &mut effect {
        for file in files {
            file.scope_schema = Some(2);
        }
    }
    effect
}

impl Effect {
    /// Keep original persisted records intact; interpret older/unknown scope
    /// conservatively at admission rather than migrating a possibly live effect.
    pub(crate) fn admission_scope(&self) -> &Self {
        match self {
            Self::Paths(files)
                if files.is_empty() || files.iter().any(|file| file.scope_schema != Some(2)) =>
            {
                &Self::Opaque
            }
            _ => self,
        }
    }

    pub(crate) fn claim_scope(&self) -> &Self {
        // Pure reads never reserve in either claim ledger. A historical Read
        // in a claim cannot establish which tool/hook boundary produced it.
        match self {
            Self::Read => &Self::Opaque,
            _ => self.admission_scope(),
        }
    }
}

#[cfg(test)]
pub(crate) fn classify_with_fixture(tool: &str, input: &str, workspace: Option<&Path>) -> Effect {
    // Resource-shape unit tests supply an explicit hook-free Native fixture;
    // production admission never constructs execution facts on the Provider.
    let facts = workspace
        .filter(|_| matches!(tool, "apply_patch" | "functions.apply_patch"))
        .map(compiler::hook_fixture);
    classify_with_execution(tool, input, workspace, facts.as_ref())
}

impl FileEffect {
    fn matches(&self, other: &Self) -> bool {
        self.entry == other.entry
            || self
                .inode
                .as_ref()
                .is_some_and(|inode| other.inode.as_ref() == Some(inode))
    }

    fn check_version(&self, workspace: &Path) -> Result<(), &'static str> {
        let workspace = if self.external_input {
            if !self.operation.is_input() {
                return Err("native_resource_path_changed");
            }
            Path::new("/")
        } else {
            workspace
        };
        let name = self
            .path
            .file_name()
            .ok_or("native_resource_path_changed")?;
        let parent = fs::canonicalize(self.path.parent().ok_or("native_resource_path_changed")?)
            .map_err(|_| "native_resource_path_changed")?;
        if !parent.starts_with(workspace) {
            return Err("native_resource_path_changed");
        }
        let meta = fs::metadata(&parent).map_err(|_| "native_resource_path_changed")?;
        if format!("{}:{}:{}", meta.dev(), meta.ino(), name.to_string_lossy()) != self.entry {
            return Err("native_resource_path_changed");
        }
        let actual = fs::symlink_metadata(&self.path);
        if matches!(
            self.operation,
            FileOperation::Add | FileOperation::AbsentInput | FileOperation::AbsentDirectory
        ) {
            return if actual.is_err_and(|err| err.kind() == std::io::ErrorKind::NotFound) {
                Ok(())
            } else {
                Err("native_resource_version_conflict")
            };
        }
        let real = fs::canonicalize(&self.path).map_err(|_| "native_resource_version_conflict")?;
        if !real.starts_with(workspace) {
            return Err("native_resource_path_changed");
        }
        let stat = fs::metadata(&real).map_err(|_| "native_resource_version_conflict")?;
        if self.operation == FileOperation::MetadataInput {
            return if stat.is_file()
                && self.inode.as_deref() == Some(&format!("{}:{}", stat.dev(), stat.ino()))
                && self.base.as_deref() == Some(&compiler::metadata_version(&stat))
            {
                Ok(())
            } else {
                Err("native_resource_version_conflict")
            };
        }
        if !stat.is_file() || stat.len() > SNAPSHOT_LIMIT {
            return Err("native_resource_version_conflict");
        }
        let path_meta =
            fs::symlink_metadata(&self.path).map_err(|_| "native_resource_path_changed")?;
        if path_meta.file_type().is_symlink() != self.linked {
            return Err("native_resource_path_changed");
        }
        let inode = format!("{}:{}", stat.dev(), stat.ino());
        // Most patch tools publish through an atomic rename, which changes the
        // inode but retains the verified parent/name entry. Non-symlink text
        // edits can still rebase against the new contents; links remain strict.
        if (self.linked || matches!(self.operation, FileOperation::Input | FileOperation::Output))
            && self.inode.as_ref() != Some(&inode)
        {
            return Err("native_resource_path_changed");
        }
        let bytes = fs::read(&real).map_err(|_| "native_resource_version_conflict")?;
        if version(&bytes) == self.base.as_deref().unwrap_or("") {
            // An operation can be prepared after another writer has already
            // committed. Its fresh version alone does not establish that the
            // old-side patch text exists: reject an impossible patch before
            // forwarding it as a Native effect. Repeated anchors are still
            // left to upstream's ordinary contextual patch resolution.
            if self.operation == FileOperation::Update {
                let current =
                    std::str::from_utf8(&bytes).map_err(|_| "native_resource_version_conflict")?;
                if !self.anchors.iter().all(|anchor| current.contains(anchor)) {
                    return Err("native_resource_version_conflict");
                }
            }
            return Ok(());
        }
        if self.operation != FileOperation::Update || self.anchors.is_empty() {
            return Err("native_resource_version_conflict");
        }
        // Rebase a text hunk only when each unchanged old-side anchor survives
        // uniquely in the new version. Upstream's contextual patch must still
        // apply successfully; a failed native tool result is not rewritten.
        let current =
            std::str::from_utf8(&bytes).map_err(|_| "native_resource_version_conflict")?;
        if self
            .anchors
            .iter()
            .all(|anchor| current.match_indices(anchor.as_str()).take(2).count() == 1)
        {
            Ok(())
        } else {
            Err("native_resource_version_conflict")
        }
    }
}

pub(crate) fn can_offer(effect: &Effect, workspace: &Path) -> Result<(), &'static str> {
    if let Effect::Paths(files) = effect {
        for file in files {
            file.check_version(workspace)?;
        }
    }
    Ok(())
}

/// Called for actual effect claims, never for entire work/session records.
pub(crate) fn conflicts(first: &Effect, second: &Effect) -> bool {
    match (first, second) {
        (Effect::Read, _) | (_, Effect::Read) => false,
        (Effect::Opaque, _) | (_, Effect::Opaque) => true,
        (Effect::Paths(left), Effect::Paths(right)) => left.iter().any(|a| {
            right.iter().any(|b| {
                !(a.operation.is_input() && b.operation.is_input())
                    && (a.matches(b)
                        || a.operation == FileOperation::AbsentDirectory
                            && b.path.starts_with(&a.path)
                        || b.operation == FileOperation::AbsentDirectory
                            && a.path.starts_with(&b.path))
            })
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn session_poll_requires_empty_known_input_and_current_exec_hook_proof() {
        let root = temp();
        let mut facts = compiler::hook_fixture(&root);
        let input = serde_json::json!({"session_id":91,"chars":"","yield_time_ms":1000,"max_output_tokens":100}).to_string();
        let check = |value: &str, execution: Option<&Execution>| {
            qualified_session_poll("functions.write_stdin", value, Some(&root), execution)
        };
        assert_eq!(check(&input, Some(&facts)), Some(91));
        assert_eq!(check(r#"{"session_id":91}"#, Some(&facts)), Some(91));
        assert_eq!(check(&input, None), None);
        for value in [
            r#"{"session_id":91,"chars":"x"}"#,
            r#"{"session_id":91,"chars":null}"#,
            r#"{"session_id":0,"chars":""}"#,
            r#"{"session_id":91,"signal":15}"#,
            r#"{"session_id":91,"yield_time_ms":-1}"#,
            r#"{"session_id":91,"max_output_tokens":"100"}"#,
        ] {
            assert_eq!(check(value, Some(&facts)), None, "{value}");
        }
        assert_eq!(empty_session_poll("functions.exec_command", &input), None);
        facts.hook_free_tools.clear();
        assert_eq!(check(&input, Some(&facts)), None);
        facts.schema = 1;
        assert_eq!(check(&input, Some(&facts)), None);
        facts = compiler::hook_fixture(&root.join("other"));
        assert_eq!(check(&input, Some(&facts)), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hook_capable_tools_require_exact_native_proof_without_changing_patch_scope() {
        let root = temp();
        fs::write(root.join("a"), "old\n").unwrap();
        let patch = update("a", "old", "new");
        let mut facts = compiler::hook_fixture(&root);
        for tool in [
            "functions.apply_patch",
            "apply_patch",
            "functions.view_image",
            "clock.curr_time",
            "clock.sleep",
        ] {
            assert_eq!(
                classify_with_execution(tool, &patch, Some(&root), None),
                Effect::Opaque,
                "{tool}"
            );
            facts.hook_free_tools.clear();
            assert_eq!(
                classify_with_execution(tool, &patch, Some(&root), Some(&facts)),
                Effect::Opaque,
                "{tool}"
            );
        }
        facts.hook_free_tools = vec!["apply_patch".into()];
        assert!(matches!(
            classify_with_execution("functions.apply_patch", &patch, Some(&root), Some(&facts)),
            Effect::Paths(_)
        ));
        assert_eq!(
            classify_with_execution("functions.view_image", "{}", Some(&root), Some(&facts)),
            Effect::Opaque
        );
        facts.hook_free_tools = vec!["view_image".into()];
        assert_eq!(
            classify_with_execution("functions.view_image", "{}", Some(&root), Some(&facts)),
            Effect::Read
        );
        assert_eq!(
            classify_with_execution("functions.apply_patch", &patch, Some(&root), Some(&facts)),
            Effect::Opaque
        );
        assert_eq!(
            classify_with_execution(
                "functions.view_image",
                "{}",
                Some(&root.join("other")),
                Some(&facts)
            ),
            Effect::Opaque
        );
        // Old metadata remains decodable for stored requests/receipts, but it
        // cannot establish current hook scope or retained asynchronous work.
        facts.schema = 1;
        facts.hook_free_tools.clear();
        assert_eq!(
            classify_with_execution("functions.apply_patch", &patch, Some(&root), Some(&facts)),
            Effect::Opaque
        );
        facts.schema = 2;
        assert_eq!(
            classify_with_execution("functions.apply_patch", &patch, Some(&root), Some(&facts)),
            Effect::Opaque
        );
        facts.hook_free_tools = vec!["unrecognized".into()];
        assert_eq!(
            classify_with_execution("functions.apply_patch", &patch, Some(&root), Some(&facts)),
            Effect::Opaque
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn persisted_scope_proof_survives_new_readers_and_degrades_after_legacy_round_trip() {
        #[derive(Serialize, Deserialize)]
        struct LegacyFileEffect {
            path: PathBuf,
            entry: String,
            inode: Option<String>,
            base: Option<String>,
            anchors: Vec<String>,
            operation: FileOperation,
            #[serde(default)]
            linked: bool,
            #[serde(default)]
            external_input: bool,
        }
        #[derive(Serialize, Deserialize)]
        enum LegacyEffect {
            Read,
            Opaque,
            Paths(Vec<LegacyFileEffect>),
        }
        let root = temp();
        fs::write(root.join("a"), "old\n").unwrap();
        let effect = classify_with_fixture(
            "functions.apply_patch",
            &update("a", "old", "new"),
            Some(&root),
        );
        let current = serde_json::to_value(&effect).unwrap();
        assert_eq!(current["Paths"][0]["scope_schema"], 2);
        let restored: Effect = serde_json::from_value(current.clone()).unwrap();
        assert_eq!(restored.claim_scope(), &effect);
        // The actual prior file-record shape ignores the additive field. Its
        // write-back remains readable, but cannot preserve current hook proof.
        let old: LegacyEffect = serde_json::from_value(current.clone()).unwrap();
        let old = serde_json::to_value(old).unwrap();
        let unproved: Effect = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(unproved.claim_scope(), &Effect::Opaque);
        assert_eq!(serde_json::to_value(&unproved).unwrap(), old);
        for marker in [
            serde_json::json!(1),
            serde_json::json!(3),
            serde_json::Value::Null,
        ] {
            let mut unknown = current.clone();
            unknown["Paths"][0]["scope_schema"] = marker;
            let unknown: Effect = serde_json::from_value(unknown).unwrap();
            assert_eq!(unknown.claim_scope(), &Effect::Opaque);
        }
        let mut mixed = current.clone();
        mixed["Paths"]
            .as_array_mut()
            .unwrap()
            .push(old["Paths"][0].clone());
        let mixed: Effect = serde_json::from_value(mixed).unwrap();
        assert_eq!(mixed.claim_scope(), &Effect::Opaque);
        assert_eq!(Effect::Paths(Vec::new()).claim_scope(), &Effect::Opaque);
        assert_eq!(Effect::Read.claim_scope(), &Effect::Opaque);
        assert_eq!(Effect::Read.admission_scope(), &Effect::Read);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_discovery_is_read_only_during_an_unconfirmed_native_effect() {
        let lookup =
            classify_with_fixture("tool_search", r#"{"query":"codex_sessions list"}"#, None);
        assert_eq!(lookup, Effect::Read);
        assert!(!conflicts(&lookup, &Effect::Opaque));
        assert!(!conflicts(&Effect::Opaque, &lookup));
    }

    fn temp() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "native-effects-{}-{}",
            std::process::id(),
            crate::journal::random_id().unwrap()
        ));
        fs::create_dir(&path).unwrap();
        fs::canonicalize(path).unwrap()
    }
    fn update(path: &str, old: &str, new: &str) -> String {
        format!("*** Begin Patch\n*** Update File: {path}\n@@\n-{old}\n+{new}\n*** End Patch\n")
    }
    #[test]
    fn direct_compiler_proves_input_output_scope_but_unknown_shell_stays_opaque() {
        let root = temp();
        fs::write(root.join("a.c"), "int a(void){return 1;}\n").unwrap();
        fs::write(root.join("b.c"), "int b(void){return 2;}\n").unwrap();
        let execution = compiler_fixture(&root);
        let compiler = execution.prefix.join("bin/clang");
        let build = |source: &str, output: &str, command: Option<&str>| {
            compiler::classify(
                &serde_json::json!({"cmd":command.map(str::to_owned).unwrap_or_else(||
                    format!("{} -c {source} -o {output}", compiler.display())),
                    "login":false,"workdir":root})
                .to_string(),
                &root,
                &execution,
            )
        };
        let a = build("a.c", "a.o", None).unwrap();
        let b = build("b.c", "b.o", None).unwrap();
        let same_source = build("a.c", "alternate.o", None).unwrap();
        let same_output = build("b.c", "a.o", None).unwrap();
        assert!(!conflicts(&a, &b));
        assert!(
            !conflicts(&a, &same_source),
            "two pinned reads are compatible"
        );
        assert!(conflicts(&a, &same_output));
        assert!(conflicts(
            &a,
            &classify_with_fixture(
                "functions.apply_patch",
                &update("a.c", "int a(void){return 1;}", "int a(void){return 3;}"),
                Some(&root)
            )
        ));
        assert_eq!(can_offer(&a, &root), Ok(()));
        fs::write(root.join("a.c"), "int a(void){return 4;}\n").unwrap();
        assert_eq!(
            can_offer(&a, &root),
            Err("native_resource_version_conflict")
        );
        for command in [
            "/trusted/clang -c a.c -o a.o; touch shared",
            "/trusted/clang -shared a.c -o a.o",
            "/trusted/clang -c a.c -o ../../shared.o",
            "/trusted/clang -c a.c -o a.c",
            "/trusted/clang -c a.c -o /outside.o",
        ] {
            let command = command.replace("/trusted/clang", compiler.to_str().unwrap());
            assert!(build("a.c", "a.o", Some(&command)).is_none(), "{command}");
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn nc_independent_reads_writes_and_inode_aliases() {
        let root = temp();
        fs::write(root.join("a"), "one\ntwo\n").unwrap();
        fs::write(root.join("b"), "three\n").unwrap();
        let hardlink_available = match fs::hard_link(root.join("a"), root.join("alias")) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                // Some Android SELinux policies prohibit link(2) even inside
                // app-owned storage. The inode identity rule is still tested
                // below, without claiming a physical hardlink integration.
                eprintln!(
                    "NC hardlink fixture: Android link(2) denied; testing inode identity directly"
                );
                false
            }
            Err(error) => panic!("unexpected hardlink fixture failure: {error}"),
        };
        symlink("a", root.join("symbolic")).unwrap();
        let a = classify_with_fixture(
            "functions.apply_patch",
            &update("a", "one", "ONE"),
            Some(&root),
        );
        let b = classify_with_fixture(
            "functions.apply_patch",
            &update("b", "three", "THREE"),
            Some(&root),
        );
        let alias = if hardlink_available {
            classify_with_fixture(
                "functions.apply_patch",
                &update("alias", "one", "ONE"),
                Some(&root),
            )
        } else {
            let Effect::Paths(mut entries) = a.clone() else {
                panic!("expected known file effect");
            };
            entries[0].entry = "another-parent:alias".into();
            entries[0].path = root.join("alias");
            Effect::Paths(entries)
        };
        let link = classify_with_fixture(
            "functions.apply_patch",
            &update("symbolic", "one", "ONE"),
            Some(&root),
        );
        assert!(!conflicts(&a, &b));
        assert!(conflicts(&a, &alias));
        assert!(conflicts(&a, &link));
        assert!(!conflicts(&a, &Effect::Read));
        assert!(conflicts(&a, &Effect::Opaque));
        assert_eq!(can_offer(&a, &root), Ok(()));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn nc_nonoverlap_rebases_but_overlap_and_delete_conflict() {
        let root = temp();
        fs::write(root.join("a"), "alpha\nbeta\ngamma\n").unwrap();
        let change = classify_with_fixture(
            "functions.apply_patch",
            &update("a", "beta", "BETA"),
            Some(&root),
        );
        let deletion = classify_with_fixture(
            "functions.apply_patch",
            "*** Begin Patch\n*** Delete File: a\n*** End Patch\n",
            Some(&root),
        );
        fs::write(root.join("a"), "ALPHA\nbeta\ngamma\n").unwrap();
        assert_eq!(can_offer(&change, &root), Ok(()));
        assert_eq!(
            can_offer(&deletion, &root),
            Err("native_resource_version_conflict")
        );
        fs::write(root.join("a"), "ALPHA\nBETA\ngamma\n").unwrap();
        assert_eq!(
            can_offer(&change, &root),
            Err("native_resource_version_conflict")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nc_new_patch_after_a_conflicting_writer_requires_live_old_hunk() {
        let root = temp();
        fs::write(root.join("file"), "ALPHA\nbeta\n").unwrap();
        let stale = classify_with_fixture(
            "functions.apply_patch",
            &update("file", "alpha", "changed"),
            Some(&root),
        );
        assert_eq!(
            can_offer(&stale, &root),
            Err("native_resource_version_conflict")
        );
        let valid = classify_with_fixture(
            "functions.apply_patch",
            &update("file", "beta", "BETA"),
            Some(&root),
        );
        assert_eq!(can_offer(&valid, &root), Ok(()));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn nc_names_moves_opaque_and_path_swap() {
        let root = temp();
        let one = classify_with_fixture(
            "functions.apply_patch",
            "*** Begin Patch\n*** Add File: one\n+hello\n*** End Patch\n",
            Some(&root),
        );
        let two = classify_with_fixture(
            "functions.apply_patch",
            "*** Begin Patch\n*** Add File: two\n+hello\n*** End Patch\n",
            Some(&root),
        );
        assert!(!conflicts(&one, &two));
        assert!(conflicts(&one, &one));
        fs::write(root.join("one"), "taken").unwrap();
        assert_eq!(
            can_offer(&one, &root),
            Err("native_resource_version_conflict")
        );
        assert_eq!(
            classify_with_fixture(
                "functions.apply_patch",
                "*** Begin Patch\n*** Add File: ../escape\n+bad\n*** End Patch\n",
                Some(&root)
            ),
            Effect::Opaque
        );
        assert_eq!(
            classify_with_fixture("functions.exec_command", r#"{"cmd":"pwd"}"#, Some(&root)),
            Effect::Opaque
        );
        assert_eq!(
            classify_with_fixture(
                "functions.exec_command",
                r#"{"cmd":"pwd; touch unwanted"}"#,
                Some(&root)
            ),
            Effect::Opaque
        );
        assert_eq!(
            classify_with_fixture(
                "functions.exec_command",
                r#"{"cmd":"pwd","shell":"/tmp/untrusted-shell"}"#,
                Some(&root)
            ),
            Effect::Opaque,
            "unrecognized executor options invalidate the read-only proof"
        );
        assert_eq!(
            classify_with_fixture(
                "functions.exec_command",
                r#"{"cmd":"git status --porcelain=v1 --untracked-files=no"}"#,
                Some(&root)
            ),
            Effect::Opaque,
            "Git status may refresh the index"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn nc_qualified_shell_reads_coexist_without_guessing_git_config_effects() {
        let root = temp();
        let execution = compiler_fixture(&root);
        assert_eq!(
            classify_with_fixture(
                "functions.exec_command",
                r#"{"cmd":"pwd","login":false}"#,
                Some(&root)
            ),
            Effect::Opaque,
            "non-login alone cannot attest PATH or startup environment"
        );
        for command in [
            "pwd",
            "ls -la",
            "cat a.c",
            "git rev-parse HEAD",
            "git branch --show-current",
            "git --no-pager -c log.showSignature=false log -1 --oneline",
        ] {
            let read = classify_with_execution(
                "functions.exec_command",
                &serde_json::json!({"cmd":command,"login":false}).to_string(),
                Some(&root),
                Some(&execution),
            );
            assert!(matches!(read, Effect::Paths(_)), "{command}");
            assert!(
                !conflicts(&read, &read),
                "shared execution inputs stay concurrent"
            );
            assert!(
                conflicts(&read, &Effect::Opaque),
                "an opaque writer may replace an execution input"
            );
        }
        for unsafe_command in [
            r#"{"cmd":"pwd"}"#,
            r#"{"cmd":"git -c core.fsmonitor=false --no-optional-locks status --porcelain=v1 --untracked-files=no"}"#,
            r#"{"cmd":"git status --porcelain=v1 --untracked-files=no","login":false}"#,
            r#"{"cmd":"git -c core.fsmonitor=false --no-optional-locks status --porcelain=v1 --untracked-files=no","login":true}"#,
            r#"{"cmd":"git -c core.fsmonitor=false --no-optional-locks status --porcelain=v1 --untracked-files=no; touch unexpected","login":false}"#,
            r#"{"cmd":"git -c core.fsmonitor=false --no-optional-locks status --porcelain=v1 --untracked-files=no","login":false}"#,
            r#"{"cmd":"git log -1 --oneline","login":false}"#,
        ] {
            assert_eq!(
                classify_with_execution(
                    "functions.exec_command",
                    unsafe_command,
                    Some(&root),
                    Some(&execution)
                ),
                Effect::Opaque,
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_status_can_execute_a_clean_filter_despite_no_optional_locks() {
        let root = temp();
        let execution = compiler_fixture(&root);
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(&root)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init", "-q"]);
        git(&[
            "config",
            "filter.probe.clean",
            "touch filter-side-effect; cat",
        ]);
        fs::write(root.join(".gitattributes"), "a.txt filter=probe\n").unwrap();
        fs::write(root.join("a.txt"), "one\n").unwrap();
        git(&["add", ".gitattributes", "a.txt"]);
        fs::remove_file(root.join("filter-side-effect")).unwrap();
        // Equal lengths force status to hash the worktree through its filter.
        fs::write(root.join("a.txt"), "two\n").unwrap();
        git(&[
            "-c",
            "core.fsmonitor=false",
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "--untracked-files=no",
        ]);
        assert!(root.join("filter-side-effect").exists());
        assert_eq!(classify_with_execution("functions.exec_command",
            &serde_json::json!({"cmd":"git -c core.fsmonitor=false --no-optional-locks status --porcelain=v1 --untracked-files=no","login":false,"workdir":root}).to_string(),
            Some(&root), Some(&execution)), Effect::Opaque);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nc_bounded_compiler_builds_hold_inputs_and_outputs_without_a_repo_lock() {
        let root = temp();
        let execution = compiler_fixture(&root);
        fs::write(root.join("a.c"), "int main(void) { return 0; }\n").unwrap();
        fs::write(root.join("b.c"), "int other(void) { return 1; }\n").unwrap();
        let build = |cmd: &str, login: bool| {
            classify_with_execution(
                "functions.exec_command",
                &serde_json::json!({"cmd":cmd,"workdir":root,"login":login}).to_string(),
                Some(&root),
                Some(&execution),
            )
        };
        let a = build("clang -O0 a.c -o a.out", false);
        let b = build("clang -O0 b.c -o b.out", false);
        assert!(
            matches!(a, Effect::Paths(_)),
            "bounded build must be a path claim"
        );
        assert!(matches!(b, Effect::Paths(_)));
        assert!(
            !conflicts(&a, &b),
            "independent inputs and outputs may overlap"
        );
        assert!(
            !conflicts(&a, &build("clang -O0 a.c -o a-second.out", false)),
            "two source readers do not conflict"
        );
        assert!(
            conflicts(&a, &build("clang -O0 b.c -o a.out", false)),
            "a shared output stays exclusive"
        );
        assert!(
            conflicts(
                &a,
                &classify_with_fixture(
                    "functions.apply_patch",
                    &update("a.c", "return 0", "return 3"),
                    Some(&root)
                )
            ),
            "a concurrent source edit conflicts with its compiler read"
        );
        assert_eq!(can_offer(&a, &root), Ok(()));
        fs::write(root.join("a.c"), "int main(void) { return 9; }\n").unwrap();
        assert_eq!(
            can_offer(&a, &root),
            Err("native_resource_version_conflict")
        );
        for unsafe_build in [
            "clang -O0 a.c -o a.out; touch outside",
            "clang -Xclang -load plugin.so a.c -o a.out",
            "clang -O0 a.c -o ../outside",
            "clang -O0 a.c -o a.out -save-temps",
            "sh -c clang a.c -o a.out",
        ] {
            assert_eq!(build(unsafe_build, false), Effect::Opaque);
        }
        assert_eq!(build("clang -O0 a.c -o a.out", true), Effect::Opaque);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nc_atomic_replace_and_verified_absolute_patch_target() {
        let root = temp();
        fs::write(root.join("a"), "alpha\nbeta\n").unwrap();
        let target = root.join("a").to_str().unwrap().to_owned();
        let effect = classify_with_fixture(
            "functions.apply_patch",
            &update(&target, "beta", "BETA"),
            Some(&root),
        );
        assert!(matches!(effect, Effect::Paths(_)));
        fs::write(root.join("replacement"), "ALPHA\nbeta\n").unwrap();
        fs::rename(root.join("replacement"), root.join("a")).unwrap();
        assert_eq!(
            can_offer(&effect, &root),
            Ok(()),
            "independent atomic replacement must permit nonoverlap"
        );
        fs::remove_file(root.join("a")).unwrap();
        symlink("/dev/null", root.join("a")).unwrap();
        assert_eq!(
            can_offer(&effect, &root),
            Err("native_resource_path_changed")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
