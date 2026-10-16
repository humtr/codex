//! Optional installed Chat catalog; ordinary upstream model actions remain intact.
use crate::legacy_core::config::Config;
use codex_config::ConfigLayerSource;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub(crate) const PROVIDER: &str = "chatgpt_desktop_trial";
const BEGIN: &str = "# codex-web-experiment ChatGPT profile begin\n";

pub(crate) fn private_bytes(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
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

pub(crate) fn valid_chat(model: &str, effort: &ReasoningEffort) -> bool {
    match model {
        "chatgpt-desktop/gpt-6" | "chatgpt-desktop/gpt-5.6-sol" => matches!(
            effort,
            ReasoningEffort::None
                | ReasoningEffort::Medium
                | ReasoningEffort::High
                | ReasoningEffort::XHigh
        ),
        "chatgpt-desktop/gpt-6-pro" | "chatgpt-desktop/gpt-5.6-sol-pro" => {
            *effort == ReasoningEffort::None
        }
        _ => false,
    }
}

pub(crate) fn needs_reentry(provider: &str, model: &str) -> bool {
    provider == PROVIDER || model.starts_with("chatgpt-desktop/")
}

/// Provider routing is planned without App/chatwidget or server lifecycle access.
pub(crate) struct ReentryRoute {
    pub args: Vec<String>,
    pub return_profile: Option<String>,
}

fn config_profile(config: &crate::legacy_core::config::Config) -> Option<String> {
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
    // Preserve explicit non-routing session settings rather than silently dropping them.
    for layer in config.config_layer_stack.layers_low_to_high() {
        if matches!(layer.name, ConfigLayerSource::SessionFlags)
            && let Some(table) = layer.config.as_table()
        {
            for (key, value) in table {
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
        }
    }
    if entering {
        // Keep the selected user configuration and add only the installed route.
        // Named permission definitions, MCP settings and secrets stay in files.
        let routing = installed_routing(config)?;
        let provider = routing.get("model_providers")?.get(PROVIDER)?;
        args.extend([
            "-c".into(),
            format!("model_provider={:?}", PROVIDER),
            "-c".into(),
            format!("model_catalog_json={}", routing.get("model_catalog_json")?),
            "-c".into(),
            format!("model_providers.{}={provider}", PROVIDER),
        ]);
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

#[derive(Deserialize)]
struct Catalog {
    models: Vec<ModelInfo>,
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
    if args.len() != 5
        || args[0].as_str()? != "-B"
        || args[2].as_str()? != "token"
        || args[3].as_str()? != "--runtime"
        || !Path::new(args[1].as_str()?).is_absolute()
        || Path::new(args[1].as_str()?).file_name()? != "desktop_lifecycle.py"
        || !Path::new(args[4].as_str()?).is_absolute()
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

fn chat_catalog(config: &Config) -> Option<Vec<ModelPreset>> {
    let value = installed_routing(config)?;
    let path = Path::new(value.get("model_catalog_json")?.as_str()?);
    if !path.is_absolute() {
        return None;
    }
    let catalog: Catalog = match serde_json::from_slice(&private_bytes(path, 4 * 1024 * 1024)?) {
        Ok(catalog) => catalog,
        Err(error) => {
            tracing::debug!(%error, "installed Chat catalog could not be decoded");
            return None;
        }
    };
    let models: Vec<ModelPreset> = catalog
        .models
        .into_iter()
        .map(Into::into)
        .filter(|m: &ModelPreset| {
            m.show_in_picker
                && valid_chat(&m.model, &m.default_reasoning_effort)
                && !m.supported_reasoning_efforts.is_empty()
                && m.supported_reasoning_efforts
                    .iter()
                    .all(|e| valid_chat(&m.model, &e.effort))
        })
        .collect();
    (!models.is_empty()).then_some(models)
}

pub(crate) fn augment(config: &Config, mut models: Vec<ModelPreset>) -> Vec<ModelPreset> {
    if !crate::termux_permissions::enabled()
        || !matches!(config.model_provider_id.as_str(), "openai" | PROVIDER)
        || config
            .config_layer_stack
            .required_model_provider()
            .is_some()
        || std::env::var_os("CODEX_PROFILE_CORE").is_none()
    {
        return models;
    }
    let Some(chat) = chat_catalog(config) else {
        return models;
    };
    let mut seen: HashSet<String> = models.iter().map(|m| m.model.clone()).collect();
    if config.model_provider_id == PROVIDER
        && let Some(bytes) = private_bytes(
            &config.codex_home.join("models_cache.json"),
            4 * 1024 * 1024,
        )
        && let Ok(native) = serde_json::from_slice::<Catalog>(&bytes)
    {
        for model in native.models.into_iter().map(ModelPreset::from) {
            if model.show_in_picker
                && !model.model.starts_with("chatgpt-desktop/")
                && seen.insert(model.model.clone())
            {
                models.push(model);
            }
        }
    }
    for model in chat {
        if seen.insert(model.model.clone()) {
            models.push(model);
        }
    }
    models
}

pub(crate) fn admits(config: &Config, model: &str, effort: &ReasoningEffort) -> bool {
    if model.starts_with("chatgpt-desktop/") {
        return valid_chat(model, effort)
            && chat_catalog(config).is_some_and(|catalog| {
                catalog.iter().any(|m| {
                    m.model == model
                        && m.supported_reasoning_efforts
                            .iter()
                            .any(|e| e.effort == *effort)
                })
            });
    }
    private_bytes(
        &config.codex_home.join("models_cache.json"),
        4 * 1024 * 1024,
    )
    .and_then(|bytes| serde_json::from_slice::<Catalog>(&bytes).ok())
    .is_some_and(|catalog| {
        catalog.models.iter().any(|m| {
            m.slug == model
                && m.supported_reasoning_levels
                    .iter()
                    .any(|e| e.effort == *effort)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn termux_chat_models_installed_catalog_decodes_in_real_config() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let catalog = home.path().join("chat-models.json");
        std::fs::write(&catalog, include_bytes!("fixtures/chat-model-catalog.json")).unwrap();
        std::fs::set_permissions(&catalog, std::fs::Permissions::from_mode(0o600)).unwrap();
        let profile = format!(
            "{BEGIN}model_provider = \"{PROVIDER}\"\nmodel_catalog_json = {:?}\n[model_providers.{PROVIDER}]\nbase_url = \"http://127.0.0.1:12345/v1\"\nwire_api = \"responses\"\nrequires_openai_auth = false\n[model_providers.{PROVIDER}.auth]\ncommand = \"/owned/python\"\nargs = [\"-B\", \"/owned/desktop_lifecycle.py\", \"token\", \"--runtime\", \"/owned/runtime.json\"]\n",
            catalog.to_str().unwrap()
        );
        let path = home.path().join("chatgpt.config.toml");
        std::fs::write(&path, profile).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .cli_overrides(vec![
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
        let models = chat_catalog(&config).expect("actual installed ModelInfo catalog must decode");
        assert_eq!(models.len(), 4);
        assert!(models.iter().any(|m| m.model == "chatgpt-desktop/gpt-6"));
        let route = reentry_route(
            &config,
            "chatgpt-desktop/gpt-6",
            &ReasoningEffort::XHigh,
            None,
        )
        .expect("route planning needs no App, server or auth execution");
        assert!(route.return_profile.is_none());
        for flag in [
            "model_provider=\"chatgpt_desktop_trial\"",
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
                .any(|arg| arg.starts_with("model_providers.chatgpt_desktop_trial="))
        );
        assert!(native.return_profile.is_none());
        assert!(safe_profile("work-2"));
        assert!(!safe_profile("../other"));
    }

    #[test]
    fn termux_chat_models_routing_carries_command_references_without_secret_headers() {
        let mut value: toml::Value = toml::from_str(
            r#"
model_provider = "chatgpt_desktop_trial"
model_catalog_json = "/owned/models.json"
[model_providers.chatgpt_desktop_trial]
base_url = "http://127.0.0.1:12345/v1"
wire_api = "responses"
requires_openai_auth = false
[model_providers.chatgpt_desktop_trial.auth]
command = "/owned/python"
args = ["-B", "/owned/desktop_lifecycle.py", "token", "--runtime", "/owned/runtime.json"]
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
    fn termux_chat_models_routes_preserve_family_power_and_pro_is_separate() {
        for model in ["chatgpt-desktop/gpt-6", "chatgpt-desktop/gpt-5.6-sol"] {
            for effort in [
                ReasoningEffort::None,
                ReasoningEffort::Medium,
                ReasoningEffort::High,
                ReasoningEffort::XHigh,
            ] {
                assert!(valid_chat(model, &effort));
            }
            for effort in [
                ReasoningEffort::Low,
                ReasoningEffort::Max,
                ReasoningEffort::Ultra,
            ] {
                assert!(!valid_chat(model, &effort));
            }
        }
        assert!(valid_chat(
            "chatgpt-desktop/gpt-6-pro",
            &ReasoningEffort::None
        ));
        assert!(!valid_chat(
            "chatgpt-desktop/gpt-6-pro",
            &ReasoningEffort::XHigh
        ));
        assert!(!valid_chat(
            "chatgpt-desktop/gpt-6-sol",
            &ReasoningEffort::None
        ));
        assert!(needs_reentry("openai", "chatgpt-desktop/gpt-6"));
        assert!(needs_reentry(PROVIDER, "gpt-6.1-sol"));
        assert!(needs_reentry(PROVIDER, "chatgpt-desktop/gpt-5.6-sol"));
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
