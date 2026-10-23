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
}
#[derive(Default)]
struct App(Mutex<State>);
impl Bridge for App {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let mut state = self.0.lock().unwrap();
        state.commands.push(command.clone());
        let result = match command["op"].as_str().unwrap() {
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
