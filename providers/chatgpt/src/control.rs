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
pub(crate) fn public_work_ids(journal: &Journal) -> Result<Vec<String>, Error> {
    let mut ids = std::collections::BTreeSet::new();
    for (key, record) in journal.records("termux-codex-operator-work-v1")? {
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
            "codex_host"|"codex_chats"=>Some(tool),
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
        crate::operator::validate_arguments(&tools(), name, &args)?;
        match name {
            "codex_chats" => {
                crate::chats::Chats::new(self.bridge.clone())
                    .with_public_ids(public_work_ids(&self.journal)?)
                    .call(args, secrets)
                    .await
            }
            "codex_host" => host(self.bridge.as_ref(), &args).await,
            "codex_sessions" => {
                let inputs = crate::chat_input::Inputs {
                    bridge: self.bridge.clone(),
                    journal: self.journal.clone(),
                    config: self.config.clone(),
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
pub(crate) async fn host(bridge: &dyn Bridge, args: &Value) -> Result<Value, Error> {
    let action = args["action"]
        .as_str()
        .ok_or(Error("host_action_invalid"))?;
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
