//! Exercise the real MCP registration/settlement consumer of an admitted Native scope.
use super::*;
use crate::protocol::{NativeScope, Request};
use std::path::PathBuf;

#[derive(Default)]
struct Remote {
    commands: std::sync::Mutex<Vec<Value>>,
}
impl Bridge for Remote {
    fn admit_submission(&self, command: Value) -> BoxFuture<'static, Result<(), Error>> {
        self.commands.lock().unwrap().push(command);
        Box::pin(async { Ok(()) })
    }
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let response = if command["op"] == "snapshot" {
            json!({"round":"first", "messages":[], "terminals":[], "dispatches":1,
                "roundStarts":1, "pending":true, "streamActive":true})
        } else {
            json!({"started":true})
        };
        self.commands.lock().unwrap().push(command);
        Box::pin(async move { Ok(response) })
    }
}
struct Fixture {
    root: PathBuf,
    journal: Arc<Journal>,
    broker: Arc<Broker>,
    bridge: McpBridge,
    remote: Arc<Remote>,
    request: Request,
    token: String,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
impl Fixture {
    async fn new(mode: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mcp-native-scope-{}", random_id().unwrap()));
        std::fs::create_dir(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        std::fs::write(root.join("a"), "old\n").unwrap();
        std::fs::write(root.join("b"), "old\n").unwrap();
        let journal = Arc::new(Journal::open(&root.join("journal")).unwrap());
        let broker = Broker::new(journal.clone());
        let remote = Arc::new(Remote::default());
        let bridge = McpBridge::new(
            remote.clone(),
            broker.clone(),
            journal.clone(),
            "Owned".into(),
        )
        .unwrap();
        let bridge = if mode == "unbound-native" {
            bridge.with_native_admission()
        } else {
            bridge
        };
        let mut request = Request::parse(
            json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":"owned",
                "client_metadata":{"x-codex-turn-metadata":{"thread_id":"scope-thread","turn_id":"scope-turn"}},
                "tools":[{"type":"namespace","name":"functions","tools":[
                    {"type":"custom","name":"apply_patch","format":{"type":"text"}}
                ]}]}), &crate::config::Config::default(), None,
        ).unwrap();
        let owner = crate::cdp::Process::read(std::process::id()).unwrap();
        let mut facts = crate::native_effect::hook_fixture(&root);
        facts.thread = request.thread.clone();
        facts.turn = request.turn.clone().unwrap();
        if mode == "legacy" {
            facts.schema = 1;
            facts.hook_free_tools.clear();
        }
        if mode == "wrong-owner" {
            facts.start.push('0');
        }
        request.execution = (mode != "missing").then_some(facts);
        if !matches!(mode, "generic" | "unbound-native") {
            request.bind_native_scope(&root, &owner).unwrap();
        }
        bridge
            .operation(
                json!({"op":"open","lane":"scope-lane","signature":{"name":"native_gateway"}}),
            )
            .await
            .unwrap();
        let mut command = json!({"op":"submit","lane":"scope-lane","round":"first",
            "nativeThread":request.thread,"nativeTurn":request.turn,"model":"gpt-6",
            "thinkingEffort":"standard","policyDigest":"policy","hostToolsDigest":"host-tools",
            "instructions":[format!("{CATALOG}{}",serde_json::to_string(&request.tools).unwrap())],
            "messages":[{"role":"user","text":"Owned source-bound request"}],"results":[]});
        if let Some(scope) = &request.native_scope {
            command[NATIVE_SCOPE] = json!(scope);
        }
        bridge.admit_submission(command.clone()).await.unwrap();
        bridge.operation(command).await.unwrap();
        let token = lane_token(&journal, "scope-lane").unwrap().unwrap();
        broker.catalog(&token).await.unwrap();
        Self {
            root,
            journal,
            broker,
            bridge,
            remote,
            request,
            token,
        }
    }
    fn stored(&self) -> Value {
        self.journal
            .read(&digest(&("mcp-turn-v1", &self.token)))
            .unwrap()
            .unwrap()
    }
    async fn offer(&self, args: &crate::mcp::Arguments) -> Value {
        self.broker.enqueue(&self.token, args).await.unwrap();
        self.bridge
            .operation(json!({"op":"snapshot","lane":"scope-lane"}))
            .await
            .unwrap();
        self.stored()["calls"][&args.call_id].clone()
    }
    fn settlement(&self, call: &Value, scope: Option<&NativeScope>) -> Value {
        let mut command = json!({"op":"submit","lane":"scope-lane","round":"second",
            "nativeThread":self.request.thread,"nativeTurn":self.request.turn,"model":"gpt-6",
            "thinkingEffort":"standard","policyDigest":"policy","hostToolsDigest":"host-tools",
            "instructions":[],"messages":[],"results":[{"id":call["id"],"result":{"output":"original result"}}]});
        if let Some(scope) = scope {
            command[NATIVE_SCOPE] = json!(scope);
        }
        command
    }
}
fn patch(id: &str, file: &str) -> crate::mcp::Arguments {
    crate::mcp::Arguments {
        call_id: id.into(),
        name: "functions.apply_patch".into(),
        input: format!("*** Begin Patch\n*** Update File: {file}\n@@\n-old\n+new\n*** End Patch"),
        report: "Owned bounded fixture patch".into(),
    }
}

