use crate::paths::{
    atomic_write_private, current_uid, ensure_private_dir, require_owned_regular_file, ExtPaths,
};
use codex_daemon_client::daemon_version_via_cli;
use serde::Serialize;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileTarget {
    Default,
    Named { name: String, home: PathBuf },
}

impl ProfileTarget {
    pub fn name(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Named { name, .. } => name,
        }
    }

    pub fn codex_home(&self, paths: &ExtPaths) -> PathBuf {
        match self {
            Self::Default => paths.shared_home.clone(),
            Self::Named { home, .. } => home.clone(),
        }
    }

    pub fn shared_home_opt<'a>(&self, paths: &'a ExtPaths) -> Option<&'a Path> {
        match self {
            Self::Default => None,
            Self::Named { .. } => Some(&paths.shared_home),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub codex_home: String,
    pub is_default: bool,
    pub has_auth: bool,
    pub daemon_status: String,
    pub daemon_pid: Option<u32>,
}

pub fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!("profile name must be 1..=64 characters: '{name}'"));
    }
    let bytes = name.as_bytes();
    if !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(format!("invalid profile name: '{name}'"));
    }
    if matches!(name, "default" | "home" | "wrapper" | "termux") {
        return Err(format!("reserved profile name: '{name}'"));
    }
    Ok(())
}

fn verify_profile_meta(meta_path: &Path, name: &str) -> Result<(), String> {
    require_owned_regular_file(meta_path, "profile metadata")?;
    let content = fs::read_to_string(meta_path)
        .map_err(|e| format!("cannot read {}: {e}", meta_path.display()))?;
    if matches!(
        content.as_str(),
        "codex-manager-profile\n" | "codex-manager-profile-v2\n"
    ) || content
        .strip_prefix("codex-manager-profile-v1\nid\t")
        .and_then(|identity| identity.strip_suffix('\n'))
        == Some(name)
    {
        Ok(())
    } else {
        Err(format!(
            "unrecognized or mismatched profile metadata: {}",
            meta_path.display()
        ))
    }
}

pub fn read_default_profile(paths: &ExtPaths) -> Result<Option<String>, String> {
    if !paths.default_profile_file.exists()
        && fs::symlink_metadata(&paths.default_profile_file).is_err()
    {
        return Ok(None);
    }
    require_owned_regular_file(&paths.default_profile_file, "default-profile")?;
    let raw = fs::read_to_string(&paths.default_profile_file)
        .map_err(|e| format!("cannot read default-profile: {e}"))?;
    let mut lines = raw.lines();
    let header = lines.next().unwrap_or("");
    let entry = lines.next().unwrap_or("");
    if header != "codex-manager-default-profile" || lines.next().is_some() {
        return Err("malformed default-profile file".to_owned());
    }
    let Some(target) = entry.strip_prefix("profile\t") else {
        return Err("malformed default-profile entry".to_owned());
    };
    if target == "default" || target == "home" {
        return Ok(None);
    }
    validate_profile_name(target)?;
    Ok(Some(target.to_owned()))
}

pub fn write_default_profile(paths: &ExtPaths, target: Option<&str>) -> Result<(), String> {
    match target {
        None | Some("default") | Some("home") => {
            if fs::symlink_metadata(&paths.default_profile_file).is_ok() {
                require_owned_regular_file(&paths.default_profile_file, "default-profile")?;
                fs::remove_file(&paths.default_profile_file)
                    .map_err(|e| format!("cannot clear default-profile: {e}"))?;
            }
            Ok(())
        }
        Some(name) => {
            let _ = resolve_profile(paths, name)?;
            let body = format!("codex-manager-default-profile\nprofile\t{name}\n");
            atomic_write_private(&paths.default_profile_file, body.as_bytes(), 0o600)
        }
    }
}

