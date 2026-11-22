use std::env;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ExtPaths {
    pub prefix: PathBuf,
    pub tmpdir: PathBuf,
    pub codex_bin: PathBuf,
    pub shared_home: PathBuf,
    pub manager_dir: PathBuf,
    pub profiles_dir: PathBuf,
    pub default_profile_file: PathBuf,
    pub sessions_dir: PathBuf,
    pub handoff_dir: PathBuf,
    pub notify_config_file: PathBuf,
}

impl ExtPaths {
    pub fn from_env() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is unset or empty".to_owned())?;
        validate_abs_path(&home, "HOME")?;

        let prefix = env::var_os("PREFIX")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/data/com.termux/files/usr"));
        validate_abs_path(&prefix, "PREFIX")?;

        let tmpdir = env::var_os("TMPDIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| prefix.join("tmp"));
        validate_abs_path(&tmpdir, "TMPDIR")?;

        let codex_bin = env::var_os("CODEX_EXT_CODEX_BIN")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| prefix.join("bin").join("codex"));

        let shared_home = home.join(".codex");
        let manager_dir = home.join(".local/share/codex/manager");
        let profiles_dir = manager_dir.join("profiles");
        let default_profile_file = manager_dir.join("default-profile");
        let state_dir = home.join(".local/state/codex-ext");
        let sessions_dir = state_dir.join("sessions");
        let handoff_dir = state_dir.join("handoff");
        let config_dir = home.join(".config/codex-ext");
        let notify_config_file = config_dir.join("notify.json");

        Ok(Self {
            prefix,
            tmpdir,
            codex_bin,
            shared_home,
            manager_dir,
            profiles_dir,
            default_profile_file,
            sessions_dir,
            handoff_dir,
            notify_config_file,
        })
    }
}

pub fn validate_abs_path(path: &Path, label: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!(
            "{label} must be an absolute path: {}",
            path.display()
        ));
    }
    for comp in path.components() {
        if matches!(comp, Component::CurDir | Component::ParentDir) {
            return Err(format!(
                "{label} must not contain dot segments: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

pub fn current_uid() -> u32 {
    unsafe { libc::geteuid() }
}

pub fn ensure_private_dir(dir: &Path) -> Result<(), String> {
    if let Ok(meta) = fs::symlink_metadata(dir) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(format!("expected real directory: {}", dir.display()));
        }
        if meta.uid() != current_uid() {
            return Err(format!(
                "directory not owned by current user: {}",
                dir.display()
            ));
        }
        return Ok(());
    }
    if let Some(parent) = dir.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create parent {}: {e}", parent.display()))?;
        }
    }
    match DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            let meta = fs::symlink_metadata(dir)
                .map_err(|e| format!("cannot stat {}: {e}", dir.display()))?;
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err(format!("expected real directory: {}", dir.display()));
            }
            Ok(())
        }
        Err(err) => Err(format!(
            "failed to create directory {}: {err}",
            dir.display()
        )),
    }
}

pub fn require_owned_regular_file(path: &Path, label: &str) -> Result<(), String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot stat {label} {}: {e}", path.display()))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    if meta.uid() != current_uid() {
        return Err(format!(
            "{label} is not owned by current user: {}",
            path.display()
        ));
    }
    Ok(())
}

pub fn atomic_write_private(path: &Path, content: &[u8], mode: u32) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    ensure_private_dir(parent)?;
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(format!(
                "refusing to overwrite non-regular or symlink file: {}",
                path.display()
            ));
        }
    }
    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let nanos = now_millis();
    let tmp = parent.join(format!(".{fname}.tmp.{}.{nanos}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&tmp)
        .map_err(|e| format!("failed to stage {}: {e}", tmp.display()))?;
    let write_res = (|| -> io::Result<()> {
        file.write_all(content)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp);
        return Err(format!("failed to write {}: {err}", tmp.display()));
    }
    drop(file);
    let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(mode));
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("failed to commit {}: {err}", path.display()));
    }
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn proc_start_ticks(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = stat.rsplit_once(") ")?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    if fields.len() < 20 {
        return None;
    }
    // Field 0 after ") " is state; field 19 is starttime (22nd field overall).
    if matches!(fields[0], "Z" | "X") {
        return None;
    }
    fields[19].parse::<u64>().ok()
}

pub fn is_live_matching_process(pid: u32, expected_start_ticks: u64) -> bool {
    proc_start_ticks(pid).is_some_and(|ticks| ticks == expected_start_ticks)
}
