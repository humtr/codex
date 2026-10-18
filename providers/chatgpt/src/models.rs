//! Model routing is configuration data; it is not proof of host entitlement.
use crate::config::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: u32,
    pub models: Vec<Model>,
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
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 || self.models.is_empty() || self.models.len() > 128 {
            return Err(ConfigError::Invalid("catalog version or size is invalid"));
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