#[tokio::test]
async fn native_scope_is_bound_before_tool_discovery_and_never_reaches_the_app() {
    let fixture = Fixture::new("qualified").await;
    let original = fixture.offer(&patch("a", "a")).await;
    assert_eq!(original["offered"], true);
    assert!(original["effect"]["Paths"]
        .as_array()
        .is_some_and(|paths| !paths.is_empty()));
    let stored = fixture.stored();
    assert_eq!(stored["workspace"], json!(fixture.root));
    assert_eq!(stored["execution"], json!(fixture.request.execution));
    assert_eq!(original["process_origin"]["thread"], "scope-thread");
    for command in fixture.remote.commands.lock().unwrap().iter() {
        assert!(
            command.get(NATIVE_SCOPE).is_none(),
            "private Native execution facts reached the app"
        );
        assert!(!serde_json::to_string(command)
            .unwrap()
            .contains("hook_free_tools"));
    }
}

#[tokio::test]
async fn native_scope_missing_legacy_or_foreign_facts_never_narrow_tool_effects() {
    for mode in ["missing", "legacy", "wrong-owner", "unbound-native"] {
        let fixture = Fixture::new(mode).await;
        let call = fixture.offer(&patch("a", "a")).await;
        assert_eq!(call["offered"], false, "{mode}");
        assert_eq!(call["effect"], "Opaque", "{mode}");
        assert_eq!(call["result"]["executed"], false);
        assert_eq!(
            call["result"]["error"],
            if mode == "legacy" {
                "native_execution_generation_unsupported"
            } else {
                "native_execution_completion_unavailable"
            }
        );
        assert_eq!(fixture.stored()["origin"], "native");
        if mode == "unbound-native" {
            assert!(
                fixture.request.execution.is_some(),
                "raw claimed facts are deliberately present"
            );
            assert!(fixture.request.native_scope.is_none());
            assert!(fixture.stored()["workspace"].is_null());
        } else {
            assert_eq!(fixture.stored()["workspace"], json!(fixture.root));
        }
        if matches!(mode, "missing" | "wrong-owner" | "unbound-native") {
            assert!(fixture.stored()["execution"].is_null());
        }
        assert_eq!(
            fixture.broker.effect_status("scope-lane").unwrap()["unconfirmed_effects"],
            0
        );
    }
    let generic = Fixture::new("generic").await;
    let call = generic.offer(&patch("generic", "a")).await;
    assert_eq!(
        call["offered"], true,
        "ordinary generic transport does not acquire Native scope"
    );
    assert_eq!(call["effect"], "Opaque");
    assert!(generic.stored()["workspace"].is_null());
    assert_eq!(generic.stored()["origin"], "generic");
    assert_eq!(
        generic.broker.require_native_origin(&generic.token),
        Err(Error("mcp_tool_origin_changed"))
    );
}

#[tokio::test]
async fn native_scope_settlement_preserves_original_receipt_and_updates_only_next_facts() {
    let fixture = Fixture::new("qualified").await;
    let original = fixture.offer(&patch("a", "a")).await;
    let next = NativeScope {
        workspace: fixture.root.clone(),
        execution: None,
    };
    let command = fixture.settlement(&original, Some(&next));
    fixture
        .bridge
        .admit_submission(command.clone())
        .await
        .unwrap();
    fixture.bridge.operation(command.clone()).await.unwrap();
    let stored = fixture.stored();
    assert!(stored["execution"].is_null());
    for field in [
        "id",
        "fingerprint",
        "name",
        "input",
        "process_origin",
        "effect",
    ] {
        assert_eq!(stored["calls"]["a"][field], original[field], "{field}");
    }
    assert_eq!(
        stored["calls"]["a"]["result"],
        json!({"output":"original result"})
    );
    assert_eq!(
        fixture
            .broker
            .retained_result(&fixture.token, &patch("a", "a"))
            .unwrap(),
        Some(json!({"output":"original result"}))
    );
    assert_eq!(
        fixture.bridge.operation(command.clone()).await.unwrap()["reused"],
        true
    );
    let followup = fixture.offer(&patch("b", "b")).await;
    assert_eq!(
        followup["effect"], "Opaque",
        "new calls cannot borrow the previous hook proof"
    );
    assert_eq!(followup["offered"], false);
    assert_eq!(
        followup["result"]["error"],
        "native_execution_completion_unavailable"
    );
    assert_eq!(
        fixture.broker.effect_status("scope-lane").unwrap()["unconfirmed_effects"],
        0,
        "the original scoped claim was settled independently of a new unavailable invocation"
    );
    fixture
        .broker
        .bind_execution(
            &fixture.token,
            fixture
                .request
                .native_scope
                .as_ref()
                .unwrap()
                .execution
                .clone(),
        )
        .await
        .unwrap();
    let qualified = fixture
        .offer(&patch("qualified-after-settlement", "b"))
        .await;
    assert_eq!(
        qualified["offered"], true,
        "a fresh qualified invocation remains available"
    );
    let mut changed = command;
    changed[NATIVE_SCOPE] = json!(fixture.request.native_scope.as_ref().unwrap());
    assert_eq!(
        fixture.bridge.operation(changed).await,
        Err(Error("mcp_submission_changed"))
    );
}

