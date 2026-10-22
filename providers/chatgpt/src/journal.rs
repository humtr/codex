//! Single-current-format private lane journal, with one process writer. This is
//! a transport reconciliation record, never another native conversation store.
use crate::{cdp::private_read, protocol::Error};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn random_id() -> Result<String, Error> {
    use std::io::Read;
    let mut bytes = [0_u8; 24];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|_| Error("randomness_unavailable"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(crate) fn private_directory(path: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(m)
            if m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o777 == 0o700 =>
        {
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(Error("invalid_state_directory"))?;
            if !parent.is_dir() {
                private_directory(parent)?;
            }
            use std::os::unix::fs::DirBuilderExt;
            match fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => Ok(()),
                // Concurrent native auth commands can create the same parent
                // before either obtains its startup lock. Revalidate the winner;
                // existence alone never admits a symlink or non-private root.
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => private_directory(path),
                Err(_) => Err(Error("state_directory_unavailable")),
            }
        }
        _ => Err(Error("private_state_directory_required")),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    owner: String,
    payload: serde_json::Value,
}

/// Owner-bound flock guard shared by state and configuration writers.
/// Closing a descriptor alone does not release a lock inherited across fork.
pub(crate) struct Lease {
    pub(crate) file: File,
    writer_pid: Option<u32>,
}
impl Lease {
    pub(crate) fn writer(file: File) -> Self {
        Self {
            file,
            writer_pid: Some(std::process::id()),
        }
    }
    fn reader(file: File) -> Self {
        Self {
            file,
            writer_pid: None,
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.writer_pid == Some(std::process::id()) {
            // An inherited child/reader cannot unlock its parent's live lease.
            unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}
pub struct Journal {
    pub root: PathBuf,
    pub owner: String,
    _lease: Lease,
}
impl Journal {
    /// Read-only inspection while the running provider holds its writer lease.
    pub fn inspect(root: &Path, key: &str) -> Result<Option<serde_json::Value>, Error> {
        let metadata =
            fs::symlink_metadata(root).map_err(|_| Error("state_directory_unavailable"))?;
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o777 != 0o700
        {
            return Err(Error("private_state_directory_required"));
        }
        let owner = String::from_utf8(private_read(&root.join("owner-v1"), 128)?)
            .map_err(|_| Error("invalid_state_owner"))?;
        let reader = Self {
            root: root.into(),
            owner,
            _lease: Lease::reader(
                File::open(root).map_err(|_| Error("state_directory_unavailable"))?,
            ),
        };
        reader.read(key)
    }
    pub fn open(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(Error("absolute_state_directory_required"));
        }
        private_directory(path)?;
        // Directory locks require no mutable lockfile or stale PID cleanup.
        let lease = File::open(path).map_err(|_| Error("state_directory_unavailable"))?;
        if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(Error("provider_already_running"));
        }
        let marker = path.join("owner-v1");
        let owner = match private_read(&marker, 128) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|_| Error("invalid_state_owner"))?,
            Err(_)
                if fs::symlink_metadata(&marker)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                let owner = random_id()?;
                let mut file = File::options()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&marker)
                    .map_err(|_| Error("state_owner_unavailable"))?;
                file.write_all(owner.as_bytes())
                    .and_then(|()| file.sync_all())
                    .map_err(|_| Error("state_owner_unavailable"))?;
                File::open(path)
                    .and_then(|f| f.sync_all())
                    .map_err(|_| Error("state_owner_unavailable"))?;
                owner
            }
            Err(error) => return Err(error),
        };
        if owner.len() != 48 || !owner.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error("invalid_state_owner"));
        }
        Ok(Self {
            root: path.into(),
            owner,
            _lease: Lease::writer(lease),
        })
    }
    fn path(&self, key: &str) -> Result<PathBuf, Error> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error("invalid_state_key"));
        }
        Ok(self.root.join(format!("{key}.json")))
    }
    pub fn read<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        let path = self.path(key)?;
        match private_read(&path, 32 * 1024 * 1024) {
            Ok(bytes) => {
                let record: Record =
                    serde_json::from_slice(&bytes).map_err(|_| Error("invalid_lane_record"))?;
                if record.schema != "termux-codex-lane-v1" || record.owner != self.owner {
                    return Err(Error("foreign_lane_record"));
                }
                serde_json::from_value(record.payload)
                    .map(Some)
                    .map_err(|_| Error("invalid_lane_record"))
            }
            Err(_)
                if fs::symlink_metadata(&path)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
    /// One-time index migration/recovery only. Never reactivate a lane or accept
    /// foreign records. Ordinary startup uses the recovered work index instead
    /// of scanning every transport receipt on every process start.
    pub(crate) fn records(&self, schema: &str) -> Result<Vec<(String, serde_json::Value)>, Error> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|_| Error("state_directory_unavailable"))? {
            let entry = entry.map_err(|_| Error("state_directory_unavailable"))?;
            let name = entry.file_name();
            let Some(key) = name.to_str().and_then(|name| name.strip_suffix(".json")) else {
                continue;
            };
            if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            let value = match self.read::<serde_json::Value>(key) {
                Ok(Some(value)) => value,
                Ok(None) | Err(Error("foreign_lane_record")) => continue,
                Err(error) => return Err(error),
            };
            if value["schema"] == schema {
                records.push((key.into(), value));
            }
        }
        Ok(records)
    }
    pub fn write<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Error> {
        let path = self.path(key)?;
        let before = self.read::<serde_json::Value>(key)?;
        let payload = serde_json::to_value(value).map_err(|_| Error("invalid_lane_record"))?;
        let bytes = serde_json::to_vec(&Record {
            schema: "termux-codex-lane-v1".into(),
            owner: self.owner.clone(),
            payload,
        })
        .map_err(|_| Error("invalid_lane_record"))?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(Error("lane_record_too_large"));
        }
        match fs::symlink_metadata(&path) {
            Ok(m)
                if m.is_file()
                    && m.uid() == unsafe { libc::geteuid() }
                    && m.permissions().mode() & 0o777 == 0o600 => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            _ => return Err(Error("private_lane_record_required")),
        }
        let temporary = self.root.join(format!(
            ".write-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| Error("lane_record_unavailable"))?;
        let result = (|| {
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|_| Error("lane_record_unavailable"))?;
            if self.read::<serde_json::Value>(key)? != before {
                return Err(Error("lane_record_changed"));
            }
            fs::rename(&temporary, &path).map_err(|_| Error("lane_record_unavailable"))?;
            File::open(&self.root)
                .and_then(|f| f.sync_all())
                .map_err(|_| Error("lane_record_unavailable"))
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ending_writer_releases_lease_despite_inherited_descriptor_without_unlocking_live_readers() {
        let root = std::env::temp_dir().join(format!("journal-inherit-{}", random_id().unwrap()));
        let first = Journal::open(&root).unwrap();
        let key = "a".repeat(64);
        first.write(&key, &true).unwrap();
        assert_eq!(
            Journal::inspect(&root, &key).unwrap(),
            Some(serde_json::json!(true))
        );
        assert!(matches!(
            Journal::open(&root),
            Err(Error("provider_already_running"))
        ));
        // A fork child's copied object may close its descriptor, but may not
        // explicitly unlock the actual parent's still-live writer.
        let child_copy = Journal {
            root: root.clone(),
            owner: first.owner.clone(),
            _lease: Lease {
                file: first._lease.file.try_clone().unwrap(),
                writer_pid: Some(0),
            },
        };
        drop(child_copy);
        assert!(matches!(
            Journal::open(&root),
            Err(Error("provider_already_running"))
        ));
        // dup models the same open file description retained by fork before exec.
        let inherited = first._lease.file.try_clone().unwrap();
        drop(first);
        let second = Journal::open(&root).unwrap();
        drop(inherited);
        assert!(matches!(
            Journal::open(&root),
            Err(Error("provider_already_running"))
        ));
        drop(second);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn journal_has_one_writer_survives_restart_and_preserves_foreign_entries() {
        let root = std::env::temp_dir().join(format!("chat-journal-{}", random_id().unwrap()));
        let key = "a".repeat(64);
        let first = Journal::open(&root).unwrap();
        assert!(matches!(
            Journal::open(&root),
            Err(Error("provider_already_running"))
        ));
        first
            .write(&key, &serde_json::json!({"phase":"dispatching"}))
            .unwrap();
        let owner = first.owner.clone();
        drop(first);
        let second = Journal::open(&root).unwrap();
        assert_eq!(second.owner, owner);
        assert_eq!(
            second.read::<serde_json::Value>(&key).unwrap().unwrap()["phase"],
            "dispatching"
        );
        fs::remove_file(second.path(&key).unwrap()).unwrap();
        let sentinel = root.join("user-sentinel");
        fs::write(&sentinel, b"preserve").unwrap();
        std::os::unix::fs::symlink(&sentinel, second.path(&key).unwrap()).unwrap();
        assert!(second.write(&key, &false).is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"preserve");
        drop(second);
        fs::remove_dir_all(root).unwrap();
    }
}
