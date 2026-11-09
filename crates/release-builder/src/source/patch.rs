//! One Git merge path for maintained source patches. Git owns context and rename
//! reconciliation; conflicts never modify the checkout or its real index.
use super::{read, Result};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};

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
    let (result, written) = communicate(&mut command, input)?;
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

fn communicate(command: &mut Command, input: Option<&[u8]>) -> Result<(Output, Result<()>)> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| format!("source merge: {e}"))?;
    let stdin = child.stdin.take();
    // Git can diagnose a large patch before consuming all of stdin. Drain both
    // output pipes while writing, so neither side waits on a full pipe.
    let (output, written) = std::thread::scope(|scope| {
        let writer = input.map(|input| {
            scope.spawn(move || {
                stdin
                    .ok_or_else(|| "source merge input unavailable".to_string())?
                    .write_all(input)
                    .map_err(|e| e.to_string())
            })
        });
        let output = child.wait_with_output().map_err(|e| e.to_string());
        let written = writer.map_or(Ok(()), |writer| {
            writer
                .join()
                .unwrap_or_else(|_| Err("source merge input writer failed".into()))
        });
        (output, written)
    });
    Ok((output?, written))
}

pub(super) fn owned_sources(
    source: &Path,
    directory: &Path,
    target: &Path,
) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    const MAX_FILE: u64 = 16 * 1024 * 1024;
    const MAX_TOTAL: u64 = 64 * 1024 * 1024;
    if !directory.is_absolute()
        || !fs::symlink_metadata(directory)
            .map_err(|e| e.to_string())?
            .is_dir()
        || !target.starts_with("codex-rs")
        || target
            .components()
            .any(|part| !matches!(part, Component::Normal(name) if name != ".git"))
    {
        return Err(
            "owned sources require a real absolute directory and a relative codex-rs target".into(),
        );
    }
    let mut pending = vec![directory.to_path_buf()];
    let mut result = BTreeMap::new();
    let mut total = 0;
    let mut entries = 0;
    while let Some(parent) = pending.pop() {
        for entry in fs::read_dir(&parent).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            entries += 1;
            if entries > 4096 {
                return Err("owned source inventory exceeds its entry bound".into());
            }
            let path = entry.path();
            let relative = path.strip_prefix(directory).map_err(|e| e.to_string())?;
            if relative.components().count() > 64
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(name) if name != ".git"))
            {
                return Err("invalid owned source path".into());
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            if !kind.is_file() {
                return Err("owned source must be a regular file".into());
            }
            let relative = target.join(relative);
            let destination = source.join(&relative);
            match fs::symlink_metadata(&destination) {
                Ok(_) => {
                    return Err(format!(
                        "owned source collides with upstream: {}",
                        relative.display()
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
            for parent in destination
                .ancestors()
                .skip(1)
                .take_while(|path| *path != source)
            {
                match fs::symlink_metadata(parent) {
                    Ok(metadata) if metadata.is_dir() => {}
                    Ok(_) => return Err("unsafe owned source destination parent".into()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
                .map_err(|e| e.to_string())?;
            let metadata = file.metadata().map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.len() > MAX_FILE {
                return Err("owned source is not a bounded regular file".into());
            }
            let mut bytes = Vec::new();
            file.take(MAX_FILE + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 != metadata.len() {
                return Err("owned source changed during snapshot".into());
            }
            total += bytes.len() as u64;
            if total > MAX_TOTAL {
                return Err("owned sources exceed their total byte bound".into());
            }
            result.insert(relative, bytes);
        }
    }
    if result.is_empty() {
        return Err("owned source inventory is empty".into());
    }
    Ok(result)
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

pub(super) fn rebase(
    source: &Path,
    patch: &[u8],
    changes: &BTreeMap<PathBuf, String>,
) -> Result<Vec<u8>> {
    rebase_with_owned(source, patch, changes, &BTreeMap::new())
}

pub(super) fn rebase_with_owned(
    source: &Path,
    patch: &[u8],
    changes: &BTreeMap<PathBuf, String>,
    owned: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<Vec<u8>> {
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
        let patch_tree = oid(run(source, Some(&index), &["write-tree"], None)?)?;
        for (path, bytes) in owned {
            let relative = path.to_str().ok_or("non-UTF-8 owned source path")?;
            if !run(
                source,
                None,
                &["ls-tree", &patch_tree, "--", relative],
                None,
            )?
            .is_empty()
            {
                return Err(format!(
                    "source patch collides with owned source: {relative}"
                ));
            }
            for parent in path
                .ancestors()
                .skip(1)
                .filter(|path| !path.as_os_str().is_empty())
            {
                let parent = parent.to_str().ok_or("non-UTF-8 owned source parent")?;
                let entry = run(source, None, &["ls-tree", &patch_tree, "--", parent], None)?;
                if !entry.is_empty() && !entry.starts_with(b"040000 tree ") {
                    return Err("source patch replaces an owned source parent".into());
                }
            }
            let blob = oid(run(
                source,
                None,
                &["hash-object", "-w", "--stdin"],
                Some(bytes),
            )?)?;
            run(
                source,
                Some(&index),
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    "100644",
                    &blob,
                    relative,
                ],
                None,
            )?;
        }
        let patched = oid(run(source, Some(&index), &["write-tree"], None)?)?;
        run(source, Some(&index), &["read-tree", "HEAD"], None)?;
        run(source, Some(&index), &["add", "-A", "--", "codex-rs"], None)?;
        let original = oid(run(source, Some(&index), &["write-tree"], None)?)?;
        for (path, text) in changes {
            let relative = path
                .strip_prefix(source)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non-UTF-8 adapted source path")?;
            let mode = String::from_utf8(run(
                source,
                None,
                &["ls-tree", "HEAD", "--", relative],
                None,
            )?)
            .map_err(|e| e.to_string())?;
            let mode = mode
                .split_whitespace()
                .next()
                .ok_or("adapted source is not tracked")?;
            let blob = oid(run(
                source,
                None,
                &["hash-object", "-w", "--stdin"],
                Some(text.as_bytes()),
            )?)?;
            run(
                source,
                Some(&index),
                &["update-index", "--cacheinfo", mode, &blob, relative],
                None,
            )?;
        }
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
        run(source, None, &["diff", "--check", &original, &merged], None)?;
        let rebased = run(
            source,
            None,
            &["diff", "--binary", &original, &merged],
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
        source: PathBuf,
        patch: Vec<u8>,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "source-patch-test-{}-{}",
                std::process::id(),
                super::super::super::STAGING_COUNTER
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let source = root.join("upstream");
            fs::create_dir_all(source.join("codex-rs/tui/src")).unwrap();
            fs::write(source.join("codex-rs/tui/src/lib.rs"), "upstream\n").unwrap();
            run(&source, None, &["init", "-q"], None).unwrap();
            run(&source, None, &["add", "."], None).unwrap();
            run(
                &source,
                None,
                &[
                    "-c",
                    "user.name=Owned fixture",
                    "-c",
                    "user.email=owned@localhost",
                    "commit",
                    "-qm",
                    "official",
                ],
                None,
            )
            .unwrap();
            let base = oid(run(&source, None, &["rev-parse", "HEAD"], None).unwrap()).unwrap();
            let patch = format!("{HEADER}{base}\ndiff --git a/codex-rs/tui/src/lib.rs b/codex-rs/tui/src/lib.rs\n--- a/codex-rs/tui/src/lib.rs\n+++ b/codex-rs/tui/src/lib.rs\n@@ -1 +1,2 @@\n upstream\n+integration\n").into_bytes();
            fs::create_dir(root.join("owned")).unwrap();
            fs::write(root.join("owned/owned.rs"), "owned implementation\n").unwrap();
            Self {
                root,
                source,
                patch,
            }
        }

        fn owned(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
            owned_sources(
                &self.source,
                &self.root.join("owned"),
                Path::new("codex-rs/tui/src"),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn large_input_and_output_are_pumped_together() {
        let mut command = Command::new("sh");
        command.args(["-c", "dd if=/dev/zero bs=65536 count=4 2>/dev/null; cat"]);
        let input = vec![b'x'; 512 * 1024];
        let (output, written) = communicate(&mut command, Some(&input)).unwrap();
        written.unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 256 * 1024 + input.len());
        assert_eq!(&output.stdout[256 * 1024..], input);
    }

    #[test]
    fn patch_and_owned_files_apply_once_without_changing_the_real_index() {
        let fixture = Fixture::new();
        let index = run(&fixture.source, None, &["write-tree"], None).unwrap();
        let merged = rebase_with_owned(
            &fixture.source,
            &fixture.patch,
            &BTreeMap::new(),
            &fixture.owned().unwrap(),
        )
        .unwrap();
        assert_eq!(
            read(&fixture.source.join("codex-rs/tui/src/lib.rs")).unwrap(),
            "upstream\n"
        );
        assert!(!fixture.source.join("codex-rs/tui/src/owned.rs").exists());
        apply(&fixture.source, &merged).unwrap();
        assert_eq!(
            read(&fixture.source.join("codex-rs/tui/src/lib.rs")).unwrap(),
            "upstream\nintegration\n"
        );
        assert_eq!(
            read(&fixture.source.join("codex-rs/tui/src/owned.rs")).unwrap(),
            "owned implementation\n"
        );
        assert_eq!(
            run(&fixture.source, None, &["write-tree"], None).unwrap(),
            index
        );
        assert!(fixture.owned().is_err());
    }

    #[test]
    fn patch_collision_and_unsafe_owned_sources_fail_before_mutation() {
        let mut fixture = Fixture::new();
        let index = run(&fixture.source, None, &["write-tree"], None).unwrap();
        fixture.patch.extend_from_slice(b"diff --git a/codex-rs/tui/src/owned.rs b/codex-rs/tui/src/owned.rs\nnew file mode 100644\n--- /dev/null\n+++ b/codex-rs/tui/src/owned.rs\n@@ -0,0 +1 @@\n+patch-owned implementation\n");
        let error = rebase_with_owned(
            &fixture.source,
            &fixture.patch,
            &BTreeMap::new(),
            &fixture.owned().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("collides with owned source"), "{error}");
        let input = fixture.root.join("owned/owned.rs");
        fs::remove_file(&input).unwrap();
        std::os::unix::fs::symlink(fixture.source.join("codex-rs/tui/src/lib.rs"), &input).unwrap();
        assert!(fixture.owned().is_err());
        assert_eq!(
            read(&fixture.source.join("codex-rs/tui/src/lib.rs")).unwrap(),
            "upstream\n"
        );
        assert!(!fixture.source.join("codex-rs/tui/src/owned.rs").exists());
        assert_eq!(
            run(&fixture.source, None, &["write-tree"], None).unwrap(),
            index
        );
    }
}