#[tokio::test]
async fn native_scope_change_cannot_settle_a_different_workspace_or_pending_call() {
    for case in ["workspace", "pending"] {
        let fixture = Fixture::new("qualified").await;
        let original = fixture.offer(&patch("a", "a")).await;
        let before = fixture.stored();
        let mut next = fixture.request.native_scope.clone().unwrap();
        let error = if case == "workspace" {
            next.workspace = fixture.root.join("other");
            std::fs::create_dir(&next.workspace).unwrap();
            next.execution.as_mut().unwrap().cwd = next.workspace.clone();
            "mcp_native_scope_changed"
        } else {
            fixture
                .broker
                .enqueue(&fixture.token, &patch("queued", "b"))
                .await
                .unwrap();
            next.execution = None;
            "mcp_pending_catalog_change"
        };
        assert_eq!(
            fixture
                .bridge
                .operation(fixture.settlement(&original, Some(&next)))
                .await,
            Err(Error(error))
        );
        let after = fixture.stored();
        assert_eq!(after["execution"], before["execution"]);
        assert_eq!(after["workspace"], before["workspace"]);
        assert_eq!(after["calls"]["a"], before["calls"]["a"]);
        let lane: Lane = fixture
            .journal
            .read(&digest(&("mcp-lane-v1", "scope-lane")))
            .unwrap()
            .unwrap();
        assert!(
            lane.settlement.is_none(),
            "validation rejected before a prepared effect"
        );
    }
}

#[tokio::test]
async fn native_scope_rejects_cross_thread_or_noncanonical_private_descriptors() {
    let fixture = Fixture::new("qualified").await;
    std::os::unix::fs::symlink(&fixture.root, fixture.root.join("alias")).unwrap();
    for case in ["thread", "turn", "cwd", "alias", "extra"] {
        let mut scope = json!(fixture.request.native_scope.as_ref().unwrap());
        match case {
            "thread" => scope["execution"]["thread"] = json!("another-thread"),
            "turn" => scope["execution"]["turn"] = json!("another-turn"),
            "cwd" => scope["execution"]["cwd"] = json!(fixture.root.join("other")),
            "alias" => scope["workspace"] = json!(fixture.root.join("alias")),
            "extra" => scope["untrusted"] = json!(true),
            _ => unreachable!(),
        }
        let mut command = fixture.settlement(&json!({"id":"unoffered"}), None);
        command[NATIVE_SCOPE] = scope;
        let before = fixture.remote.commands.lock().unwrap().len();
        assert_eq!(
            fixture.bridge.admit_submission(command.clone()).await,
            Err(Error("mcp_native_scope_invalid")),
            "{case}"
        );
        assert_eq!(
            fixture.bridge.operation(command).await,
            Err(Error("mcp_native_scope_invalid")),
            "{case}"
        );
        assert_eq!(fixture.remote.commands.lock().unwrap().len(), before);
    }
}

#[tokio::test]
async fn native_cached_offer_requires_original_scope_even_when_legacy_origin_is_absent() {
    let mut fixture = Fixture::new("generic").await;
    let args = patch("original-cached", "a");
    let original = fixture.offer(&args).await;
    assert_eq!(original["offered"], true);
    let key = digest(&("mcp-turn-v1", &fixture.token));
    let mut legacy = fixture.stored();
    legacy.as_object_mut().unwrap().remove("origin");
    fixture.journal.write(&key, &legacy).unwrap();
    let before: Value = fixture.journal.read(&key).unwrap().unwrap();
    fixture.bridge.native_adapter = true;
    assert_eq!(
        fixture
            .bridge
            .operation(json!({"op":"snapshot","lane":"scope-lane"}))
            .await,
        Err(Error("native_execution_completion_unavailable"))
    );
    let after: Value = fixture.journal.read(&key).unwrap().unwrap();
    assert_eq!(before, after, "refusal must not replace the cached original offer");
    assert_eq!(after["calls"][&args.call_id], original);
}
