//! Optional installed Chat catalog; ordinary upstream model actions remain intact.
use crate::legacy_core::config::Config;
use codex_config::ConfigLayerSource;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ModelsResponse as Catalog;
use codex_protocol::openai_models::ReasoningEffort;
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub(crate) const PROVIDER: &str = "chatgpt_desktop";
const BEGIN: &str = "# codex-chatgpt-provider generated v1\n";

pub(crate) struct ProviderCommand {
    pub binary: std::path::PathBuf,
    pub state: std::path::PathBuf,
}

/// Use the installed auth definition's exact binary/state for local read-only
/// presentation and public handoff operations, independently of the active model.
pub(crate) fn installed_command(config: &Config) -> Option<ProviderCommand> {
    let routing = installed_routing(config)?;
    let auth = routing.get("model_providers")?.get(PROVIDER)?.get("auth")?;
    let binary = std::path::PathBuf::from(auth.get("command")?.as_str()?);
    let args = auth.get("args")?.as_array()?;
    if args.len() != 3 || args[0].as_str() != Some("token") || args[1].as_str() != Some("--state") {
        return None;
    }
    let state = std::path::PathBuf::from(args[2].as_str()?);
    let metadata = std::fs::symlink_metadata(&binary).ok()?;
    if !binary.is_absolute()
        || !state.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return None;
    }
    Some(ProviderCommand { binary, state })
}

/// Account-verified app observation; never another provider's fallback catalog.
pub(crate) struct LiveCatalog {
    pub models: Vec<ModelPreset>,
    pub path: std::path::PathBuf,
    source: Vec<u8>,
}

impl LiveCatalog {
    pub(crate) fn requires_binding(&self, config: &Config) -> bool {
        config.model_catalog_source.as_deref() != Some(self.source.as_slice())
    }

    pub(crate) fn admits(&self, model: &str, effort: &ReasoningEffort, saved: bool) -> bool {
        self.models.iter().any(|entry| {
            entry.model == model
                && (saved || entry.show_in_picker)
                && entry
                    .supported_reasoning_efforts
                    .iter()
                    .any(|level| level.effort == *effort)
        })
    }
}

/// Reuse the Provider's existing authenticated discovery and immutable snapshot
/// producer. No login, profile preparation or model inference occurs here.
pub(crate) async fn live_catalog(config: &Config) -> Result<LiveCatalog, String> {
    use tokio::io::AsyncReadExt;
    let provider = installed_command(config)
        .ok_or("ChatGPT model discovery is unavailable: installed Provider command is invalid")?;
    let mut child = tokio::process::Command::new(&provider.binary)
        .args(["host", "models", "--state"])
        .arg(&provider.state)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "ChatGPT model discovery could not start")?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or("ChatGPT catalog stdout is unavailable")?
        .take(4 * 1024 * 1024 + 1);
    let mut stderr = child
        .stderr
        .take()
        .ok_or("ChatGPT catalog stderr is unavailable")?
        .take(4097);
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let status = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        let (_, _, status) = tokio::try_join!(
            stdout.read_to_end(&mut output),
            stderr.read_to_end(&mut errors),
            child.wait()
        )?;
        Ok::<_, std::io::Error>(status)
    })
    .await
    .map_err(|_| "ChatGPT model discovery timed out; selection unchanged")?
    .map_err(|_| "ChatGPT model discovery failed; selection unchanged")?;
    if output.len() > 4 * 1024 * 1024 || errors.len() > 4096 {
        return Err("ChatGPT model discovery exceeded its response limit".into());
    }
    if !status.success() {
        let code = serde_json::from_slice::<serde_json::Value>(&output)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .filter(|v| {
                !v.is_empty()
                    && v.len() < 128
                    && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            .unwrap_or_else(|| "provider_control_failed".into());
        return Err(format!(
            "ChatGPT model discovery failed: {code}; selection unchanged"
        ));
    }
    decode_live_catalog(&provider, &output)
}

fn decode_live_catalog(provider: &ProviderCommand, bytes: &[u8]) -> Result<LiveCatalog, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Invalid ChatGPT catalog response")?;
    if value["schema"] != "codex-chatgpt-app-models-v1"
        || value["source"] != "app_and_config"
        || value["account_verified_now"] != true
        || value["inference_submitted"] != false
        || value["observed_at_unix_ms"].as_u64().is_none()
        || ["account", "user", "structure"]
            .iter()
            .any(|key| value["identity"][*key].as_str().is_none_or(str::is_empty))
    {
        return Err(
            "The installed Provider does not supply an account-verified model catalog".into(),
        );
    }
    let path = value["native_catalog_path"]
        .as_str()
        .map(std::path::PathBuf::from)
        .ok_or("ChatGPT catalog snapshot path is unavailable")?;
    if !path.is_absolute() || path.parent() != Some(provider.state.join("catalogs").as_path()) {
        return Err("ChatGPT catalog snapshot is outside the Provider state".into());
    }
    let source = private_bytes(&path, 4 * 1024 * 1024)
        .ok_or("ChatGPT catalog snapshot is unavailable or unsafe")?;
    let stored: serde_json::Value =
        serde_json::from_slice(&source).map_err(|_| "Invalid ChatGPT catalog snapshot")?;
    if stored != value["native_catalog"] {
        return Err("ChatGPT catalog snapshot changed during discovery".into());
    }
    let catalog: Catalog =
        serde_json::from_value(stored).map_err(|_| "Unsupported ChatGPT native catalog schema")?;
    let models: Vec<ModelPreset> = catalog.models.into_iter().map(Into::into).collect();
    let mut seen = std::collections::HashSet::new();
    if models.iter().any(|model| {
        !valid_chat_id(&model.model)
            || !seen.insert(model.model.clone())
            || model.supported_reasoning_efforts.is_empty()
            || !model
                .supported_reasoning_efforts
                .iter()
                .any(|e| e.effort == model.default_reasoning_effort)
    }) {
        return Err("Invalid ChatGPT model identity or reasoning options".into());
    }
    Ok(LiveCatalog {
        models,
        path,
        source,
    })
}

