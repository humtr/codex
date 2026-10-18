//! Provider-owned settings and validation; no Core/Manager dependency or host I/O.
use crate::models::Catalog;
use serde::{Deserialize, Serialize};

pub const FILE_LIMIT: u64 = 128 * 1024;
pub const MAX_WIRE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_DECODED_BYTES: u64 = 32 * 1024 * 1024;
pub const FIELDS: &[&str] = &[
    "model.default",
    "model.reasoning_effort",
    "context.window_tokens",
    "context.auto_compact_token_limit",
    "transport.max_wire_bytes",
    "transport.max_decoded_bytes",
    "history.mode",
    "history.project_name",
    "history.create_if_missing",
    "catalog",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub model: ModelConfig,
    pub context: ContextConfig,
    pub transport: TransportConfig,
    pub history: HistoryConfig,
    pub catalog: Catalog,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            model: ModelConfig::default(),
            context: ContextConfig::default(),
            transport: TransportConfig::default(),
            history: HistoryConfig::default(),
            catalog: Catalog::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelConfig {
    pub default: String,
    pub reasoning_effort: String,
}
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: "chatgpt-desktop/gpt-6".into(),
            reasoning_effort: "none".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextConfig {
    pub window_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_token_limit: Option<u64>,
}
impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            window_tokens: 1_000_000,
            auto_compact_token_limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransportConfig {
    pub max_wire_bytes: u64,
    pub max_decoded_bytes: u64,
}
impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_wire_bytes: MAX_WIRE_BYTES,
            max_decoded_bytes: MAX_DECODED_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub mode: HistoryMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    pub create_if_missing: bool,
}
impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            mode: HistoryMode::Retain,
            project_name: None,
            create_if_missing: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HistoryMode {
    #[default]
    Retain,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Invalid(&'static str),
    Io(&'static str),
}
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(v) | Self::Io(v) => f.write_str(v),
        }
    }
}
impl std::error::Error for ConfigError {}

impl Config {
    pub fn parse(bytes: &[u8]) -> Result<Self, ConfigError> {
        if bytes.len() as u64 > FILE_LIMIT {
            return Err(ConfigError::Invalid("configuration exceeds size limit"));
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ConfigError::Invalid("configuration is not UTF-8"))?;
        let config: Self = toml::from_str(text)
            .map_err(|_| ConfigError::Invalid("configuration syntax or fields are invalid"))?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 {
            return Err(ConfigError::Invalid("unsupported configuration version"));
        }
        self.catalog.validate()?;
        if self
            .catalog
            .route(&self.model.default, &self.model.reasoning_effort)
            .is_none()
        {
            return Err(ConfigError::Invalid(
                "model and reasoning effort have no configured route",
            ));
        }
        if self.context.window_tokens == 0 || self.context.window_tokens > i64::MAX as u64 {
            return Err(ConfigError::Invalid(
                "context window must be a positive upstream-compatible integer",
            ));
        }
        if self
            .context
            .auto_compact_token_limit
            .is_some_and(|v| v == 0 || v >= self.context.window_tokens)
        {
            return Err(ConfigError::Invalid(
                "automatic compaction threshold must be positive and below the context window",
            ));
        }
        if !(1..=MAX_WIRE_BYTES).contains(&self.transport.max_wire_bytes)
            || !(1..=MAX_DECODED_BYTES).contains(&self.transport.max_decoded_bytes)
        {
            return Err(ConfigError::Invalid(
                "transport bounds exceed the provider limits",
            ));
        }
        if self.history.project_name.as_deref().is_some_and(|v| {
            v.trim().is_empty() || v.len() > 1024 || v.chars().any(char::is_control)
        }) {
            return Err(ConfigError::Invalid("project name is invalid"));
        }
        if self.history.mode == HistoryMode::Project && self.history.project_name.is_none() {
            return Err(ConfigError::Invalid("project mode requires a project name"));
        }
        Ok(())
    }
    pub fn render(&self) -> Result<String, ConfigError> {
        self.validate()?;
        toml::to_string_pretty(self)
            .map_err(|_| ConfigError::Invalid("configuration serialization failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_roundtrip_without_claiming_host_capacity() {
        let value = Config::default();
        assert_eq!(value.context.window_tokens, 1_000_000);
        assert_eq!(value.context.auto_compact_token_limit, None);
        assert_eq!(
            Config::parse(value.render().unwrap().as_bytes()).unwrap(),
            value
        );
    }
    #[test]
    fn preserves_user_context_override_without_arbitrary_32k_or_1m_bound() {
        for tokens in [16_384, 1_000_000, 2_000_000] {
            let mut value = Config::default();
            value.context.window_tokens = tokens;
            value.validate().unwrap();
        }
        for source in [
            "version=2",
            "unknown=true",
            "[context]\nwindow_tokens=0",
            "[transport]\nmax_wire_bytes=16777217",
            "[history]\nmode='project'",
            "[model]\nreasoning_effort='unconfigured'",
        ] {
            assert!(Config::parse(source.as_bytes()).is_err(), "{source}");
        }
        assert!(Config::parse(&vec![b'x'; FILE_LIMIT as usize + 1]).is_err());
    }
    #[test]
    fn wire_and_decoded_limits_are_independent_and_project_can_be_preconfigured() {
        let mut value = Config::default();
        value.transport.max_decoded_bytes = 1024;
        value.history.project_name = Some("none".into());
        value.validate().unwrap();
        value.history.mode = HistoryMode::Project;
        value.validate().unwrap();
        assert!(value.history.create_if_missing);
    }
}