pub fn resolve_profile(paths: &ExtPaths, name: &str) -> Result<ProfileTarget, String> {
    if matches!(name, "default" | "home") {
        return Ok(ProfileTarget::Default);
    }
    validate_profile_name(name)?;
    let profile_dir = paths.profiles_dir.join(name);
    let profile_home = profile_dir.join("home");
    let meta_path = profile_dir.join("profile.meta");
    let uid = current_uid();
    for dir in [&profile_dir, &profile_home] {
        let meta = fs::symlink_metadata(dir)
            .map_err(|_| format!("profile '{name}' does not exist ({})", dir.display()))?;
        if meta.file_type().is_symlink() || !meta.is_dir() || meta.uid() != uid {
            return Err(format!(
                "unsafe or unowned profile directory: {}",
                dir.display()
            ));
        }
    }
    verify_profile_meta(&meta_path, name)?;
    Ok(ProfileTarget::Named {
        name: name.to_owned(),
        home: profile_home,
    })
}

pub fn resolve_default_target(paths: &ExtPaths) -> Result<ProfileTarget, String> {
    if let Some(saved) = read_default_profile(paths)? {
        resolve_profile(paths, &saved)
    } else {
        Ok(ProfileTarget::Default)
    }
}

/// Determine which profile matches a given `$CODEX_HOME` path.
pub fn profile_name_for_codex_home(paths: &ExtPaths, codex_home: &Path) -> String {
    if codex_home == paths.shared_home {
        return "default".to_owned();
    }
    if let Ok(rel) = codex_home.strip_prefix(&paths.profiles_dir) {
        let comps: Vec<_> = rel.components().collect();
        if comps.len() == 2 && comps[1].as_os_str() == "home" {
            if let Some(name) = comps[0].as_os_str().to_str() {
                return name.to_owned();
            }
        }
    }
    codex_home.display().to_string()
}

pub fn effective_current_profile(paths: &ExtPaths) -> Result<String, String> {
    if let Some(env_home) = env::var_os("CODEX_HOME").filter(|v| !v.is_empty()) {
        return Ok(profile_name_for_codex_home(paths, Path::new(&env_home)));
    }
    Ok(read_default_profile(paths)?.unwrap_or_else(|| "default".to_owned()))
}

pub fn is_daemon_running(paths: &ExtPaths, target: &ProfileTarget) -> (String, Option<u32>) {
    let home = target.codex_home(paths);
    let sock = home.join("app-server-control/app-server-control.sock");
    if fs::symlink_metadata(&sock).is_err() {
        return ("stopped".to_owned(), None);
    }
    if !paths.codex_bin.is_file() {
        return ("unknown".to_owned(), None);
    }
    match daemon_version_via_cli(&paths.codex_bin, &home, target.shared_home_opt(paths)) {
        Ok(info) => (info.status, info.pid),
        // Control socket exists, so fail closed ("error") if daemon inspection fails
        // rather than misreporting "stopped" and permitting destructive rename/delete.
        Err(_) => ("error".to_owned(), None),
    }
}

pub fn list_profiles(paths: &ExtPaths, query_daemons: bool) -> Result<Vec<ProfileInfo>, String> {
    let default_name = read_default_profile(paths)?.unwrap_or_else(|| "default".to_owned());
    let mut out = Vec::new();

    let (def_status, def_pid) = if query_daemons {
        is_daemon_running(paths, &ProfileTarget::Default)
    } else {
        ("unchecked".to_owned(), None)
    };
    out.push(ProfileInfo {
        name: "default".to_owned(),
        codex_home: paths.shared_home.display().to_string(),
        is_default: default_name == "default",
        has_auth: paths.shared_home.join("auth.json").is_file(),
        daemon_status: def_status,
        daemon_pid: def_pid,
    });

    if !paths.profiles_dir.exists() {
        return Ok(out);
    }
    let mut names = Vec::new();
    for entry in
        fs::read_dir(&paths.profiles_dir).map_err(|e| format!("cannot read profiles dir: {e}"))?
    {
        let entry = entry.map_err(|e| format!("profiles dir entry error: {e}"))?;
        let fname = entry.file_name();
        let Some(name) = fname.to_str() else {
            continue;
        };
        if validate_profile_name(name).is_ok() && resolve_profile(paths, name).is_ok() {
            names.push(name.to_owned());
        }
    }
    names.sort();

    for name in names {
        let target = resolve_profile(paths, &name)?;
        let home = target.codex_home(paths);
        let (daemon_status, daemon_pid) = if query_daemons {
            is_daemon_running(paths, &target)
        } else {
            ("unchecked".to_owned(), None)
        };
        out.push(ProfileInfo {
            is_default: default_name == name,
            has_auth: home.join("auth.json").is_file(),
            codex_home: home.display().to_string(),
            name,
            daemon_status,
            daemon_pid,
        });
    }
    Ok(out)
}

