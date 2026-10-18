//! Recheck preserved completed qualification without another inference request.
use codex_chatgpt_provider::{app::Bridge, journal::Journal};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

#[tokio::test]
#[ignore = "requires explicit settled live-proof root, transcript and authorized app descriptor"]
async fn recheck_settled_live_evidence_and_release_without_new_inference() {
    let root = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_SETTLED_PROOF").expect("settled proof required"),
    );
    assert!(root.is_absolute());
    assert_eq!(
        fs::read_to_string(root.join("work/effect-count")).unwrap(),
        "effect-1\neffect-2\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("work/patched.txt")).unwrap(),
        "real custom tool\n"
    );
    let transcript = fs::read_to_string(
        std::env::var_os("CODEX_PROVIDER_LIVE_TRANSCRIPT").expect("native transcript required"),
    )
    .unwrap();
    let events = transcript
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(events.iter().any(|v| v["type"] == "turn.completed"));
    assert!(events
        .iter()
        .any(|v| v["item"]["text"] == "Native executor verified"));
    assert_eq!(
        events
            .iter()
            .filter(|v| v["type"] == "item.completed" && v["item"]["type"] == "command_execution")
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|v| v["type"] == "item.completed" && v["item"]["type"] == "file_change")
            .count(),
        1
    );
    let journal = Journal::open(&root.join("journal")).unwrap();
    let remote = codex_chatgpt_provider::app::Remote::new(
        std::env::var_os("CODEX_PROVIDER_LIVE_DESCRIPTOR")
            .expect("authorized app descriptor required")
            .into(),
        journal.owner.clone(),
    );
    let mut count = 0;
    for entry in fs::read_dir(&journal.root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|v| v == "json") {
            let key = path.file_stem().unwrap().to_str().unwrap();
            if let Some(lane) = journal
                .read::<Value>(key)
                .unwrap()
                .filter(|v| v.get("native_thread").is_some())
            {
                assert_eq!(lane["phase"], "idle");
                assert_eq!(lane["dispatches"], 4);
                assert_eq!(lane["user_messages"], 2);
                assert_eq!(lane["tool_messages"], 3);
                assert!(lane["pending"].as_array().unwrap().is_empty());
                assert_eq!(lane["confirmed"].as_object().unwrap().len(), 3);
                let identity = remote
                    .operation(json!({"op":"identity","lane":lane["marker"]}))
                    .await
                    .unwrap();
                assert!(
                    identity["account"] == lane["identity"]["account"]
                        && identity["user"] == lane["identity"]["user"]
                        && identity["generation"] == lane["identity"]["generation"],
                    "settled proof identity must still match"
                );
                remote
                    .operation(json!({"op":"release","lane":lane["marker"]}))
                    .await
                    .unwrap();
                count += 1;
            }
        }
    }
    assert_eq!(count, 1);
    remote.operation(json!({"op":"dispose"})).await.unwrap();
    println!(
        "{}",
        json!({"accepted":true,"liveNoMcp":true,"nativeEffects":3,"appDispatches":4,"appUserObjects":2,"appToolObjects":3,"additionalChatSubmissions":0,"publicNativeTranscript":true,"ownedBridgeDisposed":true,"evidenceRetained":true})
    );
}