/// Only an explicit /provider choice may read the other provider's catalog.
/// OpenAI's active list is returned unchanged, with its upstream order/options.
pub(crate) async fn native_provider_catalog(
    config: &Config,
    current: Vec<ModelPreset>,
) -> Result<Vec<ModelPreset>, String> {
    if config.model_provider_id == "openai" {
        return Ok(current);
    }
    openai_catalog(config).await
}

async fn openai_catalog(config: &Config) -> Result<Vec<ModelPreset>, String> {
    // Destination preview uses the actual upstream provider, including its
    // credential-scoped cache, bundled fallback, ordering and auth filtering.
    // This preview is offline; normal OpenAI startup/model refresh still owns
    // network discovery after reentry. Never read an unscoped cache ourselves.
    let info = config
        .model_providers
        .get("openai")
        .cloned()
        .ok_or("OpenAI model provider is unavailable")?;
    let auth = codex_login::AuthManager::shared_from_auth_config(
        config.auth_config(),
        /*enable_codex_api_key_env*/ false,
    )
    .await
    .map_err(|_| "OpenAI authentication could not be loaded for model discovery")?;
    let provider = codex_model_provider::create_model_provider(info, Some(auth));
    let manager = provider.models_manager(config.codex_home.to_path_buf(), None);
    manager.set_api_key_model_discovery_enabled(
        config
            .features
            .enabled(codex_features::Feature::ApiKeyModelDiscovery),
    );
    Ok(manager
        .list_models(
            codex_models_manager::manager::RefreshStrategy::Offline,
            config.http_client_factory(),
        )
        .await
        .into_iter()
        .filter(|model| !model.model.starts_with("chatgpt-desktop/"))
        .collect())
}

/// Wait for admitted Native execution before recording a new CLI draft.
/// The service barrier neither retrieves saved Chat nor resubmits inference.
pub(crate) async fn synchronize_input(
    provider: ProviderCommand,
    thread: codex_protocol::ThreadId,
) -> Result<(), String> {
    use tokio::io::AsyncReadExt;
    let thread = thread.to_string();
    loop {
        let mut child = tokio::process::Command::new(&provider.binary)
            .args(["work", "synchronize", "--thread-id", &thread, "--state"])
            .arg(&provider.state)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| format!("ChatGPT input synchronization unavailable: {error}"))?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or("ChatGPT synchronization stdout unavailable")?
            .take(16385);
        let mut stderr = child
            .stderr
            .take()
            .ok_or("ChatGPT synchronization stderr unavailable")?
            .take(4097);
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let status = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            let (_, _, status) = tokio::try_join!(
                stdout.read_to_end(&mut output),
                stderr.read_to_end(&mut errors),
                child.wait()
            )?;
            Ok::<_, std::io::Error>(status)
        })
        .await
        .map_err(|_| "ChatGPT input synchronization timed out; draft retained")?
        .map_err(|error| format!("ChatGPT input synchronization failed: {error}"))?;
        if output.len() > 16384 || errors.len() > 4096 {
            return Err(
                "ChatGPT input synchronization response exceeded its limit; draft retained".into(),
            );
        }
        // Older installed CLIs reject this action locally, before contacting a
        // service. Only that exact protocol result retains legacy submission.
        if status.code() == Some(2)
            && output.is_empty()
            && std::str::from_utf8(&errors).ok().map(str::trim)
                == Some("codex-chatgpt-provider: unsupported work action")
        {
            tracing::warn!(
                "Installed Provider lacks input synchronization; using its legacy protocol"
            );
            return Ok(());
        }
        if !status.success() {
            let code = serde_json::from_slice::<serde_json::Value>(&output)
                .ok()
                .and_then(|value| {
                    value["error"]
                        .as_str()
                        .or_else(|| value["error"]["code"].as_str())
                        .map(str::to_owned)
                })
                .or_else(|| {
                    std::str::from_utf8(&errors)
                        .ok()
                        .map(str::trim)
                        .filter(|text| {
                            !text.is_empty()
                                && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        })
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "provider_control_failed".into());
            return Err(format!(
                "ChatGPT input synchronization failed: {code}; draft retained"
            ));
        }
        let value: serde_json::Value = serde_json::from_slice(&output)
            .map_err(|_| "Invalid ChatGPT input synchronization response; draft retained")?;
        if value["thread_id"] != thread || value["inference_submitted"] != false {
            return Err("ChatGPT input synchronization identity mismatch; draft retained".into());
        }
        match value["status"].as_str() {
            Some("ready") => return Ok(()),
            Some("legacy") => {
                tracing::warn!(
                    "Running Provider service lacks input synchronization; using its legacy protocol"
                );
                return Ok(());
            }
            Some("waiting")
                if value["reason"]
                    .as_str()
                    .is_some_and(|reason| !reason.is_empty()) =>
            {
                let seconds = value["retry_after_seconds"]
                    .as_u64()
                    .ok_or("ChatGPT input synchronization omitted retry timing; draft retained")?;
                tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
            }
            _ => return Err("Invalid ChatGPT input synchronization status; draft retained".into()),
        }
    }
}

