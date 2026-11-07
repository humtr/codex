//! One Git merge path for maintained source patches. Git owns context and rename
//! reconciliation; conflicts never modify the checkout or its real index.
use super::{read, Result};
use std::fs;
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;
use std::process::{Command, Stdio};

const HEADER: &str = "# codex-upstream-base ";

pub(super) fn base(patch: &[u8]) -> Result<String> {
    let line = patch
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    let value = std::str::from_utf8(line)
        .map_err(|e| e.to_string())?
        .strip_prefix(HEADER)
        .ok_or("source patch must bind its exact official base")?;
    if value.len() != 40
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("source patch base must be an exact Git commit".into());
    }
    Ok(value.into())
}

fn run(
    source: &Path,
    index: Option<&Path>,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(source)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().map_err(|e| format!("source merge: {e}"))?;
    let written = if let Some(input) = input {
        child
            .stdin
            .take()
            .ok_or("source merge input unavailable")?
            .write_all(input)
            .map_err(|error| error.to_string())
    } else {
        Ok(())
    };
    let result = child.wait_with_output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "source merge failed before mutation ({}): {}{}",
            args[0],
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    written?;
    Ok(result.stdout)
}

fn oid(bytes: Vec<u8>) -> Result<String> {
    let value = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let value = value.trim();
    if value.len() != 40
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("source merge returned an invalid Git object".into());
    }
    Ok(value.into())
}

pub(super) fn rebase(source: &Path, patch: &[u8]) -> Result<Vec<u8>> {
    let base = base(patch)?;
    run(
        source,
        None,
        &["cat-file", "-e", &format!("{base}^{{commit}}")],
        None,
    )
    .map_err(|e| format!("fetch exact official patch base {base} before preparation: {e}"))?;
    let temporary = std::env::temp_dir().join(format!(
        "codex-source-merge-{}-{}",
        std::process::id(),
        super::super::STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&temporary)
        .map_err(|e| e.to_string())?;
    let index = temporary.join("index");
    let result = (|| {
        run(source, Some(&index), &["read-tree", &base], None)?;
        run(
            source,
            Some(&index),
            &["apply", "--cached", "--whitespace=error", "-"],
            Some(patch),
        )?;
        let patched = oid(run(source, Some(&index), &["write-tree"], None)?)?;
        run(source, Some(&index), &["read-tree", "HEAD"], None)?;
        run(source, Some(&index), &["add", "-A", "--", "codex-rs"], None)?;
        let prepared = oid(run(source, Some(&index), &["write-tree"], None)?)?;
        // Use commits for compatibility with Git versions that do not accept
        // standalone trees in merge-tree. No refs, config or public history change.
        let commit = |tree: &str, parent: &str| {
            oid(run(
                source,
                None,
                &[
                    "-c",
                    "user.name=Codex compatibility",
                    "-c",
                    "user.email=compatibility@localhost",
                    "commit-tree",
                    tree,
                    "-p",
                    parent,
                    "-m",
                    "Disposable source adaptation",
                ],
                None,
            )?)
        };
        let prepared_commit = commit(&prepared, "HEAD")?;
        let patched_commit = commit(&patched, &base)?;
        let merged = oid(run(
            source,
            None,
            &[
                "merge-tree",
                "--write-tree",
                "--no-messages",
                &format!("--merge-base={base}"),
                &prepared_commit,
                &patched_commit,
            ],
            None,
        )?)?;
        let rebased = run(
            source,
            None,
            &["diff", "--binary", &prepared, &merged],
            None,
        )?;
        if !rebased.is_empty() {
            run(
                source,
                None,
                &["apply", "--check", "--whitespace=error", "-"],
                Some(&rebased),
            )?;
        }
        Ok(rebased)
    })();
    if let Err(error) = fs::remove_dir_all(&temporary) {
        return Err(format!(
            "source merge cleanup failed: {error}; preparation error: {:?}",
            result.as_ref().err()
        ));
    }
    result
}

pub(super) fn apply(source: &Path, rebased: &[u8]) -> Result<()> {
    if !rebased.is_empty() {
        run(
            source,
            None,
            &["apply", "--whitespace=error", "-"],
            Some(rebased),
        )?;
    }
    Ok(())
}

pub(super) fn from_file(path: &Path) -> Result<Vec<u8>> {
    read(path).map(String::into_bytes)
}
