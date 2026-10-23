//! Private configuration transactions. Cooperative writers serialize; editor snapshots detect conflicts.
use crate::config::{Config, ConfigError, FIELDS, FILE_LIMIT};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn config_path() -> Result<PathBuf, ConfigError> {
    let root = match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(v) => PathBuf::from(v),
        None => {
            PathBuf::from(std::env::var_os("HOME").ok_or(ConfigError::Io("HOME is unavailable"))?)
                .join(".config")
        }
    };
    if !root.is_absolute()
        || root
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ConfigError::Io(
            "configuration root must be absolute without parent traversal",
        ));
    }
    Ok(root.join("codex/providers/chatgpt.toml"))
}

fn inspect(file: &File) -> Result<(), ConfigError> {
    let m = file
        .metadata()
        .map_err(|_| ConfigError::Io("configuration cannot be inspected"))?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.permissions().mode() & 0o7777 != 0o600
    {
        return Err(ConfigError::Io(
            "configuration must be an owned private regular file",
        ));
    }
    Ok(())
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, ConfigError> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ConfigError::Io("configuration cannot be opened safely")),
    };
    inspect(&file)?;
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ConfigError::Io("configuration cannot be read"))?;
    if bytes.len() as u64 > FILE_LIMIT {
        return Err(ConfigError::Invalid("configuration exceeds size limit"));
    }
    Ok(Some(bytes))
}
fn document(bytes: Option<&[u8]>) -> Result<toml::Table, ConfigError> {
    match bytes {
        None => Ok(toml::Table::new()),
        Some(b) => {
            Config::parse(b)?;
            toml::from_str(
                std::str::from_utf8(b)
                    .map_err(|_| ConfigError::Invalid("configuration is not UTF-8"))?,
            )
            .map_err(|_| ConfigError::Invalid("configuration syntax is invalid"))
        }
    }
}

