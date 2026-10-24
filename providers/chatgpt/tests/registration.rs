use codex_chatgpt_provider::{
    config::Config,
    journal::random_id,
    protocol::Error,
    registration::{native_routing, prepare, MARKER, PROVIDER},
};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
};
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("reg-{}", random_id().unwrap()));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        for name in ["state", "registry", "home"] {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(path.join(name))
                .unwrap();
        }
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn one_generated_profile_links_homes_and_retains_immutable_catalogs_and_native_config() {
    let root = Root::new();
    let state = root.0.join("state");
    let registry = root.0.join("registry");
    let home = root.0.join("home");
    let binary = std::env::current_exe().unwrap();
    let mut config = Config::default();
    config.model.default = config.catalog.models[0].id.clone();
    config.model.reasoning_effort = "xhigh".into();
    fs::write(home.join("config.toml"), "# user native settings\n").unwrap();
    let first = prepare(
        &config,
        &state,
        &registry,
        &home,
        &binary,
        "http://127.0.0.1:12345/v1",
    )
    .unwrap();
    let old = PathBuf::from(first["catalog"].as_str().unwrap());
    let bytes = fs::read(&old).unwrap();
    let profile = registry.join("chatgpt.config.toml");
    let text = fs::read_to_string(&profile).unwrap();
    assert!(text.starts_with(MARKER));
    let routing: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(routing["model_provider"].as_str(), Some(PROVIDER));
    assert_eq!(
        routing["model"].as_str(),
        Some(config.model.default.as_str())
    );
    assert_eq!(routing["model_reasoning_effort"].as_str(), Some("xhigh"));
    let restored = native_routing(&config, &state, &binary, "http://127.0.0.1:12345/v1").unwrap();
    for key in ["model", "model_provider", "model_reasoning_effort"] {
        assert!(
            restored.get(key).is_none(),
            "restoration must retain saved {key}"
        );
    }
    assert_eq!(
        restored["model_catalog_json"].as_str(),
        Some(old.to_str().unwrap())
    );
    assert_eq!(
        routing["model_providers"][PROVIDER]["auth"]["args"][0].as_str(),
        Some("token")
    );
    assert_eq!(
        fs::read_link(home.join("chatgpt.config.toml")).unwrap(),
        profile
    );
    assert_eq!(
        fs::metadata(&profile).unwrap().permissions().mode() & 0o777,
        0o600
    );
    config.catalog.models[0].context.window_tokens = Some(1500000);
    let second = prepare(
        &config,
        &state,
        &registry,
        &home,
        &binary,
        "http://127.0.0.1:12345/v1",
    )
    .unwrap();
    assert_ne!(first["catalog"], second["catalog"]);
    assert_eq!(fs::read(&old).unwrap(), bytes);
    assert_eq!(
        fs::read_to_string(home.join("config.toml")).unwrap(),
        "# user native settings\n"
    );
}
#[test]
fn user_profile_and_foreign_shared_output_are_preserved_before_projection() {
    let root = Root::new();
    let state = root.0.join("state");
    let registry = root.0.join("registry");
    let home = root.0.join("home");
    let binary = std::env::current_exe().unwrap();
    let config = Config::default();
    let local = home.join("chatgpt.config.toml");
    fs::write(&local, "# user profile\n").unwrap();
    assert_eq!(
        prepare(
            &config,
            &state,
            &registry,
            &home,
            &binary,
            "http://127.0.0.1:1/v1"
        )
        .unwrap_err(),
        Error("existing_user_profile_preserved")
    );
    assert!(!registry.join("chatgpt.config.toml").exists());
    fs::remove_file(&local).unwrap();
    let shared = registry.join("chatgpt.config.toml");
    fs::write(&shared, "# foreign generated output\n").unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        prepare(
            &config,
            &state,
            &registry,
            &home,
            &binary,
            "http://127.0.0.1:1/v1"
        )
        .unwrap_err(),
        Error("existing_user_profile_preserved")
    );
    assert_eq!(
        fs::read_to_string(shared).unwrap(),
        "# foreign generated output\n"
    );
    assert!(!state.join("catalogs").exists());
}

#[test]
fn managed_routing_preserves_user_profiles_and_only_publishes_owned_catalogs() {
    let root = Root::new();
    let home = root.0.join("home");
    let state = root.0.join("state");
    let shared = root.0.join("registry/chatgpt.config.toml");
    let local = home.join("chatgpt.config.toml");
    fs::write(&shared, "# existing shared user profile\n").unwrap();
    fs::write(&local, "# existing local user profile\n").unwrap();
    fs::write(home.join("config.toml"), "# native user defaults\n").unwrap();
    let mut config = Config::default();
    config.execution.home = Some(home.clone());
    config.execution.workspace = Some(root.0.clone());
    let binary = std::env::current_exe().unwrap();
    let pool = codex_chatgpt_provider::operator_native::Pool::new(
        config,
        state.clone(),
        "http://127.0.0.1:12345/v1".to_owned(),
        binary.clone(),
    )
    .unwrap();
    let first = pool.routing().unwrap();
    assert_eq!(first, pool.routing().unwrap());
    let routing = &first["model_providers"][PROVIDER];
    assert_eq!(routing["base_url"], "http://127.0.0.1:12345/v1");
    assert_eq!(routing["requires_openai_auth"], false);
    assert_eq!(routing["auth"]["command"], binary.to_str().unwrap());
    assert_eq!(routing["auth"]["args"][2], state.to_str().unwrap());
    let catalog = PathBuf::from(first["model_catalog_json"].as_str().unwrap());
    assert_eq!(catalog.parent().unwrap(), state.join("catalogs"));
    assert_eq!(fs::read_dir(state.join("catalogs")).unwrap().count(), 1);
    assert_eq!(
        fs::read_to_string(&shared).unwrap(),
        "# existing shared user profile\n"
    );
    assert_eq!(
        fs::read_to_string(&local).unwrap(),
        "# existing local user profile\n"
    );
    assert_eq!(
        fs::read_to_string(home.join("config.toml")).unwrap(),
        "# native user defaults\n"
    );
}
