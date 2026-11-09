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
    list_fails: bool,
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
            "project_list" => {
                if state.list_fails {
                    Err(Error("project_list_unavailable"))
                } else {
                    Ok(json!(state.projects))
                }
            }
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
async fn global_project_id_is_never_queried_in_a_different_account() {
    let (_root, journal, mut settings, identity_a) = setup();
    let app = App::default();
    let original = "g-p-account-a";
    settings.project_id = Some(original.into());
    app.0.lock().unwrap().projects = vec![json!({"id":original,"name":"Codex","compatible":true})];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity_a)
            .await
            .unwrap()
            .as_deref(),
        Some(original)
    );

    // The configured ID is global, while the project listing and journal
    // binding belong to the authenticated account.
    let identity_b = json!({"account":"account-b","user":"user-b","structure":"workspace"});
    app.0.lock().unwrap().projects =
        vec![json!({"id":"g-p-unrelated","name":"Another","compatible":true})];
    let before = app.0.lock().unwrap().commands.len();
    assert_eq!(
        resolve(&app, &journal, &settings, &identity_b)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    {
        let state = app.0.lock().unwrap();
        assert_eq!(state.creates, 1);
        assert!(state.commands[before..]
            .iter()
            .all(|request| { request["op"] != "project_read" || request["project"] != original }));
    }
    // Reopening account A keeps its own exact project, even after account B
    // has a separate project with the same name.
    app.0.lock().unwrap().projects = vec![json!({"id":original,"name":"Codex","compatible":true})];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity_a)
            .await
            .unwrap()
            .as_deref(),
        Some(original)
    );
    app.0.lock().unwrap().projects =
        vec![json!({"id":"g-p-created","name":"Codex","compatible":true})];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity_b)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    assert_eq!(app.0.lock().unwrap().creates, 1);
    assert_eq!(settings.project_id.as_deref(), Some(original));

    // A later explicit project selection changes the shared configuration to
    // account B. Neither account's previously verified destination is lost.
    settings.project_id = Some("g-p-created".into());
    app.0.lock().unwrap().projects = vec![json!({"id":original,"name":"Codex","compatible":true})];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity_a)
            .await
            .unwrap()
            .as_deref(),
        Some(original)
    );
    app.0.lock().unwrap().projects =
        vec![json!({"id":"g-p-created","name":"Codex","compatible":true})];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity_b)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-created")
    );
    assert_eq!(app.0.lock().unwrap().creates, 1);
}

#[tokio::test]
async fn saved_legacy_account_binding_is_migrated_without_reselecting_other_account() {
    use codex_chatgpt_provider::context::digest;
    let (_root, journal, mut settings, identity) = setup();
    settings.project_id = Some("g-p-legacy".into());
    let key = digest(&json!({
        "kind":"project-destination-v1",
        "identity":identity,
        "name":"Codex",
        "configured_id":settings.project_id,
    }));
    journal
        .write(
            &key,
            &json!({"name":"Codex","id":"g-p-legacy","creating":false}),
        )
        .unwrap();
    let app = App::default();
    app.0.lock().unwrap().projects =
        vec![json!({"id":"g-p-legacy","name":"Codex","compatible":true})];
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-legacy")
    );
    settings.project_id = Some("g-p-other-account".into());
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-legacy")
    );
    assert_eq!(app.0.lock().unwrap().creates, 0);
}

#[tokio::test]
async fn cross_account_project_reuses_owned_name_and_fails_closed_on_list_error() {
    let (_root, journal, mut settings, identity) = setup();
    let app = App::default();
    settings.project_id = Some("g-p-another-account".into());
    settings.create_if_missing = false;
    app.0.lock().unwrap().list_fails = true;
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap_err(),
        Error("project_list_unavailable")
    );
    assert_eq!(app.0.lock().unwrap().creates, 0);
    {
        let mut state = app.0.lock().unwrap();
        state.list_fails = false;
        state.projects = vec![json!({"id":"g-p-current-account","name":"Codex","compatible":true})];
    }
    assert_eq!(
        resolve(&app, &journal, &settings, &identity)
            .await
            .unwrap()
            .as_deref(),
        Some("g-p-current-account")
    );
    assert_eq!(app.0.lock().unwrap().creates, 0);
    assert!(app.0.lock().unwrap().commands.iter().all(|request| {
        request["op"] != "project_read" || request["project"] != "g-p-another-account"
    }));
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