pub fn create_profile(paths: &ExtPaths, name: &str) -> Result<ProfileTarget, String> {
    validate_profile_name(name)?;
    ensure_private_dir(&paths.manager_dir)?;
    ensure_private_dir(&paths.profiles_dir)?;
    let profile_dir = paths.profiles_dir.join(name);
    if fs::symlink_metadata(&profile_dir).is_ok() {
        return Err(format!("profile '{name}' already exists"));
    }
    let profile_home = profile_dir.join("home");
    ensure_private_dir(&profile_dir)?;
    ensure_private_dir(&profile_home)?;
    let meta_path = profile_dir.join("profile.meta");
    atomic_write_private(&meta_path, b"codex-manager-profile\n", 0o600)?;
    Ok(ProfileTarget::Named {
        name: name.to_owned(),
        home: profile_home,
    })
}

pub fn rename_profile(paths: &ExtPaths, old_name: &str, new_name: &str) -> Result<(), String> {
    let old_target = resolve_profile(paths, old_name)?;
    if matches!(old_target, ProfileTarget::Default) {
        return Err("cannot rename the built-in 'default' profile".to_owned());
    }
    validate_profile_name(new_name)?;
    let new_dir = paths.profiles_dir.join(new_name);
    if fs::symlink_metadata(&new_dir).is_ok() {
        return Err(format!("target profile '{new_name}' already exists"));
    }
    let (status, _) = is_daemon_running(paths, &old_target);
    if status != "stopped" {
        return Err(format!(
            "profile '{old_name}' daemon state is '{status}' (not stopped); stop it before renaming"
        ));
    }
    let old_dir = paths.profiles_dir.join(old_name);
    fs::rename(&old_dir, &new_dir)
        .map_err(|e| format!("failed to rename profile '{old_name}' -> '{new_name}': {e}"))?;
    let meta_path = new_dir.join("profile.meta");
    atomic_write_private(&meta_path, b"codex-manager-profile\n", 0o600)?;
    if read_default_profile(paths)?.as_deref() == Some(old_name) {
        write_default_profile(paths, Some(new_name))?;
    }
    Ok(())
}

pub fn delete_profile(paths: &ExtPaths, name: &str) -> Result<(), String> {
    let target = resolve_profile(paths, name)?;
    if matches!(target, ProfileTarget::Default) {
        return Err("cannot delete the built-in 'default' profile".to_owned());
    }
    let (status, _) = is_daemon_running(paths, &target);
    if status != "stopped" {
        return Err(format!(
            "profile '{name}' daemon state is '{status}' (not stopped); stop it before deleting"
        ));
    }
    let profile_dir = paths.profiles_dir.join(name);
    fs::remove_dir_all(&profile_dir)
        .map_err(|e| format!("failed to remove profile '{name}': {e}"))?;
    if read_default_profile(paths)?.as_deref() == Some(name) {
        write_default_profile(paths, None)?;
    }
    Ok(())
}

pub fn merge_shared_extra_files(existing: Option<&str>, required: &str) -> String {
    let mut items: Vec<&str> = existing
        .unwrap_or("")
        .split(':')
        .filter(|s| !s.is_empty())
        .collect();
    if !items.contains(&required) {
        items.push(required);
    }
    items.join(":")
}