pub struct Snapshot {
    path: PathBuf,
    original: Option<Vec<u8>>,
}
impl Snapshot {
    pub fn open(path: &Path) -> Result<Self, ConfigError> {
        let original = read(path)?;
        Ok(Self {
            path: path.to_owned(),
            original,
        })
    }
    pub fn config(&self) -> Result<Config, ConfigError> {
        match self.original.as_deref() {
            Some(bytes) => Config::parse(bytes),
            None => Ok(Config::default()),
        }
    }
    pub fn editor_text(&self) -> Result<String, ConfigError> {
        match &self.original {
            Some(b) => String::from_utf8(b.clone())
                .map_err(|_| ConfigError::Invalid("configuration is not UTF-8")),
            None => Config::default().render(),
        }
    }
    pub fn effective(&self) -> Result<serde_json::Value, ConfigError> {
        let doc = document(self.original.as_deref())?;
        let origins: BTreeMap<_, _> = FIELDS
            .iter()
            .map(|field| {
                let present = if *field == "catalog" {
                    doc.contains_key("catalog")
                } else {
                    let (table, key) = field.split_once('.').expect("known field");
                    doc.get(table).and_then(|v| v.get(key)).is_some()
                };
                (*field, if present { "file" } else { "default" })
            })
            .collect();
        let config = self.config()?;
        let contexts=config.catalog.models.iter().map(|model| {
            let effective=config.effective_context(&model.id,&Default::default())?;
            Ok((model.id.clone(),serde_json::json!({"settings":effective,"origins":{"window_tokens":if model.context.window_tokens.is_some(){"model"}else{origins["context.window_tokens"]},"auto_compact_token_limit":if model.context.auto_compact_token_limit.is_some(){"model"}else{origins["context.auto_compact_token_limit"]}}})))
        }).collect::<Result<BTreeMap<_,_>,ConfigError>>()?;
        let mut settings = serde_json::json!(&config);
        settings["history"]["account_change"] = serde_json::json!(config.history.account_change);
        Ok(
            serde_json::json!({"effective_model_contexts":contexts,"path":self.path,"exists":self.original.is_some(),"runtime_application":"not_queried","settings":settings,"origins":origins}),
        )
    }
    pub fn commit_bytes(&self, bytes: &[u8]) -> Result<bool, ConfigError> {
        Config::parse(bytes)?;
        let _lock = lock(&self.path)?;
        self.commit_locked(bytes)
    }
    fn commit_locked(&self, bytes: &[u8]) -> Result<bool, ConfigError> {
        if read(&self.path)? != self.original {
            return Err(ConfigError::Io(
                "configuration changed; reload before saving",
            ));
        }
        if self.original.as_deref() == Some(bytes) {
            return Ok(false);
        }
        let parent = self
            .path
            .parent()
            .ok_or(ConfigError::Io("configuration has no parent"))?;
        let temporary = parent.join(format!(
            ".chatgpt-{}-{}.tmp",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| ConfigError::Io("temporary configuration cannot be created"))?;
        let result = (|| {
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|_| ConfigError::Io("configuration cannot be synchronized"))?;
            inspect(&file)?;
            if read(&self.path)? != self.original {
                return Err(ConfigError::Io(
                    "configuration changed; reload before saving",
                ));
            }
            if self.original.is_none() {
                publish_new(&temporary, &self.path)?;
            } else {
                fs::rename(&temporary, &self.path)
                    .map_err(|_| ConfigError::Io("configuration cannot be atomically replaced"))?;
            }
            File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|_| ConfigError::Io("configuration directory cannot be synchronized"))?;
            Ok(true)
        })();
        // Only a path created by this transaction is removed.
        if temporary.exists() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn create_dirs(path: &Path) -> Result<(), ConfigError> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(ConfigError::Io("configuration directory type is unsafe")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            create_dirs(
                path.parent()
                    .ok_or(ConfigError::Io("configuration directory has no parent"))?,
            )?;
            match fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => create_dirs(path),
                Err(_) => Err(ConfigError::Io("configuration directory cannot be created")),
            }
        }
        Err(_) => Err(ConfigError::Io(
            "configuration directory cannot be inspected",
        )),
    }
}
fn lock(path: &Path) -> Result<crate::journal::Lease, ConfigError> {
    let parent = path
        .parent()
        .ok_or(ConfigError::Io("configuration has no parent"))?;
    create_dirs(parent)?;
    let m = fs::symlink_metadata(parent)
        .map_err(|_| ConfigError::Io("configuration directory cannot be inspected"))?;
    if m.uid() != unsafe { libc::geteuid() } || m.permissions().mode() & 0o7777 != 0o700 {
        return Err(ConfigError::Io(
            "configuration directory must be owned and private",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(parent.join(".chatgpt.toml.lock"))
        .map_err(|_| ConfigError::Io("configuration lock cannot be opened safely"))?;
    inspect(&file)?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(ConfigError::Io(
            "configuration is being edited; retry after the other writer finishes",
        ));
    }
    Ok(crate::journal::Lease::writer(file))
}
pub(crate) fn publish_new(source: &Path, destination: &Path) -> Result<(), ConfigError> {
    use std::os::unix::ffi::OsStrExt;
    let a = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| ConfigError::Io("configuration path is invalid"))?;
    let b = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| ConfigError::Io("configuration path is invalid"))?;
    // Android disallows general hard links. Atomic no-replace rename is qualified separately.
    if unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            a.as_ptr(),
            libc::AT_FDCWD,
            b.as_ptr(),
            1u32,
        )
    } != 0
    {
        return Err(ConfigError::Io(
            "configuration cannot be atomically published without replacement",
        ));
    }
    Ok(())
}

