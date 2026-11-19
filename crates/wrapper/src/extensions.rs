//! Opt-in Wrapper settings; bare Codex retains its upstream-owned execution path.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{
    DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::fs_util;

const SETTINGS_MAX_BYTES: u64 = 16 * 1024;
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WrapperSettings {
    manager: bool,
    tui: bool,
    provider: bool,
    pub(super) signed_update: bool,
}

impl WrapperSettings {
    fn parse(source: &str) -> io::Result<Self> {
        let mut settings = Self::default();
        let mut seen = 0u8;
        for line in source.lines() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| invalid("expected key = true|false"))?;
            let key = key.trim();
            let value = match value.trim() {
                "true" => true,
                "false" => false,
                _ => return Err(invalid("settings values must be TOML booleans")),
            };
            let (index, destination) = match key {
                "manager" => (0, &mut settings.manager),
                "tui" => (1, &mut settings.tui),
                "provider" => (2, &mut settings.provider),
                "signed_update" => (3, &mut settings.signed_update),
                _ => return Err(invalid("unknown wrapper.toml setting")),
            };
            if seen & (1 << index) != 0 {
                return Err(invalid("duplicate wrapper.toml setting"));
            }
            seen |= 1 << index;
            *destination = value;
        }
        Ok(settings)
    }

    pub(super) fn read(root: &Path) -> io::Result<Self> {
        Self::read_with_reset(root, false)
    }

    fn read_with_reset(root: &Path, allow_malformed: bool) -> io::Result<Self> {
        if !fs_util::wrapper_safe_absolute_path(root) {
            return Err(invalid("wrapper state root must be a safe absolute path"));
        }
        // Missing configuration is the zero-write, no-addon default.
        let expected_uid = fs_util::current_uid()?;
        for ancestor in root.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(meta) if meta.file_type().is_dir() && !meta.file_type().is_symlink() => {}
                Ok(_) => {
                    return Err(invalid(
                        "wrapper state root ancestor must be a real directory",
                    ))
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
        }
        match fs::symlink_metadata(root) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
            Ok(meta)
                if meta.file_type().is_dir()
                    && !meta.file_type().is_symlink()
                    && meta.uid() == expected_uid
                    && meta.permissions().mode() & 0o022 == 0 => {}
            Ok(_) => {
                return Err(invalid(
                    "wrapper state root must be an owned, non-writable real directory",
                ))
            }
        }
        let path = root.join("wrapper.toml");
        let bytes = match fs_util::read_bounded_regular(
            &path,
            SETTINGS_MAX_BYTES,
            true,
            "wrapper.toml must be a bounded, owned, non-writable regular file",
        ) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) if allow_malformed && err.kind() == io::ErrorKind::InvalidData => {
                // The explicit reset can discard oversized contents without
                // reading them, but still requires a safe file to replace.
                let meta = fs::symlink_metadata(&path)?;
                if meta.is_file()
                    && meta.uid() == expected_uid
                    && meta.permissions().mode() & 0o022 == 0
                    && meta.len() > SETTINGS_MAX_BYTES
                {
                    return Ok(Self::default());
                }
                return Err(err);
            }
            Err(err) => return Err(err),
        };
        let settings = std::str::from_utf8(&bytes)
            .map_err(|_| invalid("wrapper.toml must be UTF-8"))
            .and_then(Self::parse);
        if allow_malformed {
            Ok(settings.unwrap_or_default())
        } else {
            settings
        }
    }

    pub(super) fn clear_addons(&mut self) {
        self.manager = false;
        self.tui = false;
        self.provider = false;
    }

    pub(super) fn set_modules(&mut self, names: &str, enabled: bool) -> io::Result<()> {
        let mut seen = 0u8;
        for name in names.split(',') {
            let (index, target) = match name {
                "manager" => (0, &mut self.manager),
                "tui" => (1, &mut self.tui),
                "provider" => (2, &mut self.provider),
                _ => return Err(invalid("modules must be manager,tui,provider")),
            };
            if seen & (1 << index) != 0 {
                return Err(invalid("module listed more than once"));
            }
            seen |= 1 << index;
            *target = enabled;
        }
        Ok(())
    }

    pub(super) fn addons_label(self) -> String {
        let mut addons = Vec::new();
        if self.manager {
            addons.push("manager");
        }
        if self.tui {
            addons.push("tui");
        }
        if self.provider {
            addons.push("provider");
        }
        if addons.is_empty() {
            "none".to_owned()
        } else {
            addons.join(",")
        }
    }

    // No qualified addon binary is installed in Phase 2.1.
    pub(super) fn unavailable_addons(self) -> Option<String> {
        (self.addons_label() != "none").then(|| self.addons_label())
    }

    fn serialize(self) -> String {
        format!(
            "# codex-wrapper optional modules\nmanager = {}\ntui = {}\nprovider = {}\nsigned_update = {}\n",
            self.manager, self.tui, self.provider, self.signed_update
        )
    }

    pub(super) fn update<F>(root: &Path, allow_malformed: bool, modify: F) -> io::Result<Self>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        prepare_parent_dirs(root)?;
        fs_util::private_dir(root)?;
        // Dedicated settings lock avoids blocking on state_root generation activation locks.
        let expected_uid = fs_util::current_uid()?;
        let lock_path = root.join(".wrapper-settings.lock");
        match fs::symlink_metadata(&lock_path) {
            Ok(meta)
                if meta.file_type().is_file()
                    && !meta.file_type().is_symlink()
                    && meta.uid() == expected_uid => {}
            Ok(_) => {
                return Err(invalid(
                    "wrapper settings lock path must be an owned regular file",
                ))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(fs_util::O_NOFOLLOW)
            .open(&lock_path)?;
        let lease_meta = lease.metadata()?;
        if !lease_meta.file_type().is_file() || lease_meta.uid() != expected_uid {
            return Err(invalid(
                "wrapper settings lock must be an owned regular file",
            ));
        }
        fs_util::flock_fd(&lease, fs_util::LOCK_EX)?;

        let mut settings = Self::read_with_reset(root, allow_malformed)?;
        modify(&mut settings)?;
        let dest = root.join("wrapper.toml");
        let sequence = WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tmp_path = root.join(format!(
            ".wrapper.toml.{}.{sequence}.tmp",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(fs_util::O_NOFOLLOW)
            .open(&tmp_path)?;
        let result = (|| -> io::Result<()> {
            file.write_all(settings.serialize().as_bytes())?;
            file.sync_all()?;
            fs::rename(&tmp_path, &dest)?;
            File::open(root)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        result?;
        Ok(settings)
    }
}

// Only the three optional state-root ancestors may be created here
// ($HOME/.local, share, codex). A missing HOME or a symlinked ancestor fails
// closed rather than manufacturing a substitute account home.
fn prepare_parent_dirs(root: &Path) -> io::Result<()> {
    if !fs_util::wrapper_safe_absolute_path(root) {
        return Err(invalid(
            "wrapper settings root must be an absolute safe path",
        ));
    }
    let expected_uid = fs_util::current_uid()?;
    // Validate the *entire* ancestor chain before creating any missing
    // directory, not just the nearest existing parent.
    for ancestor in root.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_dir() && !meta.file_type().is_symlink() => (),
            Ok(_) => return Err(invalid("wrapper settings path is not a real directory")),
            Err(err) if err.kind() == io::ErrorKind::NotFound => (),
            Err(err) => return Err(err),
        }
    }
    let mut cursor = root
        .parent()
        .ok_or_else(|| invalid("wrapper settings root has no parent"))?;
    let mut missing = Vec::new();
    for _ in 0..3 {
        match fs::symlink_metadata(cursor) {
            Ok(meta) if meta.file_type().is_dir() && !meta.file_type().is_symlink() => {
                break;
            }
            Ok(_) => return Err(invalid("wrapper settings ancestor is not a real directory")),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                missing.push(cursor.to_path_buf());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| invalid("wrapper settings ancestor is absent"))?;
            }
            Err(err) => return Err(err),
        }
    }
    let anchor = fs::symlink_metadata(cursor)?;
    if !anchor.file_type().is_dir() || anchor.file_type().is_symlink() {
        return Err(invalid("wrapper settings ancestor is not a real directory"));
    }
    let codex_dir = root.parent();
    for dir in missing.iter().rev() {
        let mode = if Some(dir.as_path()) == codex_dir {
            0o700
        } else {
            // Shared XDG parents (~/.local, ~/.local/share) keep standard 0o755 rather than 0o700.
            0o755
        };
        match fs::DirBuilder::new().mode(mode).create(dir) {
            Ok(()) => (),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
            Err(err) => return Err(err),
        }
        let meta = fs::symlink_metadata(dir)?;
        if !meta.file_type().is_dir()
            || meta.file_type().is_symlink()
            || meta.uid() != expected_uid
            || meta.permissions().mode() & 0o022 != 0
        {
            return Err(invalid("wrapper settings ancestor was substituted"));
        }
        if Some(dir.as_path()) == codex_dir && meta.permissions().mode() & 0o777 != 0o700 {
            return Err(invalid("wrapper private parent directory mode is unsafe"));
        }
    }
    Ok(())
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn test_root() -> std::path::PathBuf {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "codex-wrapper-extension-test-{}-{id}",
            std::process::id()
        ))
    }

    #[test]
    fn absent_file_is_read_only_bare_default() {
        let root = test_root();
        assert_eq!(
            WrapperSettings::read(&root).unwrap(),
            WrapperSettings::default()
        );
        assert!(!root.exists());
    }

    #[test]
    fn strict_config_rejects_unknown_duplicate_non_boolean_and_oversized_inputs() {
        for invalid_text in [
            "manager = true\nmanager = false\n",
            "provider = 1\n",
            "unknown = true\n",
            "tui = 'true'\n",
            "[modules]\nmanager=true\n",
        ] {
            assert!(WrapperSettings::parse(invalid_text).is_err());
        }
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("wrapper.toml"),
            vec![b'x'; SETTINGS_MAX_BYTES as usize + 1],
        )
        .unwrap();
        assert!(WrapperSettings::read(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_update_roundtrip_and_invalid_module_preserve_previous_state() {
        let root = test_root();
        let settings = WrapperSettings::update(&root, false, |settings| {
            settings.set_modules("manager,provider", true)?;
            settings.signed_update = true;
            Ok(())
        })
        .unwrap();
        assert_eq!(settings.addons_label(), "manager,provider");
        assert_eq!(WrapperSettings::read(&root).unwrap(), settings);
        let metadata = fs::metadata(root.join("wrapper.toml")).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        let err = WrapperSettings::update(&root, false, |settings| {
            settings.set_modules("manager,unknown", false)
        })
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(WrapperSettings::read(&root).unwrap(), settings);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn symlink_config_is_rejected_without_following_or_overwriting_it() {
        let root = test_root();
        fs_util::private_dir(&root).unwrap();
        let target = root.join("target");
        fs::write(&target, "manager = false\n").unwrap();
        symlink(&target, root.join("wrapper.toml")).unwrap();
        assert!(WrapperSettings::read(&root).is_err());
        assert!(WrapperSettings::update(&root, false, |_| Ok(())).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "manager = false\n");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn initializes_missing_settings_ancestors_without_creating_other_roots() {
        let base = test_root();
        fs::create_dir_all(&base).unwrap();
        let state = base.join(".local/share/codex/wrapper");
        assert!(!state.exists());
        let new = WrapperSettings::update(&state, false, |settings| {
            settings.set_modules("manager", true)
        })
        .unwrap();
        assert_eq!(new.addons_label(), "manager");
        assert_eq!(WrapperSettings::read(&state).unwrap(), new);
        assert!(state.join("wrapper.toml").is_file());
        assert!(!state.join("config/snapshots").exists());
        let codex_mode = fs::metadata(base.join(".local/share/codex"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(codex_mode, 0o700);
        let share_mode = fs::metadata(base.join(".local/share"))
            .unwrap()
            .permissions()
            .mode()
            & 0o022;
        assert_eq!(share_mode, 0);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn settings_update_does_not_collide_with_state_root_activation_flock() {
        let root = test_root();
        fs_util::private_dir(&root).unwrap();
        let dir_handle = File::open(&root).unwrap();
        fs_util::flock_fd(&dir_handle, fs_util::LOCK_EX | fs_util::LOCK_NB).unwrap();
        let updated = WrapperSettings::update(&root, false, |s| {
            s.signed_update = true;
            Ok(())
        })
        .unwrap();
        assert!(updated.signed_update);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_group_or_world_writable_config() {
        let root = test_root();
        fs_util::private_dir(&root).unwrap();
        let path = root.join("wrapper.toml");
        fs::write(&path, "manager = false\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o664)).unwrap();
        assert!(WrapperSettings::read(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_symlinked_settings_ancestor_without_writes() {
        let base = test_root();
        fs::create_dir_all(&base).unwrap();
        let other = base.join("other");
        fs::create_dir_all(&other).unwrap();
        symlink(&other, base.join(".local")).unwrap();
        let state = base.join(".local/share/codex/wrapper");
        assert!(WrapperSettings::update(&state, false, |_| Ok(())).is_err());
        assert!(!other.join("share").exists());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn rejects_symlinked_home_before_creating_any_settings_parent() {
        let base = test_root();
        fs::create_dir_all(&base).unwrap();
        let real_home = base.join("real-home");
        fs::create_dir_all(&real_home).unwrap();
        symlink(&real_home, base.join("home")).unwrap();
        let settings = base.join("home/.local/share/codex/wrapper");
        assert!(WrapperSettings::update(&settings, false, |_| Ok(())).is_err());
        assert!(!real_home.join(".local").exists());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn default_and_explicit_opt_in_are_distinct() {
        let mut settings = WrapperSettings::default();
        assert_eq!(settings.addons_label(), "none");
        assert!(settings.unavailable_addons().is_none());
        settings.set_modules("tui", true).unwrap();
        assert_eq!(settings.unavailable_addons().as_deref(), Some("tui"));
        settings.set_modules("tui", false).unwrap();
        assert!(settings.unavailable_addons().is_none());
        assert!(settings.set_modules("manager,manager", true).is_err());
        assert!(settings.set_modules("", true).is_err());
    }
}
