//! Explicit project metadata mutation/readback; this never submits inference.
use codex_chatgpt_provider::{
    app::{Bridge, Remote},
    config::{HistoryConfig, HistoryMode},
    journal::Journal,
    project,
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires authorized app, retained project state and explicit project-create-reuse arming"]
async fn actual_app_creates_or_reuses_one_project_without_chat() {
    assert_eq!(
        std::env::var("CHATGPT_PROVIDER_PROJECT_PROOF").as_deref(),
        Ok("project-create-reuse")
    );
    let state = std::path::PathBuf::from(
        std::env::var_os("CHATGPT_PROVIDER_PROJECT_STATE")
            .expect("retained private state required"),
    );
    let journal = Journal::open(&state).unwrap();
    let bridge = Remote::new(
        std::env::var_os("CHATGPT_PROVIDER_DESCRIPTOR")
            .expect("authorized app descriptor required")
            .into(),
        journal.owner.clone(),
    );
    let identity = bridge
        .operation(json!({"op":"identity","lane":"no-inference-project-proof"}))
        .await
        .unwrap();
    let history = HistoryConfig {
        mode: HistoryMode::Project,
        project_name: Some("Termux Codex".into()),
        project_id: None,
        create_if_missing: true,
    };
    let first = project::resolve(&bridge, &journal, &history, &identity).await;
    // Disposing our empty bridge is safe even when a project operation failed;
    // retain uncertain creation state and never delete or recreate remote data.
    if first.is_err() {
        bridge.operation(json!({"op":"dispose"})).await.unwrap();
    }
    let first = first.unwrap();
    let second = project::resolve(&bridge, &journal, &history, &identity)
        .await
        .unwrap();
    assert!(first.is_some());
    assert_eq!(first, second);
    bridge.operation(json!({"op":"dispose"})).await.unwrap();
    println!(
        "{}",
        json!({"accepted":true,"exactProjectReused":true,"projectReadback":true,"chatSubmissions":0,"remoteProjectPreserved":true})
    );
}