pub(crate) fn private_bytes(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || meta.len() > limit
    {
        return None;
    }
    let mut bytes = Vec::new();
    file.by_ref().take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

fn valid_chat_id(model: &str) -> bool {
    model
        .strip_prefix("chatgpt-desktop/")
        .is_some_and(|suffix| {
            !suffix.is_empty()
                && model.len() <= 128
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-:/".contains(&b))
        })
}

pub(crate) fn needs_reentry(provider: &str, model: &str) -> bool {
    (provider == PROVIDER) != model.starts_with("chatgpt-desktop/")
}

/// Provider routing is planned without App/chatwidget or server lifecycle access.
pub(crate) struct ReentryRoute {
    pub args: Vec<String>,
    pub return_profile: Option<String>,
}

fn config_profile(config: &Config) -> Option<String> {
    config
        .config_layer_stack
        .layers_high_to_low()
        .find_map(|layer| match &layer.name {
            ConfigLayerSource::User {
                profile: Some(profile),
                ..
            } => Some(profile.clone()),
            _ => None,
        })
}
fn safe_profile(profile: &str) -> bool {
    profile.parse::<codex_config::ProfileV2Name>().is_ok()
}

pub(crate) fn reentry_route(
    config: &Config,
    model: &str,
    effort: &ReasoningEffort,
    saved_return: Option<String>,
) -> Option<ReentryRoute> {
    let entering = model.starts_with("chatgpt-desktop/");
    let current_profile = config_profile(config);
    let saved = if current_profile.as_deref() != Some("chatgpt") {
        current_profile.clone()
    } else {
        saved_return
    };
    if saved
        .as_ref()
        .is_some_and(|p| !safe_profile(p) || p == "chatgpt")
    {
        return None;
    }
    let mut args = Vec::new();
    if entering && current_profile.as_deref() == Some("chatgpt") {
        args.extend(["--profile".into(), "chatgpt".into()]);
    } else if let Some(profile) = &saved {
        args.extend(["--profile".into(), profile.clone()]);
    }
    // Preserve explicit session settings. Compose provider defaults before
    // serializing root tables: a later CLI table replaces an earlier one.
    let mut session = toml::Value::Table(toml::Table::new());
    for layer in config.config_layer_stack.layers_low_to_high() {
        if matches!(layer.name, ConfigLayerSource::SessionFlags) {
            codex_config::merge_toml_values(&mut session, &layer.config);
        }
    }
    if entering {
        // Keep the selected user configuration and add only the installed route.
        // Named permission definitions, MCP settings and secrets stay in files.
        let routing = installed_routing(config)?;
        add_provider_defaults(config, &routing, &mut session);
        args.extend([
            "-c".into(),
            format!("model_provider={PROVIDER:?}"),
            "-c".into(),
            format!("model_catalog_json={}", routing.get("model_catalog_json")?),
        ]);
    }
    for (key, value) in session.as_table()? {
        if !matches!(
            key.as_str(),
            "model"
                | "model_provider"
                | "model_catalog_json"
                | "model_reasoning_effort"
                | "profile"
                | "sandbox_mode"
                | "default_permissions"
        ) {
            args.extend(["-c".into(), format!("{key}={value}")]);
        }
    }
    if !entering {
        // Resume otherwise restores the saved Chat provider when the base
        // configuration omits an explicit provider. Native picker entries are
        // the upstream OpenAI catalog, so request that provider explicitly.
        args.extend(["-c".into(), "model_provider=\"openai\"".into()]);
    }
    let permission_flag = if let Some(permission) = config.permissions.active_permission_profile() {
        format!(
            "default_permissions={}",
            serde_json::to_string(&permission.id).ok()?
        )
    } else {
        use codex_protocol::protocol::SandboxPolicy;
        let mode = match config.legacy_sandbox_policy() {
            SandboxPolicy::DangerFullAccess => "danger-full-access",
            SandboxPolicy::WorkspaceWrite { .. } => "workspace-write",
            SandboxPolicy::ReadOnly { .. } => "read-only",
            _ => return None,
        };
        format!("sandbox_mode={mode:?}")
    };
    let policy = toml::Value::try_from(config.permissions.approval_policy.value()).ok()?;
    let reviewer = toml::Value::try_from(config.approvals_reviewer).ok()?;
    args.extend([
        "-c".into(),
        permission_flag,
        "-c".into(),
        format!("approval_policy={policy}"),
        "-c".into(),
        format!("approvals_reviewer={reviewer}"),
        "-c".into(),
        format!(
            "model_reasoning_effort={}",
            serde_json::to_string(effort.as_str()).ok()?
        ),
    ]);
    Some(ReentryRoute {
        args,
        return_profile: if entering { saved } else { None },
    })
}

pub(crate) fn installed_routing(config: &Config) -> Option<toml::Value> {
    let local = config.codex_home.join("chatgpt.config.toml");
    let shared = std::env::var_os("HOME").map(|home| {
        std::path::PathBuf::from(home)
            .join(".local/share/codex/core/config/profiles/chatgpt.config.toml")
    });
    let path = routing_profile_path(&local, shared.as_deref())?;
    let profile = String::from_utf8(private_bytes(&path, 65536)?).ok()?;
    if !profile.starts_with(BEGIN) {
        return None;
    }
    let value: toml::Value = toml::from_str(&profile).ok()?;
    checked_routing(value)
}

/// Make a saved provider resolvable without changing the current selection.
pub(crate) fn add_installed_provider(config: &Config, session: &mut toml::Value) {
    let Some(routing) = installed_routing(config) else {
        return;
    };
    add_provider_defaults(config, &routing, session);
}

fn add_provider_defaults(config: &Config, routing: &toml::Value, session: &mut toml::Value) {
    let mut defaults = toml::Value::Table(toml::Table::from_iter([(
        "model_providers".into(),
        toml::Value::Table(toml::Table::from_iter([(
            PROVIDER.into(),
            routing["model_providers"][PROVIDER].clone(),
        )])),
    )]));
    // Upstream's raw, enabled layers preserve explicit values without expanding
    // typed defaults. Only missing installed fields enter the session overlay;
    // file-owned auth/headers stay in their files, never newly copied to argv.
    retain_missing_defaults(&mut defaults, &config.config_layer_stack.effective_config());
    codex_config::merge_toml_values(&mut defaults, session);
    *session = defaults;
}

fn retain_missing_defaults(defaults: &mut toml::Value, configured: &toml::Value) -> bool {
    if let (toml::Value::Table(defaults), toml::Value::Table(configured)) = (defaults, configured) {
        defaults.retain(|key, value| {
            configured
                .get(key)
                .is_none_or(|configured| retain_missing_defaults(value, configured))
        });
        !defaults.is_empty()
    } else {
        // Scalars and arrays are atomic upstream overrides, including invalid
        // values. Do not repair them by silently restoring installed defaults.
        false
    }
}

fn routing_profile_path(local: &Path, shared: Option<&Path>) -> Option<std::path::PathBuf> {
    match std::fs::symlink_metadata(local) {
        Ok(meta)
            if meta.file_type().is_symlink()
                && meta.uid() == unsafe { libc::geteuid() }
                && shared.is_some()
                && std::fs::read_link(local).ok().as_deref() == shared =>
        {
            shared.map(Path::to_path_buf)
        }
        Ok(_) => Some(local.to_path_buf()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => shared.map(Path::to_path_buf),
        Err(_) => None,
    }
}

fn checked_routing(value: toml::Value) -> Option<toml::Value> {
    if value.get("model_provider")?.as_str()? != PROVIDER {
        return None;
    }
    let provider = value.get("model_providers")?.get(PROVIDER)?.as_table()?;
    // Only command-backed references enter argv, never a bearer or API key.
    if provider.keys().any(|key| {
        !matches!(
            key.as_str(),
            "name"
                | "base_url"
                | "wire_api"
                | "requires_openai_auth"
                | "supports_websockets"
                | "request_max_retries"
                | "stream_max_retries"
                | "auth"
        )
    }) {
        return None;
    }
    let auth = provider.get("auth")?.as_table()?;
    if auth.keys().any(|key| {
        !matches!(
            key.as_str(),
            "command" | "args" | "timeout_ms" | "cwd" | "refresh_interval_ms"
        )
    }) {
        return None;
    }
    let args = auth.get("args")?.as_array()?;
    if args.len() != 3
        || args[0].as_str()? != "token"
        || args[1].as_str()? != "--state"
        || !Path::new(args[2].as_str()?).is_absolute()
        || !Path::new(auth.get("command")?.as_str()?).is_absolute()
        || provider.get("wire_api")?.as_str()? != "responses"
        || provider.get("requires_openai_auth")?.as_bool()?
    {
        return None;
    }
    let endpoint: url::Url = provider.get("base_url")?.as_str()?.parse().ok()?;
    if endpoint.scheme() != "http"
        || endpoint.host_str() != Some("127.0.0.1")
        || endpoint.port().is_none()
        || endpoint.path() != "/v1"
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return None;
    }
    Some(value)
}

fn installed_catalog(config: &Config) -> Option<(Catalog, Vec<u8>)> {
    let value = installed_routing(config)?;
    let path = Path::new(value.get("model_catalog_json")?.as_str()?);
    if !path.is_absolute() {
        return None;
    }
    let source = private_bytes(path, 4 * 1024 * 1024)?;
    let catalog: Catalog = match serde_json::from_slice(&source) {
        Ok(catalog) => catalog,
        Err(error) => {
            tracing::debug!(%error, "installed Chat catalog could not be decoded");
            return None;
        }
    };
    Some((catalog, source))
}

/// Resolve launch-scoped metadata without changing selected model/effort defaults.
pub(crate) fn restore_catalog(
    config: &mut Config,
    model: Option<&str>,
    observed_path: Option<&Path>,
) -> Result<bool, &'static str> {
    let Some(model) = model.filter(|model| model.starts_with("chatgpt-desktop/")) else {
        return Ok(false);
    };
    if config.model_catalog.is_some() {
        return Ok(false);
    }
    let (catalog, source) = if let Some(path) = observed_path {
        let source =
            private_bytes(path, 4 * 1024 * 1024).ok_or("observed_chat_catalog_unavailable")?;
        let catalog =
            serde_json::from_slice(&source).map_err(|_| "observed_chat_catalog_invalid")?;
        (catalog, source)
    } else {
        installed_catalog(config).ok_or("installed_chat_catalog_unavailable")?
    };
    if !valid_chat_id(model) || !catalog.models.iter().any(|entry| entry.slug == model) {
        return Err("saved_chat_model_unavailable");
    }
    config.model_catalog = Some(catalog);
    config.model_catalog_source = Some(source);
    Ok(true)
}

