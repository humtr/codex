//! Model routing is configuration data; it is not proof of host entitlement.
use crate::config::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const APP_MODEL_PREFIX: &str = "chatgpt-desktop/app/";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppSnapshot {
    pub identity: crate::app::Identity,
    pub options: serde_json::Value,
    pub observed_at_unix_ms: u64,
}
pub(crate) fn snapshot_key() -> String {
    crate::context::digest(&"app-model-catalog-v1")
}
pub(crate) fn cached_catalog(
    journal: &crate::journal::Journal,
) -> Result<Option<AppSnapshot>, crate::protocol::Error> {
    let cached = journal.read::<AppSnapshot>(&snapshot_key())?;
    if let Some(snapshot) = &cached {
        snapshot.identity.validate()?;
    }
    Ok(cached)
}

pub(crate) fn is_app_model(id: &str) -> bool {
    id.starts_with(APP_MODEL_PREFIX) && id.len() > APP_MODEL_PREFIX.len() && identifier(id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    #[serde(default = "catalog_version")]
    pub version: u32,
    #[serde(default = "default_models")]
    pub models: Vec<Model>,
    #[serde(default = "default_effort_map", deserialize_with = "read_effort_map")]
    pub effort_map: BTreeMap<String, String>,
}

fn catalog_version() -> u32 {
    1
}
fn default_models() -> Vec<Model> {
    let seed: toml::Table = toml::from_str(include_str!("../default-models.toml"))
        .expect("bundled model catalog is tested");
    seed["models"]
        .clone()
        .try_into()
        .expect("bundled models are tested")
}

fn default_effort_map() -> BTreeMap<String, String> {
    toml::from_str(include_str!("../default-efforts.toml"))
        .expect("bundled effort mapping is tested")
}
fn read_effort_map<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    let additions = BTreeMap::<String, String>::deserialize(deserializer)?;
    let mut effective = default_effort_map();
    effective.extend(additions);
    Ok(effective)
}

pub(crate) fn native_effort(value: &str) -> bool {
    matches!(
        value,
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub id: String,
    pub family: String,
    pub display_name: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub pro: bool,
    pub routes: Vec<Route>,
    #[serde(default)]
    pub context: ContextOverride,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_token_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub effort: String,
    pub host_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_effort: Option<String>,
}

impl Default for Catalog {
    fn default() -> Self {
        toml::from_str(include_str!("../default-models.toml"))
            .expect("bundled default catalog is tested")
    }
}

pub(crate) fn merge_models(models: &mut Vec<Model>, configured: &[Model]) {
    for addition in configured {
        if let Some(model) = models.iter_mut().find(|model| model.id == addition.id) {
            for route in &addition.routes {
                if let Some(old) = model.routes.iter_mut().find(|r| r.effort == route.effort) {
                    *old = route.clone();
                } else {
                    model.routes.push(route.clone());
                }
            }
            model.display_name = addition.display_name.clone();
            model.family = addition.family.clone();
            model.hidden = addition.hidden;
            model.pro = addition.pro;
            model.context = addition.context.clone();
        } else {
            models.push(addition.clone());
        }
    }
}

fn label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:/".contains(&b))
}

