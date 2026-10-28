//! Upstream model metadata projection. Families/routes/context remain config data;
//! this module owns the current upstream schema and protocol capabilities only.
use crate::{
    config::{Config, ConfigError},
    models::ContextOverride,
};
use serde_json::{json, Value};

pub fn catalog(config: &Config) -> Result<Value, ConfigError> {
    config.validate()?;
    if config
        .catalog
        .route(&config.model.default, &config.model.reasoning_effort)
        .is_none()
    {
        return Err(ConfigError::Invalid(
            "selected app model requires discovery before Native preparation",
        ));
    }
    let mut models = Vec::new();
    for model in &config.catalog.models {
        let context = config.effective_context(&model.id, &ContextOverride::default())?;
        if model
            .routes
            .iter()
            .any(|r| !crate::models::native_effort(&r.effort))
        {
            return Err(ConfigError::Invalid(
                "configured effort is not supported by this upstream catalog schema",
            ));
        }
        let default = if model
            .routes
            .iter()
            .any(|r| r.effort == config.model.reasoning_effort)
        {
            config.model.reasoning_effort.as_str()
        } else {
            model.routes[0].effort.as_str()
        };
        let mut native: Value = serde_json::from_str(include_str!("../native-model.json"))
            .expect("bundled upstream schema is tested");
        let fields = json!({
            "slug":model.id,"display_name":model.display_name,"description":format!("ChatGPT app through the independent Termux Provider{}",if model.pro {"; Pro requires explicit use and is excluded from automated qualification"}else{""}),
            "priority":2,"visibility":if model.hidden{"hide"}else{"list"},"supported_in_api":true,
            "context_window":context.window_tokens,"max_context_window":context.window_tokens,
            "auto_compact_token_limit":context.auto_compact_token_limit,"comp_hash":null,
            "default_reasoning_level":default,"supported_reasoning_levels":model.routes.iter().map(|r|json!({"effort":r.effort,"description":r.effort})).collect::<Vec<_>>(),
            "supports_reasoning_effort_updates":model.routes.len()>1,"available_in_plans":[]
        });
        native
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        models.push(native);
    }
    Ok(json!({"models":models}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_app_default_requires_discovery_before_native_catalog_publication() {
        let mut config = Config::parse(
            b"[model]\ndefault='chatgpt-desktop/app/unseen-thinking'\nreasoning_effort='high'\n",
        )
        .unwrap();
        assert!(catalog(&config).is_err());
        let groups = json!([{"label":"Unseen family","options":[{"slug":"unseen-thinking","lane":"thinking","modelLabel":"Thinking","isAvailable":true,"thinkingEffort":"extended"}]}]);
        config.catalog = config.catalog.with_app_options(&groups, &[]).unwrap();
        let output = catalog(&config).unwrap();
        let model = output["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["slug"] == config.model.default)
            .unwrap();
        assert_eq!(model["supported_reasoning_levels"][0]["effort"], "high");
        assert_eq!(model["default_reasoning_level"], "high");
    }
    #[test]
    fn model_context_precedence_and_data_catalog_are_projected_without_family_code() {
        let mut config = Config::default();
        config.context.window_tokens = 800000;
        config.context.auto_compact_token_limit = Some(700000);
        config.catalog.models[0].context.window_tokens = Some(1200000);
        config.catalog.models[0].context.auto_compact_token_limit = Some(1000000);
        let projected = catalog(&config).unwrap();
        let id = &config.catalog.models[0].id;
        assert_eq!(projected["models"][0]["context_window"], 1200000);
        assert!(projected["models"][0]["comp_hash"].is_null());
        let value = config
            .effective_context(
                id,
                &ContextOverride {
                    window_tokens: Some(1500000),
                    auto_compact_token_limit: Some(1300000),
                },
            )
            .unwrap();
        assert_eq!(value.window_tokens, 1500000);
        assert_eq!(value.auto_compact_token_limit, Some(1300000));
        config.catalog.models[0].context.window_tokens = Some(500000);
        assert!(config.validate().is_err());
    }
    #[test]
    fn configured_models_and_efforts_are_native_data_with_no_56_or_false_private_reasoning_claim() {
        let mut config = Config::default();
        config.catalog.models[0].id = "chatgpt-desktop/configured".into();
        config.model.default = config.catalog.models[0].id.clone();
        let output = catalog(&config).unwrap();
        assert_eq!(output["models"][0]["slug"], config.model.default);
        assert_eq!(output["models"][0]["context_window"], 1000000);
        assert_eq!(output["models"][0]["supports_reasoning_summaries"], false);
        assert!(!output.to_string().contains("5.6"));
        config.catalog.models[0].routes[0].effort = "future-schema-effort".into();
        config.model.reasoning_effort = "future-schema-effort".into();
        assert!(catalog(&config).is_err());
    }
}
