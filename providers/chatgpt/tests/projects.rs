use codex_chatgpt_provider::{
    app::Bridge,
    config::{HistoryConfig, HistoryMode},
    journal::{random_id, Journal},
    project::resolve,
    protocol::Error,
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Mutex};
#[derive(Default)]
struct State {
    projects: Vec<Value>,
    creates: usize,
    lose_reply: bool,
    rejected: bool,
    commands: Vec<Value>,
    identity_reads: usize,
    concurrent_config: Option<PathBuf>,
}
#[derive(Default)]
struct App(Mutex<State>);
impl Bridge for App {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let mut state = self.0.lock().unwrap();
        state.commands.push(command.clone());
        let result = match command["op"].as_str().unwrap() {
            "identity" => {
                state.identity_reads += 1;
                if state.identity_reads == 2 {
                    if let Some(path) = &state.concurrent_config {
                        std::fs::write(
                            path,
                            b"[history]\nmode='retain'\n[context]\nwindow_tokens=900000\n",
                        )
                        .unwrap();
                    }
                }
                Ok(json!({"account":"account","user":"user","structure":"personal"}))
            }
            "project_list" => Ok(json!(state.projects)),
            "project_read" => state
                .projects
                .iter()
                .find(|v| v["id"] == command["project"])
                .cloned()
                .ok_or(Error("project_unavailable")),
            "project_create" => {
                state.creates += 1;
                if state.rejected {
                    return Box::pin(async { Err(Error("project_creation_rejected")) });
                }
                let project = json!({"id":"g-p-created","name":command["name"],"compatible":true});
                state.projects.push(project.clone());
                if state.lose_reply {
                    Err(Error("reply_lost"))
                } else {
                    Ok(project)
                }
            }
            _ => Err(Error("unexpected_operation")),
        };
        Box::pin(async move { result })
    }
}
#[tokio::test]
async fn explicit_server_rejection_is_not_an_uncertain_delivery_or_permanent_latch() {
    let (_root, journal, settings, identity) = setup();
    let app = App::default();
    app.0.lock().unwrap().rejected = true;
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_creation_rejected")
    );
    app.0.lock().unwrap().rejected = false;
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    assert_eq!(app.0.lock().unwrap().creates, 2);
}
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn setup() -> (Root, Journal, HistoryConfig, Value) {
    let root = Root(std::env::temp_dir().join(format!("project-{}", random_id().unwrap())));
    let journal = Journal::open(&root.0).unwrap();
    (
        root,
        journal,
        HistoryConfig {
            mode: HistoryMode::Project,
            project_name: Some("Codex".into()),
            project_id: None,
            create_if_missing: true,
            ..HistoryConfig::default()
        },
        json!({"account":"account","user":"user","structure":"personal"}),
    )
}
#[tokio::test]
async fn create_once_read_back_reuse_and_preserve_exact_deleted_binding() {
    let (_root, journal, settings, identity) = setup();
    let app = App::default();
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    assert_eq!(app.0.lock().unwrap().creates, 1);
    app.0.lock().unwrap().projects.clear();
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_unavailable")
    );
    assert_eq!(app.0.lock().unwrap().creates, 1);
}
#[tokio::test]
async fn lost_creation_reply_reconciles_without_duplicate_and_missing_readback_stays_uncertain() {
    let (_root, journal, settings, identity) = setup();
    let app = App::default();
    app.0.lock().unwrap().lose_reply = true;
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_creation_unconfirmed")
    );
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    assert_eq!(app.0.lock().unwrap().creates, 1);
    let another = json!({"account":"other","user":"user","structure":"workspace"});
    app.0.lock().unwrap().projects.clear();
    assert_eq!(
        resolve(&app, &journal, &settings, &another)
            .await
            .unwrap_err(),
        Error("project_creation_unconfirmed")
    );
    app.0.lock().unwrap().projects.clear();
    assert_eq!(
        resolve(&app, &journal, &settings, &another)
            .await
            .unwrap_err(),
        Error("project_creation_unconfirmed")
    );
    assert_eq!(app.0.lock().unwrap().creates, 2);
}
#[tokio::test]
async fn ambiguous_names_explicit_id_incompatible_and_no_create_are_distinct() {
    let (_root, journal, mut settings, identity) = setup();
    let app = App::default();
    settings.create_if_missing = false;
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_destination_missing")
    );
    app.0.lock().unwrap().projects = vec![
        json!({"id":"g-p-one","name":"Codex","compatible":true}),
        json!({"id":"g-p-two","name":"Codex","compatible":true}),
    ];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_destination_ambiguous")
    );
    settings.project_id = Some("g-p-two".into());
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-two")
    );
    app.0.lock().unwrap().projects[1]["compatible"] = json!(false);
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_destination_incompatible")
    );
    assert_eq!(app.0.lock().unwrap().creates, 0);
}

