//! Explicit, default-off local frontend trial; the signed runtime remains the server.
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const DIRECTORY: &str = ".local/lib/codex/profile-tui-preview/424e0950";
const NATIVE_SHA256: &str = env!("PROFILE_PREVIEW_NATIVE_SHA256");
const MANAGER_SHA256: &str = env!("PROFILE_PREVIEW_MANAGER_SHA256");
const STABLE_SHA256: &str = "8acc8219507780095a3a03e0cd0f8bf4b6346bbe7bd4b41b0a8f761851978cfa";

fn invalid() -> io::Error {
    io::Error::other("experimental UI asset is unavailable or has changed")
}

fn directory() -> io::Result<PathBuf> {
    let home = super::required_absolute_env_path("HOME").map_err(|_| invalid())?;
    Ok(home.join(DIRECTORY))
}

fn executable(path: &Path, digest: &str, openssl: &Path) -> io::Result<()> {
    let uid = fs::metadata("/proc/self")?.uid();
    let directory = path.parent().ok_or_else(invalid)?;
    for parent in directory.ancestors() {
        let metadata = fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(invalid());
        }
    }
    let parent = fs::symlink_metadata(directory)?;
    let metadata = fs::symlink_metadata(path)?;
    if parent.uid() != uid
        || parent.permissions().mode() & 0o777 != 0o700
        || !metadata.is_file()
        || metadata.uid() != uid
        || metadata.permissions().mode() & 0o022 != 0
        || metadata.permissions().mode() & 0o111 == 0
        || super::openssl_sha256(openssl, path).map_err(|_| invalid())? != digest
    {
        return Err(invalid());
    }
    Ok(())
}

fn native_args(socket: &Path, original: &[OsString]) -> io::Result<Vec<OsString>> {
    let socket = socket
        .to_str()
        .filter(|_| socket.is_absolute())
        .ok_or_else(invalid)?;
    let mut args = vec!["--remote".into(), format!("unix://{socket}").into()];
    args.extend_from_slice(original);
    Ok(args)
}

fn supported_options(args: &[OsString]) -> bool {
    !args.iter().any(|arg| {
        arg.to_str().is_some_and(|arg| {
            arg == "--add-dir"
                || arg.starts_with("--add-dir=")
                || arg == "--worktree"
                || arg.starts_with("--worktree=")
        })
    })
}

pub(crate) fn launch(
    socket: &Path,
    profile: &Path,
    original: &[OsString],
    resolver: &Path,
    config: &Path,
    plan: &super::TermuxBaseEnvPlan,
) -> Option<io::Error> {
    if !supported_options(original) {
        eprintln!("Using installed Codex for these workspace options.");
        return None;
    }
    let prepare = || -> io::Result<_> {
        let roots = super::LocalCoreRoots::from_environment().map_err(|_| invalid())?;
        let native = directory()?.join("native");
        executable(&native, NATIVE_SHA256, &roots.openssl)?;
        let core = super::validated_core_entrypoint()?;
        let args = native_args(socket, original)?;
        let fds = super::RuntimeFdSources::open(resolver, config)?;
        let mut command = Command::new(native);
        command
            .args(args)
            .env("CODEX_HOME", profile)
            .env("CODEX_PROFILE_CORE", core);
        super::apply_child_env_plan_and_fence(&mut command, Some(plan));
        fds.configure(&mut command);
        Ok((command, fds))
    };
    match prepare() {
        Ok((mut command, _fds)) => Some(command.exec()),
        Err(_) => {
            eprintln!("Experimental UI unavailable; using installed Codex.");
            None
        }
    }
}

fn restore(roots: &super::LocalCoreRoots) -> io::Result<()> {
    let saved = directory()?.join("stable-core");
    executable(&saved, STABLE_SHA256, &roots.openssl)?;
    let paths = super::m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(|_| invalid())?;
    let _lock =
        super::m2_generation_state::acquire_activation_lock(&paths).map_err(|_| invalid())?;
    let state = super::m2_generation_state::read_pointer_state(&paths)
        .map_err(|_| invalid())?
        .ok_or_else(invalid)?;
    let (_, active) = super::verify_installed_local_release(
        roots,
        &state.current,
        state.current_key,
        "preview rollback active generation does not match current",
    )
    .map_err(|_| invalid())?;
    if active.manifest.core_artifact_digest != STABLE_SHA256 {
        return Err(invalid());
    }
    let installed = super::stable_core_entrypoint_path(roots).map_err(|_| invalid())?;
    let current = fs::metadata("/proc/self/exe")?;
    let installed_metadata = fs::symlink_metadata(&installed)?;
    if !installed_metadata.is_file() {
        return Err(invalid());
    }
    if (current.ino(), current.dev()) != (installed_metadata.ino(), installed_metadata.dev())
        && super::openssl_sha256(&roots.openssl, &installed).map_err(|_| invalid())?
            != STABLE_SHA256
    {
        return Err(invalid());
    }
    super::install_core_entrypoint(roots, &saved, STABLE_SHA256).map_err(|_| invalid())
}

pub(crate) fn handle(args: &[OsString]) -> Option<i32> {
    let snapshot = matches!(args, [termux, endpoint] if termux == "termux"
        && (endpoint == "__profile-snapshot-v1" || endpoint == "__task-snapshot-v1"));
    let update = args.first().is_some_and(|arg| arg == "update");
    if !snapshot && !update {
        return None;
    }
    let execute = || -> io::Result<i32> {
        let roots = super::LocalCoreRoots::from_environment().map_err(|_| invalid())?;
        if update {
            restore(&roots)?;
            if args == [OsStr::new("update"), OsStr::new("--rollback")] {
                println!("Restored Codex release 40; activation state is unchanged.");
                return Ok(0);
            }
            let core = super::stable_core_entrypoint_path(&roots).map_err(|_| invalid())?;
            return Err(Command::new(core).args(args).exec());
        }
        let manager = directory()?.join("manager");
        executable(&manager, MANAGER_SHA256, &roots.openssl)?;
        let core = super::validated_core_entrypoint()?;
        Err(Command::new(manager)
            .arg(&args[1])
            .env(super::MANAGER_CORE_API_ENV, super::MANAGER_CORE_API)
            .env(super::MANAGER_CORE_ENTRYPOINT_ENV, core)
            .env_remove(super::CODEX_SQLITE_HOME_ENV)
            .exec())
    };
    Some(match execute() {
        Ok(code) => code,
        Err(_) => {
            eprintln!("codex: experimental preview operation failed");
            1
        }
    })
}

#[cfg(test)]
#[path = "live_preview_tests.rs"]
mod tests;
