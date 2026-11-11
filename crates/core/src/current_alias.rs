//! Derived compatibility path for legacy callers; never a runtime authority.
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{m2_generation_state, LocalCoreRoots};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn real_directory(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(invalid(
            "Core compatibility alias parent or generation is not a real directory",
        ));
    }
    Ok(())
}

fn safe_old_target(target: &Path) -> bool {
    let mut parts = target.components();
    matches!(parts.next(), Some(Component::Normal(first)) if first == "generations")
        && matches!(parts.next(), Some(Component::Normal(generation)) if generation
            .to_str()
            .is_some_and(|name| m2_generation_state::validate_generation_identity(name, "existing Core compatibility alias").is_ok()))
        && parts.next().is_none()
}

/// Synchronize only this derived alias. Caller must hold the Core state writer
/// lock when a generation transition is in progress. Never follow the alias or
/// replace a non-symlink / foreign symlink under this compatibility name.
pub(super) fn sync(roots: &LocalCoreRoots, generation: &str) -> io::Result<()> {
    m2_generation_state::validate_generation_identity(generation, "active Core generation")
        .map_err(|_| invalid("Core compatibility alias generation identity is invalid"))?;
    real_directory(&roots.generation_root)?;
    let parent = roots
        .generation_root
        .parent()
        .ok_or_else(|| invalid("Core compatibility alias has no parent"))?;
    real_directory(parent)?;
    real_directory(&roots.generation_root.join(generation))?;
    let alias = parent.join("current");
    let desired = PathBuf::from("generations").join(generation);
    match fs::symlink_metadata(&alias) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            // Creation is atomic and does not overwrite an independently
            // created user file or directory.
            symlink(&desired, &alias)?;
        }
        Err(err) => return Err(err),
        Ok(meta) if meta.file_type().is_symlink() => {
            let previous = fs::read_link(&alias)?;
            if previous == desired {
                return Ok(());
            }
            if !safe_old_target(&previous) {
                return Err(invalid(
                    "Core compatibility alias points outside owned generations",
                ));
            }
            for _ in 0..32 {
                let seq = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
                let temporary = parent.join(format!(".current-alias-{}-{seq}", std::process::id()));
                match symlink(&desired, &temporary) {
                    Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(err) => return Err(err),
                    Ok(()) => {
                        let result = fs::rename(&temporary, &alias);
                        if result.is_err() {
                            let _ = fs::remove_file(&temporary);
                        }
                        result?;
                        break;
                    }
                }
            }
            if fs::read_link(&alias)? != desired {
                return Err(invalid(
                    "Core compatibility alias could not be atomically replaced",
                ));
            }
        }
        Ok(_) => return Err(invalid("Core compatibility alias is not a symlink")),
    }
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