#[tokio::test]
async fn explicit_project_use_creates_reads_back_and_selects_without_inference() {
    use codex_chatgpt_provider::{project, store};
    let (root, journal, _, _) = setup();
    let path = root.0.join("config/chatgpt.toml");
    store::update(&path, &[("history.mode".into(), "retain".into())], &[]).unwrap();
    let app = App::default();
    let listed = project::control(&app, &journal, &path, &json!({"action":"list"}))
        .await
        .unwrap();
    assert_eq!(listed["projects"], json!([]));
    assert_eq!(
        store::Snapshot::open(&path)
            .unwrap()
            .config()
            .unwrap()
            .history
            .mode,
        HistoryMode::Retain
    );
    let args = json!({"action":"use","name":"Codex"});
    let selected = project::control(&app, &journal, &path, &args)
        .await
        .unwrap();
    assert_eq!(selected["configuration_saved"], true);
    assert_eq!(selected["project"]["id"], "g-p-created");
    assert_eq!(selected["inference_submitted"], false);
    let saved = store::Snapshot::open(&path).unwrap().config().unwrap();
    assert_eq!(saved.history.mode, HistoryMode::Project);
    assert_eq!(saved.history.project_name.as_deref(), Some("Codex"));
    assert_eq!(saved.history.project_id.as_deref(), Some("g-p-created"));
    let repeated = project::control(&app, &journal, &path, &args)
        .await
        .unwrap();
    assert_eq!(repeated["configuration_changed"], false);
    let state = app.0.lock().unwrap();
    assert_eq!(state.creates, 1);
    assert!(state.commands.iter().all(|c| matches!(
        c["op"].as_str(),
        Some("identity" | "project_list" | "project_read" | "project_create")
    )));
}

#[tokio::test]
async fn project_selection_preserves_configuration_on_remote_failure_and_concurrent_edit() {
    use codex_chatgpt_provider::{project, store};
    for conflict in [false, true] {
        let (root, journal, _, _) = setup();
        let path = root.0.join("config/chatgpt.toml");
        store::update(&path, &[("history.mode".into(), "retain".into())], &[]).unwrap();
        let before = std::fs::read(&path).unwrap();
        let app = App::default();
        {
            let mut state = app.0.lock().unwrap();
            state.rejected = !conflict;
            state.concurrent_config = conflict.then_some(path.clone());
        }
        let args = json!({"action":"use","name":"Codex"});
        let error = project::control(&app, &journal, &path, &args)
            .await
            .unwrap_err();
        assert_eq!(
            error,
            Error(if conflict {
                "project_configuration_save_unconfirmed"
            } else {
                "project_creation_rejected"
            })
        );
        let saved = store::Snapshot::open(&path).unwrap().config().unwrap();
        assert_eq!(saved.history.mode, HistoryMode::Retain);
        if conflict {
            assert_eq!(saved.context.window_tokens, 900000);
            app.0.lock().unwrap().concurrent_config = None;
            assert!(
                project::control(&app, &journal, &path, &args)
                    .await
                    .unwrap()["configuration_saved"]
                    == true
            );
            assert_eq!(
                app.0.lock().unwrap().creates,
                1,
                "configuration retry reuses the verified remote project"
            );
        } else {
            assert_eq!(std::fs::read(path).unwrap(), before);
        }
    }
}