#[tokio::test]
async fn a_pending_project_read_blocks_only_its_existing_journal_destination_binding() {
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    struct HeldRead {
        app: Arc<App>,
        held: Arc<AtomicBool>,
        reads: Arc<AtomicUsize>,
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }
    impl Bridge for HeldRead {
        fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
            let app = self.app.clone();
            let held = self.held.clone();
            let reads = self.reads.clone();
            let entered = self.entered.clone();
            let release = self.release.clone();
            Box::pin(async move {
                if command["op"] == "project_read" && command["project"] == "g-p-held" {
                    reads.fetch_add(1, Ordering::SeqCst);
                    if held.load(Ordering::SeqCst) {
                        entered.notify_one();
                        release.notified().await;
                    }
                }
                app.operation(command).await
            })
        }
    }
    let (_root, journal, settings, identity) = setup();
    let journal = Arc::new(journal);
    let app = Arc::new(App::default());
    app.0.lock().unwrap().projects = vec![
        json!({"id":"g-p-held","name":"Codex","compatible":true}),
        json!({"id":"g-p-other","name":"Independent","compatible":true}),
    ];
    resolve(app.as_ref(), &journal, &settings, &identity)
        .await
        .unwrap();
    let bridge = Arc::new(HeldRead {
        app: app.clone(),
        held: Arc::new(AtomicBool::new(true)),
        reads: Arc::new(AtomicUsize::new(0)),
        entered: Arc::new(tokio::sync::Notify::new()),
        release: Arc::new(tokio::sync::Notify::new()),
    });
    let first = {
        let bridge = bridge.clone();
        let journal = journal.clone();
        let settings = settings.clone();
        let identity = identity.clone();
        tokio::spawn(async move { resolve(bridge.as_ref(), &journal, &settings, &identity).await })
    };
    bridge.entered.notified().await;
    let same = {
        let bridge = bridge.clone();
        let journal = journal.clone();
        let settings = settings.clone();
        let identity = identity.clone();
        tokio::spawn(async move { resolve(bridge.as_ref(), &journal, &settings, &identity).await })
    };
    let mut independent = settings.clone();
    independent.project_name = Some("Independent".into());
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            resolve(bridge.as_ref(), &journal, &independent, &identity)
        )
        .await
        .expect("an unrelated destination must not acquire the held binding lock")
        .unwrap()
        .as_deref(),
        Some("g-p-other")
    );
    assert!(!same.is_finished());
    assert_eq!(bridge.reads.load(Ordering::SeqCst), 1);
    bridge.held.store(false, Ordering::SeqCst);
    bridge.release.notify_one();
    assert_eq!(first.await.unwrap().unwrap().as_deref(), Some("g-p-held"));
    assert_eq!(same.await.unwrap().unwrap().as_deref(), Some("g-p-held"));
    assert_eq!(app.0.lock().unwrap().creates, 0);
}

#[tokio::test]
async fn concurrent_same_destination_reconciles_one_uncertain_creation() {
    use std::sync::Arc;
    struct HeldCreate {
        app: Arc<App>,
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }
    impl Bridge for HeldCreate {
        fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
            let creating = command["op"] == "project_create";
            let result = self.app.operation(command);
            let entered = self.entered.clone();
            let release = self.release.clone();
            Box::pin(async move {
                if creating {
                    entered.notify_one();
                    release.notified().await;
                }
                result.await
            })
        }
    }
    let (_root, journal, settings, identity) = setup();
    let journal = Arc::new(journal);
    let app = Arc::new(App::default());
    app.0.lock().unwrap().lose_reply = true;
    let bridge = Arc::new(HeldCreate {
        app: app.clone(),
        entered: Arc::new(tokio::sync::Notify::new()),
        release: Arc::new(tokio::sync::Notify::new()),
    });
    let first = {
        let bridge = bridge.clone();
        let journal = journal.clone();
        let settings = settings.clone();
        let identity = identity.clone();
        tokio::spawn(async move { resolve(bridge.as_ref(), &journal, &settings, &identity).await })
    };
    bridge.entered.notified().await;
    let second = {
        let bridge = bridge.clone();
        let journal = journal.clone();
        let settings = settings.clone();
        let identity = identity.clone();
        tokio::spawn(async move { resolve(bridge.as_ref(), &journal, &settings, &identity).await })
    };
    tokio::task::yield_now().await;
    assert!(!second.is_finished());
    bridge.release.notify_one();
    assert_eq!(
        first.await.unwrap(),
        Err(Error("project_creation_unconfirmed"))
    );
    assert_eq!(
        second.await.unwrap().unwrap().as_deref(),
        Some("g-p-created")
    );
    assert_eq!(app.0.lock().unwrap().creates, 1);
}