pub fn apply_profile_env(cmd: &mut Command, target: &ProfileTarget, paths: &ExtPaths) {
    match target {
        ProfileTarget::Default => {
            cmd.env_remove("CODEX_HOME");
            cmd.env_remove("CODEX_SHARED_HOME");
            cmd.env_remove("CODEX_SQLITE_HOME");
        }
        ProfileTarget::Named { home, .. } => {
            cmd.env("CODEX_HOME", home);
            cmd.env("CODEX_SHARED_HOME", &paths.shared_home);
            cmd.env("CODEX_SQLITE_HOME", &paths.shared_home);
            let merged = merge_shared_extra_files(
                env::var("CODEX_SHARED_EXTRA_FILES").ok().as_deref(),
                "hooks.json",
            );
            cmd.env("CODEX_SHARED_EXTRA_FILES", merged);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::now_millis;

    fn test_paths(prefix: &str) -> (PathBuf, ExtPaths) {
        let root =
            std::env::temp_dir().join(format!("{prefix}-{}-{}", std::process::id(), now_millis()));
        fs::create_dir_all(root.join(".codex")).unwrap();
        let paths = ExtPaths {
            prefix: root.join("usr"),
            tmpdir: root.join("tmp"),
            codex_bin: root.join("usr/bin/codex"),
            shared_home: root.join(".codex"),
            manager_dir: root.join(".local/share/codex/manager"),
            profiles_dir: root.join(".local/share/codex/manager/profiles"),
            default_profile_file: root.join(".local/share/codex/manager/default-profile"),
            sessions_dir: root.join(".local/state/codex/ext/sessions"),
            handoff_dir: root.join(".local/state/codex/ext/handoff"),
            notify_config_file: root.join(".config/codex/ext/notify.json"),
        };
        (root, paths)
    }

    #[test]
    fn profile_crud_and_meta_validation() {
        let (root, paths) = test_paths("cdx-prof");
        assert!(validate_profile_name("default").is_err());
        assert!(validate_profile_name("wrapper").is_err());
        assert!(validate_profile_name("../bad").is_err());
        assert!(validate_profile_name("szob-1").is_ok());

        let created = create_profile(&paths, "work").unwrap();
        assert_eq!(created.name(), "work");
        assert!(create_profile(&paths, "work").is_err());

        // Malformed metadata must fail closed
        let bad_dir = paths.profiles_dir.join("badmeta");
        fs::create_dir_all(bad_dir.join("home")).unwrap();
        atomic_write_private(&bad_dir.join("profile.meta"), b"invalid-header\n", 0o600).unwrap();
        assert!(resolve_profile(&paths, "badmeta").is_err());

        let work_meta = paths.profiles_dir.join("work/profile.meta");
        for metadata in [
            b"codex-manager-profile-v1\nid\twork\n".as_slice(),
            b"codex-manager-profile-v2\n",
        ] {
            atomic_write_private(&work_meta, metadata, 0o600).unwrap();
            assert!(resolve_profile(&paths, "work").is_ok());
        }
        atomic_write_private(
            &bad_dir.join("profile.meta"),
            b"codex-manager-profile-v1\nid\twork\n",
            0o600,
        )
        .unwrap();
        assert!(resolve_profile(&paths, "badmeta").is_err());

        write_default_profile(&paths, Some("work")).unwrap();
        assert_eq!(
            read_default_profile(&paths).unwrap().as_deref(),
            Some("work")
        );

        rename_profile(&paths, "work", "work2").unwrap();
        assert_eq!(
            read_default_profile(&paths).unwrap().as_deref(),
            Some("work2")
        );
        assert!(resolve_profile(&paths, "work").is_err());
        assert!(resolve_profile(&paths, "work2").is_ok());

        let list = list_profiles(&paths, false).unwrap();
        let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["default", "work2"]);

        delete_profile(&paths, "work2").unwrap();
        assert_eq!(read_default_profile(&paths).unwrap(), None);

        assert_eq!(
            merge_shared_extra_files(Some("review.config.toml"), "hooks.json"),
            "review.config.toml:hooks.json"
        );
        assert_eq!(
            merge_shared_extra_files(Some("hooks.json:review.config.toml"), "hooks.json"),
            "hooks.json:review.config.toml"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