fn chat_catalog(config: &Config) -> Option<Vec<ModelPreset>> {
    let models: Vec<ModelPreset> = installed_catalog(config)?
        .0
        .models
        .into_iter()
        .map(Into::into)
        .filter(|m: &ModelPreset| {
            m.show_in_picker
                && valid_chat_id(&m.model)
                && !m.supported_reasoning_efforts.is_empty()
                && m.supported_reasoning_efforts
                    .iter()
                    .any(|e| e.effort == m.default_reasoning_effort)
        })
        .collect();
    (!models.is_empty()).then_some(models)
}

/// Scope all current-provider surfaces without querying or augmenting OpenAI.
pub(crate) fn augment(config: &Config, models: Vec<ModelPreset>) -> Vec<ModelPreset> {
    if config.model_provider_id != PROVIDER {
        return models;
    }
    models
        .into_iter()
        .filter(|model| valid_chat_id(&model.model))
        .collect()
}

pub(crate) async fn admits(config: &Config, model: &str, effort: &ReasoningEffort) -> bool {
    if model.starts_with("chatgpt-desktop/") {
        return valid_chat_id(model)
            && chat_catalog(config).is_some_and(|catalog| {
                catalog.iter().any(|m| {
                    m.model == model
                        && m.supported_reasoning_efforts
                            .iter()
                            .any(|e| e.effort == *effort)
                })
            });
    }
    openai_catalog(config).await.is_ok_and(|catalog| {
        catalog.iter().any(|entry| {
            entry.model == model
                && entry.show_in_picker
                && entry
                    .supported_reasoning_efforts
                    .iter()
                    .any(|level| level.effort == *effort)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed_profile(home: &Path) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let catalog = home.join("chat-models.json");
        std::fs::write(&catalog, include_bytes!("fixtures/chat-model-catalog.json")).unwrap();
        std::fs::set_permissions(&catalog, std::fs::Permissions::from_mode(0o600)).unwrap();
        let profile = format!(
            "{BEGIN}model_provider = \"{PROVIDER}\"\nmodel_catalog_json = {:?}\n[model_providers.{PROVIDER}]\nname = \"Chat Provider\"\nbase_url = \"http://127.0.0.1:12345/v1\"\nwire_api = \"responses\"\nrequires_openai_auth = false\n[model_providers.{PROVIDER}.auth]\ncommand = \"/owned/codex-chatgpt-provider\"\nargs = [\"token\", \"--state\", \"/owned/provider-state\"]\n",
            catalog.to_str().unwrap()
        );
        let path = home.join("chatgpt.config.toml");
        std::fs::write(&path, profile).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        catalog
    }

    fn route_overrides(route: ReentryRoute) -> Vec<(String, toml::Value)> {
        codex_utils_cli::CliConfigOverrides {
            raw_overrides: route
                .args
                .windows(2)
                .filter(|pair| pair[0] == "-c")
                .map(|pair| pair[1].clone())
                .collect(),
        }
        .parse_overrides()
        .unwrap()
    }

    #[tokio::test]
    async fn termux_chat_models_current_provider_catalog_preserves_openai_order_without_app_lookup()
    {
        let home = tempfile::tempdir().unwrap();
        installed_profile(home.path());
        let mut config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .unwrap();
        let catalog: Catalog =
            serde_json::from_slice(include_bytes!("fixtures/chat-model-catalog.json")).unwrap();
        let chat: Vec<ModelPreset> = catalog.models.into_iter().map(Into::into).collect();
        let mut native = chat.clone();
        for (index, model) in native.iter_mut().enumerate() {
            model.model = format!("upstream-{index}");
        }
        config.model_provider_id = "openai".into();
        assert_eq!(augment(&config, native.clone()), native);
        assert_eq!(
            native_provider_catalog(&config, native.clone())
                .await
                .unwrap(),
            native
        );
        // A missing or invalid installed Chat catalog must have no OpenAI effect.
        std::fs::remove_file(home.path().join("chat-models.json")).unwrap();
        assert_eq!(augment(&config, native.clone()), native);
        config.model_provider_id = PROVIDER.into();
        assert!(augment(&config, native.clone()).is_empty());
        let mixed: Vec<_> = native.into_iter().chain(chat.clone()).collect();
        assert_eq!(augment(&config, mixed), chat);
    }

    #[tokio::test]
    async fn termux_chat_models_cold_openai_return_uses_upstream_catalog_and_rejects_foreign_cache()
    {
        use codex_models_manager::manager::{ModelsManager, StaticModelsManager};
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let mut config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .unwrap();
        config.cli_auth_credentials_store_mode = codex_login::AuthCredentialsStoreMode::File;
        config.model_provider_id = PROVIDER.into();
        let bundled = codex_models_manager::bundled_models_response().unwrap();
        let upstream = StaticModelsManager::new(
            None,
            bundled.clone(),
            codex_models_manager::ModelOrigin::Bundled,
        );
        let expected = upstream.try_list_models().unwrap();
        assert!(!expected.is_empty());
        assert!(!home.path().join("models_cache.json").exists());
        let actual = native_provider_catalog(&config, Vec::new()).await.unwrap();
        assert_eq!(actual, expected);
        let selected = actual.iter().find(|model| model.show_in_picker).unwrap();
        assert!(admits(&config, &selected.model, &selected.default_reasoning_effort).await);
        assert!(!admits(&config, "not-an-upstream-model", &ReasoningEffort::None).await);
        // A different provider/account's otherwise well-formed, recent cache
        // must not grant its catalog to OpenAI or change the bundled ordering.
        let mut foreign = bundled;
        foreign.models[0].slug = "foreign-cache-only".into();
        let cache = serde_json::json!({
            "fetched_at": chrono::Utc::now(),
            "client_version": codex_models_manager::client_version_to_whole(),
            "identity": "another-provider-and-account",
            "models": foreign.models
        });
        let path = home.path().join("models_cache.json");
        let bytes = serde_json::to_vec(&cache).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            native_provider_catalog(&config, Vec::new()).await.unwrap(),
            expected
        );
        assert!(!admits(&config, "foreign-cache-only", &ReasoningEffort::None).await);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!home.path().join("auth.json").exists());
    }

    #[tokio::test]
    async fn termux_chat_models_live_catalog_requires_current_account_and_exact_private_snapshot() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        let snapshots = state.join("catalogs");
        std::fs::create_dir_all(&snapshots).unwrap();
        let path = snapshots.join("live.json");
        let mut native: serde_json::Value =
            serde_json::from_slice(include_bytes!("fixtures/chat-model-catalog.json")).unwrap();
        native["models"][0]["slug"] = serde_json::json!("chatgpt-desktop/app/server-added-model");
        std::fs::write(&path, serde_json::to_vec(&native).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let provider = ProviderCommand {
            binary: root.path().join("provider"),
            state,
        };
        let reply = serde_json::json!({
            "schema":"codex-chatgpt-app-models-v1", "source":"app_and_config",
            "account_verified_now":true, "inference_submitted":false,
            "observed_at_unix_ms":1, "identity":{"account":"owned","user":"fixture","structure":"personal"},
            "native_catalog":native,"native_catalog_path":path
        });
        let decoded = decode_live_catalog(&provider, &serde_json::to_vec(&reply).unwrap()).unwrap();
        assert!(
            decoded
                .models
                .iter()
                .any(|model| model.model == "chatgpt-desktop/app/server-added-model")
        );
        assert_eq!(decoded.path, path);
        let mut config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(root.path().to_path_buf())
            .build()
            .await
            .unwrap();
        assert!(decoded.requires_binding(&config));
        config.model_catalog_source = Some(decoded.source.clone());
        assert!(!decoded.requires_binding(&config));
        for (field, value) in [
            ("account_verified_now", serde_json::json!(false)),
            ("source", serde_json::json!("configured_only")),
            ("inference_submitted", serde_json::json!(true)),
            ("schema", serde_json::json!("old-protocol")),
            (
                "native_catalog_path",
                serde_json::json!(root.path().join("foreign.json")),
            ),
        ] {
            let mut invalid = reply.clone();
            invalid[field] = value;
            assert!(
                decode_live_catalog(&provider, &serde_json::to_vec(&invalid).unwrap()).is_err(),
                "{field}"
            );
        }
        let mut changed = reply.clone();
        changed["native_catalog"]["models"][0]["slug"] =
            serde_json::json!("chatgpt-desktop/app/changed");
        assert!(decode_live_catalog(&provider, &serde_json::to_vec(&changed).unwrap()).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(decode_live_catalog(&provider, &serde_json::to_vec(&reply).unwrap()).is_err());
    }

    #[tokio::test]
    async fn termux_chat_models_installed_catalog_decodes_in_real_config() {
        let home = tempfile::tempdir().unwrap();
        let catalog = installed_profile(home.path());
        let config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(vec![
                ("model".into(), "gpt-6.1-sol".into()),
                ("model_reasoning_effort".into(), "high".into()),
                ("sandbox_mode".into(), "danger-full-access".into()),
                ("approval_policy".into(), "on-request".into()),
                ("approvals_reviewer".into(), "auto_review".into()),
                ("shell_environment_policy.inherit".into(), "none".into()),
            ])
            .build()
            .await
            .unwrap();
        assert!(
            installed_routing(&config).is_some(),
            "marked routing must decode"
        );
        let mut cold = config.clone();
        assert_eq!(
            restore_catalog(&mut cold, Some("gpt-6.1-sol"), None),
            Ok(false)
        );
        assert!(cold.model_catalog.is_none());
        assert_eq!(
            restore_catalog(&mut cold, Some("chatgpt-desktop/gpt-6"), None),
            Ok(true)
        );
        assert_eq!(
            cold.model_catalog_source.as_deref(),
            Some(include_bytes!("fixtures/chat-model-catalog.json").as_slice())
        );
        assert_eq!(cold.model.as_deref(), Some("gpt-6.1-sol"));
        assert_eq!(cold.model_reasoning_effort, Some(ReasoningEffort::High));
        let fresh_path = home.path().join("fresh-models.json");
        let mut fresh: serde_json::Value =
            serde_json::from_slice(include_bytes!("fixtures/chat-model-catalog.json")).unwrap();
        fresh["models"][0]["slug"] = serde_json::json!("chatgpt-desktop/app/new-server-model");
        std::fs::write(&fresh_path, serde_json::to_vec(&fresh).unwrap()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fresh_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut fresh_resume = config.clone();
        assert_eq!(
            restore_catalog(
                &mut fresh_resume,
                Some("chatgpt-desktop/app/new-server-model"),
                Some(&fresh_path)
            ),
            Ok(true)
        );
        assert_eq!(fresh_resume.model.as_deref(), config.model.as_deref());
        assert_eq!(
            fresh_resume.model_reasoning_effort,
            config.model_reasoning_effort
        );
        assert!(
            restore_catalog(
                &mut config.clone(),
                Some("chatgpt-desktop/app/retired-model"),
                Some(&fresh_path)
            )
            .is_err()
        );
        let mut explicit_catalog = cold.clone();
        explicit_catalog
            .model_catalog
            .as_mut()
            .unwrap()
            .models
            .retain(|m| m.slug == "chatgpt-desktop/gpt-6-pro");
        assert_eq!(
            restore_catalog(&mut explicit_catalog, Some("chatgpt-desktop/gpt-6"), None),
            Ok(false)
        );
        assert_eq!(
            explicit_catalog.model_catalog.unwrap().models[0].slug,
            "chatgpt-desktop/gpt-6-pro"
        );
        // An ordinary resume can resolve its saved provider; a fresh native
        // thread retains its original defaults and explicit routes still win.
        let mut session = toml::Value::Table(toml::Table::new());
        add_installed_provider(&config, &mut session);
        assert_eq!(session.as_table().unwrap().len(), 1);
        assert_eq!(
            session["model_providers"][PROVIDER]["auth"]["command"].as_str(),
            Some("/owned/codex-chatgpt-provider")
        );
        assert!(session.get("model_provider").is_none());
        assert!(session.get("model_catalog_json").is_none());
        let mut resumed_overrides: Vec<_> = session
            .as_table()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        resumed_overrides.push(("model_provider".into(), PROVIDER.into()));
        let resumed = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(resumed_overrides)
            .build()
            .await
            .unwrap();
        assert_eq!(resumed.model_provider_id, PROVIDER);
        let mut explicit = toml::Value::Table(toml::Table::from_iter([(
            "model_providers".into(),
            installed_routing(&config).unwrap()["model_providers"].clone(),
        )]));
        explicit["model_providers"][PROVIDER]["name"] = "User route".into();
        let original = explicit.clone();
        add_installed_provider(&config, &mut explicit);
        assert_eq!(explicit, original);
        let models = chat_catalog(&config).expect("actual installed ModelInfo catalog must decode");
        assert_eq!(models.len(), 2);
        assert!(models.iter().any(|m| m.model == "chatgpt-desktop/gpt-6"));
        assert!(admits(&config, "chatgpt-desktop/gpt-6", &ReasoningEffort::XHigh).await);
        assert!(
            !admits(
                &config,
                "chatgpt-desktop/gpt-6-pro",
                &ReasoningEffort::XHigh
            )
            .await
        );
        let mut custom: serde_json::Value =
            serde_json::from_slice(include_bytes!("fixtures/chat-model-catalog.json")).unwrap();
        let mut added = custom["models"][0].clone();
        added["slug"] = serde_json::json!("chatgpt-desktop/configured-model");
        custom["models"].as_array_mut().unwrap().push(added);
        std::fs::write(&catalog, serde_json::to_vec(&custom).unwrap()).unwrap();
        assert!(
            admits(
                &config,
                "chatgpt-desktop/configured-model",
                &ReasoningEffort::None
            )
            .await
        );
        custom["models"].as_array_mut().unwrap().last_mut().unwrap()["visibility"] =
            serde_json::json!("hide");
        std::fs::write(&catalog, serde_json::to_vec(&custom).unwrap()).unwrap();
        let mut hidden_resume = config.clone();
        assert_eq!(
            restore_catalog(
                &mut hidden_resume,
                Some("chatgpt-desktop/configured-model"),
                None
            ),
            Ok(true),
            "a saved model remains resumable when it is absent from the picker"
        );

        let route = reentry_route(
            &config,
            "chatgpt-desktop/gpt-6",
            &ReasoningEffort::XHigh,
            None,
        )
        .expect("route planning needs no App, server or auth execution");
        assert!(route.return_profile.is_none());
        for flag in [
            "model_provider=\"chatgpt_desktop\"",
            "model_reasoning_effort=\"xhigh\"",
            "approval_policy=\"on-request\"",
            "approvals_reviewer=\"auto_review\"",
        ] {
            assert!(
                route.args.iter().any(|arg| arg == flag),
                "missing {flag}: {:?}",
                route.args
            );
        }
        assert!(
            route
                .args
                .iter()
                .any(|arg| arg.starts_with("shell_environment_policy=") && arg.contains("none"))
        );
        assert!(
            !route
                .args
                .iter()
                .any(|arg| matches!(arg.as_str(), "resume" | "--model"))
        );
        let native = reentry_route(&config, "gpt-6.1-sol", &ReasoningEffort::Medium, None).unwrap();
        assert!(
            native
                .args
                .iter()
                .any(|arg| arg == "model_provider=\"openai\"")
        );
        assert!(
            !native
                .args
                .iter()
                .any(|arg| arg.starts_with("model_providers.chatgpt_desktop="))
        );
        assert!(native.return_profile.is_none());
        assert!(safe_profile("work-2"));
        assert!(!safe_profile("../other"));
    }

    #[tokio::test]
    async fn termux_chat_models_partial_cli_provider_keeps_defaults_through_reentry() {
        let home = tempfile::tempdir().unwrap();
        installed_profile(home.path());
        let overrides = vec![
            (
                "model_providers.chatgpt_desktop.name".into(),
                "Explicit Chat route".into(),
            ),
            (
                "model_providers.chatgpt_desktop.base_url".into(),
                "http://127.0.0.1:23456/v1".into(),
            ),
            (
                "model_providers.chatgpt_desktop.request_max_retries".into(),
                2.into(),
            ),
            ("model_providers.other.name".into(), "Other provider".into()),
        ];
        let config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(overrides.clone())
            .build()
            .await
            .unwrap();
        assert_eq!(config.model_provider_id, "openai");
        assert!(config.model_providers[PROVIDER].auth.is_none());

        let route = reentry_route(
            &config,
            "chatgpt-desktop/gpt-6",
            &ReasoningEffort::None,
            None,
        )
        .unwrap();
        let resumed = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(route_overrides(route))
            .build()
            .await
            .unwrap();
        assert_eq!(resumed.model_provider_id, PROVIDER);
        assert_eq!(
            resumed.model_provider.base_url.as_deref(),
            Some("http://127.0.0.1:23456/v1")
        );
        assert_eq!(resumed.model_provider.request_max_retries, Some(2));
        assert_eq!(resumed.model_provider.name, "Explicit Chat route");
        assert_eq!(
            resumed.model_provider.auth.as_ref().unwrap().command,
            "/owned/codex-chatgpt-provider"
        );
        assert_eq!(resumed.model_providers["other"].name, "Other provider");

        let mut session = codex_config::build_cli_overrides_layer(&overrides);
        add_installed_provider(&config, &mut session);
        let mut session_overrides: Vec<_> =
            session.as_table().unwrap().clone().into_iter().collect();
        session_overrides.push(("model_provider".into(), PROVIDER.into()));
        let attached = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(session_overrides)
            .build()
            .await
            .unwrap();
        assert_eq!(attached.model_provider, resumed.model_provider);
        assert_eq!(
            attached.model_providers["other"],
            resumed.model_providers["other"]
        );
    }

    #[tokio::test]
    async fn termux_chat_models_file_provider_values_remain_in_files() {
        let home = tempfile::tempdir().unwrap();
        installed_profile(home.path());
        let user_config = r#"
[model_providers.chatgpt_desktop]
name="File-owned provider"
base_url="http://127.0.0.1:34567/v1"
[model_providers.chatgpt_desktop.http_headers]
X-Private="file-only-header-secret"
[model_providers.chatgpt_desktop.auth]
command="/file-owned/auth-command"
args=["file-only-auth-secret"]
"#;
        std::fs::write(home.path().join("config.toml"), user_config).unwrap();
        let profile_path = home.path().join("chatgpt.config.toml");
        let profile_before = std::fs::read(&profile_path).unwrap();
        let config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .unwrap();
        let route = reentry_route(
            &config,
            "chatgpt-desktop/gpt-6",
            &ReasoningEffort::None,
            None,
        )
        .unwrap();
        for private in [
            "file-only-header-secret",
            "file-only-auth-secret",
            "/file-owned/auth-command",
        ] {
            assert!(route.args.iter().all(|arg| !arg.contains(private)));
        }
        let resumed = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(route_overrides(route))
            .build()
            .await
            .unwrap();
        assert_eq!(resumed.model_provider.name, "File-owned provider");
        assert_eq!(
            resumed.model_provider.base_url.as_deref(),
            Some("http://127.0.0.1:34567/v1")
        );
        assert_eq!(
            resumed.model_provider.auth.as_ref().unwrap().command,
            "/file-owned/auth-command"
        );
        let effective = resumed.config_layer_stack.effective_config();
        assert_eq!(
            effective["model_providers"][PROVIDER]["http_headers"]["X-Private"].as_str(),
            Some("file-only-header-secret")
        );
        assert_eq!(
            effective["model_providers"][PROVIDER]["auth"]["args"]
                .as_array()
                .unwrap()
                .as_slice(),
            [toml::Value::from("file-only-auth-secret")]
        );
        assert_eq!(
            std::fs::read_to_string(home.path().join("config.toml")).unwrap(),
            user_config
        );
        assert_eq!(std::fs::read(&profile_path).unwrap(), profile_before);
    }

    #[tokio::test]
    async fn termux_chat_models_defaults_do_not_repair_invalid_provider_values() {
        let home = tempfile::tempdir().unwrap();
        installed_profile(home.path());
        let config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .unwrap();
        for (path, value) in [
            (
                "model_providers.chatgpt_desktop.name",
                toml::Value::from(""),
            ),
            (
                "model_providers.chatgpt_desktop",
                toml::Value::from("invalid-provider"),
            ),
            (
                "model_providers.chatgpt_desktop.auth",
                toml::Value::Array(vec![]),
            ),
        ] {
            let overrides = vec![(path.into(), value)];
            let mut session = codex_config::build_cli_overrides_layer(&overrides);
            add_installed_provider(&config, &mut session);
            let mut session_overrides: Vec<_> =
                session.as_table().unwrap().clone().into_iter().collect();
            session_overrides.push(("model_provider".into(), PROVIDER.into()));
            assert!(
                crate::legacy_core::config::ConfigBuilder::default()
                    .codex_home(home.path().to_path_buf())
                    .cli_overrides(session_overrides)
                    .build()
                    .await
                    .is_err()
            );
        }
    }

    #[test]
    fn termux_chat_models_routing_carries_command_references_without_secret_headers() {
        let mut value: toml::Value = toml::from_str(
            r#"
model_provider = "chatgpt_desktop"
model_catalog_json = "/owned/models.json"
[model_providers.chatgpt_desktop]
base_url = "http://127.0.0.1:12345/v1"
wire_api = "responses"
requires_openai_auth = false
[model_providers.chatgpt_desktop.auth]
command = "/owned/codex-chatgpt-provider"
args = ["token", "--state", "/owned/provider-state"]
"#,
        )
        .unwrap();
        assert!(checked_routing(value.clone()).is_some());
        value["model_providers"][PROVIDER]
            .as_table_mut()
            .unwrap()
            .insert(
                "http_headers".into(),
                toml::Value::Table(toml::Table::new()),
            );
        assert!(checked_routing(value.clone()).is_none());
        value["model_providers"][PROVIDER]
            .as_table_mut()
            .unwrap()
            .remove("http_headers");
        value["model_providers"][PROVIDER]["base_url"] = "https://unowned.example/v1".into();
        assert!(checked_routing(value).is_none());
    }
    #[test]
    fn termux_chat_models_identity_and_reentry_do_not_hardcode_families_or_efforts() {
        assert!(valid_chat_id("chatgpt-desktop/gpt-6"));
        assert!(valid_chat_id("chatgpt-desktop/configured-future-model"));
        for invalid in [
            "chatgpt-desktop/",
            "chatgpt-desktop/space name",
            "gpt-6.1-sol",
        ] {
            assert!(!valid_chat_id(invalid));
        }
        assert!(needs_reentry("openai", "chatgpt-desktop/gpt-6"));
        assert!(needs_reentry(PROVIDER, "gpt-6.1-sol"));
        assert!(!needs_reentry(
            PROVIDER,
            "chatgpt-desktop/configured-future-model"
        ));
        assert!(!needs_reentry("openai", "gpt-6.1-sol"));
    }
    #[test]
    fn termux_chat_models_private_catalog_refuses_symlink_nonprivate_and_oversize() {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("catalog");
        std::fs::write(&file, b"fixture").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(private_bytes(&file, 8), Some(b"fixture".to_vec()));
        assert!(private_bytes(&file, 2).is_none());
        let link = temp.path().join("link");
        symlink(&file, &link).unwrap();
        assert!(private_bytes(&link, 8).is_none());
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(private_bytes(&file, 8).is_none());
        let fifo = temp.path().join("fifo");
        let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(private_bytes(&fifo, 8).is_none());
    }

    #[test]
    fn termux_chat_models_shared_registration_alias_and_fallback_without_local_copy() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("shared.config.toml");
        let local = root.path().join("chatgpt.config.toml");
        assert_eq!(
            routing_profile_path(&local, Some(&shared)),
            Some(shared.clone())
        );
        symlink(&shared, &local).unwrap();
        assert_eq!(routing_profile_path(&local, Some(&shared)), Some(shared));
    }

    #[test]
    fn termux_chat_models_local_override_and_foreign_alias_never_fall_through_to_shared() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("shared.config.toml");
        let local = root.path().join("chatgpt.config.toml");
        std::fs::write(&local, "model=\"user-choice\"\n").unwrap();
        assert_eq!(
            routing_profile_path(&local, Some(&shared)),
            Some(local.clone())
        );
        assert_eq!(routing_profile_path(&local, None), Some(local.clone()));
        std::fs::remove_file(&local).unwrap();
        symlink("user-missing", &local).unwrap();
        assert_eq!(
            routing_profile_path(&local, Some(&shared)),
            Some(local.clone())
        );
        assert!(private_bytes(&local, 65536).is_none());
    }
}
