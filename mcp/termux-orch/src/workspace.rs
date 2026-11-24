use crate::ledger::{ensure_private_dir, Ledger};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTree {
    pub path: PathBuf,
    pub repo: PathBuf,
    pub branch: String,
    pub base: String,
}

fn git(cwd: &Path, args: &[&OsStr]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(crate::ledger::truncate_chars(
            &String::from_utf8_lossy(&output.stderr),
            500,
        ))
    }
}

fn git_text(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let args: Vec<_> = args.iter().map(OsStr::new).collect();
    String::from_utf8(git(cwd, &args)?)
        .map(|text| text.trim_end_matches('\n').to_owned())
        .map_err(|_| "Git returned a path that is not UTF-8".to_owned())
}

pub fn root(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("cwd must be absolute".to_owned());
    }
    let path = path.canonicalize().map_err(|err| err.to_string())?;
    if !path.is_dir() {
        return Err("cwd must be a directory".to_owned());
    }
    match git_text(&path, &["rev-parse", "--show-toplevel"]) {
        Ok(repo) => PathBuf::from(repo)
            .canonicalize()
            .map_err(|err| err.to_string()),
        Err(_) => Ok(path),
    }
}

pub fn validate_paths(paths: &[String]) -> Result<(), String> {
    if paths.len() > 64
        || paths.iter().any(|path| {
            path.is_empty()
                || path.len() > 512
                || path.contains('\0')
                || path.split('/').any(|segment| {
                    segment.is_empty() || segment == "." || segment == ".." || segment == ".git"
                })
                || !Path::new(path)
                    .components()
                    .all(|c| matches!(c, Component::Normal(_)))
        })
    {
        return Err("write_paths must be literal workspace-relative paths".to_owned());
    }
    Ok(())
}

impl WorkerTree {
    pub fn plan(ledger: &Ledger, work_id: &str, id: &str, repo: &Path) -> Result<Self, String> {
        if root(repo)? != repo {
            return Err("worktree requires the repository root".to_owned());
        }
        let base = git_text(repo, &["rev-parse", "--verify", "HEAD"])?;
        if !git_text(repo, &["status", "--porcelain", "--untracked-files=normal"])?.is_empty() {
            return Err("the source repository has existing changes. Commit the intended inputs before creating a worker worktree".to_owned());
        }
        let parent = ledger
            .work_dir(work_id)
            .map_err(|err| err.to_string())?
            .join("worktrees");
        ensure_private_dir(&parent).map_err(|err| err.to_string())?;
        Ok(Self {
            path: parent.join(id),
            repo: repo.to_owned(),
            branch: format!("orch/{work_id}/{id}"),
            base,
        })
    }

    pub fn prepare(&self) -> Result<(), String> {
        if !self.path.exists() {
            let reference = format!("refs/heads/{}", self.branch);
            let branch_exists = git_text(&self.repo, &["show-ref", "--verify", &reference]).is_ok();
            if branch_exists {
                if git_text(&self.repo, &["rev-parse", "--verify", &reference])? != self.base {
                    return Err("existing worker branch has moved".to_owned());
                }
                git(
                    &self.repo,
                    &[
                        OsStr::new("worktree"),
                        OsStr::new("add"),
                        self.path.as_os_str(),
                        OsStr::new(&self.branch),
                    ],
                )?;
            } else {
                git(
                    &self.repo,
                    &[
                        OsStr::new("worktree"),
                        OsStr::new("add"),
                        OsStr::new("-b"),
                        OsStr::new(&self.branch),
                        self.path.as_os_str(),
                        OsStr::new(&self.base),
                    ],
                )?;
            }
        }
        if root(&self.path)? != self.path
            || git_text(&self.path, &["symbolic-ref", "--short", "HEAD"])? != self.branch
        {
            return Err("worker repository identity could not be verified".to_owned());
        }
        Ok(())
    }

    pub fn declared_changes(&self, allowed: &[String]) -> Result<Vec<String>, String> {
        let paths = changed(&self.path, &self.base)?;
        let outside: Vec<_> = paths
            .iter()
            .filter(|path| {
                !allowed
                    .iter()
                    .any(|prefix| Path::new(path).starts_with(prefix))
            })
            .collect();
        if !outside.is_empty() {
            return Err(format!(
                "Undeclared changes in {}: {outside:?}",
                self.path.display()
            ));
        }
        Ok(paths.into_iter().collect())
    }

    pub fn conflicting_paths(&self, paths: &[String]) -> Result<Vec<String>, String> {
        let current = changed(&self.repo, &self.base)?;
        Ok(paths
            .iter()
            .filter(|path| current.iter().any(|other| overlaps(path, other)))
            .cloned()
            .collect())
    }
}

pub fn overlaps(a: &str, b: &str) -> bool {
    Path::new(a).starts_with(b) || Path::new(b).starts_with(a)
}

fn changed(repo: &Path, base: &str) -> Result<BTreeSet<String>, String> {
    let diff = git(
        repo,
        &["diff", "--name-only", "--no-renames", "-z", base, "--"].map(OsStr::new),
    )?;
    let untracked = git(
        repo,
        &["ls-files", "--others", "--exclude-standard", "-z"].map(OsStr::new),
    )?;
    diff.split(|byte| *byte == 0)
        .chain(untracked.split(|byte| *byte == 0))
        .filter(|path| !path.is_empty())
        .map(|path| {
            String::from_utf8(path.to_vec())
                .map_err(|_| "worker changed a path that is not UTF-8".to_owned())
        })
        .collect()
}

// A verifier must approve the same Git snapshot that the controller completes.
// The bytes of tracked and untracked changes stay local; only digests go to the ledger.
pub fn verification_fingerprint(cwd: &Path) -> Result<Option<String>, String> {
    let Ok(repo) = git_text(cwd, &["rev-parse", "--show-toplevel"]) else {
        return Ok(None);
    };
    let repo = Path::new(&repo);
    let head = git_text(repo, &["rev-parse", "HEAD"])?;
    let diff = git(
        repo,
        &["diff", "--binary", "--no-ext-diff", "HEAD", "--"].map(OsStr::new),
    )?;
    let entries = git(
        repo,
        &["ls-files", "--others", "--exclude-standard", "-z"].map(OsStr::new),
    )?;
    let mut evidence = vec![head, mcp_kit::sha256_hex(&diff)];
    use std::io::Read;
    use std::os::unix::ffi::OsStrExt;
    for name in entries
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let path = repo.join(OsStr::from_bytes(name));
        let meta = path.symlink_metadata().map_err(|error| error.to_string())?;
        let content = if meta.is_symlink() {
            path.read_link()
                .map_err(|error| error.to_string())?
                .as_os_str()
                .as_bytes()
                .to_vec()
        } else if meta.is_file() && meta.len() <= 16 * 1024 * 1024 {
            use std::os::unix::fs::OpenOptionsExt;
            let mut content = vec![];
            std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
                .map_err(|error| error.to_string())?
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut content)
                .map_err(|error| error.to_string())?;
            if content.len() > 16 * 1024 * 1024 {
                return Err("untracked review file exceeds the evidence bound".into());
            }
            content
        } else {
            return Err(
                "untracked review file exceeds the evidence bound or has unsupported type".into(),
            );
        };
        evidence.push(mcp_kit::sha256_hex(name));
        evidence.push(mcp_kit::sha256_hex(&content));
    }
    Ok(Some(mcp_kit::sha256_hex(evidence.join("\n").as_bytes())))
}
