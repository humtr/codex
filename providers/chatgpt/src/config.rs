//! One user-owned provider configuration, independent of Core and Manager.
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const FORMAT_VERSION: u32 = 1;
const MAX_CONTEXT_TOKENS: u64 = 1_000_000;
const MIN_CONTEXT_TOKENS: u64 = 32_768;
const MAX_WIRE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DECODED_BYTES: u64 = 32 * 1024 * 1024;
const MIN_TRANSPORT_BYTES: u64 = 1024 * 1024;
const FILE_LIMIT: u64 = 32 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub model: ModelConfig,
    pub context: ContextConfig,
    pub transport: TransportConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelConfig {
    pub default: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextConfig {
    pub window_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_token_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransportConfig {
    pub max_wire_bytes: u64,
    pub max_decoded_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub mode: HistoryMode,
    pub project_name: Option<String>,
    pub create_if_missing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HistoryMode {
    #[default]
    Retain,
    Project,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            model: ModelConfig::default(),
            context: ContextConfig::default(),
            transport: TransportConfig::default(),
            history: HistoryConfig::default(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: "chatgpt-desktop/gpt-6".into(),
            reasoning_effort: "none".into(),
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            window_tokens: MAX_CONTEXT_TOKENS,
            auto_compact_token_limit: None,
        }
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_wire_bytes: MAX_WIRE_BYTES,
            max_decoded_bytes: MAX_DECODED_BYTES,
        }
    }
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            mode: HistoryMode::Retain,
            project_name: None,
            create_if_missing: false,
        }
    }
}

impl Config {
    pub fn parse(bytes: &[u8]) -> Result<Self, ConfigError> {
        if bytes.len() as u64 > FILE_LIMIT {
            return Err(ConfigError::Invalid("configuration exceeds size limit"));
        }
        let config: Self = toml::from_str(
            std::str::from_utf8(bytes)
                .map_err(|_| ConfigError::Invalid("configuration is not UTF-8"))?,
        )
        .map_err(|_| ConfigError::Invalid("configuration syntax or fields are invalid"))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != FORMAT_VERSION {
            return Err(ConfigError::Invalid("unsupported configuration version"));
        }
        if self.model.default != "chatgpt-desktop/gpt-6"
            && self.model.default != "chatgpt-desktop/gpt-6-pro"
        {
            return Err(ConfigError::Invalid("unsupported default model"));
        }
        if !["none", "medium", "high", "xhigh"].contains(&self.model.reasoning_effort.as_str())
            || self.model.default.ends_with("-pro") && self.model.reasoning_effort != "none"
        {
            return Err(ConfigError::Invalid("unsupported model reasoning effort"));
        }
        if !(MIN_CONTEXT_TOKENS..=MAX_CONTEXT_TOKENS).contains(&self.context.window_tokens) {
            return Err(ConfigError::Invalid(
                "context window is outside the qualified range",
            ));
        }
        if self
            .context
            .auto_compact_token_limit
            .is_some_and(|limit| limit == 0 || limit >= self.context.window_tokens)
        {
            return Err(ConfigError::Invalid(
                "automatic compaction threshold is invalid",
            ));
        }
        if !(MIN_TRANSPORT_BYTES..=MAX_WIRE_BYTES).contains(&self.transport.max_wire_bytes)
            || !(MIN_TRANSPORT_BYTES..=MAX_DECODED_BYTES)
                .contains(&self.transport.max_decoded_bytes)
            || self.transport.max_decoded_bytes < self.transport.max_wire_bytes
        {
            return Err(ConfigError::Invalid(
                "transport bounds exceed the provider limits",
            ));
        }
        match self.history.mode {
            HistoryMode::Retain
                if self.history.project_name.is_some() || self.history.create_if_missing =>
            {
                return Err(ConfigError::Invalid(
                    "project settings require history.mode = project",
                ));
            }
            HistoryMode::Project => {
                let Some(name) = self.history.project_name.as_deref() else {
                    return Err(ConfigError::Invalid("project name is required"));
                };
                if name.trim().is_empty() || name.len() > 128 || name.chars().any(char::is_control)
                {
                    return Err(ConfigError::Invalid("project name is invalid"));
                }
            }
            HistoryMode::Retain => {}
        }
        Ok(())
    }

