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
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

// One creation intent belongs to one account/name binding in its existing
// journal. Other destinations and independent Provider journals must not wait
// for that binding's app network I/O. Weak entries retain no idle lock history.
type ResolutionLocks = BTreeMap<(String, String), Weak<tokio::sync::Mutex<()>>>;
static RESOLUTIONS: tokio::sync::Mutex<ResolutionLocks> =
    tokio::sync::Mutex::const_new(BTreeMap::new());

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
    resolve_with_context(bridge, journal, history, identity, &Value::Null).await
}
pub(crate) async fn resolve_with_context(
    bridge: &dyn Bridge,
    journal: &Journal,
    history: &HistoryConfig,
    identity: &Value,
    context: &Value,
) -> Result<Option<String>, Error> {
    if history.mode == HistoryMode::Retain {
        return Ok(None);
    }
    let operation = |mut command: Value| {
        if let Some(context) = context.as_object() {
            for (key, value) in context {
                command[key] = value.clone();
            }
        }
        bridge.operation(command)
    };
    let name = history
        .project_name
        .as_deref()
        .ok_or(Error("project_name_required"))?;
    let identity = json!({"account":identity["account"],"user":identity["user"],"structure":identity["structure"]});
    // The selected destination belongs to an authenticated account, not to
    // whichever account last wrote the shared Provider config. Use a stable
    // account/name binding across changes to the shared configured project ID.
    let key =
        digest(&json!({"kind":"project-destination-account-v2","identity":identity,"name":name}));
    let legacy_key = digest(
        &json!({"kind":"project-destination-v1","identity":identity,"name":name,"configured_id":history.project_id}),
    );
    let lock = {
        let mut locks = RESOLUTIONS.lock().await;
        locks.retain(|_, lock| lock.strong_count() > 0);
        let lock_key = (journal.owner.clone(), key.clone());
        match locks.get(&lock_key).and_then(Weak::upgrade) {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(tokio::sync::Mutex::new(()));
                locks.insert(lock_key, Arc::downgrade(&lock));
                lock
            }
        }
    };
    let _guard = lock.lock().await;
    let previous = match journal.read::<Binding>(&key)? {
        Some(binding) => Some(binding),
        None => journal.read::<Binding>(&legacy_key)?,
    };
    let pinned = previous.as_ref().and_then(|v| v.id.as_deref());
    // A verified prior binding is stable across account switches. Check the
    // shared explicit ID only when it differs from that account's binding.
    if let Some(id) = pinned.filter(|id| {
        history
            .project_id
            .as_deref()
            .is_none_or(|configured| configured == *id)
    }) {
        let found =
            operation(json!({"op":"project_read","project":id,"identity":identity})).await?;
        let verified = project(found, name)?;
        if verified != *id {
            return Err(Error("project_identity_changed"));
        }
        journal.write(
            &key,
            &Binding {
                name: name.into(),
                id: Some(verified.clone()),
                creating: false,
            },
        )?;
        return Ok(Some(verified));
    }
    let found = operation(json!({"op":"project_list","identity":identity})).await?;
    let owned = found.as_array().ok_or(Error("invalid_project_list"))?;
    // Only an ID offered by this authenticated account can pin the current
    // destination. An ID left by another account falls back to name-based
    // resolution in this account without erasing the other account's binding.
    if let Some(configured) = history.project_id.as_deref() {
        if let Some(owned_project) = owned.iter().find(|v| v["id"] == configured) {
            project(owned_project.clone(), name)?;
            let current =
                operation(json!({"op":"project_read","project":configured,"identity":identity}))
                    .await?;
            if project(current, name)? != configured {
                return Err(Error("project_identity_changed"));
            }
            journal.write(
                &key,
                &Binding {
                    name: name.into(),
                    id: Some(configured.into()),
                    creating: false,
                },
            )?;
            return Ok(Some(configured.into()));
        }
    }
    // The global ID is absent from this account's owned list. Keep its own
    // verified journal binding rather than silently switching destinations.
    if let Some(id) = pinned {
        let found =
            operation(json!({"op":"project_read","project":id,"identity":identity})).await?;
        let verified = project(found, name)?;
        if verified != id {
            return Err(Error("project_identity_changed"));
        }
        journal.write(
            &key,
            &Binding {
                name: name.into(),
                id: Some(verified.clone()),
                creating: false,
            },
        )?;
        return Ok(Some(verified));
    }
    let candidates = owned
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
            let created =
                match operation(json!({"op":"project_create","name":name,"identity":identity}))
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
            let verified =
                operation(json!({"op":"project_read","project":id,"identity":identity})).await?;
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

pub async fn control(
    bridge: &dyn Bridge,
    journal: &Journal,
    path: &std::path::Path,
    args: &Value,
) -> Result<Value, Error> {
    match args["action"].as_str() {
        Some("list") => {
            let identity = bridge.operation(json!({"op":"identity"})).await?;
            let projects = bridge
                .operation(json!({"op":"project_list","identity":identity}))
                .await?;
            if !projects.is_array() {
                return Err(Error("invalid_project_list"));
            }
            Ok(json!({"projects":projects,"inference_submitted":false}))
        }
        Some("use") => {
            let snapshot = crate::store::Snapshot::open(path)
                .map_err(|_| Error("invalid_provider_configuration"))?;
            let mut settings = snapshot
                .config()
                .map_err(|_| Error("invalid_provider_configuration"))?;
            let name = args["name"]
                .as_str()
                .ok_or(Error("project_name_required"))?;
            let previous_id = (settings.history.project_name.as_deref() == Some(name))
                .then(|| settings.history.project_id.clone())
                .flatten();
            settings.history.mode = HistoryMode::Project;
            settings.history.project_name = Some(name.into());
            settings.history.project_id = args
                .get("project_id")
                .map(|id| {
                    id.as_str()
                        .map(str::to_owned)
                        .ok_or(Error("invalid_project_identity"))
                })
                .transpose()?
                .or(previous_id);
            settings.history.create_if_missing = true;
            settings
                .validate()
                .map_err(|_| Error("invalid_project_configuration"))?;
            let identity = bridge.operation(json!({"op":"identity"})).await?;
            let id = resolve(bridge, journal, &settings.history, &identity)
                .await?
                .ok_or(Error("project_destination_missing"))?;
            // Recheck the live identity before selecting this account's project.
            let current = bridge.operation(json!({"op":"identity"})).await?;
            for key in ["account", "user", "structure"] {
                if current[key] != identity[key] {
                    return Err(Error("app_account_changed"));
                }
            }
            let values = [
                ("history.mode".into(), "project".into()),
                ("history.project_name".into(), name.into()),
                ("history.project_id".into(), id.clone()),
                ("history.create_if_missing".into(), "true".into()),
            ];
            let changed = snapshot
                .commit_edits(&values, &[])
                .map_err(|_| Error("project_configuration_save_unconfirmed"))?;
            Ok(
                json!({"project":{"id":id,"name":name},"history_mode":"project",
                "configuration_saved":true,"configuration_changed":changed,
                "applies_to":"new_lanes","inference_submitted":false}),
            )
        }
        _ => Err(Error("project_action_invalid")),
    }
}