impl Catalog {
    pub(crate) fn with_app_options(
        &self,
        groups: &serde_json::Value,
        configured: &[Model],
    ) -> Result<Self, crate::protocol::Error> {
        use crate::protocol::Error;
        let groups = groups
            .as_array()
            .filter(|g| !g.is_empty() && g.len() <= 128)
            .ok_or(Error("app_model_catalog_invalid"))?;
        let mut models: Vec<Model> = Vec::new();
        for group in groups {
            let family = group["label"]
                .as_str()
                .filter(|v| label(v))
                .ok_or(Error("app_model_catalog_invalid"))?;
            let options = group["options"]
                .as_array()
                .ok_or(Error("app_model_catalog_invalid"))?;
            for option in options {
                if option["isAvailable"] == false {
                    continue;
                }
                if option["isAvailable"] != true {
                    return Err(Error("app_model_catalog_invalid"));
                }
                let slug = option["slug"]
                    .as_str()
                    .filter(|v| identifier(v))
                    .ok_or(Error("app_model_catalog_invalid"))?;
                let pro = option["lane"] == "pro";
                let id = format!("{APP_MODEL_PREFIX}{slug}");
                let thinking = match &option["thinkingEffort"] {
                    serde_json::Value::Null => None,
                    serde_json::Value::String(v) if identifier(v) => Some(v.clone()),
                    _ => return Err(Error("app_model_catalog_invalid")),
                };
                let configured_routes: Vec<_> = configured
                    .iter()
                    .filter(|m| m.id == id)
                    .flat_map(|m| &m.routes)
                    .filter(|r| r.host_model == slug && r.thinking_effort == thinking)
                    .collect();
                let effort = if let Some(first) = configured_routes.first() {
                    if configured_routes.iter().any(|r| r.effort != first.effort) {
                        return Err(Error("app_model_route_ambiguous"));
                    }
                    first.effort.clone()
                } else {
                    match &thinking {
                        None => "none".to_owned(),
                        Some(value) => self
                            .effort_map
                            .get(value)
                            .cloned()
                            .ok_or(Error("app_effort_mapping_required"))?,
                    }
                };
                let route = Route {
                    effort,
                    host_model: slug.to_owned(),
                    thinking_effort: thinking,
                };
                if let Some(model) = models.iter_mut().find(|model| model.id == id) {
                    if model.pro != pro {
                        return Err(Error("app_model_catalog_invalid"));
                    }
                    if let Some(existing) = model.routes.iter().find(|r| r.effort == route.effort) {
                        if existing != &route {
                            return Err(Error("app_model_route_ambiguous"));
                        }
                    } else {
                        model.routes.push(route);
                    }
                } else {
                    let title = option["modelLabel"]
                        .as_str()
                        .filter(|v| label(v))
                        .ok_or(Error("app_model_catalog_invalid"))?;
                    models.push(Model {
                        id,
                        family: family.to_owned(),
                        display_name: format!("ChatGPT · {family} · {title}"),
                        hidden: false,
                        pro,
                        routes: vec![route],
                        context: ContextOverride::default(),
                    });
                }
            }
        }
        // Existing family aliases retain their execution meaning for saved
        // bindings; newly offered models have their own host-slug identity.
        for model in self
            .models
            .iter()
            .filter(|m| !m.id.starts_with(APP_MODEL_PREFIX))
        {
            if !models.iter().any(|offered| offered.id == model.id) {
                let mut retained = model.clone();
                retained.hidden = true;
                models.push(retained);
            }
        }
        merge_models(&mut models, configured);
        let result = Self {
            version: catalog_version(),
            models,
            effort_map: self.effort_map.clone(),
        };
        result
            .validate()
            .map_err(|_| Error("app_model_catalog_invalid"))?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != catalog_version() || self.models.is_empty() || self.models.len() > 128 {
            return Err(ConfigError::Invalid("catalog version or size is invalid"));
        }
        if self.effort_map.len() > 128
            || self
                .effort_map
                .iter()
                .any(|(host, native)| !identifier(host) || !native_effort(native))
        {
            return Err(ConfigError::Invalid(
                "app effort mapping is invalid or unsupported by Native",
            ));
        }
        let mut ids = BTreeSet::new();
        for model in &self.models {
            if !identifier(&model.id)
                || !model.id.starts_with("chatgpt-desktop/")
                || model.id.len() <= "chatgpt-desktop/".len()
                || !ids.insert(&model.id)
                || !label(&model.family)
                || !label(&model.display_name)
                || model.routes.is_empty()
                || model.routes.len() > 16
            {
                return Err(ConfigError::Invalid(
                    "catalog model identity or routes are invalid",
                ));
            }
            let mut efforts = BTreeSet::new();
            for route in &model.routes {
                if !identifier(&route.effort)
                    || !efforts.insert(&route.effort)
                    || !identifier(&route.host_model)
                    || route
                        .thinking_effort
                        .as_deref()
                        .is_some_and(|v| !identifier(v))
                {
                    return Err(ConfigError::Invalid(
                        "catalog route is invalid or duplicated",
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn find(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.id == id)
    }
    pub fn route(&self, id: &str, effort: &str) -> Option<&Route> {
        self.find(id)?
            .routes
            .iter()
            .find(|route| route.effort == effort)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_preserve_current_gpt6_routes_and_legacy_alias_without_56() {
        let catalog = Catalog::default();
        catalog.validate().unwrap();
        assert_eq!(catalog.models.len(), 3);
        assert!(catalog.models.iter().all(|m| !m.id.contains("5.6")));
        assert!(catalog.find("chatgpt-desktop/instant").unwrap().hidden);
        assert!(catalog.find("chatgpt-desktop/gpt-6-pro").unwrap().pro);
        assert_eq!(
            catalog
                .route("chatgpt-desktop/gpt-6", "xhigh")
                .unwrap()
                .thinking_effort
                .as_deref(),
            Some("max")
        );
    }
    #[test]
    fn common_effort_mapping_accepts_partial_config_and_validates_native_values() {
        let catalog: Catalog =
            toml::from_str("[effort_map]\ncustom_host_effort = 'ultra'\nextended = 'xhigh'\n")
                .unwrap();
        catalog.validate().unwrap();
        assert_eq!(catalog.effort_map["standard"], "medium");
        assert_eq!(catalog.effort_map["max"], "xhigh");
        assert_eq!(catalog.effort_map["extended"], "xhigh");
        assert_eq!(catalog.effort_map["custom_host_effort"], "ultra");
        let invalid: Catalog =
            toml::from_str("[effort_map]\ncustom_host_effort = 'invented-native-value'\n").unwrap();
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn app_choices_and_configured_additions_resolve_without_model_name_branches() {
        use serde_json::json;
        let catalog = Catalog::default();
        let mut options = json!([{"label":"Unseen family","defaultModelSlug":"unseen-auto","options":[
            {"slug":"unseen-auto","lane":"instant","modelLabel":"Instant","isAvailable":true},
            {"slug":"unseen-thinking","lane":"thinking","modelLabel":"Thinking","thinkingEffort":"extended","isAvailable":true},
            {"slug":"unseen-pro","lane":"pro","modelLabel":"Pro","isAvailable":false}
        ]}]);
        let discovered = catalog.with_app_options(&options, &[]).unwrap();
        assert_eq!(
            discovered
                .route("chatgpt-desktop/app/unseen-thinking", "high")
                .unwrap()
                .host_model,
            "unseen-thinking"
        );
        assert!(discovered.find("chatgpt-desktop/app/unseen-pro").is_none());
        for old in catalog.models.iter().filter(|m| !m.hidden) {
            assert!(discovered.find(&old.id).unwrap().hidden);
        }
        let mut configured = discovered
            .find("chatgpt-desktop/app/unseen-thinking")
            .unwrap()
            .clone();
        configured.routes.push(Route {
            effort: "ultra".into(),
            host_model: "explicit-worker".into(),
            thinking_effort: Some("custom-effort".into()),
        });
        let merged = catalog
            .with_app_options(&options, std::slice::from_ref(&configured))
            .unwrap();
        assert_eq!(
            merged.route(&configured.id, "ultra").unwrap().host_model,
            "explicit-worker"
        );
        assert!(merged.route(&configured.id, "high").is_some());
        options[0]["options"][1]["thinkingEffort"] = json!("custom-effort");
        assert_eq!(
            catalog.with_app_options(&options, &[]),
            Err(crate::protocol::Error("app_effort_mapping_required"))
        );
        configured.routes.push(Route {
            effort: "xhigh".into(),
            host_model: "unseen-thinking".into(),
            thinking_effort: Some("custom-effort".into()),
        });
        let corrected = catalog.with_app_options(&options, &[configured]).unwrap();
        assert_eq!(
            corrected
                .route("chatgpt-desktop/app/unseen-thinking", "xhigh")
                .unwrap()
                .thinking_effort
                .as_deref(),
            Some("custom-effort")
        );
    }
    #[test]
    fn edited_data_can_add_model_or_effort_without_rust_changes() {
        let mut catalog = Catalog::default();
        let mut model = catalog.models[0].clone();
        model.id = "chatgpt-desktop/configured-model".into();
        model.routes[0].host_model = "configured-host-model".into();
        model.routes[0].effort = "low".into();
        catalog.models.push(model);
        catalog.validate().unwrap();
        assert_eq!(
            catalog
                .route("chatgpt-desktop/configured-model", "low")
                .unwrap()
                .host_model,
            "configured-host-model"
        );
        let duplicate = catalog.models[0].routes[0].clone();
        catalog.models[0].routes.push(duplicate);
        assert!(catalog.validate().is_err());
    }
}
