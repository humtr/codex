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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Effect {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum FileOperation {
    Update,
    Add,
    Delete,
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

/// Only syntactically proven, non-mutating commands are classified as reads.
/// Shell chaining, redirects, expansions and unknown tools remain opaque.
fn is_simple_read_command(input: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input) else {
        return false;
    };
    let Some(arguments) = value.as_object() else {
        return false;
    };
    // An alternate shell, login startup, custom environment or any unknown
    // executor parameter can run additional code even for an innocent cmd.
    if arguments.keys().any(|key| {
        !matches!(
            key.as_str(),
            "cmd" | "workdir" | "yield_time_ms" | "max_output_tokens"
        )
    }) {
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
            | "git log -1 --oneline"
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

pub(crate) fn classify(tool: &str, input: &str, workspace: Option<&Path>) -> Effect {
    match tool {
        "clock.curr_time" | "clock.sleep" | "functions.view_image" => Effect::Read,
        "functions.exec_command" if is_simple_read_command(input) => Effect::Read,
        "functions.apply_patch" | "apply_patch" => workspace
            .and_then(|root| parse_patch(input, root))
            .unwrap_or(Effect::Opaque),
        _ => Effect::Opaque,
    }
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
        if self.operation == FileOperation::Add {
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
        if self.linked && self.inode.as_ref() != Some(&inode) {
            return Err("native_resource_path_changed");
        }
        let bytes = fs::read(&real).map_err(|_| "native_resource_version_conflict")?;
        if version(&bytes) == self.base.as_deref().unwrap_or("") {
            return Ok(());
        }
        if self.operation != FileOperation::Update {
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
        (Effect::Paths(left), Effect::Paths(right)) => {
            left.iter().any(|a| right.iter().any(|b| a.matches(b)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

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
        let a = classify(
            "functions.apply_patch",
            &update("a", "one", "ONE"),
            Some(&root),
        );
        let b = classify(
            "functions.apply_patch",
            &update("b", "three", "THREE"),
            Some(&root),
        );
        let alias = if hardlink_available {
            classify(
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
        let link = classify(
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
        let change = classify(
            "functions.apply_patch",
            &update("a", "beta", "BETA"),
            Some(&root),
        );
        let deletion = classify(
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
    fn nc_names_moves_opaque_and_path_swap() {
        let root = temp();
        let one = classify(
            "functions.apply_patch",
            "*** Begin Patch\n*** Add File: one\n+hello\n*** End Patch\n",
            Some(&root),
        );
        let two = classify(
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
            classify(
                "functions.apply_patch",
                "*** Begin Patch\n*** Add File: ../escape\n+bad\n*** End Patch\n",
                Some(&root)
            ),
            Effect::Opaque
        );
        assert_eq!(
            classify("functions.exec_command", r#"{"cmd":"pwd"}"#, Some(&root)),
            Effect::Read
        );
        assert_eq!(
            classify(
                "functions.exec_command",
                r#"{"cmd":"pwd; touch unwanted"}"#,
                Some(&root)
            ),
            Effect::Opaque
        );
        assert_eq!(
            classify(
                "functions.exec_command",
                r#"{"cmd":"pwd","shell":"/tmp/untrusted-shell"}"#,
                Some(&root)
            ),
            Effect::Opaque,
            "unrecognized executor options invalidate the read-only proof"
        );
        assert_eq!(
            classify(
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
    fn nc_atomic_replace_and_verified_absolute_patch_target() {
        let root = temp();
        fs::write(root.join("a"), "alpha\nbeta\n").unwrap();
        let target = root.join("a").to_str().unwrap().to_owned();
        let effect = classify(
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
