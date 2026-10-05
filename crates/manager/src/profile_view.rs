//! One read-only profile selection view for public text and the experimental native client.
use super::*;
use serde_json::json;

struct CurrentProfile {
    // None identifies an inherited home outside the registered profile inventory.
    id: Option<String>,
    source: &'static str,
}

pub(super) fn registered_id(
    context: &Context,
    home: &Path,
) -> Result<Option<String>, ManagerError> {
    if home == context.home.join(".codex") {
        return Ok(Some("default".to_owned()));
    }
    let ids = list_custom_profiles(context)?;
    let dirs = manager_base(context).join("manager").join(PROFILES_DIR);
    Ok(ids
        .into_iter()
        .find(|id| home == profile_home_path(&dirs, id)))
}

fn current(context: &Context) -> Result<CurrentProfile, ManagerError> {
    if let Some(inherited) = context
        .inherited_codex_home
        .as_ref()
        .filter(|value| value.to_str().is_some_and(|value| !value.is_empty()))
    {
        let id = registered_id(context, Path::new(inherited))?;
        return Ok(CurrentProfile {
            id,
            source: "inherited",
        });
    }
    let saved = profile::read_default(context)?;
    Ok(CurrentProfile {
        source: if saved.is_some() { "saved" } else { "default" },
        id: Some(
            saved
                .as_ref()
                .map_or("default", profile::target_name)
                .to_owned(),
        ),
    })
}

pub(super) fn format_current(context: &Context) -> Result<String, ManagerError> {
    let current = current(context)?;
    Ok(format!(
        "current: {}\nsource: {}\n",
        current.id.as_deref().unwrap_or("external"),
        current.source,
    ))
}

pub(super) fn snapshot(context: &Context) -> Result<String, ManagerError> {
    let mut profiles = vec!["default".to_owned()];
    profiles.extend(list_custom_profiles(context)?);
    if profiles.len() > 256 {
        return Err(ERR_PROFILE);
    }
    let current = current(context)?;
    let saved = profile::read_default(context)?;
    let saved_default = saved.as_ref().map_or("default", profile::target_name);
    Ok(format!(
        "{}\n",
        json!({
            "schema": "codex-manager-profiles-v1",
            "profiles": profiles,
            "current": current.id,
            "current_source": current.source,
            "saved_default": saved_default,
        })
    ))
}