pub fn update(
    path: &Path,
    values: &[(String, String)],
    unset: &[String],
) -> Result<bool, ConfigError> {
    let _lock = lock(path)?;
    let snapshot = Snapshot::open(path)?;
    let mut doc = document(snapshot.original.as_deref())?;
    let before = doc.clone();
    for (field, value) in values {
        set(&mut doc, field, Some(value))?;
    }
    for field in unset {
        set(&mut doc, field, None)?;
    }
    if doc == before {
        return Ok(false);
    }
    let text = toml::to_string_pretty(&doc)
        .map_err(|_| ConfigError::Invalid("configuration serialization failed"))?;
    Config::parse(text.as_bytes())?;
    snapshot.commit_locked(text.as_bytes())
}
fn set(doc: &mut toml::Table, field: &str, value: Option<&str>) -> Result<(), ConfigError> {
    if !FIELDS.contains(&field) || field == "catalog" {
        return Err(ConfigError::Invalid(
            "unknown scalar configuration field; edit catalog as structured data",
        ));
    }
    let (section, key) = field.split_once('.').expect("known field");
    let Some(value) =
        value.filter(|v| !(field == "context.auto_compact_token_limit" && *v == "none"))
    else {
        if let Some(table) = doc.get_mut(section).and_then(toml::Value::as_table_mut) {
            table.remove(key);
            if table.is_empty() {
                doc.remove(section);
            }
        }
        return Ok(());
    };
    let table = doc
        .entry(section)
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or(ConfigError::Invalid(
            "configuration section must be a table",
        ))?;
    let parsed = match field {
        "context.window_tokens"
        | "context.auto_compact_token_limit"
        | "transport.max_wire_bytes"
        | "transport.max_decoded_bytes" => toml::Value::Integer(
            value
                .parse()
                .map_err(|_| ConfigError::Invalid("configuration field requires an integer"))?,
        ),
        "history.create_if_missing" => toml::Value::Boolean(
            value
                .parse()
                .map_err(|_| ConfigError::Invalid("configuration field requires true or false"))?,
        ),
        _ => toml::Value::String(value.into()),
    };
    table.insert(key.into(), parsed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "provider-store-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        path
    }
    #[test]
    fn dependent_update_is_atomic_and_stale_editor_cannot_lose_a_newer_update() {
        let root = root();
        let path = root.join("config/chatgpt.toml");
        update(
            &path,
            &[
                ("history.mode".into(), "project".into()),
                ("history.project_name".into(), "Codex".into()),
            ],
            &[],
        )
        .unwrap();
        let old = Snapshot::open(&path).unwrap();
        update(
            &path,
            &[("context.window_tokens".into(), "2000000".into())],
            &[],
        )
        .unwrap();
        assert!(old
            .commit_bytes(old.editor_text().unwrap().as_bytes())
            .is_err());
        assert_eq!(
            Snapshot::open(&path)
                .unwrap()
                .config()
                .unwrap()
                .context
                .window_tokens,
            2_000_000
        );
        let before = fs::read(&path).unwrap();
        assert!(update(&path, &[("context.window_tokens".into(), "0".into())], &[]).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn completed_config_writer_releases_inherited_descriptor_without_false_busy_retry() {
        let root = root();
        let path = root.join("chatgpt.toml");
        let held = lock(&path).unwrap();
        let inherited = held.file.try_clone().unwrap();
        assert!(lock(&path).is_err());
        drop(held);
        update(
            &path,
            &[("context.window_tokens".into(), "1000000".into())],
            &[],
        )
        .unwrap();
        drop(inherited);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_op_preserves_file_and_origin_tracking_is_not_defaults_materialized_as_file() {
        let root = root();
        let path = root.join("chatgpt.toml");
        update(
            &path,
            &[("context.window_tokens".into(), "1000000".into())],
            &[],
        )
        .unwrap();
        let inode = fs::metadata(&path).unwrap().ino();
        assert!(!update(
            &path,
            &[("context.window_tokens".into(), "1000000".into())],
            &[]
        )
        .unwrap());
        assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
        let effective = Snapshot::open(&path).unwrap().effective().unwrap();
        assert_eq!(effective["origins"]["context.window_tokens"], "file");
        assert_eq!(effective["origins"]["model.default"], "default");
        assert_eq!(
            effective["settings"]["history"]["account_change"],
            "new_chat"
        );
        assert_eq!(effective["origins"]["history.account_change"], "default");
        assert_eq!(effective["runtime_application"], "not_queried");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_symlink_special_file_and_busy_writer_without_changing_destination() {
        use std::os::unix::fs::symlink;
        let root = root();
        let path = root.join("chatgpt.toml");
        let other = root.join("outside");
        fs::write(&other, b"keep").unwrap();
        symlink(&other, &path).unwrap();
        assert!(Snapshot::open(&path).is_err());
        assert!(update(
            &path,
            &[("context.window_tokens".into(), "1000000".into())],
            &[]
        )
        .is_err());
        assert_eq!(fs::read(&other).unwrap(), b"keep");
        fs::remove_file(&path).unwrap();
        use std::os::unix::ffi::OsStrExt;
        let fifo = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(Snapshot::open(&path).is_err());
        fs::remove_file(&path).unwrap();
        let held = lock(&path).unwrap();
        assert!(update(
            &path,
            &[("context.window_tokens".into(), "1000000".into())],
            &[]
        )
        .is_err());
        drop(held);
        fs::remove_dir_all(root).unwrap();
    }
}