/// Repair an interrupted post-commit alias update on the next normal Core
/// dispatch. The authoritative activation state, not the alias, selects Core.
pub(super) fn repair_from_state(roots: &LocalCoreRoots) -> io::Result<()> {
    let paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(|error| io::Error::other(error.to_string()))?;
    let pointer = m2_generation_state::read_pointer_state(&paths)
        .map_err(|error| io::Error::other(error.to_string()))?;
    let Some(pointer) = pointer else {
        return Ok(());
    };
    let alias = roots
        .generation_root
        .parent()
        .ok_or_else(|| invalid("missing compatibility parent"))?
        .join("current");
    if fs::read_link(&alias)
        .is_ok_and(|target| target == PathBuf::from("generations").join(&pointer.current))
    {
        return Ok(());
    }
    let _lock = m2_generation_state::acquire_activation_lock(&paths)
        .map_err(|error| io::Error::other(error.to_string()))?;
    let pointer = m2_generation_state::read_pointer_state(&paths)
        .map_err(|error| io::Error::other(error.to_string()))?;
    if let Some(pointer) = pointer {
        sync(roots, &pointer.current)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
        roots: LocalCoreRoots,
    }
    impl Fixture {
        fn new(label: &str) -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "codex-current-alias-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&root).unwrap();
            let core = root.join("core");
            let generations = core.join("generations");
            let state = root.join("state");
            fs::create_dir_all(&generations).unwrap();
            fs::create_dir(&state).unwrap();
            Self {
                root: root.clone(),
                roots: LocalCoreRoots {
                    generation_root: generations,
                    state_root: state.clone(),
                    config_dir: state.join("config"),
                    resolver_path: root.join("resolv.conf"),
                    cert_file: root.join("cert.pem"),
                    cert_dir: root.join("certs"),
                    openssl: root.join("openssl"),
                    curl: root.join("curl"),
                },
            }
        }
        fn add(&self, generation: &str) {
            fs::create_dir(self.roots.generation_root.join(generation)).unwrap();
        }
        fn alias(&self) -> PathBuf {
            self.roots.generation_root.parent().unwrap().join("current")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    const FIRST: &str = "full-v0.167.0-aaaaaaa-s1";
    const SECOND: &str = "full-v0.167.1-bbbbbbb-s2";

    #[test]
    fn creates_repoints_and_preserves_signed_generation_roots() {
        let f = Fixture::new("repoint");
        f.add(FIRST);
        f.add(SECOND);
        sync(&f.roots, FIRST).unwrap();
        assert_eq!(
            fs::read_link(f.alias()).unwrap(),
            PathBuf::from("generations").join(FIRST)
        );
        sync(&f.roots, SECOND).unwrap();
        assert_eq!(
            fs::read_link(f.alias()).unwrap(),
            PathBuf::from("generations").join(SECOND)
        );
        sync(&f.roots, FIRST).unwrap();
        sync(&f.roots, FIRST).unwrap();
        assert_eq!(
            fs::read_link(f.alias()).unwrap(),
            PathBuf::from("generations").join(FIRST)
        );
        assert!(f.roots.generation_root.join(SECOND).is_dir());
        assert!(f.roots.generation_root.join(FIRST).is_dir());
    }

    #[test]
    fn refuses_foreign_symlinks_regular_files_and_invalid_generations() {
        let f = Fixture::new("reject");
        f.add(FIRST);
        f.add(SECOND);
        let alias = f.alias();
        fs::write(&alias, b"owned content").unwrap();
        assert!(sync(&f.roots, FIRST).is_err());
        assert_eq!(fs::read(&alias).unwrap(), b"owned content");
        fs::remove_file(&alias).unwrap();
        symlink("../../external", &alias).unwrap();
        assert!(sync(&f.roots, SECOND).is_err());
        assert_eq!(
            fs::read_link(&alias).unwrap(),
            PathBuf::from("../../external")
        );
        fs::remove_file(&alias).unwrap();
        assert!(sync(&f.roots, "../outside").is_err());
        assert!(!alias.exists());
        assert!(sync(&f.roots, "full-v0.167.2-ccccccc-s3").is_err());
        assert!(!alias.exists());
    }

    #[test]
    fn recovery_uses_authoritative_state_not_stale_link() {
        let f = Fixture::new("recover");
        f.add(FIRST);
        f.add(SECOND);
        let paths = m2_generation_state::CoreStatePaths::new(&f.roots.state_root).unwrap();
        let before = m2_generation_state::plan_initial_pointer_state_with_key(
            FIRST,
            crate::ReleasePublicKey([1_u8; 32]),
        )
        .unwrap();
        m2_generation_state::activate_pointer_state(&paths, None, &before).unwrap();
        repair_from_state(&f.roots).unwrap();
        assert_eq!(
            fs::read_link(f.alias()).unwrap(),
            PathBuf::from("generations").join(FIRST)
        );
        let after = m2_generation_state::plan_activation_pointer_state_with_key(
            &before,
            SECOND,
            crate::ReleasePublicKey([1_u8; 32]),
        )
        .unwrap();
        m2_generation_state::activate_pointer_state(&paths, Some(&before), &after).unwrap();
        assert_eq!(
            fs::read_link(f.alias()).unwrap(),
            PathBuf::from("generations").join(FIRST)
        );
        repair_from_state(&f.roots).unwrap();
        assert_eq!(
            fs::read_link(f.alias()).unwrap(),
            PathBuf::from("generations").join(SECOND)
        );
    }
}
