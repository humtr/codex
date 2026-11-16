//! Opt-in Wrapper settings; bare Codex retains its upstream-owned execution path.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::fd::AsRawFd as _;
use std::os::raw::c_int;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::fs_util;

const SETTINGS_MAX_BYTES: u64 = 16 * 1024;
const O_NOFOLLOW: c_int = 0o400000;
const LOCK_EX: c_int = 2;
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn flock(fd: c_int, operation: c_int) -> c_int;
}

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
        // Missing configuration is the zero-write, no-addon default.
        match fs::symlink_metadata(root) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
            Ok(meta) if meta.file_type().is_dir() && !meta.file_type().is_symlink() => {}
            Ok(_) => return Err(invalid("wrapper state root must be a real directory")),
        }
        let path = root.join("wrapper.toml");
        let mut file = match OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(&path)
        {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
        };
        let meta = file.metadata()?;
        if !meta.file_type().is_file()
            || meta.permissions().mode() & 0o022 != 0
            || meta.len() > SETTINGS_MAX_BYTES
        {
            return Err(invalid(
                "wrapper.toml must be a bounded, non-writable regular file",
            ));
        }
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take(SETTINGS_MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > SETTINGS_MAX_BYTES {
            return Err(invalid("wrapper.toml exceeds size limit"));
        }
        let text =
            std::str::from_utf8(&bytes).map_err(|_| invalid("wrapper.toml must be UTF-8"))?;
        Self::parse(text)
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

    pub(super) fn update<F>(root: &Path, modify: F) -> io::Result<Self>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        fs_util::private_dir(root)?;
        // A directory flock serializes read/modify/write with no new lock file.
        let lease = File::open(root)?;
        if unsafe { flock(lease.as_raw_fd(), LOCK_EX) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut settings = Self::read(root)?;
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
            .custom_flags(O_NOFOLLOW)
            .open(&tmp_path)?;
        let result = (|| -> io::Result<()> {
            file.write_all(settings.serialize().as_bytes())?;
            file.sync_all()?;
            fs::rename(&tmp_path, &dest)?;
            lease.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        result?;
        Ok(settings)
    }
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
        let settings = WrapperSettings::update(&root, |settings| {
            settings.set_modules("manager,provider", true)?;
            settings.signed_update = true;
            Ok(())
        })
        .unwrap();
        assert_eq!(settings.addons_label(), "manager,provider");
        assert_eq!(WrapperSettings::read(&root).unwrap(), settings);
        let metadata = fs::metadata(root.join("wrapper.toml")).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        let err = WrapperSettings::update(&root, |settings| {
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
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target");
        fs::write(&target, "manager = false\n").unwrap();
        symlink(&target, root.join("wrapper.toml")).unwrap();
        assert!(WrapperSettings::read(&root).is_err());
        assert!(WrapperSettings::update(&root, |_| Ok(())).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "manager = false\n");
        fs::remove_dir_all(root).unwrap();
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
