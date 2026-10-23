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
    "transport.return_path",
    "transport.connector_name",
    "history.mode",
    "history.account_change",
    "history.project_name",
    "history.project_id",
    "history.create_if_missing",
    "catalog",
    "host.rootfs",
    "host.viewer_assets",
    "host.viewer_socket",
    "execution.mode",
    "execution.core",
    "execution.home",
    "execution.workspace",
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
    pub host: HostConfig,
    pub execution: ExecutionConfig,
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
            host: HostConfig::default(),
            execution: ExecutionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    #[default]
    ModelAdapter,
    Operator,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutionConfig {
    pub mode: ExecutionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HostConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer_socket: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer_assets: Option<std::path::PathBuf>,
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
    pub return_path: ReturnPath,
    pub connector_name: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReturnPath {
    #[default]
    LocalFunction,
    Mcp,
}
impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_wire_bytes: MAX_WIRE_BYTES,
            max_decoded_bytes: MAX_DECODED_BYTES,
            return_path: ReturnPath::LocalFunction,
            connector_name: "Termux Bridge".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub mode: HistoryMode,
    // Preserve pre-policy request fingerprints for durable response replay.
    #[serde(skip_serializing_if = "AccountChange::is_default")]
    pub account_change: AccountChange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub create_if_missing: bool,
}
impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            mode: HistoryMode::Retain,
            account_change: AccountChange::NewChat,
            project_name: None,
            project_id: None,
            create_if_missing: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccountChange {
    #[default]
    NewChat,
    Refuse,
}
impl AccountChange {
    fn is_default(&self) -> bool {
        *self == Self::NewChat
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
        if [
            &self.host.rootfs,
            &self.host.viewer_assets,
            &self.host.viewer_socket,
            &self.execution.core,
            &self.execution.home,
            &self.execution.workspace,
        ]
        .into_iter()
        .flatten()
        .any(|p| {
            !p.is_absolute()
                || p.to_str().is_none_or(|s| s.contains([':', '\0']))
                || p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
        }) {
            return Err(ConfigError::Invalid(
                "host paths must be absolute and unambiguous",
            ));
        }
        if self.execution.mode == ExecutionMode::Operator
            && [
                &self.execution.core,
                &self.execution.home,
                &self.execution.workspace,
            ]
            .iter()
            .any(|v| v.is_none())
        {
            return Err(ConfigError::Invalid(
                "operator execution requires explicit Core, execution home and workspace",
            ));
        }
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
        for model in &self.catalog.models {
            self.effective_context(&model.id, &Default::default())?;
        }
        if !(1..=MAX_WIRE_BYTES).contains(&self.transport.max_wire_bytes)
            || !(1..=MAX_DECODED_BYTES).contains(&self.transport.max_decoded_bytes)
        {
            return Err(ConfigError::Invalid(
                "transport bounds exceed the provider limits",
            ));
        }
        if self.transport.connector_name.trim().is_empty()
            || self.transport.connector_name.len() > 160
            || self.transport.connector_name.chars().any(char::is_control)
        {
            return Err(ConfigError::Invalid("connector display name is invalid"));
        }
        if self.history.project_name.as_deref().is_some_and(|v| {
            v.trim().is_empty() || v.len() > 1024 || v.chars().any(char::is_control)
        }) {
            return Err(ConfigError::Invalid("project name is invalid"));
        }
        if self.history.mode == HistoryMode::Project && self.history.project_name.is_none() {
            return Err(ConfigError::Invalid("project mode requires a project name"));
        }
        if self.history.project_id.as_deref().is_some_and(|v| {
            !v.starts_with("g-p-")
                || v.len() > 256
                || !v.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        }) {
            return Err(ConfigError::Invalid("project ID is invalid"));
        }
        Ok(())
    }
    pub fn effective_context(
        &self,
        model: &str,
        invocation: &crate::models::ContextOverride,
    ) -> Result<ContextConfig, ConfigError> {
        let model = self
            .catalog
            .find(model)
            .ok_or(ConfigError::Invalid("unknown model for context settings"))?;
        let value = ContextConfig {
            window_tokens: invocation
                .window_tokens
                .or(model.context.window_tokens)
                .unwrap_or(self.context.window_tokens),
            auto_compact_token_limit: invocation
                .auto_compact_token_limit
                .or(model.context.auto_compact_token_limit)
                .or(self.context.auto_compact_token_limit),
        };
        if value.window_tokens == 0
            || value.window_tokens > i64::MAX as u64
            || value
                .auto_compact_token_limit
                .is_some_and(|v| v == 0 || v >= value.window_tokens)
        {
            return Err(ConfigError::Invalid(
                "effective model context limits are invalid",
            ));
        }
        Ok(value)
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
    fn app_account_policy_is_configuration_without_account_identifiers() {
        assert_eq!(
            Config::default().history.account_change,
            AccountChange::NewChat
        );
        let strict = Config::parse(b"[history]\naccount_change='refuse'\n").unwrap();
        assert_eq!(strict.history.account_change, AccountChange::Refuse);
        assert_eq!(
            Config::parse(strict.render().unwrap().as_bytes()).unwrap(),
            strict
        );
        assert!(Config::parse(b"[history]\naccount_change='some-fixed-account'\n").is_err());
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
    #[test]
    fn return_transport_is_explicit_and_connector_branding_is_configurable() {
        assert_eq!(Config::default().transport.connector_name, "Termux Bridge");
        assert_eq!(
            Config::default().transport.return_path,
            ReturnPath::LocalFunction
        );
        let settings =
            Config::parse(b"[transport]\nreturn_path='mcp'\nconnector_name='My Owned Connector'\n")
                .unwrap();
        assert_eq!(settings.transport.return_path, ReturnPath::Mcp);
        assert_eq!(settings.transport.connector_name, "My Owned Connector");
        for text in [
            "[transport]\nreturn_path='automatic'",
            "[transport]\nconnector_name=''",
            "[transport]\nconnector_name=\"bad\\u0000name\"",
        ] {
            assert!(Config::parse(text.as_bytes()).is_err());
        }
    }
}

#[cfg(test)]
mod operator_tests {
    use super::*;
    #[test]
    fn operator_is_explicit_and_requires_a_complete_execution_binding_for_both_transports() {
        let base = Config::default();
        assert_eq!(base.execution.mode, ExecutionMode::ModelAdapter);
        let mut config = base;
        config.execution.mode = ExecutionMode::Operator;
        assert!(config.validate().is_err());
        config.execution.core = Some("/owned/core".into());
        config.execution.home = Some("/owned/codex".into());
        config.execution.workspace = Some("/owned/work".into());
        config.validate().unwrap();
        config.transport.return_path = ReturnPath::Mcp;
        config.validate().unwrap();
        config.execution.home = Some("/owned/../other".into());
        assert!(config.validate().is_err());
        config.execution.home = Some("relative-home".into());
        assert!(config.validate().is_err());
    }
}
