//! Generic shared upstream configuration profiles; account auth/state stays local.
use std::fs;
use std::io;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::Path;

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(super) fn project(config: &Path, home: &Path) -> io::Result<()> {
    let registry = config.join("profiles");
    let meta = match fs::symlink_metadata(&registry) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        result => result?,
    };
    let uid = unsafe { geteuid() };
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return Err(io::Error::other(
            "shared config registry must be private and owned",
        ));
    }
    let meta = fs::symlink_metadata(home)?;
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o022 != 0 {
        return Err(io::Error::other(
            "config profile home must be an owned directory",
        ));
    }
    let registry = registry.canonicalize()?;
    for entry in fs::read_dir(&registry)?.take(64) {
        let entry = entry?;
        let name = entry.file_name();
        let Some(profile) = name.to_str().and_then(|s| s.strip_suffix(".config.toml")) else {
            continue;
        };
        if profile.is_empty()
            || profile.len() > 128
            || !profile
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            continue;
        }
        let meta = fs::symlink_metadata(entry.path())?;
        if !meta.is_file() || meta.uid() != uid || meta.mode() & 0o077 != 0 || meta.len() > 65536 {
            continue;
        }
        // Symlink creation is atomic and no-replace; an existing user file,
        // dangling link or a racing writer is preserved byte-for-byte.
        match symlink(entry.path(), home.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    struct Root(std::path::PathBuf);
    impl Drop for Root {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn roots() -> (Root, std::path::PathBuf, std::path::PathBuf) {
        let id = super::super::TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = Root(
            std::env::temp_dir().join(format!("manager-shared-config-{}-{id}", std::process::id())),
        );
        fs::create_dir(&root.0).unwrap();
        let config = root.0.join("config");
        let home = root.0.join("home");
        fs::create_dir_all(config.join("profiles")).unwrap();
        fs::create_dir(&home).unwrap();
        fs::set_permissions(config.join("profiles"), fs::Permissions::from_mode(0o700)).unwrap();
        (root, config, home)
    }
    #[test]
    fn one_shared_profile_reaches_new_homes_and_atomic_updates_without_touching_local_config() {
        let (_root, config, home) = roots();
        let source = config.join("profiles/chatgpt.config.toml");
        fs::write(&source, "model=\"first\"\n").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(home.join("config.toml"), "# user config\n").unwrap();
        project(&config, &home).unwrap();
        assert_eq!(
            fs::read_to_string(home.join("chatgpt.config.toml")).unwrap(),
            "model=\"first\"\n"
        );
        let replacement = source.with_extension("new");
        fs::write(&replacement, "model=\"second\"\n").unwrap();
        fs::set_permissions(&replacement, fs::Permissions::from_mode(0o600)).unwrap();
        fs::rename(replacement, source).unwrap();
        project(&config, &home).unwrap();
        assert_eq!(
            fs::read_to_string(home.join("chatgpt.config.toml")).unwrap(),
            "model=\"second\"\n"
        );
        assert_eq!(
            fs::read_to_string(home.join("config.toml")).unwrap(),
            "# user config\n"
        );
    }
    #[test]
    fn existing_user_profile_and_dangling_link_win_over_shared_registration() {
        let (_root, config, home) = roots();
        let source = config.join("profiles/chatgpt.config.toml");
        fs::write(&source, "# shared\n").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(home.join("chatgpt.config.toml"), "# user choice\n").unwrap();
        project(&config, &home).unwrap();
        assert_eq!(
            fs::read_to_string(home.join("chatgpt.config.toml")).unwrap(),
            "# user choice\n"
        );
        fs::remove_file(home.join("chatgpt.config.toml")).unwrap();
        symlink("user-missing", home.join("chatgpt.config.toml")).unwrap();
        project(&config, &home).unwrap();
        assert_eq!(
            fs::read_link(home.join("chatgpt.config.toml")).unwrap(),
            Path::new("user-missing")
        );
    }
    #[test]
    fn absent_registry_is_optional_and_untrusted_sources_are_never_projected() {
        let (_root, config, home) = roots();
        fs::remove_dir(config.join("profiles")).unwrap();
        project(&config, &home).unwrap();
        symlink(&home, config.join("profiles")).unwrap();
        assert!(project(&config, &home).is_err());
        assert!(!home.join("chatgpt.config.toml").exists());
    }
}