    pub fn render(&self) -> Result<String, ConfigError> {
        self.validate()?;
        toml::to_string_pretty(self)
            .map_err(|_| ConfigError::Invalid("configuration serialization failed"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Invalid(&'static str),
    Io(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) | Self::Io(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    let home = std::env::var_os("HOME").ok_or(ConfigError::Io("HOME is unavailable"))?;
    let root = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) if Path::new(&value).is_absolute() => PathBuf::from(value),
        Some(_) => return Err(ConfigError::Io("XDG_CONFIG_HOME must be absolute")),
        None => PathBuf::from(home).join(".config"),
    };
    Ok(root.join("codex/providers/chatgpt.toml"))
}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let Some(parent) = path.parent() else {
        return Err(ConfigError::Io("configuration path has no parent"));
    };
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(_) => return Err(ConfigError::Io("configuration cannot be inspected")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > FILE_LIMIT {
        return Err(ConfigError::Io("configuration file type or size is unsafe"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(ConfigError::Io(
                "configuration ownership or permissions are unsafe",
            ));
        }
    }
    let _ = parent;
    let bytes = fs::read(path).map_err(|_| ConfigError::Io("configuration cannot be read"))?;
    Config::parse(&bytes)
}

pub fn save(path: &Path, config: &Config) -> Result<(), ConfigError> {
    let rendered = config.render()?;
    let parent = path
        .parent()
        .ok_or(ConfigError::Io("configuration path has no parent"))?;
    create_private_dirs(parent)?;
    reject_unsafe_parent(parent)?;
    if path.exists() || path.symlink_metadata().is_ok() {
        let _ = load(path)?;
    }
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".chatgpt.toml.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(|_| ConfigError::Io("temporary configuration cannot be created"))?;
        file.write_all(rendered.as_bytes())
            .map_err(|_| ConfigError::Io("configuration cannot be written"))?;
        file.sync_all()
            .map_err(|_| ConfigError::Io("configuration cannot be synchronized"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if file
                .metadata()
                .map_err(|_| ConfigError::Io("configuration cannot be inspected"))?
                .permissions()
                .mode()
                & 0o777
                != 0o600
            {
                return Err(ConfigError::Io("configuration mode is unsafe"));
            }
        }
        drop(file);
        fs::rename(&temporary, path)
            .map_err(|_| ConfigError::Io("configuration cannot be atomically published"))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| ConfigError::Io("configuration directory cannot be synchronized"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn create_private_dirs(path: &Path) -> Result<(), ConfigError> {
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(path)
        .map_err(|_| ConfigError::Io("configuration directory cannot be created"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| ConfigError::Io("configuration directory permissions cannot be set"))?;
    }
    Ok(())
}

fn reject_unsafe_parent(path: &Path) -> Result<(), ConfigError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| ConfigError::Io("configuration directory cannot be inspected"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ConfigError::Io("configuration directory type is unsafe"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(ConfigError::Io(
                "configuration directory ownership or permissions are unsafe",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "codex-provider-config-{label}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        path
    }

    #[test]
    fn default_is_explicit_and_roundtrips_without_null_or_secret_fields() {
        let config = Config::default();
        assert_eq!(config.context.window_tokens, 1_000_000);
        assert_eq!(config.context.auto_compact_token_limit, None);
        let rendered = config.render().unwrap();
        assert!(!rendered.contains("null"));
        assert!(!rendered.contains("api_key"));
        assert!(!rendered.contains("authorization"));
        assert_eq!(Config::parse(rendered.as_bytes()).unwrap(), config);
    }

    #[test]
    fn parser_rejects_unknown_fields_invalid_effort_and_unsafe_limits() {
        for source in [
            "version = 2\n",
            "mystery = true\n",
            "[model]\ndefault = 'other'\n",
            "[model]\nreasoning_effort = 'instant'\n",
            "[context]\nwindow_tokens = 1000001\n",
            "[transport]\nmax_wire_bytes = 20000000\n",
            "[history]\nmode = 'project'\n",
            "[history]\nproject_name = 'hidden-unused'\n",
        ] {
            assert!(Config::parse(source.as_bytes()).is_err(), "{source}");
        }
        assert!(Config::parse(&vec![b'x'; FILE_LIMIT as usize + 1]).is_err());
    }

    #[test]
    fn project_and_model_specific_constraints_are_validated_before_render() {
        let mut config = Config::default();
        config.model.default = "chatgpt-desktop/gpt-6-pro".into();
        config.model.reasoning_effort = "high".into();
        assert!(config.validate().is_err());
        config.model.reasoning_effort = "none".into();
        config.history.mode = HistoryMode::Project;
        config.history.project_name = Some("   ".into());
        assert!(config.validate().is_err());
        config.history.project_name = Some("Codex Termux".into());
        config.history.create_if_missing = true;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn save_is_private_atomic_and_preserves_unrelated_files() {
        let root = root("atomic");
        let dir = root.join("codex/providers");
        let path = dir.join("chatgpt.toml");
        save(&path, &Config::default()).unwrap();
        let local = fs::symlink_metadata(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(local.permissions().mode() & 0o777, 0o600);
            assert_eq!(
                fs::symlink_metadata(&dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert_eq!(load(&path).unwrap(), Config::default());
        let sentinel = root.join("unrelated");
        fs::write(&sentinel, b"keep").unwrap();
        let mut changed = Config::default();
        changed.context.window_tokens = 900_000;
        save(&path, &changed).unwrap();
        assert_eq!(load(&path).unwrap(), changed);
        assert_eq!(fs::read(sentinel).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn load_and_save_reject_symlinks_and_unprivate_configuration() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = root("unsafe");
        let outside = root.join("outside");
        fs::write(&outside, Config::default().render().unwrap()).unwrap();
        let directory = root.join("codex/providers");
        fs::create_dir_all(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("chatgpt.toml");
        symlink(&outside, &path).unwrap();
        assert!(load(&path).is_err());
        assert!(save(&path, &Config::default()).is_err());
        fs::remove_file(path).unwrap();
        fs::write(
            directory.join("chatgpt.toml"),
            Config::default().render().unwrap(),
        )
        .unwrap();
        fs::set_permissions(
            directory.join("chatgpt.toml"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(load(&directory.join("chatgpt.toml")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
