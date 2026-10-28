//! Optional app/source controls shared by services with and without native work.
use crate::{app::Bridge, config::Config, journal::Journal, protocol::Error};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) struct Controls {
    pub bridge: Arc<dyn Bridge>,
    pub journal: Arc<Journal>,
    pub config: Config,
    pub mutations: tokio::sync::Mutex<()>,
}
pub(crate) fn ensure_public_text<'a>(
    text: &str,
    private_values: impl IntoIterator<Item = &'a str>,
) -> Result<(), Error> {
    if private_values
        .into_iter()
        .any(|value| !value.is_empty() && text.contains(value))
    {
        return Err(Error("private_control_in_public_text"));
    }
    Ok(())
}
pub(crate) fn public_work_ids(journal: &Journal) -> Result<Vec<String>, Error> {
    let mut ids = std::collections::BTreeSet::new();
    for (key, record) in journal.records(crate::operator::WORK_SCHEMA)? {
        if let (Some(id), Some(generation)) = (record["id"].as_str(), record["generation"].as_u64())
        {
            if id.len() == 48
                && id.bytes().all(|v| v.is_ascii_hexdigit())
                && (key == crate::context::digest(&("operator-work-v1", id))
                    || key
                        == crate::context::digest(&(
                            "operator-previous-controller-v1",
                            id,
                            generation,
                        )))
            {
                ids.insert(id.to_owned());
            }
        }
    }
    Ok(ids.into_iter().collect())
}
pub(crate) fn tools() -> Vec<Value> {
    crate::operator::tools().into_iter().filter_map(|mut tool|{
        match tool["name"].as_str()? {
            "codex_host"|"codex_chats"|"codex_projects"=>Some(tool),
            "codex_sessions"=>{
                tool["description"]=json!("Explicit saved Chat input and original result retrieval. chat_input requires conversation_id, complete source_cursor, task, stable call_id and finite budget; chat_result observes input_id without resending. Native work requires a separately configured execution binding.");
                tool["inputSchema"]["properties"]["action"]=json!({"type":"string","enum":["chat_input","chat_result"]});Some(tool)
            },
            _=>None,
        }
    }).collect()
}
impl Controls {
    pub async fn call(&self, name: &str, args: Value, secrets: &[String]) -> Result<Value, Error> {
        let mut redactions = secrets.to_vec();
        redactions.push(self.journal.owner.clone());
        let secrets = redactions.as_slice();
        crate::operator::validate_arguments(&tools(), name, &args)?;
        match name {
            "codex_projects" => {
                let path =
                    crate::store::config_path().map_err(|_| Error("invalid_configuration_path"))?;
                crate::project::control(self.bridge.as_ref(), &self.journal, &path, &args).await
            }
            "codex_chats" => {
                crate::chats::Chats::new(self.bridge.clone())
                    .with_public_ids(public_work_ids(&self.journal)?)
                    .call(args, secrets)
                    .await
            }
            "codex_host" => {
                host(
                    self.bridge.as_ref(),
                    &self.journal,
                    &crate::store::config_path()
                        .map_err(|_| Error("invalid_configuration_path"))?,
                    &args,
                )
                .await
            }
            "codex_sessions" => {
                let inputs = crate::chat_input::Inputs {
                    bridge: self.bridge.clone(),
                    journal: self.journal.clone(),
                    config: self.config.clone(),
                    config_path: Some(
                        crate::store::config_path()
                            .map_err(|_| Error("invalid_configuration_path"))?,
                    ),
                };
                match args["action"].as_str() {
                    Some("chat_input") => {
                        let _guard = self.mutations.lock().await;
                        inputs.submit(args, secrets).await
                    }
                    Some("chat_result") => inputs.result(args, secrets).await,
                    _ => Err(Error("native_execution_binding_required")),
                }
            }
            _ => Err(Error("mcp_tool_unknown")),
        }
    }
}
pub(crate) async fn host(
    bridge: &dyn Bridge,
    journal: &Journal,
    config_path: &std::path::Path,
    args: &Value,
) -> Result<Value, Error> {
    let action = args["action"]
        .as_str()
        .ok_or(Error("host_action_invalid"))?;
    if action != "models" && args.get("refresh").is_some() {
        return Err(Error("host_action_invalid"));
    }
    if action == "models" {
        let snapshot = crate::store::Snapshot::open(config_path)
            .map_err(|_| Error("invalid_provider_configuration"))?;
        let config = snapshot
            .config()
            .map_err(|_| Error("invalid_provider_configuration"))?;
        let configured = snapshot
            .configured_models()
            .map_err(|_| Error("invalid_provider_configuration"))?;
        if args["refresh"] == false {
            let cached = crate::models::cached_catalog(journal)?;
            let (catalog, identity, observed, source) = match cached {
                Some(cached) => (
                    config
                        .catalog
                        .with_app_options(&cached.options, &configured)?,
                    serde_json::to_value(cached.identity).unwrap(),
                    json!(cached.observed_at_unix_ms),
                    "retained_app_and_config",
                ),
                None => (config.catalog, Value::Null, Value::Null, "configured_only"),
            };
            return Ok(
                json!({"catalog":catalog,"identity":identity,"observed_at_unix_ms":observed,"source":source,"account_verified_now":false,"inference_submitted":false}),
            );
        }
        let offered = bridge.operation(json!({"op":"model_catalog"})).await?;
        let catalog = config
            .catalog
            .with_app_options(&offered["options"], &configured)?;
        let identity = bridge.operation(json!({"op":"identity"})).await?;
        if ["account", "user", "structure"]
            .iter()
            .any(|key| identity[*key] != offered["identity"][*key])
        {
            return Err(Error("app_account_changed"));
        }
        if !snapshot
            .unchanged()
            .map_err(|_| Error("invalid_provider_configuration"))?
        {
            return Err(Error("provider_configuration_changed"));
        }
        let observed_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let snapshot = crate::models::AppSnapshot {
            identity: crate::app::Identity::parse(&identity)?,
            options: offered["options"].clone(),
            observed_at_unix_ms,
        };
        journal.write(&crate::models::snapshot_key(), &snapshot)?;
        return Ok(
            json!({"catalog":catalog,"identity":offered["identity"],"observed_at_unix_ms":observed_at_unix_ms,"source":"app_and_config","inference_submitted":false}),
        );
    }
    let op = match action {
        "status" => "host_status",
        "ensure" => "host_ensure",
        "open" => "host_open",
        "stop" => "host_stop",
        _ => return Err(Error("host_action_invalid")),
    };
    let observed = bridge.operation(json!({"op":op})).await;
    if action != "status" {
        return observed;
    }
    Ok(match observed {
        Ok(value) => {
            json!({"service":"authenticated","host":value,"connector":"not_queried","viewer":"separate","observed_at_unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(),"inference_submitted":false})
        }
        Err(error) => {
            json!({"service":"authenticated","host":{"ready":false,"state":"unknown","cause":error.0},"connector":"not_queried","inference_submitted":false})
        }
    })
}

/// Resolve current configuration against account-bound offered options. Discovery
/// is on demand when no observation exists; this never polls saved Chat history.
pub(crate) async fn admitted_catalog(
    bridge: &dyn Bridge,
    journal: &Journal,
    config_path: &std::path::Path,
) -> Result<crate::models::Catalog, Error> {
    if let Some((identity, catalog)) = cached_model_catalog(journal, config_path)? {
        let current =
            crate::app::Identity::parse(&bridge.operation(json!({"op":"identity"})).await?)?;
        if !identity.account_matches(&current) {
            return Err(Error("app_account_changed"));
        }
        return Ok(catalog);
    }
    let discovered = host(bridge, journal, config_path, &json!({"action":"models"})).await?;
    serde_json::from_value(discovered["catalog"].clone())
        .map_err(|_| Error("app_model_catalog_invalid"))
}

pub(crate) fn cached_model_catalog(
    journal: &Journal,
    config_path: &std::path::Path,
) -> Result<Option<(crate::app::Identity, crate::models::Catalog)>, Error> {
    let Some(cached) = crate::models::cached_catalog(journal)? else {
        return Ok(None);
    };
    let snapshot = crate::store::Snapshot::open(config_path)
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let config = snapshot
        .config()
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let configured = snapshot
        .configured_models()
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let catalog = config
        .catalog
        .with_app_options(&cached.options, &configured)?;
    Ok(Some((cached.identity, catalog)))
}
