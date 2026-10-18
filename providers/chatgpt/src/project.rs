//! Rust owns destination policy and durable uncertain-create reconciliation.
//! The app bridge only invokes its authenticated project operations.
use crate::{
    app::Bridge,
    config::{HistoryConfig, HistoryMode},
    context::digest,
    journal::Journal,
    protocol::Error,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Serialize, Deserialize)]
struct Binding {
    name: String,
    id: Option<String>,
    creating: bool,
}
fn project(value: Value, name: &str) -> Result<String, Error> {
    if value["name"] != name || value["compatible"] != true {
        return Err(Error("project_destination_incompatible"));
    }
    value["id"]
        .as_str()
        .filter(|s| {
            s.starts_with("g-p-")
                && s.len() <= 256
                && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
        .map(str::to_owned)
        .ok_or(Error("invalid_project_identity"))
}
pub async fn resolve(
    bridge: &dyn Bridge,
    journal: &Journal,
    history: &HistoryConfig,
    identity: &Value,
) -> Result<Option<String>, Error> {
    if history.mode == HistoryMode::Retain {
        return Ok(None);
    }
    let name = history
        .project_name
        .as_deref()
        .ok_or(Error("project_name_required"))?;
    let identity = json!({"account":identity["account"],"user":identity["user"],"structure":identity["structure"]});
    let key = digest(
        &json!({"kind":"project-destination-v1","identity":identity,"name":name,"configured_id":history.project_id}),
    );
    let previous = journal.read::<Binding>(&key)?;
    let selected = history
        .project_id
        .as_ref()
        .or_else(|| previous.as_ref().and_then(|v| v.id.as_ref()));
    if let Some(id) = selected {
        let found = bridge
            .operation(json!({"op":"project_read","project":id,"identity":identity}))
            .await?;
        let verified = project(found, name)?;
        if verified != *id {
            return Err(Error("project_identity_changed"));
        }
        return Ok(Some(verified));
    }
    let found = bridge
        .operation(json!({"op":"project_list","identity":identity}))
        .await?;
    let candidates = found
        .as_array()
        .ok_or(Error("invalid_project_list"))?
        .iter()
        .filter(|v| v["name"] == name)
        .cloned()
        .collect::<Vec<_>>();
    let id = match candidates.as_slice() {
        [one] => project(one.clone(), name)?,
        [] => {
            if previous.as_ref().is_some_and(|v| v.creating) {
                return Err(Error("project_creation_unconfirmed"));
            }
            if !history.create_if_missing {
                return Err(Error("project_destination_missing"));
            }
            journal.write(
                &key,
                &Binding {
                    name: name.into(),
                    id: None,
                    creating: true,
                },
            )?;
            let created = match bridge
                .operation(json!({"op":"project_create","name":name,"identity":identity}))
                .await
            {
                Ok(value) => value,
                Err(Error("project_creation_rejected")) => {
                    journal.write(
                        &key,
                        &Binding {
                            name: name.into(),
                            id: None,
                            creating: false,
                        },
                    )?;
                    return Err(Error("project_creation_rejected"));
                }
                Err(_) => return Err(Error("project_creation_unconfirmed")),
            };
            let id = project(created, name)?;
            // Keep the returned exact ID before readback, even if verification
            // later fails; never create another destination in its place.
            journal.write(
                &key,
                &Binding {
                    name: name.into(),
                    id: Some(id.clone()),
                    creating: false,
                },
            )?;
            let verified = bridge
                .operation(json!({"op":"project_read","project":id,"identity":identity}))
                .await?;
            if project(verified, name)? != id {
                return Err(Error("project_identity_changed"));
            }
            id
        }
        _ => return Err(Error("project_destination_ambiguous")),
    };
    journal.write(
        &key,
        &Binding {
            name: name.into(),
            id: Some(id.clone()),
            creating: false,
        },
    )?;
    Ok(Some(id))
}
