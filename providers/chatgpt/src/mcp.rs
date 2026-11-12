//! Retained MCP invocations. This module never executes a tool or owns native
//! history. A detached HTTP peer may retry its exact logical call; it cannot
//! cause a second native action. Only the common app lane can offer/settle it.
use crate::{
    context::digest,
    journal::{random_id, Journal},
    native_claims::Guard as SharedNativeGuard,
    native_effect::{self, Effect},
    protocol::{gateway_calls, Error, Tool},
};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify};

/// Host metadata can locate related contexts, never authorize or choose work.
/// The authenticated service owner scopes private correlation across restarts.
pub(crate) fn chat_correlation(params: &Value, owner: &str) -> Result<Option<String>, Error> {
    let Some(meta) = params.get("_meta") else {
        return Ok(None);
    };
    let meta = meta.as_object().ok_or(Error("mcp_chat_metadata_invalid"))?;
    let field = |key: &str| match meta.get(key) {
        None => Ok(None),
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value.as_str())),
        _ => Err(Error("mcp_chat_metadata_invalid")),
    };
    let session = field("openai/session")?;
    let subject = field("openai/subject")?;
    let organization = field("openai/organization")?;
    let Some(session) = session else {
        return Ok(None);
    };
    use hmac::{Hmac, Mac};
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(owner.as_bytes()).unwrap();
    mac.update(
        &serde_json::to_vec(&("mcp-chat-correlation-v1", session, subject, organization)).unwrap(),
    );
    Ok(Some(
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}

const LIMIT: usize = 16 * 1024 * 1024;
const TURN_CACHE_CAPACITY: usize = 256;
#[cfg(test)]
mod source_tests {
    use super::*;
    #[test]
    fn chat_correlation_uses_only_host_envelope_and_authenticated_owner() {
        let params = json!({"_meta":{"openai/session":"chat-A","openai/subject":"subject-A","openai/organization":"organization-A"}});
        let first = chat_correlation(&params, "owner-A").unwrap().unwrap();
        assert_eq!(first.len(), 64);
        assert_ne!(
            Some(first.clone()),
            chat_correlation(&params, "owner-B").unwrap()
        );
        for field in ["openai/session", "openai/subject", "openai/organization"] {
            let mut other = params.clone();
            other["_meta"][field] = json!("different");
            assert_ne!(
                Some(first.clone()),
                chat_correlation(&other, "owner-A").unwrap()
            );
        }
        let mut branch = params.clone();
        branch["_meta"]["unverified_branch"] = json!("another branch");
        assert_eq!(
            Some(first.clone()),
            chat_correlation(&branch, "owner-A").unwrap()
        );
        assert_eq!(
            chat_correlation(&json!({"arguments":params}), "owner-A").unwrap(),
            None
        );
        assert_eq!(
            chat_correlation(&json!({"_meta":{"openai/subject":"subject-A"}}), "owner-A").unwrap(),
            None
        );
        for invalid in [
            json!({"_meta":null}),
            json!({"_meta":[]}),
            json!({"_meta":{"openai/session":false}}),
        ] {
            assert_eq!(
                chat_correlation(&invalid, "owner-A"),
                Err(Error("mcp_chat_metadata_invalid"))
            );
        }
    }
    #[tokio::test]
    async fn stop_retries_evicted_source_revocation_after_journal_failure() {
        let root = std::env::temp_dir().join(format!("source-revocation-{}", random_id().unwrap()));
        let journal = Arc::new(Journal::open(&root).unwrap());
        let broker = Broker::new(journal.clone());
        let source = broker
            .register("stopped-source", BTreeMap::new(), true)
            .await
            .unwrap();
        broker.retire(&source, false).await.unwrap();
        for _ in 0..300 {
            let token = broker
                .register("other-source", BTreeMap::new(), false)
                .await
                .unwrap();
            broker.retire(&token, false).await.unwrap();
        }
        let key = digest(&("mcp-turn-v1", &source));
        let record = root.join(format!("{key}.json"));
        let original = root.join("original-source-record");
        std::fs::rename(&record, &original).unwrap();
        std::fs::create_dir(&record).unwrap();
        let failed = broker.stop_lane("stopped-source", None).await;
        std::fs::remove_dir(&record).unwrap();
        std::fs::rename(&original, &record).unwrap();
        assert!(
            failed.is_err(),
            "unreadable retirement must not report confirmed Stop"
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), broker.catalog_readback(&source))
                .await
                .unwrap(),
            Err(Error("mcp_turn_unavailable"))
        );
        broker.stop_lane("stopped-source", None).await.unwrap();
        assert!(
            !journal.read::<Turn>(&key).unwrap().unwrap().source_bound,
            "retry must durably finish the original evicted revocation"
        );
        broker.close().await;
        assert_eq!(
            Broker::new(journal)
                .restore_source_discovery(&source, &source, "stopped-source")
                .await,
            Err(Error("mcp_source_discovery_invalid"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn retained_source_discovery_survives_payload_eviction_without_restoring_execution() {
        let root = std::env::temp_dir().join(format!("source-retention-{}", random_id().unwrap()));
        let journal = Arc::new(Journal::open(&root).unwrap());
        let broker = Broker::new(journal.clone());
        let first = broker
            .register("retained-source", BTreeMap::new(), true)
            .await
            .unwrap();
        broker.retire(&first, false).await.unwrap();
        let mut last = first.clone();
        // More completed source inputs than the payload cache retains. The
        // original prompt's discovery token remains the actual consumer.
        for _ in 0..300 {
            let next = broker
                .register("retained-source", BTreeMap::new(), true)
                .await
                .unwrap();
            broker.source_discovery(&last, &next).await.unwrap();
            broker.retire(&next, false).await.unwrap();
            last = next;
        }
        let visible = broker
            .register("retained-source", BTreeMap::new(), true)
            .await
            .unwrap();
        broker.source_discovery(&last, &visible).await.unwrap();
        assert_eq!(
            broker.catalog_readback(&first).await.unwrap()["turn_token"],
            visible
        );
        broker.retire(&visible, false).await.unwrap();
        last = visible;
        // Traffic from another lane can evict even the settled predecessor.
        for _ in 0..300 {
            let token = broker
                .register("other-source", BTreeMap::new(), false)
                .await
                .unwrap();
            broker.retire(&token, false).await.unwrap();
        }
        let next = broker
            .register("retained-source", BTreeMap::new(), true)
            .await
            .unwrap();
        broker.source_discovery(&last, &next).await.unwrap();
        assert_eq!(
            broker.catalog_readback(&first).await.unwrap()["turn_token"],
            next
        );
        assert_eq!(
            broker.catalog(&first).await,
            Err(Error("mcp_turn_unavailable"))
        );
        broker.retire(&next, false).await.unwrap();
        broker.close().await;
        let restored = Broker::new(journal.clone());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), restored.catalog_readback(&first))
                .await
                .unwrap(),
            Err(Error("mcp_turn_unavailable")),
            "journal presence alone never admits discovery to a new owner"
        );
        restored
            .restore_source_discovery(&first, &next, "retained-source")
            .await
            .unwrap();
        let current = restored
            .register("retained-source", BTreeMap::new(), true)
            .await
            .unwrap();
        restored.source_discovery(&next, &current).await.unwrap();
        assert_eq!(
            restored.catalog_readback(&first).await.unwrap()["turn_token"],
            current
        );
        assert_eq!(
            restored.catalog(&next).await,
            Err(Error("mcp_turn_unavailable"))
        );
        restored
            .stop_lane("retained-source", Some(&current))
            .await
            .unwrap();
        let new_primary = restored
            .register("retained-source", BTreeMap::new(), true)
            .await
            .unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), restored.catalog_readback(&first))
                .await
                .unwrap(),
            Err(Error("mcp_turn_unavailable")),
            "a later primary cannot reopen the stopped discovery chain"
        );
        assert!(restored.catalog(&new_primary).await.is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn source_discovery_crosses_only_admitted_boundaries_and_keeps_old_calls_retired() {
        let root = std::env::temp_dir().join(format!("source-discovery-{}", random_id().unwrap()));
        let journal = Arc::new(Journal::open(&root).unwrap());
        let broker = Broker::new(journal.clone());
        let request = crate::protocol::Request::parse(json!({"model":"chatgpt-desktop/gpt-6","stream":true,"input":[{"role":"user","content":"Owned native source"}],"tools":[{"type":"function","name":"exec","parameters":{"type":"object"}}]}),&crate::config::Config::default(),Some("source-native")).unwrap();
        let first = broker
            .register("source-lane", request.tools.clone(), true)
            .await
            .unwrap();
        assert_eq!(
            broker.source_participation(&first).unwrap()["state"],
            "execution_not_joined"
        );
        broker.require_source_context(&first).await.unwrap();
        let before_context = Arguments {
            call_id: "before-context".into(),
            name: "exec".into(),
            input: "{}".into(),
            report: "Must not execute before current native context".into(),
        };
        assert_eq!(
            broker.enqueue(&first, &before_context).await,
            Err(Error("source_context_required"))
        );
        assert!(broker.offer(&first).await.unwrap().is_empty());
        broker
            .accept_source_context(&first, "verified-context")
            .await
            .unwrap();
        assert_eq!(
            broker.source_participation(&first).unwrap()["context_accepted"],
            true
        );
        let call = Arguments {
            call_id: "original".into(),
            name: "exec".into(),
            input: "{}".into(),
            report: "Original native action".into(),
        };
        broker.enqueue(&first, &call).await.unwrap();
        let offered = broker.offer(&first).await.unwrap();
        let output = json!({"output":"Original result"});
        assert_eq!(
            broker.interrupt_source(&first).await,
            Err(Error("mcp_pending_completion")),
            "offered native effects must settle before interruption advances"
        );
        broker
            .settle(
                &first,
                &[(offered[0]["id"].as_str().unwrap().into(), output.clone())],
            )
            .await
            .unwrap();
        let rejected = Arguments {
            call_id: "not-offered".into(),
            name: "exec".into(),
            input: "{}".into(),
            report: "Never admitted to Native".into(),
        };
        broker.enqueue(&first, &rejected).await.unwrap();
        broker.interrupt_source(&first).await.unwrap();
        assert_eq!(
            broker.accept_source_context(&first, "other-context").await,
            Err(Error("mcp_turn_unavailable"))
        );
        assert_eq!(
            broker.source_participation(&first).unwrap()["context_accepted"],
            true
        );
        assert_eq!(
            broker.invoke(&first, rejected).await.unwrap(),
            json!({"error":"source_response_interrupted","executed":false})
        );
        assert_eq!(
            broker.offer(&first).await,
            Err(Error("mcp_turn_unavailable"))
        );
        let observing = {
            let broker = broker.clone();
            let first = first.clone();
            tokio::spawn(async move { broker.catalog_readback(&first).await })
        };
        tokio::task::yield_now().await;
        assert!(
            !observing.is_finished(),
            "completed bound discovery waits for actual new admission"
        );
        let second = broker
            .register("source-lane", request.tools.clone(), true)
            .await
            .unwrap();
        broker.source_discovery(&first, &second).await.unwrap();
        assert_eq!(observing.await.unwrap().unwrap()["turn_token"], second);
        assert_eq!(
            broker.catalog_readback(&first).await.unwrap()["turn_token"],
            second
        );
        assert_eq!(
            broker.invoke(&first, call).await.unwrap(),
            output,
            "an original receipt is still readable"
        );
        assert_eq!(
            broker
                .enqueue(
                    &first,
                    &Arguments {
                        call_id: "new".into(),
                        name: "exec".into(),
                        input: "{}".into(),
                        report: "Must not execute".into()
                    }
                )
                .await,
            Err(Error("mcp_turn_unavailable"))
        );
        broker.retire(&second, false).await.unwrap();
        let third = broker
            .register("source-lane", request.tools.clone(), true)
            .await
            .unwrap();
        broker.source_discovery(&second, &third).await.unwrap();
        assert_eq!(
            broker.catalog_readback(&first).await.unwrap()["turn_token"],
            third,
            "consecutive external inputs can discover the current admission"
        );
        let unrelated = broker
            .register("other-lane", request.tools.clone(), false)
            .await
            .unwrap();
        assert!(broker.source_discovery(&first, &unrelated).await.is_err());
        assert!(
            Broker::new(journal).catalog_readback(&first).await.is_err(),
            "restart never restores live execution authority from disk"
        );
        broker.retire(&third, false).await.unwrap();
        let native = broker
            .register("source-lane", request.tools, true)
            .await
            .unwrap();
        // A later native primary installs its own instructions rather than
        // linking the prior discovery chain. Stop still owns the entire lane.
        let old_discovery = {
            let broker = broker.clone();
            let first = first.clone();
            tokio::spawn(async move { broker.catalog_readback(&first).await })
        };
        tokio::task::yield_now().await;
        assert!(!old_discovery.is_finished());
        broker
            .stop_lane("source-lane", Some(&native))
            .await
            .unwrap();
        assert!(
            broker.catalog(&unrelated).await.is_ok(),
            "Stop must retain unrelated native work"
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), old_discovery)
                .await
                .unwrap()
                .unwrap(),
            Err(Error("mcp_turn_unavailable")),
            "Stop wakes/fences old discovery instead of reviving execution"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
/// Rust Provider declarations share the qualified operation policy. Mixed
/// legacy controls stay conservative; this hint never changes authorization.
pub(crate) fn tool_annotations(name: &str) -> Value {
    let idempotent = matches!(
        name,
        "termux_shell"
            | "local_context"
            | "local_execute"
            | "local_process"
            | "native_catalog"
            | "native_tool"
            | "codex_chats"
            | "codex_history"
            | "codex_tools"
            | "codex_status"
            | "codex_execute"
            | "codex_report"
            | "codex_finish"
            | "codex_handoff"
            | "codex_answer"
            | "codex_attach"
    );
    json!({"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false,
        "idempotentHint":idempotent})
}
#[derive(Clone, Serialize, Deserialize)]
struct Invocation {
    fingerprint: String,
    pub id: String,
    pub name: String,
    pub input: String,
    pub report: String,
    offered: bool,
    // Written with the actual offer. A newer sampling request must not change
    // whether this original receipt must reconcile the shared reservation.
    // Older readers may omit it; unknown legacy offers require exact lookup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shared_reserved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    process_hooks_absent: Option<bool>,
    result: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    native_input: Vec<Value>,
    #[serde(default)]
    effect: Option<Effect>,
    #[serde(default)]
    process_origin: Option<NativeProcessOrigin>,
    #[serde(default)]
    completed_session: Option<u64>,
    // Positive Native harness evidence supplements, never replaces, the
    // original immutable function result (which may still be unavailable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    terminal_completion: Option<crate::native_claims::completion::Proof>,
    // The old completed_session marker did not retain the exit sentinel.
    // Legacy reuse needs a newly observed original qualified ordinary exit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_legacy_exit: Option<i64>,
}
// A terminal command item may arrive before its final write_stdin result.
// Keep this evidence on the original receipt, never in a second session registry.
#[derive(Clone, Serialize, Deserialize)]
struct NativeProcessOrigin {
    pid: u32,
    start: String,
    thread: String,
    turn: String,
}
impl NativeProcessOrigin {
    fn from_execution(execution: &native_effect::Execution) -> Self {
        Self {
            pid: execution.pid,
            start: execution.start.clone(),
            thread: execution.thread.clone(),
            turn: execution.turn.clone(),
        }
    }
    fn matches_evidence(
        &self,
        evidence: &crate::operator_native::CompletionEvidence,
        turn: &str,
    ) -> bool {
        self.pid == evidence.owner.process.pid
            && self.start == evidence.owner.process.start
            && self.thread == evidence.thread
            && self.turn == turn
            && !turn.is_empty()
    }
    fn same_owner(&self, execution: &native_effect::Execution) -> bool {
        self.pid == execution.pid
            && self.start == execution.start
            && self.thread == execution.thread
            && !self.turn.is_empty()
            && !execution.turn.is_empty()
    }
}
impl Invocation {
    fn matches_native_call(
        &self,
        turn: &Turn,
        evidence: &crate::operator_native::CompletionEvidence,
    ) -> bool {
        let Some((native_turn, raw)) = evidence.calls.get(&self.id) else {
            return false;
        };
        if !self
            .process_origin
            .as_ref()
            .is_some_and(|origin| origin.matches_evidence(evidence, native_turn))
        {
            return false;
        }
        let Ok((calls, _)) = gateway_calls(
            &self.id,
            &json!({
            "name":self.name,"input":self.input,"report":self.report})
            .to_string(),
            &turn.tools,
        ) else {
            return false;
        };
        let [call] = calls.as_slice() else {
            return false;
        };
        let Ok(expected) = call.output_item() else {
            return false;
        };
        [
            "type",
            "call_id",
            "name",
            "namespace",
            "arguments",
            "input",
            "execution",
        ]
        .iter()
        .all(|key| expected[*key] == raw[*key])
    }
    fn requires_shared_settlement(&self) -> bool {
        self.shared_reserved != Some(false)
    }

    fn admission_effect(
        &self,
        workspace: Option<&Path>,
        execution: Option<&native_effect::Execution>,
    ) -> Effect {
        match self.effect.as_ref() {
            // Recheck queued legacy reads against their original turn facts.
            // Admission and settlement must agree without rewriting identity,
            // receipts, arguments or stored resource snapshots.
            Some(Effect::Read) => native_effect::classify_with_execution(
                &self.name,
                &self.input,
                workspace,
                execution,
            ),
            Some(effect) => effect.admission_scope().clone(),
            None => Effect::Opaque,
        }
    }

    fn original_result(&self) -> Option<Value> {
        self.result.as_ref().map(|result| {
            if self.native_input.is_empty() {
                result.clone()
            } else {
                json!({"result":result,"native_input":self.native_input})
            }
        })
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Turn {
    lane: String,
    #[serde(default)]
    origin: crate::protocol::ToolOrigin,
    #[serde(default)]
    workspace: Option<PathBuf>,
    #[serde(default)]
    execution: Option<native_effect::Execution>,
    tools: BTreeMap<String, Tool>,
    calls: BTreeMap<String, Invocation>,
    retired: bool,
    order: u64,
    #[serde(default)]
    discovery: Option<String>,
    #[serde(default)]
    source_bound: bool,
    #[serde(default)]
    catalog_discovered: bool,
    #[serde(default)]
    context_receipt: Option<String>,
    #[serde(default)]
    requires_context: bool,
}
impl Turn {
    fn retire(&mut self, stopped: bool) {
        self.retired = true;
        if stopped {
            self.source_bound = false;
        }
    }
}
struct DiscoveryAdmission {
    lane: String,
    // The already-adopted chain authorizes readback, never execution of an old
    // token. It survives payload eviction and is restored only by source adoption.
    root: String,
    revoked: bool,
}
#[derive(Default)]
struct Registry {
    turns: BTreeMap<String, Turn>,
    // Admission survives payload eviction. Revoked entries retain only the
    // identities needed to retry an unfinished durable Stop.
    discovery: BTreeMap<String, DiscoveryAdmission>,
}
#[derive(Clone, Serialize, Deserialize)]
struct ClaimedEffect {
    lane: String,
    effect: Effect,
    #[serde(default)]
    session_id: Option<u64>,
    // Link only Provider-mediated MCP calls to their original durable turn.
    // Legacy claims without a token remain unresolved instead of being guessed away.
    #[serde(default)]
    turn_token: Option<String>,
    // A polled process may finish in another Native turn after the original
    // exec result was committed. Preserve each offered continuation's turn
    // identity so its authoritative exit can be replayed after a crash.
    #[serde(default)]
    continuations: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    continuation_effect: Option<crate::native_claims::ContinuationEffect>,
    #[serde(default)]
    process_hooks_absent: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct ExternalEffectReceipt {
    fingerprint: String,
    name: String,
    continuation: Option<u64>,
    // Set only from this call's authoritative Native-bound classification.
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    settled: bool,
    #[serde(default)]
    workspace: Option<PathBuf>,
    #[serde(default)]
    lane: Option<String>,
    // Minimal durable exit/session proof, not upstream output or secrets.
    #[serde(default)]
    completion: Option<ExternalCompletion>,
    #[serde(default)]
    process_origin: Option<NativeProcessOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    process_hooks_absent: Option<bool>,
    #[serde(default)]
    completed_session: Option<u64>,
    // Positive Native harness evidence supplements, never replaces, the
    // original immutable function result (which may still be unavailable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    terminal_completion: Option<crate::native_claims::completion::Proof>,
    // The old completed_session marker did not retain the exit sentinel.
    // Legacy reuse needs a newly observed original qualified ordinary exit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_legacy_exit: Option<i64>,
}
impl ExternalEffectReceipt {
    fn matches_native_call(
        &self,
        id: &str,
        evidence: &crate::operator_native::CompletionEvidence,
    ) -> bool {
        let Some((turn, raw)) = evidence.calls.get(id) else {
            return false;
        };
        self.process_origin
            .as_ref()
            .is_some_and(|origin| origin.matches_evidence(evidence, turn))
            && native_effect::process_tool(&self.name) == Some("exec_command")
            && raw["type"] == "function_call"
            && raw["name"] == "exec_command"
            && native_effect::default_namespace(&raw["namespace"])
            && raw["arguments"]
                .as_str()
                .is_some_and(|input| self.fingerprint == digest(&(&self.name, input)))
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct ExternalCompletion {
    fingerprint: String,
    running: Option<u64>,
    exited: bool,
    #[serde(default)]
    not_spawned: bool,
    // Old exited/not_spawned booleans lost the underlying evidence. Never
    // recreate a successful exit or non-dispatch marker from those alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_result: Option<ExternalResultProof>,
}
#[derive(Clone, Serialize, Deserialize)]
struct ExternalResultProof {
    schema: u8,
    #[serde(default)]
    source_call_id: Option<String>,
    #[serde(default)]
    running: Option<u64>,
    exit_code: Option<i64>,
    #[serde(default)]
    native_completion: Option<Value>,
    #[serde(default)]
    completion_protocol: bool,
}
impl ExternalCompletion {
    #[cfg(test)]
    fn from_result(call_id: &str, result: &Value) -> Self {
        Self::from_result_for(call_id, None, result)
    }
    fn from_result_for(call_id: &str, session: Option<u64>, result: &Value) -> Self {
        let observation = crate::native_claims::process_observation(result, call_id, session);
        Self {
            fingerprint: digest(result),
            running: observation.running,
            exited: observation.terminal,
            not_spawned: observation.not_spawned,
            native_result: Some(ExternalResultProof {
                schema: 1,
                source_call_id: Some(call_id.to_owned()),
                running: observation.running,
                exit_code: observation.exit_code,
                native_completion: observation
                    .proof
                    .and_then(|proof| serde_json::to_value(proof).ok()),
                completion_protocol: observation.completion_protocol,
            }),
        }
    }
    fn source_call_id(&self) -> Option<&str> {
        self.native_result
            .as_ref()
            .filter(|proof| proof.schema == 1)?
            .source_call_id
            .as_deref()
            .filter(|id| !id.is_empty())
    }
    fn confirmed_terminal(&self) -> bool {
        let Some(call) = self.source_call_id() else {
            return false;
        };
        let Some(saved) = self.native_result.as_ref() else {
            return false;
        };
        if saved.running.is_some() {
            return false;
        }
        match saved.native_completion.as_ref() {
            Some(value) => {
                crate::native_claims::completion::Proof::parse(value).is_some_and(|proof| {
                    proof.belongs_to(call, None)
                        && proof.wait_completed()
                        && proof.waited() == saved.exit_code
                })
            }
            None if !saved.completion_protocol => saved
                .exit_code
                .is_some_and(crate::native_claims::confirmed_exit_code),
            None => false,
        }
    }
    fn confirmed_exit(&self) -> Option<i64> {
        self.confirmed_terminal()
            .then(|| {
                self.native_result
                    .as_ref()
                    .and_then(|saved| saved.exit_code)
            })
            .flatten()
    }
    fn status(&self) -> (Option<u64>, bool) {
        if self.source_call_id().is_none() {
            return (None, false);
        }
        (
            self.native_result.as_ref().and_then(|proof| proof.running),
            self.confirmed_terminal(),
        )
    }
    fn marker(&self) -> Value {
        let Some(call) = self.source_call_id() else {
            return json!({});
        };
        let saved = self.native_result.as_ref().unwrap();
        if let Some(value) = &saved.native_completion {
            let Some(proof) = crate::native_claims::completion::Proof::parse(value)
                .filter(|proof| proof.belongs_to(call, None))
            else {
                return json!({});
            };
            if proof.not_started()
                || proof.running().is_some()
                || proof.wait_completed() && self.confirmed_terminal()
            {
                return json!({"output":format!("Native completion: {value}\nOutput:\n")});
            }
            return json!({});
        }
        if let Some(session) = saved.running {
            let output = if saved.completion_protocol {
                format!(
                    "Native completion: null\nProcess running with session ID {session}\nOutput:\n"
                )
            } else {
                format!("Process running with session ID {session}")
            };
            json!({"output":output})
        } else if let Some(code) = self.confirmed_exit() {
            json!({"output":format!("Process exited with code {code}")})
        } else {
            json!({})
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct ActiveEffects {
    claims: BTreeMap<String, ClaimedEffect>,
    #[serde(default)]
    external: BTreeMap<String, ExternalEffectReceipt>,
}
impl ActiveEffects {
    fn session_poll_scope(&self, lane: &str, session: u64) -> Option<Effect> {
        let mut owners = self
            .claims
            .iter()
            .filter(|(_, held)| held.lane == lane && held.session_id == Some(session));
        let (id, original) = owners.next()?;
        if owners.next().is_some() || !matches!(original.effect.claim_scope(), Effect::Paths(_)) {
            return None;
        }
        if original.turn_token.is_none()
            && self
                .external
                .get(id)
                .and_then(|receipt| receipt.completion.as_ref())
                .map(ExternalCompletion::status)
                != Some((Some(session), false))
        {
            // Old external status booleans are not session provenance. Only
            // the identical original result can supplement that receipt.
            return None;
        }
        Some(original.effect.clone())
    }
}
/// A completed session is observable only while its original receipt proves
/// that output has not been collected. Native reuses numeric session IDs after
/// removal: any later launch, terminal poll, unknown result or pending poll for
/// that same owner/session invalidates this narrow observation path.
fn completed_session_poll(
    registry: &Registry,
    state: &ActiveEffects,
    lane: &str,
    session: u64,
    execution: &native_effect::Execution,
) -> bool {
    let mut evidence = SessionPollEvidence::new(state, lane, session, execution);
    for turn in registry.turns.values() {
        evidence.observe_turn(turn, lane, session, execution);
    }
    evidence.qualifies()
}

#[derive(Default)]
struct SessionPollEvidence {
    originals: usize,
    completed: usize,
    blocked: bool,
}
impl SessionPollEvidence {
    fn new(
        state: &ActiveEffects,
        lane: &str,
        session: u64,
        execution: &native_effect::Execution,
    ) -> Self {
        let mut evidence = Self {
            blocked: state
                .claims
                .values()
                .any(|claim| claim.lane == lane && claim.session_id == Some(session)),
            ..Self::default()
        };
        for receipt in state.external.values().filter(|receipt| {
            receipt.lane.as_deref() == Some(lane)
                && receipt
                    .process_origin
                    .as_ref()
                    .is_some_and(|origin| origin.same_owner(execution))
        }) {
            evidence.observe(
                &receipt.name,
                receipt.continuation,
                receipt.completion.as_ref().map(ExternalCompletion::status),
                receipt.completed_session.filter(|_| {
                    receipt
                        .terminal_completion
                        .as_ref()
                        .is_some_and(|proof| proof.wait_completed())
                        || receipt
                            .completed_legacy_exit
                            .is_some_and(crate::native_claims::confirmed_exit_code)
                }),
                session,
            );
        }
        evidence
    }
    fn observe_turn(
        &mut self,
        turn: &Turn,
        lane: &str,
        session: u64,
        execution: &native_effect::Execution,
    ) {
        if turn.lane != lane {
            return;
        }
        for call in turn.calls.values().filter(|call| {
            call.offered
                && call
                    .process_origin
                    .as_ref()
                    .is_some_and(|origin| origin.same_owner(execution))
        }) {
            self.observe(
                &call.name,
                continued_process(&call.name, &call.input),
                call.result.as_ref().map(|result| {
                    let observed = crate::native_claims::process_observation(
                        result,
                        &call.id,
                        continued_process(&call.name, &call.input),
                    );
                    (observed.running, observed.terminal)
                }),
                call.completed_session.filter(|_| {
                    call.terminal_completion
                        .as_ref()
                        .is_some_and(|proof| proof.wait_completed())
                        || call
                            .completed_legacy_exit
                            .is_some_and(crate::native_claims::confirmed_exit_code)
                }),
                session,
            );
        }
    }
    fn observe(
        &mut self,
        name: &str,
        continuation: Option<u64>,
        status: Option<(Option<u64>, bool)>,
        completed_session: Option<u64>,
        session: u64,
    ) {
        if native_effect::process_tool(name) == Some("exec_command")
            && status == Some((Some(session), false))
        {
            self.originals += 1;
            self.completed += usize::from(completed_session == Some(session));
        } else if continuation == Some(session) && status != Some((Some(session), false)) {
            self.blocked = true;
        }
    }
    fn qualifies(&self) -> bool {
        !self.blocked && self.originals == 1 && self.completed == 1
    }
}

fn external_effect_id(lane: &str, tool_id: &str) -> String {
    digest(&("upstream-native-effect-v1", lane, tool_id))
}
fn busy_effect_reply() -> Value {
    json!({
        "error":"native_resource_busy",
        "executed":false,
        "retryable":true,
        "message":"Conflicting Native effect remains active. Inspect codex_status.native_effects, wait for its settlement, and submit a new call ID; this rejected logical call will not be dispatched later."
    })
}
fn settle_turn_effect(claims: &mut ActiveEffects, turn: &Turn, call: &Invocation, result: &Value) {
    if turn.origin == crate::protocol::ToolOrigin::Generic {
        // A finite generic tool response is not a Native process result, even
        // when the original tool happens to use a builtin-looking name.
        claims.claims.remove(&call.id);
        return;
    }
    settle_effect(
        claims,
        &call.id,
        &turn.lane,
        &call.name,
        continued_process(&call.name, &call.input),
        result,
    );
}

fn settle_effect(
    claims: &mut ActiveEffects,
    id: &str,
    lane: &str,
    name: &str,
    continuation: Option<u64>,
    result: &Value,
) {
    settle_effect_for(claims, id, lane, name, continuation, result, id);
}
fn settle_effect_for(
    claims: &mut ActiveEffects,
    id: &str,
    lane: &str,
    name: &str,
    continuation: Option<u64>,
    result: &Value,
    source_call_id: &str,
) {
    let observation =
        crate::native_claims::process_observation(result, source_call_id, continuation);
    if continuation.is_some() && observation.not_spawned {
        // Only this pre-dispatch continuation is proven effect-free. Its
        // original process and every other pending invocation remain owned.
        claims.claims.remove(id);
        return;
    }
    if observation.proof.as_ref().is_some_and(|proof| {
        claims
            .claims
            .get(id)
            .and_then(|held| held.session_id)
            .is_some_and(|session| proof.session_id != Some(session))
    }) {
        return;
    }
    let running = observation.running;
    let exited = observation.terminal
        && (observation.proof.is_some()
            || claims.claims.get(id).is_some_and(|held| {
                held.process_hooks_absent || matches!(held.effect.claim_scope(), Effect::Paths(_))
            })
            || !claims.claims.contains_key(id)
                && continuation.is_some_and(|session| {
                    let mut owners = claims
                        .claims
                        .values()
                        .filter(|held| held.lane == lane && held.session_id == Some(session));
                    let Some(first) = owners.next() else {
                        return false;
                    };
                    let safe = |held: &ClaimedEffect| {
                        held.process_hooks_absent
                            || matches!(held.effect.claim_scope(), Effect::Paths(_))
                    };
                    safe(first) && owners.all(safe)
                }));
    let not_spawned =
        native_effect::process_tool(name) == Some("exec_command") && observation.not_spawned;
    if let Some(pending) = claims
        .claims
        .get(id)
        .and_then(|held| held.continuation_effect.as_ref())
    {
        if continuation != Some(pending.session) || !pending.settled_by(&observation) {
            return;
        }
        claims.claims.remove(id);
        if !exited {
            return;
        }
    }
    if native_effect::process_tool(name) == Some("exec_command")
        && running.is_some()
        && !exited
        && !not_spawned
    {
        if let Some(claim) = claims.claims.get_mut(id) {
            claim.session_id = running;
        }
        return;
    }
    // A response string without an observed Native exit is not proof that
    // a possibly backgrounded shell writer stopped. Keep BOTH the Provider
    // journal and the shared Provider claim until a real terminal receipt arrives.
    if native_effect::process_tool(name) == Some("exec_command") && !exited && !not_spawned {
        return;
    }
    if let Some(session) = continuation {
        if !exited {
            // An orphan poll is still an unresolved original invocation. Mirror
            // the shared reservation and retain a positively observed session
            // for a later legitimate continuation, without claiming termination.
            if let Some(claim) = claims
                .claims
                .get_mut(id)
                .filter(|_| running == Some(session))
            {
                claim.session_id = running;
            }
            return;
        }
        claims
            .claims
            .retain(|_, held| !(held.lane == lane && held.session_id == Some(session)));
    }
    claims.claims.remove(id);
}
/// Only the qualified Native envelope establishes a process observation;
/// child stdout, synthetic timeout codes and legacy signal text cannot.
fn native_process_result(value: &Value) -> (Option<u64>, bool) {
    let (running, exit) = crate::native_claims::process_result(value);
    (running, exit.is_some())
}

fn continued_process(name: &str, input: &str) -> Option<u64> {
    (native_effect::process_tool(name) == Some("write_stdin"))
        .then(|| {
            serde_json::from_str::<Value>(input)
                .ok()?
                .get("session_id")?
                .as_u64()
        })
        .flatten()
}
impl Registry {
    fn source_admitted(&self, token: &str, lane: &str) -> bool {
        self.discovery
            .get(token)
            .is_some_and(|source| !source.revoked && source.lane == lane)
    }
    fn reserve_payload(&mut self, token: &str) -> Result<(), Error> {
        if !self.turns.contains_key(token) && self.turns.len() >= TURN_CACHE_CAPACITY {
            let oldest = self
                .turns
                .iter()
                .filter(|(_, turn)| turn.retired)
                .min_by_key(|(_, turn)| turn.order)
                .map(|(token, _)| token.clone())
                .ok_or(Error("mcp_active_turn_limit"))?;
            self.turns.remove(&oldest);
        }
        Ok(())
    }
}
pub struct Broker {
    journal: Arc<Journal>,
    registry: Mutex<Registry>,
    changed: Notify,
    controls: Option<crate::control::Controls>,
    closed: AtomicBool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arguments {
    pub call_id: String,
    pub name: String,
    pub input: String,
    pub report: String,
}
impl Arguments {
    fn validate(&self) -> Result<(), Error> {
        if self.call_id.is_empty()
            || self.call_id.len() > 160
            || !self
                .call_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
            || self.input.len() > LIMIT
            || self.report.len() > 64 * 1024
        {
            return Err(Error("mcp_call_invalid"));
        }
        Ok(())
    }
    fn fingerprint(&self) -> String {
        digest(&(&self.name, &self.input, &self.report))
    }
}
impl Broker {
    fn shared_guard_home(&self) -> Option<PathBuf> {
        // A Provider unit test is not an installed execution host; its
        // disposable Journal has its own private, process-shareable ledger.
        // Provider-mediated callers share the same account-scoped ledger.
        #[cfg(test)]
        {
            Some(self.journal.root.join("native-shared-test"))
        }
        #[cfg(not(test))]
        {
            None
        }
    }

    fn shared_guard_error(reason: String) -> Error {
        match reason.as_str() {
            "native_resource_busy" => Error("native_resource_busy"),
            "native_resource_version_conflict" => Error("native_resource_version_conflict"),
            "native_effect_duplicate_unconfirmed" => Error("native_effect_duplicate_unconfirmed"),
            "native_effect_identity_changed" => Error("native_effect_identity_changed"),
            _ => Error("native_resource_shared_guard_failed"),
        }
    }

    /// Bounded, account-private status data. Never export paths, file contents,
    /// command text or authentication material from another work.
    #[cfg(test)]
    fn effect_status(&self, lane: &str) -> Result<Value, Error> {
        self.effect_status_with_owners(&[lane], &BTreeMap::new())
    }
    pub(crate) fn effect_status_with_owners(
        &self,
        owned_lanes: &[&str],
        lane_owners: &BTreeMap<String, String>,
    ) -> Result<Value, Error> {
        let state: ActiveEffects = self
            .journal
            .read(&digest(&"native-effect-claims-v1"))?
            .unwrap_or_default();
        let entries = state
            .claims
            .iter()
            .map(|(id, held)| {
                let owner_work = lane_owners
                    .get(&held.lane)
                    .map(String::as_str)
                    .unwrap_or(held.lane.as_str());
                json!({"call":id,"owner_work":owner_work,
                "resource_kind":match held.effect.claim_scope() {
                    Effect::Read => "read",
                    Effect::Paths(_) => "bounded",
                    Effect::Opaque => "opaque",
                },
                "process_session":held.session_id,
                "owned":owned_lanes.contains(&held.lane.as_str()) || owned_lanes.contains(&owner_work),
                "pending_result":true})
            })
            .collect::<Vec<_>>();
        let shared = SharedNativeGuard::shared_status(
            self.shared_guard_home().as_deref(),
            owned_lanes,
            lane_owners,
        )
        .unwrap_or_else(|reason| json!({"available":false,"error":reason}));
        Ok(
            json!({"active":entries,"unconfirmed_effects":state.claims.len(),
            "external_call_receipts":state.external.len(),"shared":shared}),
        )
    }
    /// Read the existing original receipts, including an admitted cold source
    /// chain. This grants no execution and never restores retired tokens.
    pub(crate) async fn unloaded_receipts(
        &self,
        lanes: &[&str],
        roots: &[(&str, &str)],
        evidence: &crate::operator_native::UnloadedHistory,
    ) -> Result<crate::operator_native::UnloadedReceipts, Error> {
        let registry = self.registry.lock().await;
        let state: ActiveEffects = self
            .journal
            .read(&digest(&"native-effect-claims-v1"))?
            .unwrap_or_default();
        if state
            .claims
            .values()
            .any(|held| lanes.contains(&held.lane.as_str()))
            || state.external.values().any(|receipt| {
                !receipt.settled
                    && receipt
                        .lane
                        .as_deref()
                        .is_some_and(|lane| lanes.contains(&lane))
            })
        {
            return Err(Error("native_unloaded_effects_unsettled"));
        }
        let originals = evidence.original_calls()?;
        let mut receipts = crate::operator_native::UnloadedReceipts::default();
        let same_origin = |origin: Option<&NativeProcessOrigin>, turn: &str| {
            origin.is_some_and(|origin| {
                !turn.is_empty()
                    && origin.pid == evidence.owner.process.pid
                    && origin.start == evidence.owner.process.start
                    && origin.thread == evidence.thread
                    && origin.turn == turn
            })
        };
        let mut inspect = |token: &str, turn: &Turn| -> Result<(), Error> {
            if !lanes.contains(&turn.lane.as_str())
                || turn.calls.values().any(|call| call.result.is_none())
            {
                return Err(Error("native_unloaded_effects_unsettled"));
            }
            for (key, call) in &turn.calls {
                let Some((native_turn, raw, result)) = originals.get(&call.id) else {
                    continue;
                };
                if turn.workspace.as_deref() != Some(evidence.workspace.as_path())
                    || !call.process_origin.as_ref().is_some_and(|origin| {
                        origin.pid > 1
                            && !origin.start.is_empty()
                            && origin.start.bytes().all(|byte| byte.is_ascii_digit())
                            && origin.thread == evidence.thread
                            && origin.turn == *native_turn
                            && !native_turn.is_empty()
                    })
                    || call.id != digest(&(token, key))
                    || call.fingerprint != digest(&(&call.name, &call.input, &call.report))
                {
                    continue;
                }
                let (expected, _) = gateway_calls(
                    &call.id,
                    &json!({"name":call.name,"input":call.input,"report":call.report}).to_string(),
                    &turn.tools,
                )?;
                let [expected] = expected.as_slice() else {
                    return Err(Error("mcp_call_changed"));
                };
                let expected = expected.output_item()?;
                if [
                    "type",
                    "call_id",
                    "name",
                    "namespace",
                    "arguments",
                    "input",
                    "execution",
                ]
                .iter()
                .any(|key| expected[*key] != raw[*key])
                {
                    continue;
                }
                let saved = call.result.as_ref().unwrap();
                let rejected_before_offer = same_origin(call.process_origin.as_ref(), native_turn)
                    && !call.offered
                    && call.shared_reserved != Some(true)
                    && saved["executed"] == false
                    && result["output"]
                        .as_str()
                        .and_then(|text| serde_json::from_str::<Value>(text).ok())
                        .is_some_and(|output| output == *saved);
                let closed_non_process = call.offered
                    && native_effect::process_tool(&call.name).is_none()
                    && match call.effect.as_ref() {
                        Some(Effect::Read) => call.shared_reserved == Some(false),
                        Some(effect @ Effect::Paths(_)) => effect.claim_scope() == effect,
                        _ => false,
                    }
                    && saved
                        .get("output")
                        .is_some_and(|output| *output == result["output"])
                    && saved.get("call_id").is_none_or(|id| id == &call.id)
                    && saved.get("id").is_none_or(|id| id == &call.id)
                    && saved.get("name").is_none_or(|name| name == &call.name);
                if rejected_before_offer || closed_non_process {
                    receipts.non_commands.insert(call.id.clone());
                }
                let original_hooks = call.process_hooks_absent == Some(true)
                    || matches!(call.effect.as_ref(),Some(effect @ Effect::Paths(_))
                        if effect.claim_scope() == effect)
                    // With a real workspace, write_stdin is an unreserved Read
                    // only after original qualified completion-poll admission.
                    || native_effect::process_tool(&call.name) == Some("write_stdin")
                        && call.shared_reserved == Some(false);
                if call.offered
                    && original_hooks
                    && native_effect::process_tool(&call.name).is_some()
                    && saved
                        .get("output")
                        .is_some_and(|output| *output == result["output"])
                    && saved.get("call_id").is_none_or(|value| value == &call.id)
                    && saved.get("id").is_none_or(|value| value == &call.id)
                {
                    receipts.legacy_processes.insert(call.id.clone());
                }
            }
            Ok(())
        };
        let mut observed = BTreeSet::new();
        for (root, last) in roots {
            let mut current = (*root).to_owned();
            let mut order = None;
            let mut lane = None;
            let mut seen = BTreeSet::new();
            loop {
                if !seen.insert(current.clone()) {
                    return Err(Error("mcp_source_discovery_invalid"));
                }
                let turn: Turn = self
                    .journal
                    .read(&digest(&("mcp-turn-v1", &current)))?
                    .ok_or(Error("mcp_turn_unavailable"))?;
                if order.is_some_and(|previous| previous >= turn.order)
                    || lane.as_ref().is_some_and(|lane| lane != &turn.lane)
                    || current != *last
                        && (!turn.retired || !turn.source_bound || turn.discovery.is_none())
                    || current == *last && turn.discovery.is_some()
                {
                    return Err(Error("mcp_source_discovery_invalid"));
                }
                inspect(&current, &turn)?;
                observed.insert(current.clone());
                if current == *last {
                    break;
                }
                order = Some(turn.order);
                lane = Some(turn.lane);
                current = turn
                    .discovery
                    .ok_or(Error("mcp_source_discovery_invalid"))?;
            }
        }
        for (token, turn) in &registry.turns {
            if lanes.contains(&turn.lane.as_str()) && !observed.contains(token) {
                inspect(token, turn)?;
            }
        }
        for (resource_id, receipt) in &state.external {
            let Some(lane) = receipt.lane.as_deref().filter(|lane| lanes.contains(lane)) else {
                continue;
            };
            if !receipt.settled
                || receipt.process_hooks_absent != Some(true)
                || receipt.workspace.as_deref() != Some(evidence.workspace.as_path())
            {
                continue;
            }
            let Some(completion) = receipt.completion.as_ref() else {
                continue;
            };
            let Some(id) = completion.source_call_id() else {
                continue;
            };
            let Some((turn, raw, result)) = originals.get(id) else {
                continue;
            };
            if *resource_id != external_effect_id(lane, id)
                || !same_origin(receipt.process_origin.as_ref(), turn)
                || completion.fingerprint != digest(result)
                || raw["type"] != "function_call"
                || !native_effect::default_namespace(&raw["namespace"])
                || raw["name"].as_str() != native_effect::process_tool(&receipt.name)
                || !raw["arguments"]
                    .as_str()
                    .is_some_and(|input| receipt.fingerprint == digest(&(&receipt.name, input)))
            {
                continue;
            }
            receipts.legacy_processes.insert(id.to_owned());
        }
        // Already-offered rejection strings and legacy external not_spawned
        // booleans do not prove that backend startup caused no effect.
        let shared = SharedNativeGuard::shared_status(
            self.shared_guard_home().as_deref(),
            lanes,
            &BTreeMap::new(),
        )
        .map_err(Self::shared_guard_error)?;
        if shared["available"] != true
            || shared["truncated"] != false
            || !shared["active"]
                .as_array()
                .is_some_and(|rows| rows.iter().all(|row| row["owned"] == false))
        {
            return Err(Error("native_unloaded_effects_unsettled"));
        }
        Ok(receipts)
    }

    pub(crate) fn has_unsettled_stopped_effects(
        &self,
        lanes: &[&str],
        tokens: &[&str],
    ) -> Result<bool, Error> {
        let state: ActiveEffects = self
            .journal
            .read(&digest(&"native-effect-claims-v1"))?
            .unwrap_or_default();
        if state
            .claims
            .values()
            .any(|claim| lanes.contains(&claim.lane.as_str()) && claim.session_id.is_none())
            || state.external.values().any(|receipt| {
                !receipt.settled
                    && receipt.continuation.is_none()
                    && receipt.lane.as_deref().is_some_and(|l| lanes.contains(&l))
            })
        {
            return Ok(true);
        }
        for token in tokens {
            if let Some(turn) = self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", token)))?
            {
                if lanes.contains(&turn.lane.as_str())
                    && turn.calls.values().any(|call| {
                        call.offered
                            && call.result.is_none()
                            && !call.terminal_completion.as_ref().is_some_and(|proof| {
                                native_effect::process_tool(&call.name) == Some("exec_command")
                                    && proof.belongs_to(&call.id, None)
                                    && call.process_origin.is_some()
                            })
                    })
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    /// Called only after the authoritative Native thread and any owned host
    /// lane have confirmed Stop (idle/systemError, no owned background
    /// terminals) and `committed_native_items` has been read from
    /// `thread/items/list`. Reconcile only offered calls and non-session claims
    /// Stop fences future dispatch; only durable never-offered state, an
    /// original result, or an admitted positive terminal witness can settle
    /// its effects. Missing history/item IDs are deliberately not an input.
    pub(crate) async fn reconcile_stopped_receipts(
        &self,
        lanes: &[&str],
        tokens: &[&str],
        results: &BTreeMap<String, Vec<(String, Value)>>,
        evidence: Option<&crate::operator_native::CompletionEvidence>,
    ) -> Result<BTreeSet<String>, Error> {
        let mut registry = self.registry.lock().await;
        let key = digest(&"native-effect-claims-v1");
        let state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let mut candidates: BTreeSet<String> = tokens.iter().map(|token| (*token).into()).collect();
        candidates.extend(
            registry
                .turns
                .iter()
                .filter(|(_, turn)| lanes.contains(&turn.lane.as_str()))
                .map(|(token, _)| token.clone()),
        );
        candidates.extend(
            state
                .claims
                .values()
                .filter(|claim| lanes.contains(&claim.lane.as_str()))
                .filter_map(|claim| claim.turn_token.clone()),
        );
        let mut settled = BTreeSet::new();
        for token in candidates {
            let Some(mut turn) = registry.turns.get(&token).cloned().or(self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", &token)))?)
            else {
                continue;
            };
            if !lanes.contains(&turn.lane.as_str()) {
                continue;
            }
            if let Some(results) = results.get(&token) {
                for (id, result) in results {
                    let call = turn
                        .calls
                        .values_mut()
                        .find(|call| call.id == *id && call.offered)
                        .ok_or(Error("mcp_result_invalid"))?;
                    if result["call_id"].as_str() != Some(id.as_str()) {
                        return Err(Error("mcp_result_invalid"));
                    }
                    if let Some(previous) = &call.result {
                        if previous != result {
                            return Err(Error("native_effect_result_changed"));
                        }
                    } else {
                        call.result = Some(result.clone());
                    }
                }
            }
            if let Some(evidence) = evidence {
                let accepted: Vec<_> = turn
                    .calls
                    .values()
                    .filter(|call| {
                        call.offered
                            && native_effect::process_tool(&call.name) == Some("exec_command")
                            && call.matches_native_call(&turn, evidence)
                    })
                    .filter_map(|call| {
                        let witness = evidence.witnesses.get(&call.id)?;
                        let crate::native_claims::completion::Presence::Present(proof) =
                            &witness.proof
                        else {
                            return None;
                        };
                        let session = call
                            .result
                            .as_ref()
                            .and_then(|result| native_process_result(result).0);
                        (call
                            .process_origin
                            .as_ref()
                            .is_some_and(|origin| origin.turn == witness.turn_id)
                            && proof.belongs_to(&call.id, session))
                        .then(|| (call.id.clone(), proof.clone()))
                    })
                    .collect();
                for (id, proof) in accepted {
                    let call = turn.calls.values_mut().find(|call| call.id == id).unwrap();
                    if call
                        .terminal_completion
                        .as_ref()
                        .is_some_and(|old| old != &proof)
                    {
                        return Err(Error("native_effect_result_changed"));
                    }
                    call.terminal_completion = Some(proof);
                }
            }
            turn.retire(true);
            for call in turn
                .calls
                .values_mut()
                .filter(|call| !call.offered && call.result.is_none())
            {
                call.result = Some(json!({"error":"mcp_turn_stopped","executed":false}));
            }
            // The original result/supplement is durable before any physical
            // reservation is released, including recovery after a crash.
            self.persist(&token, &turn)?;
            if registry.turns.contains_key(&token) {
                registry.turns.insert(token.clone(), turn.clone());
            }
            for (logical, call) in &turn.calls {
                if call.id != digest(&(&token, logical))
                    || call.fingerprint != digest(&(&call.name, &call.input, &call.report))
                {
                    return Err(Error("native_effect_identity_changed"));
                }
                if !call.offered {
                    if let Some(workspace) = turn
                        .workspace
                        .as_deref()
                        .filter(|_| call.requires_shared_settlement())
                    {
                        SharedNativeGuard::abort_unoffered(
                            workspace,
                            self.shared_guard_home().as_deref(),
                            &turn.lane,
                            &call.id,
                            &call.name,
                            &call.input,
                        )
                        .map_err(Self::shared_guard_error)?;
                    }
                    settled.insert(call.id.clone());
                    continue;
                }
                let terminal = call
                    .terminal_completion
                    .as_ref()
                    .filter(|proof| {
                        native_effect::process_tool(&call.name) == Some("exec_command")
                            && proof.belongs_to(&call.id, None)
                            && call.process_origin.is_some()
                    })
                    .map(|proof| {
                        json!({"output":format!(
                        "Native completion: {}\nOutput:\n",serde_json::to_string(proof).unwrap())})
                    });
                let Some(result) = terminal.as_ref().or(call.result.as_ref()) else {
                    continue;
                };
                // Also reconciles an original shared reservation whose local
                // mirror was already committed before a crash.
                if let Some(workspace) = turn
                    .workspace
                    .as_deref()
                    .filter(|_| call.requires_shared_settlement())
                {
                    SharedNativeGuard::settle_external(
                        workspace,
                        self.shared_guard_home().as_deref(),
                        &turn.lane,
                        &call.id,
                        &call.name,
                        &call.input,
                        result,
                    )
                    .map_err(Self::shared_guard_error)?;
                }
                if let Some(session) = call.completed_session.filter(|_| {
                    call.process_origin.is_some()
                        && call
                            .completed_legacy_exit
                            .is_some_and(crate::native_claims::confirmed_exit_code)
                }) {
                    if let Some(workspace) = turn
                        .workspace
                        .as_deref()
                        .filter(|_| call.requires_shared_settlement())
                    {
                        SharedNativeGuard::release_observed_processes(
                            workspace,
                            self.shared_guard_home().as_deref(),
                            &turn.lane,
                            &BTreeMap::from([(call.id.clone(), session)]),
                        )
                        .map_err(Self::shared_guard_error)?;
                    }
                }
                let observation = crate::native_claims::process_observation(
                    result,
                    &call.id,
                    continued_process(&call.name, &call.input),
                );
                if native_effect::process_tool(&call.name).is_none()
                    || observation.terminal
                    || observation.not_spawned
                    || call
                        .completed_legacy_exit
                        .is_some_and(crate::native_claims::confirmed_exit_code)
                {
                    settled.insert(call.id.clone());
                }
            }
        }
        drop(registry);
        self.reconcile_persisted_integrated_receipts()?;
        let state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        for (id, receipt) in &state.external {
            let Some(lane) = receipt.lane.as_deref().filter(|lane| lanes.contains(lane)) else {
                continue;
            };
            if !receipt.settled || state.claims.contains_key(id) {
                continue;
            }
            let original = receipt
                .terminal_completion
                .as_ref()
                .map(|proof| proof.call_id.as_str())
                .or_else(|| {
                    receipt
                        .completion
                        .as_ref()
                        .and_then(ExternalCompletion::source_call_id)
                });
            let Some(original) =
                original.filter(|original| external_effect_id(lane, original) == *id)
            else {
                continue;
            };
            let proven = receipt
                .terminal_completion
                .as_ref()
                .is_some_and(|proof| proof.valid())
                || receipt
                    .completion
                    .as_ref()
                    .is_some_and(|result| result.confirmed_terminal())
                || native_effect::process_tool(&receipt.name).is_none();
            if proven {
                settled.insert(original.to_owned());
            }
        }
        self.changed.notify_waiters();
        Ok(settled)
    }

    #[cfg(test)]
    async fn reconcile_stopped_unexecuted(
        &self,
        lanes: &[&str],
        tokens: &[&str],
        _absent_items: &BTreeSet<String>,
    ) -> Result<(), Error> {
        self.reconcile_stopped_receipts(lanes, tokens, &BTreeMap::new(), None)
            .await
            .map(|_| ())
    }

    #[cfg(test)]
    async fn settle_stopped_processes(&self, _lane: &str, _workspace: &Path) -> Result<(), Error> {
        // Legacy empty-jobs callers supply no process termination evidence.
        self.reconcile_persisted_integrated_receipts()
    }
    pub(crate) fn active_claim_ids(&self, lanes: &[&str]) -> Result<BTreeSet<String>, Error> {
        let state: ActiveEffects = self
            .journal
            .read(&digest(&"native-effect-claims-v1"))?
            .unwrap_or_default();
        Ok(state
            .claims
            .iter()
            .filter(|(_, claim)| lanes.contains(&claim.lane.as_str()))
            .map(|(id, _)| id.clone())
            .collect())
    }
    pub(crate) fn has_pending_processes(&self, lanes: &[&str]) -> Result<bool, Error> {
        let state: ActiveEffects = self
            .journal
            .read(&digest(&"native-effect-claims-v1"))?
            .unwrap_or_default();
        for claim in state
            .claims
            .values()
            .filter(|claim| lanes.contains(&claim.lane.as_str()))
        {
            if claim.session_id.is_some() {
                return Ok(true);
            }
            if let Some(token) = claim.turn_token.as_deref() {
                if self
                    .journal
                    .read::<Turn>(&digest(&("mcp-turn-v1", token)))?
                    .is_some_and(|turn| turn.retired)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    #[cfg(test)]
    pub(crate) async fn settle_native_processes(
        &self,
        lanes: &[&str],
        workspace: &Path,
        history: &[Value],
    ) -> Result<(), Error> {
        self.observe_native_processes(lanes, workspace, history, None)
            .await
    }

    /// Positive evidence is read from the same verified Native Owner as this
    /// work. A new-protocol null/invalid witness never falls back to a legacy
    /// UI exit code. Original function results are immutable throughout.
    pub(crate) async fn settle_native_processes_with_evidence(
        &self,
        lanes: &[&str],
        workspace: &Path,
        history: &[Value],
        evidence: &crate::operator_native::CompletionEvidence,
    ) -> Result<(), Error> {
        self.observe_native_processes(lanes, workspace, history, Some(evidence))
            .await
    }
    async fn observe_native_processes(
        &self,
        lanes: &[&str],
        workspace: &Path,
        history: &[Value],
        evidence: Option<&crate::operator_native::CompletionEvidence>,
    ) -> Result<(), Error> {
        use crate::native_claims::completion::{Presence, Proof};
        let mut registry = self.registry.lock().await;
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let mut ended = BTreeMap::<String, BTreeMap<String, u64>>::new();
        let claims: Vec<_> = state
            .claims
            .iter()
            .map(|(id, claim)| (id.clone(), claim.clone()))
            .collect();
        let mut changed = false;
        for (id, claim) in claims {
            if !lanes.contains(&claim.lane.as_str()) {
                continue;
            }
            let mut recorded_turn = match claim.turn_token.as_ref() {
                Some(token) => registry.turns.get(token).cloned().or(self
                    .journal
                    .read::<Turn>(&digest(&("mcp-turn-v1", token)))?),
                None => None,
            };
            let call = recorded_turn
                .as_ref()
                .filter(|turn| {
                    turn.lane == claim.lane && turn.workspace.as_deref() == Some(workspace)
                })
                .and_then(|turn| {
                    turn.calls
                        .values()
                        .find(|call| call.id == id && call.offered)
                })
                .cloned();
            let external = state.external.get(&id).cloned();
            let (name, source_id, origin, legacy_running) = if let Some(call) = &call {
                let legacy = call.result.as_ref().is_some_and(|result| {
                    matches!(
                        crate::native_claims::completion::result_proof(result),
                        Presence::Absent
                    ) && native_process_result(result) == (claim.session_id, false)
                        && claim.session_id.is_some()
                });
                (
                    call.name.clone(),
                    call.id.clone(),
                    call.process_origin.clone(),
                    legacy,
                )
            } else if let Some(receipt) = &external {
                if receipt.lane.as_deref() != Some(claim.lane.as_str())
                    || receipt.workspace.as_deref() != Some(workspace)
                {
                    continue;
                }
                let source = receipt
                    .completion
                    .as_ref()
                    .and_then(ExternalCompletion::source_call_id)
                    .map(str::to_owned)
                    .or_else(|| {
                        evidence.and_then(|evidence| {
                            evidence
                                .witnesses
                                .keys()
                                .find(|native_id| external_effect_id(&claim.lane, native_id) == id)
                                .cloned()
                        })
                    });
                let Some(source) = source else {
                    continue;
                };
                let legacy = receipt.completion.as_ref().is_some_and(|completion| {
                    completion.native_result.as_ref().is_some_and(|saved| {
                        !saved.completion_protocol && saved.native_completion.is_none()
                    }) && completion.status() == (claim.session_id, false)
                        && claim.session_id.is_some()
                });
                (
                    receipt.name.clone(),
                    source,
                    receipt.process_origin.clone(),
                    legacy,
                )
            } else {
                continue;
            };
            if native_effect::process_tool(&name) != Some("exec_command") {
                continue;
            }
            let witness = evidence.and_then(|evidence| evidence.witnesses.get(&source_id));
            let original_matches = evidence.is_some_and(|evidence| {
                if let (Some(call), Some(turn)) = (&call, &recorded_turn) {
                    call.matches_native_call(turn, evidence)
                } else {
                    external
                        .as_ref()
                        .is_some_and(|receipt| receipt.matches_native_call(&source_id, evidence))
                }
            });
            let typed: Option<Proof> = witness.and_then(|witness| match &witness.proof {
                Presence::Present(proof)
                    if original_matches
                        && origin.as_ref().is_some_and(|origin| {
                            evidence.is_some_and(|evidence| {
                                origin.matches_evidence(evidence, &witness.turn_id)
                            })
                        })
                        && proof.belongs_to(&source_id, claim.session_id)
                        && (claim.session_id.is_none() || proof.session_id == claim.session_id) =>
                {
                    Some(proof.clone())
                }
                _ => None,
            });
            if let Some(proof) = typed {
                if let (Some(token), Some(turn)) =
                    (claim.turn_token.as_ref(), recorded_turn.as_mut())
                {
                    let original = turn.calls.values_mut().find(|call| call.id == id).unwrap();
                    if original
                        .terminal_completion
                        .as_ref()
                        .is_some_and(|old| old != &proof)
                    {
                        return Err(Error("native_effect_result_changed"));
                    }
                    original.terminal_completion = Some(proof);
                    if matches!(claim.effect.claim_scope(), Effect::Paths(_)) {
                        original.completed_session = claim.session_id;
                    }
                    self.persist(token, turn)?;
                    if registry.turns.contains_key(token) {
                        registry.turns.insert(token.clone(), turn.clone());
                    }
                } else if let Some(receipt) = state.external.get_mut(&id) {
                    if receipt
                        .terminal_completion
                        .as_ref()
                        .is_some_and(|old| old != &proof)
                    {
                        return Err(Error("native_effect_result_changed"));
                    }
                    receipt.terminal_completion = Some(proof);
                    receipt.settled = false;
                    if matches!(claim.effect.claim_scope(), Effect::Paths(_)) {
                        receipt.completed_session = claim.session_id;
                    }
                    changed = true;
                }
                continue;
            }
            // Only an original legacy running result can use the established
            // qualified legacy item path. A new witness, even null, blocks it.
            let Some(session) = claim.session_id.filter(|_| {
                legacy_running
                    && witness.is_none()
                    && (claim.process_hooks_absent
                        || matches!(claim.effect.claim_scope(), Effect::Paths(_)))
                    && (evidence.is_none() || original_matches && origin.is_some())
            }) else {
                continue;
            };
            let row = history.iter().find(|row| {
                let item = &row["item"];
                item["type"] == "commandExecution"
                    && item["id"] == source_id
                    && item["processId"]
                        .as_str()
                        .and_then(|id| id.parse::<u64>().ok())
                        == Some(session)
                    && matches!(item["status"].as_str(), Some("completed" | "failed"))
                    && item["exitCode"]
                        .as_i64()
                        .is_some_and(crate::native_claims::confirmed_exit_code)
                    && origin.as_ref().is_none_or(|origin| {
                        row["turn_id"] == origin.turn
                            && evidence.is_none_or(|evidence| {
                                origin.matches_evidence(evidence, &origin.turn)
                            })
                    })
            });
            let Some(row) = row else {
                continue;
            };
            ended
                .entry(claim.lane.clone())
                .or_default()
                .insert(id.clone(), session);
            if matches!(claim.effect.claim_scope(), Effect::Paths(_)) {
                if let (Some(token), Some(turn)) =
                    (claim.turn_token.as_ref(), recorded_turn.as_mut())
                {
                    let original = turn.calls.values_mut().find(|call| call.id == id).unwrap();
                    if original
                        .process_origin
                        .as_ref()
                        .is_some_and(|origin| row["turn_id"] == origin.turn)
                    {
                        original.completed_session = Some(session);
                        original.completed_legacy_exit = row["item"]["exitCode"].as_i64();
                        self.persist(token, turn)?;
                        if registry.turns.contains_key(token) {
                            registry.turns.insert(token.clone(), turn.clone());
                        }
                    }
                } else if let Some(receipt) = state.external.get_mut(&id) {
                    if receipt
                        .process_origin
                        .as_ref()
                        .is_some_and(|origin| row["turn_id"] == origin.turn)
                    {
                        receipt.completed_session = Some(session);
                        receipt.completed_legacy_exit = row["item"]["exitCode"].as_i64();
                        changed = true;
                    }
                }
            }
        }
        // Commit existing receipt supplements before releasing shared claims.
        if changed {
            self.journal.write(&key, &state)?;
        }
        for (lane, processes) in &ended {
            SharedNativeGuard::release_observed_processes(
                workspace,
                self.shared_guard_home().as_deref(),
                lane,
                processes,
            )
            .map_err(Self::shared_guard_error)?;
        }
        if !ended.is_empty() {
            state.claims.retain(|id, claim| {
                !ended
                    .get(&claim.lane)
                    .is_some_and(|set| set.contains_key(id))
            });
            self.journal.write(&key, &state)?;
        }
        drop(registry);
        self.reconcile_persisted_integrated_receipts()?;
        self.changed.notify_waiters();
        Ok(())
    }
    /// Provider-owned AppHost tools also execute in upstream Native, but do
    /// not use MCP tool turns. Reserve before forwarding their actual Tool
    /// event; release only on the *next* verified native result. No re-exec on
    /// repeated/uncertain delivery IDs.
    #[cfg(test)]
    pub(crate) async fn admit_external_tool(
        &self,
        lane: &str,
        id: &str,
        name: &str,
        input: &str,
        workspace: &Path,
    ) -> Result<(), Error> {
        // This test helper models a Native patch with an explicit hook proof.
        // Missing-metadata tests call the production method with None directly.
        let facts = matches!(
            name,
            "apply_patch"
                | "functions.apply_patch"
                | "exec_command"
                | "functions.exec_command"
                | "write_stdin"
                | "functions.write_stdin"
        )
        .then(|| native_effect::hook_fixture(workspace));
        self.admit_external_tool_with_execution(lane, id, name, input, workspace, facts.as_ref())
            .await
    }

    pub(crate) async fn admit_external_native_tool(
        &self,
        lane: &str,
        id: &str,
        tool: &Tool,
        input: &str,
        workspace: &Path,
        execution: Option<&native_effect::Execution>,
    ) -> Result<(), Error> {
        let name = tool.key();
        self.admit_external_tool_with_definition(
            lane,
            id,
            (&name, Some(tool)),
            input,
            workspace,
            execution,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn admit_external_tool_with_execution(
        &self,
        lane: &str,
        id: &str,
        name: &str,
        input: &str,
        workspace: &Path,
        execution: Option<&native_effect::Execution>,
    ) -> Result<(), Error> {
        self.admit_external_tool_with_definition(
            lane,
            id,
            (name, None),
            input,
            workspace,
            execution,
        )
        .await
    }

    async fn admit_external_tool_with_definition(
        &self,
        lane: &str,
        id: &str,
        tool: (&str, Option<&Tool>),
        input: &str,
        workspace: &Path,
        execution: Option<&native_effect::Execution>,
    ) -> Result<(), Error> {
        let (name, definition) = tool;
        let registry = self.registry.lock().await;
        self.reconcile_persisted_integrated_receipts()?;
        if self.closed.load(Ordering::Acquire) {
            return Err(Error("mcp_turn_unavailable"));
        }
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let resource_id = external_effect_id(lane, id);
        let fingerprint = digest(&(name, input));
        if let Some(previous) = state.external.get(&resource_id) {
            return if previous.fingerprint == fingerprint {
                Err(Error("native_effect_delivery_unconfirmed"))
            } else {
                Err(Error("native_effect_identity_changed"))
            };
        }
        if state.external.len() >= 4096 {
            return Err(Error("native_effect_receipt_limit"));
        }
        let root =
            std::fs::canonicalize(workspace).map_err(|_| Error("native_workspace_unavailable"))?;
        let reason = match definition {
            Some(tool) => native_effect::tool_dispatch_error(tool, Some(&root), execution),
            None => native_effect::dispatch_error(name, Some(&root), execution),
        };
        if let Some(reason) = reason {
            return Err(Error(reason));
        }
        let inherited_poll =
            native_effect::qualified_session_poll(name, input, Some(&root), execution).and_then(
                |session| {
                    state.session_poll_scope(lane, session).or_else(|| {
                        completed_session_poll(&registry, &state, lane, session, execution?)
                            .then_some(Effect::Read)
                    })
                },
            );
        let effect = inherited_poll.clone().unwrap_or_else(|| {
            native_effect::classify_with_execution(name, input, Some(&root), execution)
        });
        let read_only = effect == Effect::Read;
        let continuation = continued_process(name, input);
        let continuation_hooks_absent =
            native_effect::qualified_process_hooks(name, Some(&root), execution);
        if state.claims.values().any(|held| {
            native_effect::conflicts(&effect, held.effect.claim_scope())
                && !(continuation.is_some() && held.lane == lane && held.session_id == continuation)
        }) {
            return Err(Error("native_resource_busy"));
        }
        if inherited_poll.is_none() {
            native_effect::can_offer(&effect, &root).map_err(Error)?;
        }
        // There is no authoritative external receipt for this identity yet:
        // a previous reservation could have crashed before delivery. Do not
        // mistake that known unoffered claim for a live Native invocation.
        if !read_only {
            SharedNativeGuard::reconcile_unoffered(
                &root,
                self.shared_guard_home().as_deref(),
                lane,
                &resource_id,
                name,
                input,
            )
            .map_err(Self::shared_guard_error)?;
            SharedNativeGuard::offer_external_with_continuation(
                &root,
                self.shared_guard_home().as_deref(),
                lane,
                &resource_id,
                name,
                input,
                crate::native_claims::Admission {
                    effect: effect.clone(),
                    hooks_absent: continuation_hooks_absent,
                },
            )
            .map_err(Self::shared_guard_error)?;
        }
        let continuing_owned = continuation.is_some_and(|session| {
            state
                .claims
                .values()
                .any(|held| held.lane == lane && held.session_id == Some(session))
        });
        if continuing_owned && inherited_poll.is_none() && effect != Effect::Read {
            for held in state
                .claims
                .values_mut()
                .filter(|held| held.lane == lane && held.session_id == continuation)
            {
                held.process_hooks_absent |= matches!(held.effect.claim_scope(), Effect::Paths(_));
                held.effect = Effect::Opaque;
            }
            state.claims.insert(
                resource_id.clone(),
                ClaimedEffect {
                    lane: lane.into(),
                    effect: Effect::Opaque,
                    session_id: None,
                    turn_token: None,
                    continuations: BTreeMap::new(),
                    process_hooks_absent: continuation_hooks_absent,
                    continuation_effect: Some(crate::native_claims::ContinuationEffect {
                        session: continuation.unwrap(),
                        hooks_absent: continuation_hooks_absent,
                    }),
                },
            );
        } else if effect != Effect::Read && !continuing_owned {
            state.claims.insert(
                resource_id.clone(),
                ClaimedEffect {
                    lane: lane.into(),
                    effect,
                    session_id: None,
                    turn_token: None,
                    continuations: BTreeMap::new(),
                    continuation_effect: None,
                    process_hooks_absent: continuation_hooks_absent,
                },
            );
        }
        state.external.insert(
            resource_id.clone(),
            ExternalEffectReceipt {
                fingerprint,
                name: name.into(),
                continuation,
                read_only,
                settled: false,
                workspace: Some(root.clone()),
                lane: Some(lane.into()),
                completion: None,
                process_origin: execution.map(NativeProcessOrigin::from_execution),
                process_hooks_absent: Some(continuation_hooks_absent),
                completed_session: None,
                terminal_completion: None,
                completed_legacy_exit: None,
            },
        );
        if let Err(error) = self.journal.write(&key, &state) {
            // No upstream tool is forwarded until this durable receipt
            // commits; release the reservation when that commit fails.
            if !read_only {
                SharedNativeGuard::abort_unoffered(
                    &root,
                    self.shared_guard_home().as_deref(),
                    lane,
                    &resource_id,
                    name,
                    input,
                )
                .map_err(Self::shared_guard_error)?;
            }
            return Err(error);
        }
        Ok(())
    }

    /// Canonical upstream tool output is the only ordinary settlement proof.
    /// Stop, a lost Chat, provider restart and an idle process are not proof.
    pub(crate) async fn settle_external_tools(
        &self,
        lane: &str,
        results: &[(String, Value)],
    ) -> Result<(), Error> {
        if results.is_empty() {
            return Ok(());
        }
        let _registry = self.registry.lock().await;
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let mut changed = false;
        // Save a minimal result proof before touching the process-shared
        // lease. A failed commit cannot free a writer that may be running.
        for (id, result) in results {
            let effect_id = external_effect_id(lane, id);
            if let Some(receipt) = state.external.get_mut(&effect_id) {
                // This trusted original lane and delivery ID also identifies
                // receipts created before durable lane ownership was recorded.
                // Never guess a lane from an arbitrary orphaned journal entry.
                if receipt.lane.is_none() {
                    receipt.lane = Some(lane.to_owned());
                    changed = true;
                }
                let completion =
                    ExternalCompletion::from_result_for(id, receipt.continuation, result);
                if receipt
                    .completion
                    .as_ref()
                    .is_some_and(|old| old.fingerprint != completion.fingerprint)
                {
                    return Err(Error("native_effect_result_changed"));
                }
                if let Some(old) = receipt.completion.as_mut() {
                    if old.native_result.is_none() && completion.native_result.is_some() {
                        old.native_result = completion.native_result;
                        receipt.settled = false;
                        changed = true;
                    }
                } else {
                    receipt.completion = Some(completion);
                    receipt.settled = false;
                    changed = true;
                }
            }
        }
        if changed {
            self.journal.write(&key, &state)?;
        }
        self.reconcile_persisted_integrated_receipts()?;
        if changed {
            self.changed.notify_waiters();
        }
        Ok(())
    }
    /// An Integrated MCP receipt can commit immediately before the Provider process dies.
    /// Reconcile only claims whose original offered call and result are both
    /// durable. Never infer process exit from a work record or a departed PID.
    fn reconcile_persisted_integrated_receipts(&self) -> Result<(), Error> {
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let before = serde_json::to_vec(&state).map_err(|_| Error("invalid_lane_record"))?;
        let linked: Vec<_> = state
            .claims
            .iter()
            .filter_map(|(id, claim)| {
                claim
                    .turn_token
                    .as_ref()
                    .map(|token| (id.clone(), token.clone(), claim.lane.clone()))
            })
            .collect();
        for (id, token, lane) in linked {
            let Some(turn) = self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", &token)))?
            else {
                continue;
            };
            if turn.lane != lane {
                return Err(Error("native_effect_identity_changed"));
            }
            let Some(call) = turn
                .calls
                .values()
                .find(|call| call.id == id && call.offered)
            else {
                continue;
            };
            let terminal = call.terminal_completion.as_ref().filter(|proof|
                native_effect::process_tool(&call.name) == Some("exec_command") && proof.belongs_to(&call.id, None)
                && call.process_origin.is_some()).map(|proof|json!({
                    "output":format!("Native completion: {}\nOutput:\n",serde_json::to_string(proof).unwrap())}));
            let Some(result) = terminal.as_ref().or(call.result.as_ref()) else {
                continue;
            };
            if let Some(workspace) = turn
                .workspace
                .as_deref()
                .filter(|_| call.requires_shared_settlement())
            {
                SharedNativeGuard::settle_external(
                    workspace,
                    self.shared_guard_home().as_deref(),
                    &lane,
                    &id,
                    &call.name,
                    &call.input,
                    result,
                )
                .map_err(Self::shared_guard_error)?;
            }
            settle_turn_effect(&mut state, &turn, call, result);
        }
        // A running command may exit in a later write_stdin call. A crash
        // after that call's result journal commit but before lease settlement
        // must not leave its original process claim permanently stuck.
        let continuations: Vec<_> = state
            .claims
            .iter()
            .flat_map(|(original_id, held)| {
                held.session_id.into_iter().flat_map(move |session| {
                    held.continuations.iter().map(move |(id, token)| {
                        (
                            original_id.clone(),
                            held.lane.clone(),
                            session,
                            id.clone(),
                            token.clone(),
                        )
                    })
                })
            })
            .collect();
        for (original_id, lane, session, id, token) in continuations {
            let Some(turn) = self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", &token)))?
            else {
                continue;
            };
            if turn.lane != lane {
                return Err(Error("native_effect_identity_changed"));
            }
            let Some(call) = turn
                .calls
                .values()
                .find(|call| call.id == id && call.offered)
            else {
                continue;
            };
            if turn.origin != crate::protocol::ToolOrigin::Native
                || native_effect::process_tool(&call.name) != Some("write_stdin")
                || continued_process(&call.name, &call.input) != Some(session)
            {
                return Err(Error("native_effect_identity_changed"));
            }
            let Some(result) = call.result.as_ref() else {
                continue;
            };
            if let Some(workspace) = turn
                .workspace
                .as_deref()
                .filter(|_| call.requires_shared_settlement())
            {
                SharedNativeGuard::settle_external(
                    workspace,
                    self.shared_guard_home().as_deref(),
                    &lane,
                    &id,
                    &call.name,
                    &call.input,
                    result,
                )
                .map_err(Self::shared_guard_error)?;
            }
            settle_effect(&mut state, &id, &lane, &call.name, Some(session), result);
            if let Some(held) = state.claims.get_mut(&original_id) {
                held.continuations.remove(&id);
            }
        }
        let completed: Vec<_> = state
            .external
            .iter()
            .filter_map(|(id, receipt)| {
                if receipt.settled {
                    return None;
                }
                let terminal = receipt.terminal_completion.as_ref().filter(|proof| {
                    native_effect::process_tool(&receipt.name) == Some("exec_command")
                        && proof.valid()
                        && receipt.process_origin.is_some()
                });
                let (source_call_id, result) = if let Some(proof) = terminal {
                    (
                        proof.call_id.clone(),
                        json!({"output":format!(
                    "Native completion: {}\nOutput:\n",serde_json::to_string(proof).unwrap())}),
                    )
                } else {
                    let completion = receipt.completion.as_ref()?;
                    (completion.source_call_id()?.to_owned(), completion.marker())
                };
                Some((id.clone(), receipt.clone(), source_call_id, result))
            })
            .collect();
        for (id, receipt, source_call_id, result) in completed {
            let Some(lane) = receipt.lane.as_deref() else {
                continue;
            };
            if external_effect_id(lane, &source_call_id) != id {
                return Err(Error("native_effect_identity_changed"));
            }
            if let Some(root) = receipt.workspace.as_deref().filter(|_| !receipt.read_only) {
                SharedNativeGuard::settle_external_receipt_for(
                    root,
                    self.shared_guard_home().as_deref(),
                    lane,
                    &id,
                    &receipt.name,
                    receipt.continuation,
                    crate::native_claims::OriginalResult {
                        call_id: &source_call_id,
                        value: &result,
                    },
                )
                .map_err(Self::shared_guard_error)?;
            }
            settle_effect_for(
                &mut state,
                &id,
                lane,
                &receipt.name,
                receipt.continuation,
                &result,
                &source_call_id,
            );
            state.external.get_mut(&id).unwrap().settled = true;
        }
        if before != serde_json::to_vec(&state).map_err(|_| Error("invalid_lane_record"))? {
            self.journal.write(&key, &state)?;
        }
        Ok(())
    }

    pub fn new(journal: Arc<Journal>) -> Arc<Self> {
        Arc::new(Self {
            journal,
            registry: Mutex::new(Registry::default()),
            changed: Notify::new(),
            controls: None,
            closed: AtomicBool::new(false),
        })
    }
    pub fn with_controls(
        journal: Arc<Journal>,
        bridge: Arc<dyn crate::app::Bridge>,
        config: crate::config::Config,
    ) -> Arc<Self> {
        Arc::new(Self {
            journal: journal.clone(),
            registry: Mutex::new(Registry::default()),
            changed: Notify::new(),
            controls: Some(crate::control::Controls {
                bridge,
                journal,
                config,
                mutations: Mutex::new(()),
            }),
            closed: AtomicBool::new(false),
        })
    }
    fn persist(&self, token: &str, turn: &Turn) -> Result<(), Error> {
        self.journal.write(&digest(&("mcp-turn-v1", token)), turn)
    }
    pub(crate) fn require_native_origin(&self, token: &str) -> Result<(), Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.origin != crate::protocol::ToolOrigin::Native {
            return Err(Error("mcp_tool_origin_changed"));
        }
        Ok(())
    }

    pub(crate) fn validate_native_offers(
        &self,
        token: &str,
        offered: &[Value],
    ) -> Result<(), Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.origin != crate::protocol::ToolOrigin::Native {
            return Err(Error("mcp_tool_origin_changed"));
        }
        for item in offered {
            let call = turn
                .calls
                .values()
                .find(|call| item["id"] == call.id)
                .ok_or(Error("mcp_call_changed"))?;
            let original =
                json!({"id":call.id,"name":call.name,"input":call.input,"report":call.report});
            if *item != original || !call.offered || call.result.is_some() {
                return Err(Error("mcp_call_changed"));
            }
            let tool = turn.tools.get(&call.name).ok_or(Error("mcp_call_changed"))?;
            if let Some(reason) = native_effect::tool_dispatch_error(
                tool,
                turn.workspace.as_deref(),
                turn.execution.as_ref(),
            ) {
                return Err(Error(reason));
            }
        }
        Ok(())
    }

    pub async fn register(
        &self,
        lane: &str,
        tools: BTreeMap<String, Tool>,
        source_bound: bool,
    ) -> Result<String, Error> {
        self.register_with_origin(
            lane,
            tools,
            source_bound,
            None,
            crate::protocol::ToolOrigin::Generic,
        )
        .await
    }
    /// A Native origin remains Native even if its verified scope is unavailable.
    /// Missing scope permits original reads and real discovery, not new effects.
    pub async fn register_scoped(
        &self,
        lane: &str,
        tools: BTreeMap<String, Tool>,
        source_bound: bool,
        workspace: Option<&Path>,
    ) -> Result<String, Error> {
        self.register_with_origin(
            lane,
            tools,
            source_bound,
            workspace,
            crate::protocol::ToolOrigin::Native,
        )
        .await
    }

    async fn register_with_origin(
        &self,
        lane: &str,
        tools: BTreeMap<String, Tool>,
        source_bound: bool,
        workspace: Option<&Path>,
        origin: crate::protocol::ToolOrigin,
    ) -> Result<String, Error> {
        let workspace = workspace
            .map(|path| {
                std::fs::canonicalize(path).map_err(|_| Error("native_workspace_unavailable"))
            })
            .transpose()?;
        let mut registry = self.registry.lock().await;
        if self.closed.load(Ordering::Acquire) {
            return Err(Error("mcp_turn_unavailable"));
        }
        if registry
            .turns
            .values()
            .any(|v| !v.retired && v.lane == lane)
        {
            return Err(Error("mcp_owner_conflict"));
        }
        let order = registry
            .turns
            .values()
            .map(|v| v.order)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let token = random_id()?;
        registry.reserve_payload(&token)?;
        let turn = Turn {
            lane: lane.into(),
            origin,
            workspace,
            execution: None,
            tools,
            calls: BTreeMap::new(),
            retired: false,
            order,
            discovery: None,
            source_bound,
            catalog_discovered: false,
            context_receipt: None,
            requires_context: false,
        };
        self.persist(&token, &turn)?;
        if source_bound {
            registry.discovery.insert(
                token.clone(),
                DiscoveryAdmission {
                    lane: lane.into(),
                    root: token.clone(),
                    revoked: false,
                },
            );
        }
        registry.turns.insert(token.clone(), turn);
        Ok(token)
    }
    fn source_turn<'a>(&self, registry: &'a Registry, token: &str) -> Result<Cow<'a, Turn>, Error> {
        if let Some(turn) = registry.turns.get(token) {
            return Ok(Cow::Borrowed(turn));
        }
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        if !turn.retired
            || !turn.source_bound
            || turn.calls.values().any(|call| call.result.is_none())
        {
            return Err(Error("mcp_turn_unavailable"));
        }
        Ok(Cow::Owned(turn))
    }
    fn completed_source_session_poll(
        &self,
        registry: &Registry,
        state: &ActiveEffects,
        token: &str,
        lane: &str,
        session: u64,
        execution: &native_effect::Execution,
    ) -> Result<bool, Error> {
        let Some(admission) = registry
            .discovery
            .get(token)
            .filter(|source| !source.revoked && source.lane == lane)
        else {
            return Ok(completed_session_poll(
                registry, state, lane, session, execution,
            ));
        };
        let mut evidence = SessionPollEvidence::new(state, lane, session, execution);
        // Bound temporary memory to the existing payload cache. Stream the
        // admitted chain one immutable Turn at a time; never cache or reactivate
        // a retired token, nor search unrelated journal files for authority.
        let mut cached: BTreeSet<String> = registry.turns.keys().cloned().collect();
        let mut current = admission.root.clone();
        let mut order = None;
        loop {
            let turn = self.source_turn(registry, &current)?;
            if !turn.source_bound
                || turn.lane != lane
                || order.is_some_and(|previous| previous >= turn.order)
                || (current != token
                    && (!turn.retired || turn.calls.values().any(|call| call.result.is_none())))
                || (current == token && (turn.retired || turn.discovery.is_some()))
            {
                return Err(Error("mcp_source_discovery_invalid"));
            }
            evidence.observe_turn(&turn, lane, session, execution);
            cached.remove(&current);
            if current == token {
                break;
            }
            order = Some(turn.order);
            current = turn
                .discovery
                .clone()
                .ok_or(Error("mcp_source_discovery_invalid"))?;
        }
        // Preserve existing hot same-lane/Native-owner evidence, including an
        // original, pending collector or numeric reuse outside this source chain.
        // Only cold disk readback requires the adopted root; never count a
        // cached chain receipt twice.
        for key in cached {
            evidence.observe_turn(&registry.turns[&key], lane, session, execution);
        }
        Ok(evidence.qualifies())
    }

    // Records are not silently reactivated after process/host loss. A live turn
    // capability belongs to this owner generation, not merely to a journal file.
    pub async fn catalog(&self, token: &str) -> Result<BTreeMap<String, Tool>, Error> {
        let registry = self.registry.lock().await;
        let turns = &registry.turns;
        let turn = turns
            .get(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn.tools.clone())
    }
    /// An account/branch-verified saved-Chat adoption can restore its retired
    /// discovery chain. Execution stays retired; a fresh source admission must
    /// still publish the next live lease after validating its saved input.
    pub(crate) async fn restore_source_discovery(
        &self,
        root: &str,
        last: &str,
        lane: &str,
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        if self.closed.load(Ordering::Acquire)
            || registry
                .turns
                .values()
                .any(|turn| turn.lane == lane && !turn.retired)
        {
            return Err(Error("mcp_owner_conflict"));
        }
        let mut token = root.to_owned();
        let mut order = None;
        let last_turn = loop {
            let turn: Turn = self
                .journal
                .read(&digest(&("mcp-turn-v1", &token)))?
                .ok_or(Error("mcp_source_discovery_invalid"))?;
            if !turn.retired
                || !turn.source_bound
                || turn.lane != lane
                || order.is_some_and(|previous| previous >= turn.order)
                || turn.calls.values().any(|call| call.result.is_none())
            {
                return Err(Error("mcp_source_discovery_invalid"));
            }
            order = Some(turn.order);
            if token == last {
                if turn.discovery.is_some() {
                    return Err(Error("mcp_source_discovery_invalid"));
                }
                break turn;
            }
            token = turn
                .discovery
                .ok_or(Error("mcp_source_discovery_invalid"))?;
        };
        registry.reserve_payload(last)?;
        registry.turns.insert(last.into(), last_turn);
        registry.discovery.insert(
            last.into(),
            DiscoveryAdmission {
                lane: lane.into(),
                root: root.into(),
                revoked: false,
            },
        );
        self.changed.notify_waiters();
        Ok(())
    }
    /// Only discovery can cross a completed turn boundary. Old execution
    /// capabilities and their logical-call receipts retain their original scope.
    pub(crate) async fn source_discovery(&self, previous: &str, next: &str) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let current = registry
            .turns
            .get(next)
            .filter(|turn| !turn.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        let old = self.source_turn(&registry, previous)?;
        if !old.retired
            || !old.source_bound
            || !current.source_bound
            || old.order >= current.order
            || old.lane != current.lane
            || old.calls.values().any(|call| call.result.is_none())
        {
            return Err(Error("mcp_source_discovery_invalid"));
        }
        if !registry.source_admitted(previous, &old.lane) {
            return if old.discovery.as_deref() == Some(next)
                && registry.source_admitted(next, &old.lane)
            {
                Ok(())
            } else {
                Err(Error("mcp_source_discovery_invalid"))
            };
        }
        if !registry.source_admitted(next, &old.lane) {
            return Err(Error("mcp_source_discovery_invalid"));
        }
        let root = registry.discovery[previous].root.clone();
        let mut old = old.into_owned();
        old.discovery = Some(next.into());
        self.persist(previous, &old)?;
        registry
            .discovery
            .get_mut(next)
            .expect("admitted successor")
            .root = root;
        registry.discovery.remove(previous);
        if let Some(cached) = registry.turns.get_mut(previous) {
            *cached = old;
        }
        self.changed.notify_waiters();
        Ok(())
    }
    pub(crate) async fn catalog_readback(&self, token: &str) -> Result<Value, Error> {
        tokio::time::timeout(Duration::from_secs(600), async {
            loop {
                // Arm before reading state so admission/Stop cannot be lost
                // between the state read and retaining this MCP observer.
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                {
                    let mut registry = self.registry.lock().await;
                    if self.closed.load(Ordering::Acquire) { return Err(Error("mcp_turn_unavailable")); }
                    let mut current = self.source_turn(&registry, token)?;
                    let lane = current.lane.clone();
                    let mut next = token.to_owned();
                    while current.retired {
                        if !current.source_bound { return Err(Error("mcp_turn_unavailable")); }
                        let Some(discovery) = current.discovery.as_deref() else {
                            if !registry.source_admitted(&next, &lane) {
                                return Err(Error("mcp_turn_unavailable"));
                            }
                            break;
                        };
                        let candidate = self.source_turn(&registry, discovery)?;
                        if candidate.lane != lane || candidate.order <= current.order || !candidate.source_bound {
                            return Err(Error("mcp_turn_unavailable"));
                        }
                        next = discovery.to_owned();
                        current = candidate;
                    }
                    if !current.retired {
                        let mut admitted = current.into_owned();
                        if !admitted.catalog_discovered {
                            admitted.catalog_discovered = true;
                            self.persist(&next, &admitted)?;
                            registry.turns.insert(next.clone(),admitted.clone());
                        }
                        return Ok(if next == token { json!(admitted.tools) } else {
                            json!({"turn_token":next,"tools":admitted.tools,"message":"This saved Chat input is now admitted to its new native Codex turn. Use the returned turn_token for subsequent native_tool calls; old execution tokens remain retired."})
                        });
                    }
                }
                changed.await;
            }
        }).await.map_err(|_| Error("mcp_observer_timeout"))?
    }
    pub(crate) async fn require_source_context(&self, token: &str) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let turn = registry
            .turns
            .get_mut(token)
            .filter(|turn| !turn.retired && turn.source_bound && turn.calls.is_empty())
            .ok_or(Error("source_context_policy_unsettled"))?;
        let mut next = turn.clone();
        next.requires_context = true;
        self.persist(token, &next)?;
        *turn = next;
        Ok(())
    }
    pub(crate) async fn accept_source_context(
        &self,
        token: &str,
        receipt: &str,
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let turn = registry
            .turns
            .get_mut(token)
            .filter(|turn| !turn.retired && turn.source_bound)
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.context_receipt.as_deref() != Some(receipt) {
            let mut next = turn.clone();
            next.context_receipt = Some(receipt.into());
            self.persist(token, &next)?;
            *turn = next;
        }
        Ok(())
    }
    pub(crate) fn source_participation(&self, token: &str) -> Result<Value, Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(
            json!({"state":if turn.catalog_discovered || turn.context_receipt.is_some() || !turn.calls.is_empty() {
            "participated" } else { "execution_not_joined" },
            "catalog_discovered":turn.catalog_discovered,"context_accepted":turn.context_receipt.is_some(),"context_required":turn.requires_context,
            "tool_requests":turn.calls.len(),"call_observations":turn.calls.values().map(|call|
                json!({"id":call.id,"offered":call.offered,"settled":call.result.is_some()})).collect::<Vec<_>>()}),
        )
    }
    pub async fn update(&self, token: &str, tools: BTreeMap<String, Tool>) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let turns = &mut registry.turns;
        let turn = turns
            .get_mut(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.calls.values().any(|v| v.result.is_none()) {
            return Err(Error("mcp_pending_catalog_change"));
        }
        let mut next = turn.clone();
        next.tools = tools;
        self.persist(token, &next)?;
        *turn = next;
        Ok(())
    }

    /// Bind facts from the original Native sampling request after its previous
    /// results have settled. A caller's tool args never supply these facts.
    pub(crate) async fn bind_execution(
        &self,
        token: &str,
        execution: Option<native_effect::Execution>,
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let turn = registry
            .turns
            .get_mut(token)
            .filter(|turn| !turn.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.execution == execution {
            return Ok(());
        }
        if turn.calls.values().any(|call| call.result.is_none()) {
            return Err(Error("mcp_pending_catalog_change"));
        }
        let mut next = turn.clone();
        next.execution = execution;
        self.persist(token, &next)?;
        *turn = next;
        Ok(())
    }
    /// Register before waiting. Cancelling this future only drops the observer.
    pub async fn enqueue(&self, token: &str, args: &Arguments) -> Result<(), Error> {
        args.validate()?;
        let fingerprint = args.fingerprint();
        let mut registry = self.registry.lock().await;
        let turns = &mut registry.turns;
        let turn = turns.get_mut(token).ok_or(Error("mcp_turn_unavailable"))?;
        if let Some(call) = turn.calls.get(&args.call_id) {
            if call.fingerprint != fingerprint {
                return Err(Error("mcp_call_changed"));
            }
            if turn.retired && call.result.is_none() {
                return Err(Error("mcp_turn_unavailable"));
            }
            return Ok(());
        }
        if turn.retired {
            return Err(Error("mcp_turn_unavailable"));
        }
        crate::control::ensure_public_text(&args.report, [token, self.journal.owner.as_str()])?;
        if turn.requires_context && turn.context_receipt.is_none() {
            return Err(Error("source_context_required"));
        }
        if turn.calls.len() >= 4096 {
            return Err(Error("mcp_turn_call_limit"));
        }
        let id = digest(&(token, &args.call_id));
        gateway_calls(
            &id,
            &json!({"name":args.name,"input":args.input,"report":args.report}).to_string(),
            &turn.tools,
        )?;
        let mut next = turn.clone();
        let effect = native_effect::classify_with_execution(
            &args.name,
            &args.input,
            turn.workspace.as_deref(),
            turn.execution.as_ref(),
        );
        next.calls.insert(
            args.call_id.clone(),
            Invocation {
                fingerprint,
                id,
                name: args.name.clone(),
                input: args.input.clone(),
                report: args.report.clone(),
                offered: false,
                shared_reserved: None,
                process_hooks_absent: None,
                result: None,
                native_input: Vec::new(),
                effect: Some(effect),
                process_origin: turn
                    .execution
                    .as_ref()
                    .map(NativeProcessOrigin::from_execution),
                completed_session: None,
                terminal_completion: None,
                completed_legacy_exit: None,
            },
        );
        self.persist(token, &next)?;
        *turn = next;
        self.changed.notify_waiters();
        Ok(())
    }
    pub fn recorded_calls(&self, token: &str) -> Result<Vec<Value>, Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn
            .calls
            .values()
            .map(
                |call| json!({"id":call.id,"offered":call.offered,"settled":call.result.is_some()}),
            )
            .collect())
    }
    /// Derive the original Native call from its admitted invocation and catalog.
    /// This is the same producer used to offer it, not a second call journal.
    pub(crate) fn unreturned_native_calls(
        &self,
        token: &str,
    ) -> Result<BTreeMap<String, Value>, Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        let mut calls = BTreeMap::new();
        for call in turn
            .calls
            .values()
            .filter(|call| call.offered && call.result.is_none())
        {
            let (native, _) = gateway_calls(
                &call.id,
                &json!({"name":call.name,"input":call.input,"report":call.report}).to_string(),
                &turn.tools,
            )?;
            let [native] = native.as_slice() else {
                return Err(Error("mcp_call_changed"));
            };
            if native.id != call.id
                || calls
                    .insert(call.id.clone(), native.output_item()?)
                    .is_some()
            {
                return Err(Error("mcp_call_changed"));
            }
        }
        Ok(calls)
    }
    /// Correlate a public host request with an already admitted invocation.
    /// Rejected/unknown calls are not evidence of another native effect.
    pub(crate) async fn source_calls(
        &self,
        token: &str,
        requests: Vec<(String, Arguments)>,
    ) -> Vec<Value> {
        let registry = self.registry.lock().await;
        let Some(turn) = registry.turns.get(token) else {
            return Vec::new();
        };
        requests.into_iter().filter_map(|(source, args)| {
            turn.calls.get(&args.call_id).filter(|call|
                call.name==args.name && call.input==args.input && call.report==args.report)
                .map(|call|json!({"source_message_id":source,"logical_call_id":args.call_id,"native_call_id":call.id}))
        }).collect()
    }
    pub fn contains_call(&self, token: &str, call_id: &str) -> Result<bool, Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn.calls.contains_key(call_id))
    }
    /// Read only an already committed original result. Missing or pending calls
    /// cannot enqueue, acquire a claim or regain execution through this path.
    pub(crate) fn retained_result(
        &self,
        token: &str,
        args: &Arguments,
    ) -> Result<Option<Value>, Error> {
        args.validate()?;
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        let Some(call) = turn.calls.get(&args.call_id) else {
            return Ok(None);
        };
        if call.fingerprint != args.fingerprint() {
            return Err(Error("mcp_call_changed"));
        }
        Ok(call.original_result())
    }
    pub async fn unsettled(&self, token: &str) -> Result<Vec<String>, Error> {
        let registry = self.registry.lock().await;
        let turns = &registry.turns;
        let turn = turns.get(token).ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn
            .calls
            .values()
            .filter(|v| v.result.is_none())
            .map(|v| v.id.clone())
            .collect())
    }
    pub async fn invoke(&self, token: &str, args: Arguments) -> Result<Value, Error> {
        self.enqueue(token, &args).await?;
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let registry = self.registry.lock().await;
                let turns = &registry.turns;
                let turn = turns.get(token).ok_or(Error("mcp_turn_unavailable"))?;
                let call = turn
                    .calls
                    .get(&args.call_id)
                    .ok_or(Error("mcp_turn_unavailable"))?;
                if let Some(result) = call.original_result() {
                    return Ok(result);
                }
                if turn.retired {
                    return Err(Error("mcp_turn_unavailable"));
                }
            }
            changed.await;
        }
    }
    pub async fn offer(&self, token: &str) -> Result<Vec<Value>, Error> {
        let mut registry = self.registry.lock().await;
        self.reconcile_persisted_integrated_receipts()?;
        let turn = registry
            .turns
            .get(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        let mut next = turn.clone();
        // The reservation journal is committed BEFORE a tool is offered to
        // upstream, and released only AFTER upstream's result is journaled.
        // A crash may leave a conservative unresolved claim, never an
        // unrecorded still-running writer mistaken for a safe new operation.
        let claims_key = digest(&"native-effect-claims-v1");
        let mut claims: ActiveEffects = self.journal.read(&claims_key)?.unwrap_or_default();
        let mut calls = Vec::new();
        let mut changed = false;
        let mut claims_changed = false;
        let mut reserved = Vec::new();
        let admission = (|| -> Result<(), Error> {
            for call in next
                .calls
                .values_mut()
                .filter(|v| !v.offered && v.result.is_none())
            {
                if calls.len() == 32 {
                    break;
                }
                let discovery = next.tools.get(&call.name).is_some_and(|tool| {
                    tool.kind == crate::protocol::ToolKind::Search && tool.key() == "tool_search"
                });
                // This workspace is the existing Core-admitted NativeScope,
                // retained even when its metadata is missing or foreign.
                // Unscoped generic transport keeps its ordinary tool contract.
                let native_origin = next.origin == crate::protocol::ToolOrigin::Native;
                let reason = if !native_origin {
                    None
                } else if call.name == "tool_search" && !discovery {
                    Some("native_tool_identity_unsupported")
                } else if discovery {
                    None
                } else {
                    next.tools.get(&call.name).map_or(
                        Some("native_tool_identity_unsupported"),
                        |tool| {
                            native_effect::tool_dispatch_error(
                                tool,
                                next.workspace.as_deref(),
                                next.execution.as_ref(),
                            )
                        },
                    )
                };
                if let Some(reason) = reason {
                    call.result = Some(json!({"error":reason,"executed":false,
                        "message":"This Native invocation lacks a supported dispatch-bound completion capability. The original call will not execute later; saved result reads remain available."}));
                    call.shared_reserved = Some(false);
                    changed = true;
                    continue;
                }
                let inherited_poll = match (
                    native_effect::qualified_session_poll(
                        &call.name,
                        &call.input,
                        next.workspace.as_deref(),
                        next.execution.as_ref(),
                    ),
                    next.execution.as_ref(),
                ) {
                    (Some(session), Some(execution)) => {
                        if let Some(scope) = claims.session_poll_scope(&next.lane, session) {
                            Some(scope)
                        } else if self.completed_source_session_poll(
                            &registry, &claims, token, &next.lane, session, execution,
                        )? && !calls.iter().any(|offered: &Value| {
                            continued_process(
                                offered["name"].as_str().unwrap_or_default(),
                                offered["input"].as_str().unwrap_or_default(),
                            ) == Some(session)
                        }) {
                            Some(Effect::Read)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                let effect = inherited_poll.clone().unwrap_or_else(|| {
                    call.admission_effect(next.workspace.as_deref(), next.execution.as_ref())
                });
                let continuation = native_origin
                    .then(|| continued_process(&call.name, &call.input))
                    .flatten();
                let continuation_hooks_absent = native_effect::qualified_process_hooks(
                    &call.name,
                    next.workspace.as_deref(),
                    next.execution.as_ref(),
                );
                // The call's authoritative turn still says it was not offered.
                // Repair a crash between a previous shared reservation and turn
                // commit, including its local non-authoritative claim mirror.
                if claims.claims.remove(&call.id).is_some() {
                    claims_changed = true;
                }
                if claims.claims.values().any(|held| {
                    native_effect::conflicts(&effect, held.effect.claim_scope())
                        && !(continuation.is_some()
                            && held.lane == next.lane
                            && held.session_id == continuation)
                }) {
                    // A tool caller has not registered an explicit wait/queue.
                    // Persist a pre-effect conflict so it cannot be dispatched
                    // minutes later after its original observer times out.
                    call.result = Some(busy_effect_reply());
                    changed = true;
                    continue;
                }
                if inherited_poll.is_none() && matches!(&effect, Effect::Paths(_)) {
                    let verified = next
                        .workspace
                        .as_ref()
                        .ok_or(Error("native_workspace_unavailable"))
                        .and_then(|workspace| {
                            native_effect::can_offer(&effect, workspace).map_err(Error)
                        });
                    if let Err(reason) = verified {
                        call.result = Some(json!({"error":reason.0,"executed":false}));
                        changed = true;
                        continue;
                    }
                }
                if let Some(workspace) =
                    next.workspace.as_deref().filter(|_| effect != Effect::Read)
                {
                    SharedNativeGuard::reconcile_unoffered(
                        workspace,
                        self.shared_guard_home().as_deref(),
                        &next.lane,
                        &call.id,
                        &call.name,
                        &call.input,
                    )
                    .map_err(Self::shared_guard_error)?;
                    match SharedNativeGuard::offer_external_with_continuation(
                        workspace,
                        self.shared_guard_home().as_deref(),
                        &next.lane,
                        &call.id,
                        &call.name,
                        &call.input,
                        crate::native_claims::Admission {
                            effect: effect.clone(),
                            hooks_absent: continuation_hooks_absent,
                        },
                    ) {
                        Ok(()) => reserved.push((
                            workspace.to_path_buf(),
                            next.lane.clone(),
                            call.id.clone(),
                            call.name.clone(),
                            call.input.clone(),
                        )),
                        Err(reason) if reason == "native_resource_busy" => {
                            call.result = Some(busy_effect_reply());
                            changed = true;
                            continue;
                        }
                        Err(reason) if reason == "native_resource_version_conflict" => {
                            call.result = Some(json!({"error":reason,"executed":false}));
                            changed = true;
                            continue;
                        }
                        Err(reason) => return Err(Self::shared_guard_error(reason)),
                    }
                }
                let shared_reserved = next.workspace.is_some() && effect != Effect::Read;
                let continuing_owned = continuation.is_some_and(|session| {
                    claims
                        .claims
                        .values()
                        .any(|held| held.lane == next.lane && held.session_id == Some(session))
                });
                if continuing_owned {
                    // Persist which *offered* follow-up owns a process session.
                    // This association is written before forwarding the tool,
                    // including when the follow-up is in a different Native turn.
                    for held in claims.claims.values_mut() {
                        if held.lane == next.lane && held.session_id == continuation {
                            if inherited_poll.is_none() && effect != Effect::Read {
                                held.process_hooks_absent |=
                                    matches!(held.effect.claim_scope(), Effect::Paths(_));
                                held.effect = Effect::Opaque;
                                claims_changed = true;
                            }
                            if let std::collections::btree_map::Entry::Vacant(item) =
                                held.continuations.entry(call.id.clone())
                            {
                                item.insert(token.to_owned());
                                claims_changed = true;
                            }
                        }
                    }
                    if inherited_poll.is_none() && effect != Effect::Read {
                        claims.claims.insert(
                            call.id.clone(),
                            ClaimedEffect {
                                lane: next.lane.clone(),
                                effect: Effect::Opaque,
                                session_id: None,
                                turn_token: Some(token.to_owned()),
                                continuations: BTreeMap::new(),
                                process_hooks_absent: continuation_hooks_absent,
                                continuation_effect: Some(
                                    crate::native_claims::ContinuationEffect {
                                        session: continuation.unwrap(),
                                        hooks_absent: continuation_hooks_absent,
                                    },
                                ),
                            },
                        );
                        claims_changed = true;
                    }
                } else if effect != Effect::Read {
                    claims.claims.insert(
                        call.id.clone(),
                        ClaimedEffect {
                            lane: next.lane.clone(),
                            effect,
                            session_id: None,
                            turn_token: Some(token.to_owned()),
                            continuations: BTreeMap::new(),
                            continuation_effect: None,
                            process_hooks_absent: continuation_hooks_absent,
                        },
                    );
                    claims_changed = true;
                }
                call.shared_reserved = Some(shared_reserved);
                call.process_hooks_absent = Some(continuation_hooks_absent);
                call.offered = true;
                changed = true;
                calls.push(
                    json!({"id":call.id,"name":call.name,"input":call.input,"report":call.report}),
                );
            }
            if claims_changed {
                self.journal.write(&claims_key, &claims)?;
            }
            if changed {
                self.persist(token, &next)?;
            }
            Ok(())
        })();
        if let Err(error) = admission {
            // None of this offer batch has been returned to upstream yet.
            // Compensate only our own freshly reserved resources, not another
            // worker's live reservation or an already-offered prior batch.
            for (workspace, lane, id, name, input) in reserved {
                SharedNativeGuard::abort_unoffered(
                    &workspace,
                    self.shared_guard_home().as_deref(),
                    &lane,
                    &id,
                    &name,
                    &input,
                )
                .map_err(Self::shared_guard_error)?;
            }
            return Err(error);
        }
        if changed {
            registry.turns.insert(token.to_owned(), next);
            self.changed.notify_waiters();
        }
        Ok(calls)
    }
    pub async fn settle(&self, token: &str, results: &[(String, Value)]) -> Result<(), Error> {
        self.settle_with_catalog(token, results, None).await
    }
    pub async fn settle_with_catalog(
        &self,
        token: &str,
        results: &[(String, Value)],
        catalog: Option<BTreeMap<String, Tool>>,
    ) -> Result<(), Error> {
        self.settle_prepared(token, results, catalog, &[], || Ok(()))
            .await
    }
    /// Keep validation, preparation and commit in one admission section. A
    /// known validation rejection must not leave a newly prepared lane intent.
    pub(crate) async fn settle_prepared(
        &self,
        token: &str,
        results: &[(String, Value)],
        catalog: Option<BTreeMap<String, Tool>>,
        native_input: &[Value],
        prepare: impl FnOnce() -> Result<(), Error> + Send,
    ) -> Result<(), Error> {
        self.settle_prepared_scope(token, results, catalog, native_input, None, prepare)
            .await
    }

    pub(crate) async fn settle_prepared_scoped(
        &self,
        token: &str,
        results: &[(String, Value)],
        catalog: Option<BTreeMap<String, Tool>>,
        native_input: &[Value],
        scope: Option<&crate::protocol::NativeScope>,
        prepare: impl FnOnce() -> Result<(), Error> + Send,
    ) -> Result<(), Error> {
        self.settle_prepared_scope(token, results, catalog, native_input, Some(scope), prepare)
            .await
    }

    async fn settle_prepared_scope(
        &self,
        token: &str,
        results: &[(String, Value)],
        catalog: Option<BTreeMap<String, Tool>>,
        native_input: &[Value],
        // Outer None preserves attached/legacy callers' existing binding;
        // Some(None) is an explicitly unscoped MCP sampling request.
        scope: Option<Option<&crate::protocol::NativeScope>>,
        prepare: impl FnOnce() -> Result<(), Error> + Send,
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let turns = &mut registry.turns;
        let turn = turns
            .get_mut(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        let mut next = turn.clone();
        let mut seen = std::collections::BTreeSet::new();
        if !native_input.is_empty() && results.is_empty() {
            return Err(Error("mcp_result_missing"));
        }
        for (index, (id, result)) in results.iter().enumerate() {
            if !seen.insert(id) {
                return Err(Error("mcp_result_invalid"));
            }
            let call = next
                .calls
                .values_mut()
                .find(|v| v.id == *id && v.offered)
                .ok_or(Error("mcp_result_invalid"))?;
            let input = if index == 0 { native_input } else { &[] };
            if let Some(old) = &call.result {
                if old != result || call.native_input != input {
                    return Err(Error("mcp_result_changed"));
                }
            } else {
                call.result = Some(result.clone());
                call.native_input = input.to_vec();
            }
        }
        if let Some(catalog) = catalog {
            if next.tools != catalog && next.calls.values().any(|v| v.result.is_none()) {
                return Err(Error("mcp_pending_catalog_change"));
            }
            next.tools = catalog;
        }
        if let Some(scope) = scope {
            if scope.map(|scope| scope.workspace.as_path()) != next.workspace.as_deref() {
                return Err(Error("mcp_native_scope_changed"));
            }
            let execution = scope.and_then(|scope| scope.execution.clone());
            if next.execution != execution && next.calls.values().any(|call| call.result.is_none())
            {
                return Err(Error("mcp_pending_catalog_change"));
            }
            next.execution = execution;
        }
        // Validate the full batch, then persist its authoritative result BEFORE
        // releasing the OS-shared claim. A failed journal commit must leave the
        // old writer reserved. After a crash between these two operations, the
        // next admission replays only this already-journaled receipt, never its
        // original Native effect.
        prepare()?;
        self.persist(token, &next)?;
        *turn = next;
        for (id, result) in results {
            if let Some(workspace) = turn.workspace.as_deref() {
                let call = turn
                    .calls
                    .values()
                    .find(|v| v.id == *id)
                    .ok_or(Error("mcp_result_invalid"))?;
                // Read the original durable offer fact, not the new sample's
                // classification. This remains correct after commit/restart.
                if call.requires_shared_settlement() {
                    SharedNativeGuard::settle_external(
                        workspace,
                        self.shared_guard_home().as_deref(),
                        &turn.lane,
                        id,
                        &call.name,
                        &call.input,
                        result,
                    )
                    .map_err(Self::shared_guard_error)?;
                }
            }
        }
        if !results.is_empty() {
            let claims_key = digest(&"native-effect-claims-v1");
            let mut claims: ActiveEffects = self.journal.read(&claims_key)?.unwrap_or_default();
            let before = serde_json::to_vec(&claims).map_err(|_| Error("invalid_lane_record"))?;
            for (id, result) in results {
                let call = turn
                    .calls
                    .values()
                    .find(|v| v.id == *id)
                    .ok_or(Error("mcp_result_invalid"))?;
                settle_turn_effect(&mut claims, turn, call, result);
            }
            if before != serde_json::to_vec(&claims).map_err(|_| Error("invalid_lane_record"))? {
                self.journal.write(&claims_key, &claims)?;
            }
        }
        self.changed.notify_waiters();
        Ok(())
    }
    /// End a source response without revoking this development connection.
    /// Calls already offered to Native must settle before the source advances.
    pub(crate) async fn interrupt_source(&self, token: &str) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let turn = registry
            .turns
            .get_mut(token)
            .ok_or(Error("mcp_turn_unavailable"))?;
        if !turn.source_bound {
            return Err(Error("mcp_source_discovery_invalid"));
        }
        if turn
            .calls
            .values()
            .any(|call| call.offered && call.result.is_none())
        {
            return Err(Error("mcp_pending_completion"));
        }
        let mut next = turn.clone();
        for call in next.calls.values_mut().filter(|call| call.result.is_none()) {
            call.result = Some(json!({"error":"source_response_interrupted","executed":false}));
        }
        next.retire(false);
        self.persist(token, &next)?;
        *turn = next;
        self.changed.notify_waiters();
        Ok(())
    }
    pub async fn retire(&self, token: &str, stopped: bool) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        // An explicitly owned recorded token can be durably revoked without
        // inserting its calls into this owner's live execution registry.
        let turn = match registry.turns.get(token) {
            Some(turn) => turn.clone(),
            None => self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", token)))?
                .ok_or(Error("mcp_turn_unavailable"))?,
        };
        if stopped {
            let lane = turn.lane;
            drop(registry);
            return self.stop_lane(&lane, Some(token)).await;
        }
        if turn.calls.values().any(|v| v.result.is_none()) {
            return Err(Error("mcp_pending_completion"));
        }
        let mut next = turn;
        next.retire(false);
        self.persist(token, &next)?;
        if let Some(turn) = registry.turns.get_mut(token) {
            *turn = next;
        }
        self.changed.notify_waiters();
        Ok(())
    }

    /// A verified failed/interrupted Native turn cannot create another result
    /// request. Its original attached work fences admission before calling
    /// this method. Retire and recover in one commit so retained invocations
    /// cannot wake between revocation and their durable original result.
    /// Already committed model-input projections retain their exact bytes.
    pub(crate) async fn retire_native_results(
        &self,
        token: &str,
        lane: &str,
        results: &[(String, Value)],
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let mut turn = registry
            .turns
            .get(token)
            .cloned()
            .or(self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", token)))?)
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.lane != lane || turn.source_bound {
            return Err(Error("mcp_owner_conflict"));
        }
        let mut seen = BTreeSet::new();
        for (id, result) in results {
            if !seen.insert(id) || result["call_id"].as_str() != Some(id.as_str()) {
                return Err(Error("mcp_result_invalid"));
            }
            let call = turn
                .calls
                .values_mut()
                .find(|call| call.id == *id && call.offered)
                .ok_or(Error("mcp_result_invalid"))?;
            if call.result.is_none() {
                call.result = Some(result.clone());
            }
        }
        if turn
            .calls
            .values()
            .any(|call| call.offered && call.result.is_none())
        {
            return Err(Error("mcp_result_missing"));
        }
        turn.retire(false);
        self.persist(token, &turn)?;
        if let Some(cached) = registry.turns.get_mut(token) {
            *cached = turn;
        }
        // Native's original receipt is durable before either the shared or
        // Provider resource claim is released. No new effect is offered.
        let settled = self.reconcile_persisted_integrated_receipts();
        self.changed.notify_waiters();
        settled
    }

    pub(crate) async fn stop_lane(&self, lane: &str, recorded: Option<&str>) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let mut tails = BTreeSet::new();
        for (token, source) in registry
            .discovery
            .iter_mut()
            .filter(|(_, source)| source.lane == lane)
        {
            source.revoked = true;
            tails.insert(token.clone());
        }
        tails.extend(recorded.map(str::to_owned));
        let mut changed = Vec::new();
        for (token, turn) in registry
            .turns
            .iter_mut()
            .filter(|(_, turn)| turn.lane == lane)
        {
            turn.retire(true);
            changed.push((token.clone(), turn.clone()));
            tails.remove(token);
        }
        let mut failure = None;
        for token in tails {
            match self.journal.read::<Turn>(&digest(&("mcp-turn-v1", &token))) {
                Ok(Some(mut turn)) if turn.lane == lane => {
                    turn.retire(true);
                    // Cold receipts remain in their journal, as in retire().
                    changed.push((token, turn));
                }
                Ok(Some(_)) => {
                    failure.get_or_insert(Error("mcp_owner_conflict"));
                }
                Ok(None) => {
                    failure.get_or_insert(Error("mcp_turn_unavailable"));
                }
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        // Fence all live capabilities before fallible persistence. Attempt all
        // durable revocations, including evicted tails, and retain their cause.
        self.changed.notify_waiters();
        for (token, turn) in changed {
            match self.persist(&token, &turn) {
                Ok(()) => {
                    registry.discovery.remove(&token);
                }
                Err(error) => {
                    registry.discovery.insert(
                        token.clone(),
                        DiscoveryAdmission {
                            lane: lane.into(),
                            root: token,
                            revoked: true,
                        },
                    );
                    failure.get_or_insert(error);
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
    pub async fn close(&self) {
        let mut registry = self.registry.lock().await;
        self.closed.store(true, Ordering::Release);
        registry.discovery.clear();
        for (token, turn) in registry.turns.iter_mut() {
            turn.retire(false);
            // Source adoption by a future owner still validates the retired
            // journal chain; process close itself grants no discovery authority.
            if let Err(error) = self.persist(token, turn) {
                eprintln!("mcp_retirement_persist_failed: {}", error.0);
            }
        }
        self.changed.notify_waiters();
    }
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/mcp", axum::routing::post(http))
            .with_state(self)
    }
}

pub(crate) fn native_history_item_result(item: &Value) -> Option<Value> {
    if !matches!(item["status"].as_str(), Some("completed" | "failed")) {
        return None;
    }
    match item["type"].as_str() {
        Some("commandExecution") => {
            let code = item["exitCode"]
                .as_i64()
                .filter(|code| *code >= 0 && *code != 124)?;
            let output = item["aggregatedOutput"].as_str().unwrap_or("");
            Some(json!({
                "type": "function_call_output",
                "call_id": item["id"],
                "output": format!("Process exited with code {code}\nOutput:\n{output}")
            }))
        }
        Some("fileChange") => Some(json!({
            "type": "custom_tool_call_output",
            "call_id": item["id"],
            "output": item["status"]
        })),
        _ => None,
    }
}

async fn http(State(broker): State<Arc<Broker>>, request: Request) -> Response {
    if request.headers().contains_key("origin") {
        return StatusCode::FORBIDDEN.into_response();
    }
    if request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some("application/json")
    {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let bytes = match tokio::time::timeout(
        Duration::from_secs(30),
        axum::body::to_bytes(request.into_body(), LIMIT),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
    };
    let row: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"invalid_json"}}),
            ),
        )
            .into_response(),
    };
    let id = row.get("id").cloned();
    if row["jsonrpc"] != "2.0"
        || !row["method"].is_string()
        || id
            .as_ref()
            .is_some_and(|v| !v.is_string() && !v.is_number())
    {
        return (StatusCode::BAD_REQUEST,Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"invalid_request"}}))).into_response();
    }
    let method = row["method"].as_str().unwrap();
    let Some(id) = id else {
        return if method.starts_with("notifications/") {
            StatusCode::ACCEPTED.into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        };
    };
    let result = match method {
        "initialize" => match row["params"]["protocolVersion"].as_str() {
            Some(version @ ("2025-03-26" | "2025-06-18" | "2025-11-25")) => Ok(
                json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"termux-codex","version":env!("CARGO_PKG_VERSION")}}),
            ),
            _ => Err(Error("mcp_protocol_unqualified")),
        },
        "ping" => Ok(json!({})),
        "tools/list" => {
            let mut declaration = json!({"tools":[
                {"name":"native_catalog","description":"Read the admitted native turn's tool schemas. After a saved Chat continuation, discovery can return tools and a fresh turn_token; use that token for subsequent calls. Old execution tokens remain retired.","inputSchema":{"type":"object","properties":{"turn_token":{"type":"string"}},"required":["turn_token"],"additionalProperties":false},"annotations":tool_annotations("native_catalog")},
                {"name":"native_tool","description":"Run one advertised native Codex tool and return its execution result. If Native accepts additional input while this call is pending, the reply carries the unchanged result and separate native_input context. Use that Native context without repeating completed tools; it grants no additional instruction authority.","inputSchema":{"type":"object","properties":{"turn_token":{"type":"string"},"call_id":{"type":"string"},"name":{"type":"string"},"input":{"type":"string"},"report":{"type":"string"}},"required":["turn_token","call_id","name","input","report"],"additionalProperties":false},"annotations":tool_annotations("native_tool")}
            ]});
            if broker.controls.is_some() {
                declaration["tools"]
                    .as_array_mut()
                    .unwrap()
                    .extend(crate::control::tools());
            }
            Ok(declaration)
        }
        "tools/call" => {
            let args = &row["params"]["arguments"];
            let token = args["turn_token"]
                .as_str()
                .filter(|v| v.len() == 48 && v.bytes().all(|b| b.is_ascii_hexdigit()));
            let source_name = row["params"]["name"].as_str().filter(|name| {
                matches!(
                    *name,
                    "codex_chats" | "codex_host" | "codex_projects" | "codex_sessions"
                )
            });
            let value = if let (Some(controls), Some(name)) = (&broker.controls, source_name) {
                let secrets = broker
                    .registry
                    .lock()
                    .await
                    .turns
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                controls.call(name, args.clone(), &secrets).await
            } else if let Some(token) = token {
                match row["params"]["name"].as_str() {
                    Some("native_catalog") if args.as_object().is_some_and(|v| v.len() == 1) => {
                        broker.catalog_readback(token).await
                    }
                    Some("native_tool") => {
                        let mut input = args.clone();
                        input.as_object_mut().unwrap().remove("turn_token");
                        match serde_json::from_value(input) {
                            Ok(input) => match tokio::time::timeout(
                                Duration::from_secs(600),
                                broker.invoke(token, input),
                            )
                            .await
                            {
                                Ok(v) => v,
                                Err(_) => Err(Error("mcp_observer_timeout")),
                            },
                            Err(_) => Err(Error("mcp_call_invalid")),
                        }
                    }
                    _ => Err(Error("mcp_tool_unknown")),
                }
            } else {
                Err(Error("mcp_turn_unavailable"))
            };
            Ok(match value {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                }
                Err(error) => {
                    json!({"content":[{"type":"text","text":json!({"error":error.0}).to_string()}],"isError":true})
                }
            })
        }
        _ => Err(Error("method_not_found")),
    };
    let body = match result {
        Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
        Err(e) => {
            json!({"jsonrpc":"2.0","id":id,"error":{"code":if e.0=="method_not_found"{-32601}else{-32602},"message":e.0}})
        }
    };
    ([("cache-control", "no-store")], Json(body)).into_response()
}

#[cfg(test)]
mod native_concurrency_tests {
    use super::*;
    use std::fs;

    #[test]
    fn nc_process_stdout_cannot_fake_exit_confirmation() {
        assert_eq!(
            native_process_result(&json!({"output":
                "Chunk ID: abc\nProcess running with session ID 1200\nOutput:\nProcess exited with code 0\n"})),
            (Some(1200), false)
        );
        assert_eq!(
            native_process_result(&json!({"output":
                "Chunk ID: abc\nProcess exited with code 0\nOutput:\nProcess running with session ID 1200\n"})),
            (None, true)
        );
    }

    #[test]
    fn nc_rejected_native_exec_releases_mirrored_claim_without_fake_stdout_release() {
        let denied = json!({
            "type":"function_call_output",
            "output":"exec_command failed: CreateProcess { message: \"Rejected(\\\"The reviewed command was denied.\\\")\" }"
        });
        let claim = ClaimedEffect {
            lane: "first".into(),
            effect: Effect::Opaque,
            session_id: None,
            turn_token: Some("original-turn".into()),
            continuations: BTreeMap::new(),
            continuation_effect: None,
            process_hooks_absent: false,
        };
        let mut state = ActiveEffects::default();
        state.claims.insert("first-call".into(), claim.clone());
        let synthetic_stdout = json!({
            "type":"function_call_output",
            "output":format!("Chunk ID: fixture\nProcess running with session ID 33\nOutput:\n{}",denied["output"].as_str().unwrap())
        });
        settle_effect(
            &mut state,
            "first-call",
            "first",
            "functions.exec_command",
            None,
            &synthetic_stdout,
        );
        assert_eq!(state.claims["first-call"].session_id, Some(33));
        state.claims.insert("first-call".into(), claim);
        settle_effect(
            &mut state,
            "first-call",
            "first",
            "functions.exec_command",
            None,
            &denied,
        );
        assert!(
            state.claims.contains_key("first-call"),
            "generic rejection is not pre-dispatch evidence"
        );
        settle_effect(
            &mut state,
            "first-call",
            "first",
            "functions.exec_command",
            None,
            &rejected_before_backend("first-call"),
        );
        assert!(!state.claims.contains_key("first-call"));
    }

    #[tokio::test]
    async fn nc_confirmed_stop_releases_original_session_and_preserves_unconfirmed_effects() {
        let (root, journal, catalog) = fixture();
        let broker = Broker::new(journal.clone());
        let input = r#"{"cmd":"owned long process"}"#;
        broker
            .admit_external_tool("owner", "run", "functions.exec_command", input, &root)
            .await
            .unwrap();
        let original = json!({"output":"Process running with session ID 83\nOutput:\n"});
        broker
            .settle_external_tools("owner", &[("run".into(), original)])
            .await
            .unwrap();
        for lane in ["other", "owner"] {
            broker.settle_stopped_processes(lane, &root).await.unwrap();
            assert_eq!(
                broker
                    .admit_external_tool("other", "blocked", "functions.exec_command", input, &root)
                    .await,
                Err(Error("native_resource_busy"))
            );
        }
        assert_eq!(
            broker.effect_status("owner").unwrap()["unconfirmed_effects"],
            1
        );
        broker
            .admit_external_tool(
                "owner",
                "poll",
                "functions.write_stdin",
                r#"{"session_id":83,"chars":""}"#,
                &root,
            )
            .await
            .unwrap();
        let proof = json!({"schema":"codex-native-completion-v1","call_id":"poll","session_id":83,
            "outcome":"wait_completed","exit_code":137,"hooks":"absent"});
        broker.settle_external_tools("owner",&[("poll".into(),json!({
            "output":format!("Native completion: {proof}\nProcess exited with code 137\nOutput:\n")}))]).await.unwrap();
        assert_eq!(
            broker.effect_status("owner").unwrap()["unconfirmed_effects"],
            0
        );
        broker
            .admit_external_tool("other", "unblocked", "functions.exec_command", input, &root)
            .await
            .unwrap();
        broker
            .settle_external_tools(
                "other",
                &[(
                    "unblocked".into(),
                    json!({
            "output":format!("Native completion: {}\nOutput:\n",json!({
                "schema":"codex-native-completion-v1","call_id":"unblocked","session_id":null,
                "outcome":"wait_completed","exit_code":0,"hooks":"absent"}))}),
                )],
            )
            .await
            .unwrap();

        let token = broker
            .register_scoped("lane_app_owner", catalog, false, Some(&root))
            .await
            .unwrap();
        let call = Arguments {
            call_id: "offered-without-result".into(),
            name: "functions.exec_command".into(),
            input: input.into(),
            report: String::new(),
        };
        let id = accepted(&broker, &token, &call).await;
        let owners = BTreeMap::from([("lane_app_owner".into(), "work-owner".into())]);
        let status = broker
            .effect_status_with_owners(&["work-owner", "lane_app_owner"], &owners)
            .unwrap();
        assert_eq!(status["unconfirmed_effects"], 1);
        assert_eq!(status["active"][0]["owner_work"], "work-owner");
        assert_eq!(status["active"][0]["owned"], true);
        assert_eq!(
            broker
                .effect_status_with_owners(&["other"], &owners)
                .unwrap()["active"][0]["owned"],
            false
        );
        for ids in [BTreeSet::from([id.clone()]), BTreeSet::new()] {
            broker
                .reconcile_stopped_unexecuted(&["work-owner", "lane_app_owner"], &[&token], &ids)
                .await
                .unwrap();
            assert_eq!(
                broker
                    .admit_external_tool(
                        "other",
                        "blocked-by-mcp",
                        "functions.exec_command",
                        input,
                        &root
                    )
                    .await,
                Err(Error("native_resource_busy"))
            );
            assert_eq!(broker.recorded_calls(&token).unwrap()[0]["settled"], false);
        }
        let proof = json!({"schema":"codex-native-completion-v1","call_id":id,"session_id":null,
            "outcome":"pre_backend_rejected","exit_code":null,"hooks":"absent"});
        let result = json!({"type":"function_call_output","call_id":id,
            "output":format!("Native completion: {proof}\nOutput:\ninitial approval denied")});
        broker
            .retire_native_results(&token, "lane_app_owner", &[(id.clone(), result.clone())])
            .await
            .unwrap();
        assert_eq!(broker.recorded_calls(&token).unwrap()[0]["id"], id);
        assert_eq!(broker.recorded_calls(&token).unwrap()[0]["settled"], true);
        let saved: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        assert_eq!(saved.calls["offered-without-result"].result, Some(result));
        broker
            .admit_external_tool(
                "other",
                "after-positive-proof",
                "functions.exec_command",
                input,
                &root,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn nc_confirmed_stop_releases_orphaned_physical_claim_after_mirror_settled() {
        let (root, journal, catalog) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("stopped-work", catalog.clone(), false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&token, Some(execution.clone()))
            .await
            .unwrap();
        let call = Arguments {
            call_id: "orphaned-shell".into(),
            name: "functions.exec_command".into(),
            input: r#"{"cmd":"printf updated > file"}"#.into(),
            report: "owned fixture".into(),
        };
        let id = accepted(&broker, &token, &call).await;
        let original = json!({"output":"connection closed before terminal proof"});
        broker
            .settle(&token, &[(id.clone(), original.clone())])
            .await
            .unwrap();
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = journal.read(&key).unwrap().unwrap();
        assert!(state.claims.remove(&id).is_some());
        journal.write(&key, &state).unwrap();
        assert_eq!(
            broker.effect_status("stopped-work").unwrap()["unconfirmed_effects"],
            0
        );
        for ids in [BTreeSet::from([id.clone()]), BTreeSet::new()] {
            broker
                .reconcile_stopped_unexecuted(&["stopped-work"], &[&token], &ids)
                .await
                .unwrap();
            assert_eq!(
                broker
                    .admit_external_tool(
                        "other",
                        "blocked",
                        "functions.exec_command",
                        &call.input,
                        &root
                    )
                    .await,
                Err(Error("native_resource_busy"))
            );
        }
        let (raw, _) = gateway_calls(
            &id,
            &json!({"name":call.name,"input":call.input,"report":call.report}).to_string(),
            &catalog,
        )
        .unwrap();
        let proof = json!({"schema":"codex-native-completion-v1","call_id":id,"session_id":null,
            "outcome":"wait_completed","exit_code":0,"hooks":"absent"});
        let evidence = crate::operator_native::CompletionEvidence::fixture(
            &execution,
            vec![
                json!({"type":"turn_context","payload":{"turn_id":execution.turn}}),
                json!({"type":"response_item","payload":raw[0].output_item().unwrap()}),
                json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":id,
                "turn_id":execution.turn,"process_id":null,"exit_code":0,"status":"completed",
                "native_completion":proof}}),
            ],
        );
        let settled = broker
            .reconcile_stopped_receipts(
                &["stopped-work"],
                &[&token],
                &BTreeMap::new(),
                Some(&evidence),
            )
            .await
            .unwrap();
        assert!(settled.contains(&id));
        let saved: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        assert_eq!(saved.calls["orphaned-shell"].result, Some(original));
        assert!(saved.calls["orphaned-shell"].terminal_completion.is_some());
        broker
            .admit_external_tool(
                "other",
                "after-original-wait",
                "functions.exec_command",
                &call.input,
                &root,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn unloaded_no_offer_receipt_survives_cold_chain_without_reactivating_execution() {
        let (root, journal, tools) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        let broker = Broker::new(journal.clone());
        broker
            .admit_external_tool(
                "other",
                "held",
                "functions.exec_command",
                r#"{"cmd":"unknown independent work"}"#,
                &root,
            )
            .await
            .unwrap();
        let token = broker
            .register_scoped("no-offer", tools.clone(), true, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&token, Some(execution.clone()))
            .await
            .unwrap();
        let args = Arguments {
            call_id: "busy".into(),
            name: "functions.exec_command".into(),
            input: r#"{"cmd":"must not run"}"#.into(),
            report: String::new(),
        };
        broker.enqueue(&token, &args).await.unwrap();
        assert!(broker.offer(&token).await.unwrap().is_empty());
        let saved: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        let call = &saved.calls[&args.call_id];
        assert!(!call.offered);
        assert_ne!(call.shared_reserved, Some(true));
        let result = call.result.clone().unwrap();
        assert_eq!(result["executed"], false);
        let (expected, _) = gateway_calls(
            &call.id,
            &json!({"name":call.name,"input":call.input,"report":call.report}).to_string(),
            &saved.tools,
        )
        .unwrap();
        let raw = expected[0].output_item().unwrap();
        let mut evidence = crate::operator_native::UnloadedHistory {
            state: json!({"thread":{"id":execution.thread,"status":{"type":"notLoaded"},
                "turns":[{"id":execution.turn,"status":"completed","error":null,"items":[]}]}}),
            rows: vec![
                json!({"type":"session_meta","payload":{"id":execution.thread}}),
                json!({"type":"turn_context","payload":{"turn_id":execution.turn}}),
                json!({"type":"response_item","payload":raw}),
                json!({"type":"response_item","payload":{"type":"function_call_output",
                    "call_id":call.id,"output":result.to_string()}}),
            ],
            owner: crate::operator_native::Owner {
                directory: root.clone(),
                home: root.clone(),
                process: crate::cdp::Process::read(std::process::id()).unwrap(),
            },
            workspace: root.clone(),
            thread: execution.thread.clone(),
        };
        let ids = broker
            .unloaded_receipts(&["no-offer"], &[(&token, &token)], &evidence)
            .await
            .unwrap();
        assert_eq!(ids.non_commands, BTreeSet::from([call.id.clone()]));
        evidence.validate(&execution.turn, &ids).unwrap();
        for kind in 0..4 {
            let mut wrong = evidence.clone();
            match kind {
                0 => wrong.owner.process.start.push('0'),
                1 => wrong.rows[1]["payload"]["turn_id"] = json!("foreign-turn"),
                2 => wrong.rows[2]["payload"]["arguments"] = json!("{\"cmd\":\"different\"}"),
                _ => wrong.workspace = root.join("other"),
            }
            let missing = broker
                .unloaded_receipts(&["no-offer"], &[(&token, &token)], &wrong)
                .await
                .unwrap();
            assert!(missing.non_commands.is_empty() && missing.legacy_processes.is_empty());
            assert!(wrong.validate(&execution.turn, &missing).is_err());
        }
        broker.retire(&token, false).await.unwrap();
        let tail = broker
            .register_scoped("no-offer", tools, true, Some(&root))
            .await
            .unwrap();
        broker.source_discovery(&token, &tail).await.unwrap();
        broker.retire(&tail, false).await.unwrap();
        drop(broker);
        let restored = Broker::new(journal);
        restored
            .restore_source_discovery(&token, &tail, "no-offer")
            .await
            .unwrap();
        let cold = restored
            .unloaded_receipts(&["no-offer"], &[(&token, &tail)], &evidence)
            .await
            .unwrap();
        assert_eq!(cold, ids);
        assert!(!restored.registry.lock().await.turns.contains_key(&token));
        assert_eq!(
            restored.retained_result(&token, &args).unwrap(),
            Some(result)
        );
        let current = restored
            .register_scoped("no-offer", saved.tools, true, Some(&root))
            .await
            .unwrap();
        restored.source_discovery(&tail, &current).await.unwrap();
        let next = Arguments {
            call_id: "pending".into(),
            ..args
        };
        restored.enqueue(&current, &next).await.unwrap();
        assert_eq!(
            restored
                .unloaded_receipts(&["no-offer"], &[(&token, &current)], &evidence)
                .await,
            Err(Error("native_unloaded_effects_unsettled"))
        );
        assert_eq!(
            restored.effect_status("no-offer").unwrap()["shared"]["active"][0]["owned"],
            false
        );
        evidence.rows.clear();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn unloaded_prior_settled_non_process_receipt_is_reused_but_current_unknown_is_not() {
        let (root, journal, mut tools) = fixture();
        for tool in tools.values_mut() {
            if let Some(name) = tool.name.strip_prefix("functions.") {
                tool.name = name.to_owned();
                tool.namespace = Some("functions".into());
            }
        }
        fs::write(root.join("file"), "alpha\n").unwrap();
        let mut facts = native_effect::compiler_fixture(&root);
        if !facts
            .hook_free_tools
            .iter()
            .any(|name| name == "apply_patch")
        {
            facts.hook_free_tools.push("apply_patch".into());
        }
        facts.turn = "past-patch".into();
        let broker = Broker::new(journal.clone());
        let first = broker
            .register_scoped("owned", tools.clone(), true, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&first, Some(facts.clone()))
            .await
            .unwrap();
        let edit = patch("file", "alpha", "beta");
        let patch_id = accepted(&broker, &first, &edit).await;
        let patched =
            json!({"type":"custom_tool_call_output","call_id":patch_id,"output":"updated file"});
        fs::write(root.join("file"), "beta\n").unwrap();
        broker
            .settle(&first, &[(patch_id.clone(), patched.clone())])
            .await
            .unwrap();
        let old: Turn = journal
            .read(&digest(&("mcp-turn-v1", &first)))
            .unwrap()
            .unwrap();
        assert!(matches!(
            old.calls[&edit.call_id].effect,
            Some(Effect::Paths(_))
        ));
        broker.retire(&first, false).await.unwrap();
        let last = broker
            .register_scoped("owned", tools, true, Some(&root))
            .await
            .unwrap();
        broker.source_discovery(&first, &last).await.unwrap();
        facts.turn = "current-compiler".into();
        broker
            .bind_execution(&last, Some(facts.clone()))
            .await
            .unwrap();
        let command = Arguments {
            call_id: "compiler".into(),
            name: "functions.exec_command".into(),
            input: r#"{"cmd":"printf owned"}"#.into(),
            report: String::new(),
        };
        let id = accepted(&broker, &last, &command).await;
        let result = json!({"type":"function_call_output","call_id":id,
            "output":"Wall time: 0 seconds\nProcess exited with code 0\nOutput:\nowned"});
        broker
            .settle(&last, &[(id.clone(), result.clone())])
            .await
            .unwrap();
        let raw = |id: &str, call: &Arguments| {
            gateway_calls(
                id,
                &json!({"name":call.name,"input":call.input,"report":call.report}).to_string(),
                &old.tools,
            )
            .unwrap()
            .0[0]
                .output_item()
                .unwrap()
        };
        let mut evidence = crate::operator_native::UnloadedHistory {
            state: json!({"thread":{"id":facts.thread,"status":{"type":"notLoaded"},"turns":[
                {"id":"past-patch","status":"completed","error":null,"items":[
                    {"id":patch_id,"type":"fileChange","status":"completed","changes":[]}]},
                {"id":facts.turn,"status":"completed","error":null,"items":[
                    {"id":id,"type":"commandExecution","processId":null,"status":"completed",
                        "exitCode":0,"aggregatedOutput":"owned"}]}]}}),
            rows: vec![
                json!({"type":"session_meta","payload":{"id":facts.thread}}),
                json!({"type":"turn_context","payload":{"turn_id":"past-patch"}}),
                json!({"type":"response_item","payload":raw(&patch_id,&edit)}),
                json!({"type":"response_item","payload":patched}),
                json!({"type":"turn_context","payload":{"turn_id":facts.turn}}),
                json!({"type":"response_item","payload":raw(&id,&command)}),
                json!({"type":"response_item","payload":result}),
                json!({"type":"event_msg","payload":{"type":"item_completed","thread_id":facts.thread,
                    "turn_id":facts.turn,"item":{"type":"CommandExecution","id":id,
                        "process_id":null,"status":"completed","exit_code":0}}}),
            ],
            owner: crate::operator_native::Owner {
                directory: root.clone(),
                home: root.clone(),
                process: crate::cdp::Process::read(std::process::id()).unwrap(),
            },
            workspace: root.clone(),
            thread: facts.thread.clone(),
        };
        let closed = broker
            .unloaded_receipts(&["owned"], &[(&first, &last)], &evidence)
            .await
            .unwrap();
        assert_eq!(closed.non_commands, BTreeSet::from([patch_id.clone()]));
        assert!(closed.legacy_processes.contains(&id));
        evidence.validate(&facts.turn, &closed).unwrap();
        // The same historical output is not sufficient without its original
        // matching, safely classified receipt.
        evidence.rows[3]["payload"]["output"] = json!("different result");
        let absent = broker
            .unloaded_receipts(&["owned"], &[(&first, &last)], &evidence)
            .await
            .unwrap();
        assert!(absent.non_commands.is_empty());
        assert!(evidence.validate(&facts.turn, &absent).is_err());
        let pending = patch("file", "beta", "gamma");
        accepted(&broker, &last, &pending).await;
        assert_eq!(
            broker
                .unloaded_receipts(&["owned"], &[(&first, &last)], &evidence)
                .await,
            Err(Error("native_unloaded_effects_unsettled"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn completion_safety_legacy_requery_requires_identical_original_result_and_hooks() {
        for (code, hooks) in [(0, true), (7, true), (124, true), (0, false)] {
            let (root, journal, _) = fixture();
            let facts = native_effect::compiler_fixture(&root);
            let broker = Broker::new(journal.clone());
            broker
                .admit_external_tool_with_execution(
                    "owner",
                    "original",
                    "functions.exec_command",
                    r#"{"cmd":"owned original process"}"#,
                    &root,
                    Some(&facts),
                )
                .await
                .unwrap();
            let raw = json!({"type":"function_call_output","call_id":"original",
                "output":format!("Process exited with code {code}\nOutput:\noriginal")});
            let key = digest(&"native-effect-claims-v1");
            let id = external_effect_id("owner", "original");
            let mut state: ActiveEffects = journal.read(&key).unwrap().unwrap();
            // Model a retained pre-proof reader: the old original reservation
            // remains, but its per-call hook authority was not persisted. This
            // is not a newly dispatched unsupported invocation.
            if !hooks {
                state.claims.get_mut(&id).unwrap().process_hooks_absent = false;
                state.external.get_mut(&id).unwrap().process_hooks_absent = None;
                let path = broker
                    .shared_guard_home()
                    .unwrap()
                    .join("shared-native-v1/claims.json");
                let mut shared: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                for claim in shared["claims"].as_object_mut().unwrap().values_mut() {
                    claim
                        .as_object_mut()
                        .unwrap()
                        .remove("process_hooks_absent");
                }
                fs::write(path, serde_json::to_vec(&shared).unwrap()).unwrap();
            }
            state.external.get_mut(&id).unwrap().completion = Some(
                serde_json::from_value(json!({
                "fingerprint":digest(&raw),"running":null,"exited":true,"not_spawned":false}))
                .unwrap(),
            );
            journal.write(&key, &state).unwrap();
            broker.reconcile_persisted_integrated_receipts().unwrap();
            assert_eq!(
                broker.active_claim_ids(&["owner"]).unwrap(),
                BTreeSet::from([id.clone()])
            );
            let before: Value = journal.read(&key).unwrap().unwrap();
            let mut changed = raw.clone();
            changed["output"] = json!("different original output");
            assert_eq!(
                broker
                    .settle_external_tools("owner", &[("original".into(), changed)])
                    .await,
                Err(Error("native_effect_result_changed"))
            );
            assert_eq!(journal.read::<Value>(&key).unwrap().unwrap(), before);
            broker
                .settle_external_tools("owner", &[("original".into(), raw.clone())])
                .await
                .unwrap();
            let after: ActiveEffects = journal.read(&key).unwrap().unwrap();
            assert_eq!(
                after.external[&id].completion.as_ref().unwrap().fingerprint,
                digest(&raw)
            );
            assert_eq!(after.claims.contains_key(&id), !hooks || code == 124);
            assert_eq!(
                broker.effect_status("owner").unwrap()["shared"]["unconfirmed_effects"],
                usize::from(!hooks || code == 124)
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn native_process_aliases_keep_original_names_and_reject_changed_retry_identity() {
        for (name, other_name) in [
            ("exec_command", "functions.exec_command"),
            ("functions.exec_command", "exec_command"),
        ] {
            let (root, journal, _) = fixture();
            let tools = crate::protocol::Request::parse(
                json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":"alias fixture","tools":[
                    {"type":"function","name":"exec_command","parameters":{"type":"object"}},
                    {"type":"namespace","name":"functions","tools":[
                        {"type":"function","name":"exec_command","parameters":{"type":"object"}}]}]}),
                &crate::config::Config::default(),
                Some("owned"),
            ).unwrap().tools;
            let broker = Broker::new(journal.clone());
            let token = broker
                .register_scoped("owner", tools, false, Some(&root))
                .await
                .unwrap();
            let facts = native_effect::hook_fixture(&root);
            broker
                .bind_execution(&token, Some(facts.clone()))
                .await
                .unwrap();
            let call = Arguments {
                call_id: "original".into(),
                name: name.into(),
                input: r#"{"cmd":"owned opaque process"}"#.into(),
                report: String::new(),
            };
            let id = accepted(&broker, &token, &call).await;
            let before: Turn = journal
                .read(&digest(&("mcp-turn-v1", &token)))
                .unwrap()
                .unwrap();
            assert_eq!(before.calls["original"].name, name);
            let mut changed = call.clone();
            changed.name = other_name.into();
            assert_eq!(
                broker.enqueue(&token, &changed).await,
                Err(Error("mcp_call_changed"))
            );
            let result = waited(&id, None, 7);
            broker
                .settle(&token, &[(id.clone(), result.clone())])
                .await
                .unwrap();
            drop(broker);
            let restored = Broker::new(journal.clone());
            restored.reconcile_persisted_integrated_receipts().unwrap();
            assert_eq!(
                restored.retained_result(&token, &call).unwrap(),
                Some(result.clone())
            );
            let after: Turn = journal
                .read(&digest(&("mcp-turn-v1", &token)))
                .unwrap()
                .unwrap();
            assert_eq!(after.calls["original"].name, before.calls["original"].name);
            assert_eq!(
                after.calls["original"].input,
                before.calls["original"].input
            );
            assert_eq!(
                after.calls["original"].fingerprint,
                before.calls["original"].fingerprint
            );
            assert_eq!(after.calls["original"].result, Some(result));
            assert!(restored.active_claim_ids(&["owner"]).unwrap().is_empty());
            restored
                .admit_external_tool_with_execution(
                    "owner",
                    "external-original",
                    name,
                    &call.input,
                    &root,
                    Some(&facts),
                )
                .await
                .unwrap();
            assert_eq!(
                restored
                    .admit_external_tool_with_execution(
                        "owner",
                        "external-original",
                        other_name,
                        &call.input,
                        &root,
                        Some(&facts),
                    )
                    .await,
                Err(Error("native_effect_identity_changed"))
            );
            restored
                .settle_external_tools(
                    "owner",
                    &[(
                        "external-original".into(),
                        waited("external-original", None, 0),
                    )],
                )
                .await
                .unwrap();
            let state: ActiveEffects = journal
                .read(&digest(&"native-effect-claims-v1"))
                .unwrap()
                .unwrap();
            let saved = &state.external[&external_effect_id("owner", "external-original")];
            assert_eq!(saved.name, name);
            assert_eq!(saved.fingerprint, digest(&(name, call.input.as_str())));
            assert!(saved.settled);
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn actual_wait_without_numeric_exit_settles_original_receipts_but_unknown_does_not() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for external in [false, true] {
                for confirmed in [false, true] {
                    let (root, journal, tools) = fixture_with_process_aliases(exec_tool, poll_tool);
                    let facts = native_effect::hook_fixture(&root);
                    let broker = Broker::new(journal.clone());
                    let token = broker
                        .register_scoped("owner", tools, false, Some(&root))
                        .await
                        .unwrap();
                    broker
                        .bind_execution(&token, Some(facts.clone()))
                        .await
                        .unwrap();
                    let call = Arguments {
                        call_id: "original".into(),
                        name: exec_tool.into(),
                        input: r#"{"cmd":"owned opaque process"}"#.into(),
                        report: String::new(),
                    };
                    let id = if external {
                        broker
                            .admit_external_tool_with_execution(
                                "owner",
                                "original",
                                &call.name,
                                &call.input,
                                &root,
                                Some(&facts),
                            )
                            .await
                            .unwrap();
                        "original".to_owned()
                    } else {
                        accepted(&broker, &token, &call).await
                    };
                    let proof = if confirmed {
                        json!({"schema":"codex-native-completion-v1","call_id":id,
                        "session_id":null,"outcome":"wait_completed","exit_code":null,"hooks":"absent"})
                    } else {
                        Value::Null
                    };
                    let result = json!({"type":"function_call_output","call_id":id,
                        "output":format!("Native completion: {proof}\nProcess exited with code -1\nOutput:\noriginal terminal projection")});
                    if external {
                        broker
                            .settle_external_tools("owner", &[(id.clone(), result.clone())])
                            .await
                            .unwrap();
                    } else {
                        broker
                            .settle(&token, &[(id.clone(), result.clone())])
                            .await
                            .unwrap();
                        assert_eq!(
                            broker.retained_result(&token, &call).unwrap(),
                            Some(result.clone())
                        );
                    }
                    drop(broker);
                    let restored = Broker::new(journal.clone());
                    restored.reconcile_persisted_integrated_receipts().unwrap();
                    assert_eq!(
                        !restored.active_claim_ids(&["owner"]).unwrap().is_empty(),
                        !confirmed,
                        "only the actual waited object, not whole null or public -1, settles"
                    );
                    assert_eq!(
                        restored.effect_status("owner").unwrap()["shared"]["unconfirmed_effects"],
                        usize::from(!confirmed)
                    );
                    if external {
                        let state: ActiveEffects = journal
                            .read(&digest(&"native-effect-claims-v1"))
                            .unwrap()
                            .unwrap();
                        let completion = state.external[&external_effect_id("owner", &id)]
                            .completion
                            .as_ref()
                            .unwrap();
                        assert_eq!(completion.fingerprint, digest(&result));
                        assert_eq!(
                            completion.confirmed_exit(),
                            None,
                            "never fabricate a numeric exit"
                        );
                        assert_eq!(completion.status(), (None, confirmed));
                    }
                    let admission = restored
                        .admit_external_tool_with_execution(
                            "other",
                            "new-logical-call",
                            &call.name,
                            &call.input,
                            &root,
                            Some(&facts),
                        )
                        .await;
                    assert_eq!(
                        admission,
                        if confirmed {
                            Ok(())
                        } else {
                            Err(Error("native_resource_busy"))
                        }
                    );
                    fs::remove_dir_all(root).unwrap();
                }
            }
        }
    }

    #[tokio::test]
    async fn orphan_continuation_unknown_keeps_both_claim_mirrors_after_restart() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for external in [false, true] {
                for running in [false, true] {
                    let (root, journal, tools) = fixture_with_process_aliases(exec_tool, poll_tool);
                    let facts = native_effect::hook_fixture(&root);
                    let broker = Broker::new(journal.clone());
                    let token = broker
                        .register_scoped("owner", tools, false, Some(&root))
                        .await
                        .unwrap();
                    broker
                        .bind_execution(&token, Some(facts.clone()))
                        .await
                        .unwrap();
                    let poll = Arguments {
                        call_id: "orphan".into(),
                        name: poll_tool.into(),
                        input: r#"{"session_id":51,"chars":""}"#.into(),
                        report: String::new(),
                    };
                    let (call, claim) = if external {
                        broker
                            .admit_external_tool_with_execution(
                                "owner",
                                "orphan",
                                &poll.name,
                                &poll.input,
                                &root,
                                Some(&facts),
                            )
                            .await
                            .unwrap();
                        ("orphan".to_owned(), external_effect_id("owner", "orphan"))
                    } else {
                        let id = accepted(&broker, &token, &poll).await;
                        (id.clone(), id)
                    };
                    let result = json!({"type":"function_call_output","call_id":call,"output":if running {
                        "Native completion: null\nProcess running with session ID 51\nOutput:\noriginal"
                    } else { "Native completion: null\nOutput:\nunknown original result" }});
                    if external {
                        broker
                            .settle_external_tools("owner", &[(call.clone(), result.clone())])
                            .await
                            .unwrap();
                    } else {
                        broker
                            .settle(&token, &[(call.clone(), result.clone())])
                            .await
                            .unwrap();
                        assert_eq!(
                            broker.retained_result(&token, &poll).unwrap(),
                            Some(result.clone())
                        );
                    }
                    drop(broker);
                    let restored = Broker::new(journal.clone());
                    restored.reconcile_persisted_integrated_receipts().unwrap();
                    let state: ActiveEffects = journal
                        .read(&digest(&"native-effect-claims-v1"))
                        .unwrap()
                        .unwrap();
                    assert!(
                        state.claims.contains_key(&claim),
                        "unknown original poll must retain the local mirror"
                    );
                    assert_eq!(state.claims[&claim].session_id, running.then_some(51));
                    assert_eq!(
                        restored.effect_status("owner").unwrap()["shared"]["unconfirmed_effects"],
                        1
                    );
                    if running {
                        restored
                            .admit_external_tool_with_execution(
                                "owner",
                                "collect",
                                &poll.name,
                                &poll.input,
                                &root,
                                Some(&facts),
                            )
                            .await
                            .unwrap();
                        restored
                            .settle_external_tools(
                                "owner",
                                &[("collect".into(), waited("collect", Some(51), 0))],
                            )
                            .await
                            .unwrap();
                        assert!(!restored.has_pending_processes(&["owner"]).unwrap());
                        assert_eq!(
                            restored.effect_status("owner").unwrap()["shared"]
                                ["unconfirmed_effects"],
                            0
                        );
                    }
                    fs::remove_dir_all(root).unwrap();
                }
            }
        }
    }

    #[tokio::test]
    async fn native_scope_missing_foreign_and_legacy_nonprocess_facts_never_downgrade_to_generic() {
        for name in [
            "apply_patch",
            "functions.apply_patch",
            "view_image",
            "functions.view_image",
            "exec_command",
            "functions.exec_command",
            "write_stdin",
            "functions.write_stdin",
            "clock.curr_time",
            "clock.sleep",
            "ordinary.custom",
        ] {
            for mode in ["legacy1", "legacy2", "missing", "foreign", "matching-hook"] {
                if name == "ordinary.custom" && mode == "matching-hook" {
                    continue;
                }
                let (root, journal, _) = fixture();
                let mut request=crate::protocol::Request::parse(json!({"stream":true,"model":"chatgpt-desktop/gpt-6",
                    "input":"fixture","tools":[{"type":"function","name":name,"parameters":{"type":"object"}}]}),
                    &crate::config::Config::default(),Some("owned-thread")).unwrap();
                let owner = crate::cdp::Process::read(std::process::id()).unwrap();
                let mut facts = native_effect::hook_fixture(&root);
                request.turn = Some("owned-turn".into());
                facts.thread = request.thread.clone();
                facts.turn = "owned-turn".into();
                let expected = match mode {
                    "legacy1" => {
                        facts.schema = 1;
                        facts.hook_free_tools.clear();
                        "native_execution_generation_unsupported"
                    }
                    "legacy2" => {
                        facts.schema = 2;
                        facts.hook_free_tools.retain(|t| t != "write_stdin");
                        "native_execution_generation_unsupported"
                    }
                    "foreign" => {
                        facts.pid += 1;
                        "native_execution_completion_unavailable"
                    }
                    "matching-hook" => {
                        facts.hook_free_tools.clear();
                        "native_execution_completion_unavailable"
                    }
                    _ => "native_execution_completion_unavailable",
                };
                request.execution = (mode != "missing").then_some(facts);
                request.bind_native_scope(&root, &owner).unwrap();
                let scope = request
                    .native_scope
                    .as_ref()
                    .expect("Native origin survives missing/foreign metadata");
                if matches!(mode, "foreign" | "missing") {
                    assert!(scope.execution.is_none());
                }
                let broker = Broker::new(journal.clone());
                let token = broker
                    .register_scoped(
                        "owner",
                        request.tools.clone(),
                        false,
                        Some(&scope.workspace),
                    )
                    .await
                    .unwrap();
                broker
                    .bind_execution(&token, scope.execution.clone())
                    .await
                    .unwrap();
                let call = Arguments {
                    call_id: "refused".into(),
                    name: name.into(),
                    input: "{}".into(),
                    report: String::new(),
                };
                broker.enqueue(&token, &call).await.unwrap();
                assert!(broker.offer(&token).await.unwrap().is_empty());
                let result = broker.retained_result(&token, &call).unwrap().unwrap();
                assert_eq!(result["error"], expected, "{name}/{mode}");
                assert_eq!(result["executed"], false);
                assert_eq!(
                    broker
                        .admit_external_native_tool(
                            "owner",
                            "external",
                            &request.tools[name],
                            &call.input,
                            &root,
                            scope.execution.as_ref()
                        )
                        .await,
                    Err(Error(expected))
                );
                let state: ActiveEffects = journal
                    .read(&digest(&"native-effect-claims-v1"))
                    .unwrap()
                    .unwrap_or_default();
                assert!(state.claims.is_empty());
                assert!(state.external.is_empty());
                broker.enqueue(&token, &call).await.unwrap();
                assert_eq!(broker.retained_result(&token, &call).unwrap(), Some(result));
                fs::remove_dir_all(root).unwrap();
            }
        }
        // Generic transport is not a Native origin and has no privileged scope.
        for name in ["ordinary.custom", "tool_search"] {
            let (root, journal, _) = fixture();
            let broker = Broker::new(journal);
            let tools = crate::protocol::Request::parse(
                json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":"generic",
                "tools":[{"type":"function","name":name,"parameters":{"type":"object"}}]}),
                &crate::config::Config::default(),
                Some("generic"),
            )
            .unwrap()
            .tools;
            assert_eq!(tools[name].kind, crate::protocol::ToolKind::Function);
            let token = broker.register("generic", tools, false).await.unwrap();
            let call = Arguments {
                call_id: "ordinary".into(),
                name: name.into(),
                input: "{}".into(),
                report: String::new(),
            };
            broker.enqueue(&token, &call).await.unwrap();
            assert_eq!(broker.offer(&token).await.unwrap().len(), 1);
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn only_actual_tool_search_payload_bypasses_native_generation_and_busy() {
        for actual in [false, true] {
            let (root, journal, _) = fixture();
            let broker = Broker::new(journal.clone());
            let search = if actual {
                json!({"type":"tool_search"})
            } else {
                json!({"type":"function","name":"tool_search","parameters":{"type":"object"}})
            };
            let tools=crate::protocol::Request::parse(json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":"fixture",
                "tools":[{"type":"function","name":"functions.exec_command","parameters":{"type":"object"}},search]}),
                &crate::config::Config::default(),Some("owner")).unwrap().tools;
            let token = broker
                .register_scoped("owner", tools.clone(), false, Some(&root))
                .await
                .unwrap();
            broker
                .bind_execution(&token, Some(native_effect::hook_fixture(&root)))
                .await
                .unwrap();
            let exec = Arguments {
                call_id: "active".into(),
                name: "functions.exec_command".into(),
                input: r#"{"cmd":"opaque owned process"}"#.into(),
                report: String::new(),
            };
            let id = accepted(&broker, &token, &exec).await;
            let original = json!({"output":"Native completion: null\nProcess running with session ID 51\nOutput:\noriginal"});
            broker
                .settle(&token, &[(id.clone(), original.clone())])
                .await
                .unwrap();
            broker.bind_execution(&token, None).await.unwrap();
            let call = Arguments {
                call_id: "metadata".into(),
                name: "tool_search".into(),
                input: r#"{"query":"metadata","limit":1}"#.into(),
                report: String::new(),
            };
            broker.enqueue(&token, &call).await.unwrap();
            let offered = broker.offer(&token).await.unwrap();
            assert_eq!(offered.len(), usize::from(actual));
            let external = broker
                .admit_external_native_tool(
                    "owner",
                    "external-search",
                    &tools["tool_search"],
                    &call.input,
                    &root,
                    None,
                )
                .await;
            assert_eq!(
                external,
                if actual {
                    Ok(())
                } else {
                    Err(Error("native_tool_identity_unsupported"))
                }
            );
            if !actual {
                assert_eq!(
                    broker.retained_result(&token, &call).unwrap().unwrap()["error"],
                    "native_tool_identity_unsupported"
                );
            }
            assert_eq!(
                broker.retained_result(&token, &exec).unwrap(),
                Some(original)
            );
            let state: ActiveEffects = journal
                .read(&digest(&"native-effect-claims-v1"))
                .unwrap()
                .unwrap();
            assert_eq!(state.claims.len(), 1);
            assert!(state.claims.contains_key(&id));
            assert_eq!(
                broker.effect_status("owner").unwrap()["shared"]["unconfirmed_effects"],
                1
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn legacy_empty_poll_refuses_without_mutating_existing_process_or_readback() {
        for schema in [1, 2] {
            let (root, journal, tools) = fixture();
            let facts = native_effect::hook_fixture(&root);
            let broker = Broker::new(journal.clone());
            let token = broker
                .register_scoped("owner", tools, false, Some(&root))
                .await
                .unwrap();
            broker
                .bind_execution(&token, Some(facts.clone()))
                .await
                .unwrap();
            let exec = Arguments {
                call_id: "original".into(),
                name: "functions.exec_command".into(),
                input: r#"{"cmd":"owned opaque process"}"#.into(),
                report: String::new(),
            };
            let id = accepted(&broker, &token, &exec).await;
            let original = json!({"output":"Native completion: null\nProcess running with session ID 51\nOutput:\noriginal"});
            broker
                .settle(&token, &[(id.clone(), original.clone())])
                .await
                .unwrap();
            let key = digest(&"native-effect-claims-v1");
            let before = journal.read::<Value>(&key).unwrap();
            let mut old = facts.clone();
            old.schema = schema;
            if schema == 1 {
                old.hook_free_tools.clear();
            } else {
                old.hook_free_tools.retain(|t| t != "write_stdin");
            }
            broker
                .bind_execution(&token, Some(old.clone()))
                .await
                .unwrap();
            let poll = Arguments {
                call_id: "legacy-poll".into(),
                name: "functions.write_stdin".into(),
                input: r#"{"session_id":51,"chars":""}"#.into(),
                report: String::new(),
            };
            broker.enqueue(&token, &poll).await.unwrap();
            assert!(broker.offer(&token).await.unwrap().is_empty());
            assert_eq!(
                broker.retained_result(&token, &poll).unwrap().unwrap()["error"],
                "native_execution_generation_unsupported"
            );
            assert_eq!(
                broker
                    .admit_external_tool_with_execution(
                        "owner",
                        "external-legacy-poll",
                        &poll.name,
                        &poll.input,
                        &root,
                        Some(&old)
                    )
                    .await,
                Err(Error("native_execution_generation_unsupported"))
            );
            assert_eq!(
                journal.read::<Value>(&key).unwrap(),
                before,
                "refusal cannot mutate an existing Writer claim"
            );
            assert_eq!(
                broker.retained_result(&token, &exec).unwrap(),
                Some(original)
            );
            assert_eq!(
                broker.effect_status("owner").unwrap()["shared"]["unconfirmed_effects"],
                1
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn process_dispatch_refusal_is_pre_offer_durable_and_preserves_original_replay() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for mode in ["legacy1", "legacy2", "missing", "matching-hook"] {
                let (root, journal, tools) = fixture_with_process_aliases(exec_tool, poll_tool);
                let supported = native_effect::hook_fixture(&root);
                let mut facts = supported.clone();
                let expected = match mode {
                    "legacy1" => {
                        facts.schema = 1;
                        facts.hook_free_tools.clear();
                        "native_execution_generation_unsupported"
                    }
                    "legacy2" => {
                        facts.schema = 2;
                        facts.hook_free_tools.retain(|tool| tool != "write_stdin");
                        "native_execution_generation_unsupported"
                    }
                    "matching-hook" => {
                        facts.hook_free_tools.clear();
                        "native_execution_completion_unavailable"
                    }
                    _ => "native_execution_completion_unavailable",
                };
                let execution = (mode != "missing").then_some(facts);
                let broker = Broker::new(journal.clone());
                let token = broker
                    .register_scoped("historical-owner", tools, false, Some(&root))
                    .await
                    .unwrap();
                broker
                    .bind_execution(&token, execution.clone())
                    .await
                    .unwrap();
                let call = Arguments {
                    call_id: "original-refusal".into(),
                    name: exec_tool.into(),
                    input: r#"{"cmd":"an opaque owned process"}"#.into(),
                    report: String::new(),
                };
                broker.enqueue(&token, &call).await.unwrap();
                assert!(
                    broker.offer(&token).await.unwrap().is_empty(),
                    "{mode}: unsupported work must never be forwarded"
                );
                let result = broker.retained_result(&token, &call).unwrap().unwrap();
                assert_eq!(result["error"], expected);
                assert_eq!(result["executed"], false);
                let saved: Turn = journal
                    .read(&digest(&("mcp-turn-v1", &token)))
                    .unwrap()
                    .unwrap();
                assert!(!saved.calls[&call.call_id].offered);
                assert_eq!(saved.calls[&call.call_id].shared_reserved, Some(false));
                let claims: ActiveEffects = journal
                    .read(&digest(&"native-effect-claims-v1"))
                    .unwrap()
                    .unwrap_or_default();
                assert!(claims.claims.is_empty());
                assert_eq!(
                    broker
                        .admit_external_tool_with_execution(
                            "historical-owner",
                            "external-refusal",
                            &call.name,
                            &call.input,
                            &root,
                            execution.as_ref()
                        )
                        .await,
                    Err(Error(expected))
                );
                broker
                    .bind_execution(&token, Some(supported.clone()))
                    .await
                    .unwrap();
                broker.enqueue(&token, &call).await.unwrap();
                assert!(broker.offer(&token).await.unwrap().is_empty());
                assert_eq!(broker.retained_result(&token, &call).unwrap(), Some(result));
                let mut changed = call.clone();
                changed.input = r#"{"cmd":"changed"}"#.into();
                assert_eq!(
                    broker.enqueue(&token, &changed).await,
                    Err(Error("mcp_call_changed"))
                );
                let mut fresh = call.clone();
                fresh.call_id = "current-runtime-new-input".into();
                let id = accepted(&broker, &token, &fresh).await;
                let claims: ActiveEffects = journal
                    .read(&digest(&"native-effect-claims-v1"))
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    claims.claims[&id].effect,
                    Effect::Opaque,
                    "hook-free opaque commands remain supported"
                );
                let original = waited(&id, None, 7);
                broker
                    .settle(&token, &[(id.clone(), original.clone())])
                    .await
                    .unwrap();
                broker.bind_execution(&token, None).await.unwrap();
                broker.enqueue(&token, &fresh).await.unwrap();
                assert!(broker.offer(&token).await.unwrap().is_empty());
                assert_eq!(
                    broker.retained_result(&token, &fresh).unwrap(),
                    Some(original)
                );
                assert!(!broker.has_pending_processes(&["historical-owner"]).unwrap());
                fs::remove_dir_all(root).unwrap();
            }
        }
    }

    #[tokio::test]
    async fn running_continuation_receipt_replay_keeps_original_process_reserved() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for (external, rejected) in [(false, false), (true, false), (false, true), (true, true)]
            {
                let (root, journal, tools) = fixture_with_process_aliases(exec_tool, poll_tool);
                let facts = native_effect::compiler_fixture(&root);
                let broker = Broker::new(journal.clone());
                let token = broker
                    .register_scoped("owner", tools, false, Some(&root))
                    .await
                    .unwrap();
                broker
                    .bind_execution(&token, Some(facts.clone()))
                    .await
                    .unwrap();
                let exec = Arguments {
                    call_id: "start".into(),
                    name: exec_tool.into(),
                    input: r#"{"cmd":"owned interactive process"}"#.into(),
                    report: String::new(),
                };
                let id = accepted(&broker, &token, &exec).await;
                broker.settle(&token,&[(id.clone(),json!({"output":"Native completion: null\nProcess running with session ID 51\nOutput:\n"}))]).await.unwrap();
                let poll = Arguments {
                    call_id: "poll".into(),
                    name: poll_tool.into(),
                    input: r#"{"session_id":51,"chars":"input"}"#.into(),
                    report: String::new(),
                };
                let poll_id = if external {
                    broker
                        .admit_external_tool_with_execution(
                            "owner",
                            "poll",
                            &poll.name,
                            &poll.input,
                            &root,
                            Some(&facts),
                        )
                        .await
                        .unwrap();
                    "poll".to_owned()
                } else {
                    accepted(&broker, &token, &poll).await
                };
                let proof = json!({"schema":"codex-native-completion-v1","call_id":poll_id,"session_id":51,
                    "outcome":"session_running","exit_code":null,"hooks":"absent"});
                let result = if rejected {
                    rejected_before_backend(&poll_id)
                } else {
                    json!({"call_id":poll_id,"type":"function_call_output",
                        "output":format!("Native completion: {proof}\nProcess running with session ID 51\nOutput:\n")})
                };
                // Model the result commit before shared release, using only this
                // owned original receipt, then let normal restart reconciliation run.
                if external {
                    let key = digest(&"native-effect-claims-v1");
                    let mut state: ActiveEffects = journal.read(&key).unwrap().unwrap();
                    state
                        .external
                        .get_mut(&external_effect_id("owner", &poll_id))
                        .unwrap()
                        .completion = Some(ExternalCompletion::from_result_for(
                        &poll_id,
                        Some(51),
                        &result,
                    ));
                    journal.write(&key, &state).unwrap();
                } else {
                    let mut saved: Turn = journal
                        .read(&digest(&("mcp-turn-v1", &token)))
                        .unwrap()
                        .unwrap();
                    saved.calls.get_mut("poll").unwrap().result = Some(result.clone());
                    broker.persist(&token, &saved).unwrap();
                }
                drop(broker);
                let recovered = Broker::new(journal.clone());
                recovered.reconcile_persisted_integrated_receipts().unwrap();
                let state: ActiveEffects = journal
                    .read(&digest(&"native-effect-claims-v1"))
                    .unwrap()
                    .unwrap();
                assert_eq!(state.claims.len(), 1);
                assert_eq!(state.claims[&id].effect, Effect::Opaque);
                assert_eq!(state.claims[&id].session_id, Some(51));
                assert_eq!(
                    recovered.effect_status("owner").unwrap()["shared"]["unconfirmed_effects"],
                    1
                );
                assert_eq!(
                    recovered
                        .admit_external_tool("other", "must-wait", exec_tool, &exec.input, &root)
                        .await,
                    Err(Error("native_resource_busy"))
                );
                fs::remove_dir_all(root).unwrap();
            }
        }
    }

    #[test]
    fn completion_safety_exact_exit_code_survives_but_old_reader_drops_authority() {
        for code in [0, 7, 137] {
            let raw = json!({"output":format!("Process exited with code {code}\nOutput:\n")});
            let saved = ExternalCompletion::from_result("original", &raw);
            assert_eq!(saved.confirmed_exit(), Some(code));
            assert_eq!(
                crate::native_claims::process_result(&saved.marker()),
                (None, Some(code))
            );
            let mut serialized = serde_json::to_value(&saved).unwrap();
            serialized.as_object_mut().unwrap().remove("native_result");
            let legacy: ExternalCompletion = serde_json::from_value(serialized).unwrap();
            assert_eq!(legacy.fingerprint, saved.fingerprint);
            assert_eq!(legacy.confirmed_exit(), None);
            assert_eq!(native_process_result(&legacy.marker()), (None, false));
        }
        for code in [-1, 124] {
            let saved = ExternalCompletion::from_result(
                "original",
                &json!({"output":format!("Process exited with code {code}\nOutput:\n")}),
            );
            assert_eq!(saved.confirmed_exit(), None);
        }
    }

    #[test]
    fn completion_safety_legacy_boolean_cannot_recreate_exit_or_non_dispatch() {
        for old in [
            json!({"fingerprint":"original","running":null,"exited":true}),
            json!({"fingerprint":"original","running":null,"exited":false,"not_spawned":true}),
        ] {
            let saved: ExternalCompletion = serde_json::from_value(old.clone()).unwrap();
            let marker = saved.marker();
            assert_eq!(
                native_process_result(&marker),
                (None, false),
                "legacy evidence lost the exit code"
            );
            assert!(
                !crate::native_claims::process_observation(&marker, "original", None).not_spawned,
                "legacy rejection lost dispatch provenance"
            );
            assert_eq!(saved.fingerprint, "original");
        }
    }

    fn waited(call: &str, session: Option<u64>, exit: i64) -> Value {
        let proof = json!({"schema":"codex-native-completion-v1","call_id":call,
            "session_id":session,"outcome":"wait_completed","exit_code":exit,"hooks":"absent"});
        json!({"type":"function_call_output","call_id":call,
            "output":format!("Native completion: {proof}\nProcess exited with code {exit}\nOutput:\n")})
    }
    fn session_running(call: &str, session: u64) -> Value {
        let proof = json!({"schema":"codex-native-completion-v1","call_id":call,
            "session_id":session,"outcome":"session_running","exit_code":null,"hooks":"absent"});
        json!({"type":"function_call_output","call_id":call,
            "output":format!("Native completion: {proof}\nProcess running with session ID {session}\nOutput:\n")})
    }
    fn rejected_before_backend(call: &str) -> Value {
        let proof = json!({"schema":"codex-native-completion-v1","call_id":call,
            "session_id":null,"outcome":"pre_backend_rejected","exit_code":null,"hooks":"absent"});
        json!({"type":"function_call_output","call_id":call,
            "output":format!("Native completion: {proof}\nOutput:\ninitial admission rejected")})
    }
    fn fixture_with_process_aliases(
        exec_tool: &str,
        poll_tool: &str,
    ) -> (PathBuf, Arc<Journal>, BTreeMap<String, Tool>) {
        let (root, journal, mut tools) = fixture();
        let declarations = json!([
            {"type":"function","name":"exec_command","parameters":{"type":"object"}},
            {"type":"function","name":"write_stdin","parameters":{"type":"object"}}
        ]);
        let declarations = if exec_tool == "functions.exec_command" {
            json!([{"type":"namespace","name":"functions","tools":declarations}])
        } else {
            declarations
        };
        let parsed = crate::protocol::Request::parse(
            json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":"owned alias fixture",
                "tools":declarations}),
            &crate::config::Config::default(),
            Some("native-effect-test"),
        )
        .unwrap()
        .tools;
        assert!(parsed.contains_key(exec_tool));
        assert!(parsed.contains_key(poll_tool));
        tools.remove("functions.exec_command");
        tools.remove("functions.write_stdin");
        tools.extend(parsed);
        (root, journal, tools)
    }

    fn fixture() -> (PathBuf, Arc<Journal>, BTreeMap<String, Tool>) {
        let root = std::env::temp_dir().join(format!("broker-resource-{}", random_id().unwrap()));
        fs::create_dir(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let journal = Arc::new(Journal::open(&root.join("owner")).unwrap());
        let catalog = crate::protocol::Request::parse(
            json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":[{"role":"user","content":"test"}],
                "tools":[
                    {"type":"function","name":"functions.exec_command","parameters":{"type":"object"}},
                    {"type":"function","name":"functions.write_stdin","parameters":{"type":"object"}},
                    {"type":"custom","name":"functions.apply_patch","format":{"type":"text"}}
                ]}),
            &crate::config::Config::default(),
            Some("native-effect-test"),
        )
        .unwrap()
        .tools;
        (root, journal, catalog)
    }
    fn patch(name: &str, old: &str, new: &str) -> Arguments {
        Arguments {
            call_id: format!("patch-{name}-{new}"),
            name: "functions.apply_patch".into(),
            input: format!(
                "*** Begin Patch\n*** Update File: {name}\n@@\n-{old}\n+{new}\n*** End Patch\n"
            ),
            report: String::new(),
        }
    }
    async fn accepted(broker: &Broker, token: &str, call: &Arguments) -> String {
        if native_effect::process_tool(&call.name).is_some()
            || matches!(call.name.as_str(), "apply_patch" | "functions.apply_patch")
        {
            let workspace = {
                let registry = broker.registry.lock().await;
                let turn = registry.turns.get(token).unwrap();
                turn.execution
                    .is_none()
                    .then(|| turn.workspace.clone())
                    .flatten()
            };
            if let Some(root) = workspace {
                broker
                    .bind_execution(token, Some(native_effect::hook_fixture(&root)))
                    .await
                    .unwrap();
            }
        }
        broker.enqueue(token, call).await.unwrap();
        let offered = broker.offer(token).await.unwrap();
        assert_eq!(offered.len(), 1);
        offered[0]["id"].as_str().unwrap().to_owned()
    }

    #[tokio::test]
    async fn queued_legacy_read_requires_current_hook_proof_without_changing_retry_identity() {
        let (root, journal, _) = fixture();
        let tools = crate::protocol::Request::parse(
            json!({"stream":true,"model":"chatgpt-desktop/gpt-6",
                "input":[{"role":"user","content":"fixture"}],
                "tools":[{"type":"function","name":"clock.curr_time","parameters":{"type":"object"}}]}),
            &crate::config::Config::default(), Some("legacy-read-fixture"),
        ).unwrap().tools;
        let broker = Broker::new(journal.clone());
        broker
            .admit_external_tool(
                "writer",
                "held",
                "functions.exec_command",
                r#"{"cmd":"unknown shell effect"}"#,
                &root,
            )
            .await
            .unwrap();
        let token = broker
            .register_scoped("legacy", tools, false, Some(&root))
            .await
            .unwrap();
        let call = Arguments {
            call_id: "original-clock".into(),
            name: "clock.curr_time".into(),
            input: "{}".into(),
            report: String::new(),
        };
        broker.enqueue(&token, &call).await.unwrap();
        // The old Provider stored ordinary clock calls as Read without hook
        // facts. Recreate that pending journal record, not a substitute tool.
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&token).unwrap();
            turn.calls.get_mut(&call.call_id).unwrap().effect = Some(Effect::Read);
            broker.persist(&token, turn).unwrap();
        }
        broker.enqueue(&token, &call).await.unwrap();
        assert!(broker.offer(&token).await.unwrap().is_empty());
        let result = broker.invoke(&token, call.clone()).await.unwrap();
        assert_eq!(result["error"], "native_execution_completion_unavailable");
        assert_eq!(result["executed"], false);
        assert_eq!(
            broker.effect_status("writer").unwrap()["shared"]["unconfirmed_effects"],
            1
        );
        assert_eq!(broker.invoke(&token, call.clone()).await.unwrap(), result);
        assert!(broker.offer(&token).await.unwrap().is_empty());
        let stored: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        let stored = &stored.calls[&call.call_id];
        assert_eq!(stored.fingerprint, call.fingerprint());
        assert_eq!(stored.effect, Some(Effect::Read));
        assert!(!stored.offered);
        assert_eq!(stored.result, Some(result.clone()));
        broker
            .bind_execution(&token, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        assert_eq!(
            broker.invoke(&token, call.clone()).await.unwrap(),
            result,
            "new capabilities cannot retroactively execute the same refused call"
        );

        broker
            .settle_external_tools("writer", &[("held".into(), waited("held", None, 0))])
            .await
            .unwrap();
        let call = Arguments {
            call_id: "another-intended-clock".into(),
            ..call
        };
        broker.enqueue(&token, &call).await.unwrap();
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&token).unwrap();
            turn.calls.get_mut(&call.call_id).unwrap().effect = Some(Effect::Read);
            broker.persist(&token, turn).unwrap();
        }
        let offered = broker.offer(&token).await.unwrap();
        assert_eq!(offered.len(), 1);
        let status = broker.effect_status("legacy").unwrap();
        assert_eq!(
            status["shared"]["unconfirmed_effects"], 1,
            "hook absence does not attest the configured MCP clock backend as read-only"
        );
        let current: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        assert_eq!(current.calls[&call.call_id].effect, Some(Effect::Read));
        assert_eq!(current.calls[&call.call_id].shared_reserved, Some(true));
        let original_result = json!({"output":"original clock response"});
        broker
            .settle(
                &token,
                &[(
                    offered[0]["id"].as_str().unwrap().into(),
                    original_result.clone(),
                )],
            )
            .await
            .unwrap();
        assert_eq!(
            broker.invoke(&token, call.clone()).await.unwrap(),
            original_result
        );
        assert_eq!(broker.invoke(&token, call).await.unwrap(), original_result);
        assert_eq!(
            broker.effect_status("legacy").unwrap()["shared"]["unconfirmed_effects"],
            0
        );
        assert!(broker.offer(&token).await.unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_patch_hook_proof_is_required_before_independent_shared_admission() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("a"), "old\n").unwrap();
        fs::write(root.join("b"), "old\n").unwrap();
        let broker = Broker::new(journal);
        let token = broker
            .register_scoped("first", tools, false, Some(&root))
            .await
            .unwrap();
        accepted(&broker, &token, &patch("a", "old", "new")).await;
        let second = patch("b", "old", "new");
        assert_eq!(
            broker
                .admit_external_tool_with_execution(
                    "second",
                    "unproven",
                    &second.name,
                    &second.input,
                    &root,
                    None,
                )
                .await,
            Err(Error("native_execution_completion_unavailable"))
        );
        let mut facts = native_effect::hook_fixture(&root);
        facts.hook_free_tools = vec!["exec_command".into()];
        assert_eq!(
            broker
                .admit_external_tool_with_execution(
                    "second",
                    "matching-hook",
                    &second.name,
                    &second.input,
                    &root,
                    Some(&facts),
                )
                .await,
            Err(Error("native_execution_completion_unavailable"))
        );
        facts.hook_free_tools.push("apply_patch".into());
        broker
            .admit_external_tool_with_execution(
                "second",
                "proven",
                &second.name,
                &second.input,
                &root,
                Some(&facts),
            )
            .await
            .unwrap();
        let status = broker.effect_status("second").unwrap();
        assert_eq!(
            status["unconfirmed_effects"], 2,
            "both independent calls retain their original claims"
        );
        assert!(status["active"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["resource_kind"] == "bounded"));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn shell_read_dependencies_survive_broker_and_shared_admission() {
        let (root, journal, tools) = fixture();
        let mut execution = native_effect::compiler_fixture(&root);
        let before = root.join("before");
        fs::create_dir(&before).unwrap();
        execution.path = format!("{}:{}", before.display(), execution.path);
        fs::write(root.join("a.c"), "int main(void){return 0;}\n").unwrap();
        let broker = Broker::new(journal);
        let token = broker
            .register_scoped("reader", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&token, Some(execution.clone()))
            .await
            .unwrap();
        let read = Arguments {
            call_id: "read".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"cat a.c","login":false}).to_string(),
            report: String::new(),
        };
        let read_id = accepted(&broker, &token, &read).await;
        assert_eq!(
            broker.effect_status("reader").unwrap()["unconfirmed_effects"],
            1
        );
        let other = broker
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        assert!(
            !broker.catalog(&other).await.unwrap().is_empty(),
            "work creation and metadata remain available"
        );
        let input =
            json!({"cmd":"clang a.c -o before/cat","workdir":root,"login":false}).to_string();
        let shadow = native_effect::classify_with_execution(
            "functions.exec_command",
            &input,
            Some(&root),
            Some(&execution),
        );
        assert!(matches!(shadow, Effect::Paths(_)));
        let offer = |id: &str| {
            SharedNativeGuard::offer_external_with_effect(
                &root,
                broker.shared_guard_home().as_deref(),
                "writer",
                id,
                "functions.exec_command",
                &input,
                shadow.clone(),
            )
        };
        assert_eq!(
            offer("shadow").err().as_deref(),
            Some("native_resource_busy"),
            "a separate mediated writer must see the read's shared claim"
        );
        broker
            .settle(
                &token,
                &[(
                    read_id,
                    json!({"output":"Process exited with code 0\nOutput:\nsource"}),
                )],
            )
            .await
            .unwrap();
        assert_eq!(
            broker.effect_status("reader").unwrap()["unconfirmed_effects"],
            0
        );
        offer("shadow").unwrap();
        let mut while_writing = read.clone();
        while_writing.call_id = "read-during-shadow-publication".into();
        broker.enqueue(&token, &while_writing).await.unwrap();
        assert!(broker.offer(&token).await.unwrap().is_empty());
        let denied = broker.invoke(&token, while_writing).await.unwrap();
        assert_eq!(denied["error"], "native_resource_busy");
        assert_eq!(denied["executed"], false);
        SharedNativeGuard::settle_external(
            &root,
            broker.shared_guard_home().as_deref(),
            "writer",
            "shadow",
            "functions.exec_command",
            &input,
            &json!({"output":"Process exited with code 0"}),
        )
        .unwrap();
        let mut stale = read;
        stale.call_id = "read-prepared-before-lookup-changed".into();
        broker.enqueue(&token, &stale).await.unwrap();
        fs::write(before.join("cat"), "changed after read preparation").unwrap();
        assert!(broker.offer(&token).await.unwrap().is_empty());
        let denied = broker.invoke(&token, stale).await.unwrap();
        assert_eq!(denied["error"], "native_resource_version_conflict");
        assert_eq!(denied["executed"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn native_execution_proof_is_persisted_and_shared_without_reactivating_turns() {
        let (root, journal, tools) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        fs::write(root.join("a.c"), "int main(void){return 0;}\n").unwrap();
        fs::write(root.join("b.c"), "int main(void){return 1;}\n").unwrap();
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("compiler-owner", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&token, Some(execution.clone()))
            .await
            .unwrap();
        let stored: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        assert_eq!(stored.execution, Some(execution.clone()));
        let reopened = Broker::new(journal);
        let command = |id: &str, source: &str, output: &str| Arguments {
            call_id: id.into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":format!("clang -O0 {source} -o {output}"),
                "workdir":root,"login":false})
            .to_string(),
            report: String::new(),
        };
        let first = command("compile-a", "a.c", "a.out");
        assert_eq!(
            reopened.enqueue(&token, &first).await,
            Err(Error("mcp_turn_unavailable")),
            "persisted execution facts cannot reactivate an old live capability"
        );
        let first_id = accepted(&broker, &token, &first).await;
        assert_eq!(
            broker.bind_execution(&token, None).await,
            Err(Error("mcp_pending_catalog_change")),
            "unsettled effects keep their original execution facts"
        );
        let second = command("compile-b", "b.c", "b.out");
        reopened
            .admit_external_tool_with_execution(
                "other-owner",
                &second.call_id,
                &second.name,
                &second.input,
                &root,
                Some(&execution),
            )
            .await
            .unwrap();
        let shared_output = command("compile-collision", "b.c", "a.out");
        assert_eq!(
            reopened
                .admit_external_tool_with_execution(
                    "third-owner",
                    &shared_output.call_id,
                    &shared_output.name,
                    &shared_output.input,
                    &root,
                    Some(&execution),
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        let source_edit = patch(
            "a.c",
            "int main(void){return 0;}",
            "int main(void){return 2;}",
        );
        assert_eq!(
            reopened
                .admit_external_tool(
                    "third-owner",
                    &source_edit.call_id,
                    &source_edit.name,
                    &source_edit.input,
                    &root,
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        broker
            .settle(
                &token,
                &[(first_id, json!({"output":"Process exited with code 0"}))],
            )
            .await
            .unwrap();
        broker.bind_execution(&token, None).await.unwrap();
        reopened
            .settle_external_tools(
                "other-owner",
                &[(
                    second.call_id,
                    json!({"output":"Process exited with code 0"}),
                )],
            )
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn normalized_startup_failure_retains_its_original_uncertain_effect() {
        let (root, journal, tools) = fixture();
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("startup-work", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let exec = Arguments {
            call_id: "missing-workdir".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"printf must-not-run", "workdir":root.join("missing")}).to_string(),
            report: String::new(),
        };
        let id = accepted(&broker, &token, &exec).await;
        // AppHost's production result envelope, as retained by McpBridge.
        let failure = json!({"id":id,"name":exec.name,
            "output":"exec_command failed: CreateProcess { message: \"Rejected(\\\"Failed to create unified exec process: No such file or directory (os error 2)\\\")\" }"});
        broker
            .settle(&token, &[(id.clone(), failure.clone())])
            .await
            .unwrap();
        assert_eq!(
            broker.effect_status("startup-work").unwrap()["unconfirmed_effects"],
            1
        );
        // Duplicate and late receipt delivery cannot offer the command again.
        broker
            .settle(&token, &[(id.clone(), failure.clone())])
            .await
            .unwrap();
        assert_eq!(broker.invoke(&token, exec.clone()).await.unwrap(), failure);
        assert!(broker.offer(&token).await.unwrap().is_empty());
        // A generic setup/transport rejection may follow backend effects.
        // Re-querying the identical original error must not infer non-dispatch.
        broker.settle(&token, &[(id, failure)]).await.unwrap();
        assert_eq!(
            broker.effect_status("startup-work").unwrap()["unconfirmed_effects"],
            1
        );
        let mut next = exec.clone();
        next.call_id = "other".into();
        let other = broker
            .register_scoped("other-work", tools, false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&other, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        next.call_id = "other".into();
        broker.enqueue(&other, &next).await.unwrap();
        assert!(broker.offer(&other).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&other, next).await.unwrap()["error"],
            "native_resource_busy"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_conflicting_native_tool_returns_pre_effect_receipt_without_late_dispatch() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        let owner = broker
            .register_scoped("owner", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let contender = broker
            .register_scoped("contender", tools, false, Some(&root))
            .await
            .unwrap();
        let first = patch("file", "alpha", "OWNER");
        let original_id = accepted(&broker, &owner, &first).await;
        let second = patch("file", "alpha", "SECOND");
        broker
            .bind_execution(&contender, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        broker.enqueue(&contender, &second).await.unwrap();
        assert!(broker.offer(&contender).await.unwrap().is_empty());
        let recorded: Turn = broker
            .journal
            .read(&digest(&("mcp-turn-v1", &contender)))
            .unwrap()
            .unwrap();
        let call = &recorded.calls[&second.call_id];
        assert!(!call.offered);
        assert_eq!(
            call.result.as_ref().unwrap()["error"],
            "native_resource_busy"
        );
        assert_eq!(call.result.as_ref().unwrap()["executed"], false);
        assert_eq!(call.result.as_ref().unwrap()["retryable"], true);
        broker
            .settle(
                &owner,
                &[(original_id, json!({"output":"Process exited with code 0"}))],
            )
            .await
            .unwrap();
        assert!(
            broker.offer(&contender).await.unwrap().is_empty(),
            "a pre-effect rejection cannot silently execute after the caller has left"
        );
        let retry = patch("file", "alpha", "RETRIED");
        assert_eq!(broker.enqueue(&contender, &retry).await, Ok(()));
        assert_eq!(broker.offer(&contender).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn nc_crashed_unoffered_external_reservation_can_be_recovered() {
        if let (Ok(cwd), Ok(home)) = (
            std::env::var("NATIVE_UNOFFERED_PROBE_CWD"),
            std::env::var("NATIVE_UNOFFERED_PROBE_HOME"),
        ) {
            let command = patch("file", "alpha", "ALPHA");
            SharedNativeGuard::offer_external(
                Path::new(&cwd),
                Some(Path::new(&home)),
                "app-host",
                &external_effect_id("app-host", "original"),
                &command.name,
                &command.input,
            )
            .unwrap();
            return;
        }
        let (root, journal, _) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        let home = broker.shared_guard_home().unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "mcp::native_concurrency_tests::nc_crashed_unoffered_external_reservation_can_be_recovered",
            ])
            .env("NATIVE_UNOFFERED_PROBE_CWD", &root)
            .env("NATIVE_UNOFFERED_PROBE_HOME", &home)
            .status()
            .unwrap();
        assert!(child.success());
        let command = patch("file", "alpha", "ALPHA");
        broker
            .admit_external_tool("app-host", "original", &command.name, &command.input, &root)
            .await
            .unwrap();
        assert_eq!(
            broker
                .admit_external_tool("other", "conflict", &command.name, &command.input, &root,)
                .await,
            Err(Error("native_resource_busy"))
        );
        broker
            .settle_external_tools(
                "app-host",
                &[("original".into(), json!({"output":"applied"}))],
            )
            .await
            .unwrap();
        broker
            .admit_external_tool("other", "conflict", &command.name, &command.input, &root)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_distinct_writers_and_reads_are_parallel_nonoverlap_rebases() {
        let (root, journal, tools) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        fs::write(root.join("file"), "alpha\nbeta\ngamma\n").unwrap();
        fs::write(root.join("other"), "separate\n").unwrap();
        let broker = Broker::new(journal);
        let one = broker
            .register_scoped("one", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let two = broker
            .register_scoped("two", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let three = broker
            .register_scoped("three", tools, false, Some(&root))
            .await
            .unwrap();
        broker.bind_execution(&two, Some(execution)).await.unwrap();
        let first = patch("file", "alpha", "ALPHA");
        let second = patch("file", "beta", "BETA");
        let unrelated = patch("other", "separate", "SEPARATE");
        let id_first = accepted(&broker, &one, &first).await;
        broker.enqueue(&two, &second).await.unwrap();
        assert!(
            broker.offer(&two).await.unwrap().is_empty(),
            "same-file publication serializes, not creation"
        );
        let rejected = broker.invoke(&two, second.clone()).await.unwrap();
        assert_eq!(rejected["error"], "native_resource_busy");
        assert_eq!(rejected["executed"], false);
        let id_unrelated = accepted(&broker, &three, &unrelated).await;
        let read = Arguments {
            call_id: "read".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"pwd","login":false}).to_string(),
            report: String::new(),
        };
        let read_id = accepted(&broker, &two, &read).await;
        broker
            .settle(
                &two,
                &[(
                    read_id,
                    json!({"output":"Process exited with code 0\nOutput:\nread"}),
                )],
            )
            .await
            .unwrap();
        fs::write(root.join("file"), "ALPHA\nbeta\ngamma\n").unwrap();
        broker
            .settle(&one, &[(id_first, json!("first"))])
            .await
            .unwrap();
        assert!(
            broker.offer(&two).await.unwrap().is_empty(),
            "the rejected call cannot dispatch after resource release"
        );
        let mut retry = second;
        retry.call_id = "second-hunk-after-owner-settled".into();
        broker.enqueue(&two, &retry).await.unwrap();
        let id_second = broker.offer(&two).await.unwrap();
        assert_eq!(
            id_second.len(),
            1,
            "second hunk can rebase without overwrite"
        );
        fs::write(root.join("file"), "ALPHA\nBETA\ngamma\n").unwrap();
        broker
            .settle(
                &two,
                &[(id_second[0]["id"].as_str().unwrap().into(), json!("second"))],
            )
            .await
            .unwrap();
        broker
            .settle(&three, &[(id_unrelated, json!("other"))])
            .await
            .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("file")).unwrap(),
            "ALPHA\nBETA\ngamma\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn nc_existing_integrated_receipt_reconciles_with_original_lane() {
        let (root, journal, _) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let patch = patch("file", "alpha", "ALPHA");
        broker
            .admit_external_tool("lane-one", "original", &patch.name, &patch.input, &root)
            .await
            .unwrap();
        let id = external_effect_id("lane-one", "original");
        let key = digest(&"native-effect-claims-v1");
        let mut claims: ActiveEffects = journal.read(&key).unwrap().unwrap();
        claims.external.get_mut(&id).unwrap().lane = None;
        journal.write(&key, &claims).unwrap();
        broker
            .settle_external_tools(
                "lane-one",
                &[("original".into(), json!({"output":"patch applied"}))],
            )
            .await
            .unwrap();
        let settled: ActiveEffects = journal.read(&key).unwrap().unwrap();
        assert!(settled.external[&id].settled);
        assert_eq!(settled.external[&id].lane.as_deref(), Some("lane-one"));
        broker
            .admit_external_tool("lane-two", "competing", &patch.name, &patch.input, &root)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_apphost_persisted_result_reconciles_before_next_native_offer() {
        let (root, journal, _) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let first = patch("file", "alpha", "ALPHA");
        broker
            .admit_external_tool("apphost-lane", "original", &first.name, &first.input, &root)
            .await
            .unwrap();
        let resource_id = external_effect_id("apphost-lane", "original");
        let key = digest(&"native-effect-claims-v1");
        let mut claims: ActiveEffects = journal.read(&key).unwrap().unwrap();
        assert!(!claims.external[&resource_id].settled);
        // Simulate a process crash after the minimal completion proof reaches
        // disk, but before the corresponding OS claim has been released.
        claims.external.get_mut(&resource_id).unwrap().completion = Some(
            ExternalCompletion::from_result("original", &json!({"output":"patch applied"})),
        );
        journal.write(&key, &claims).unwrap();
        drop(broker);
        let recovered = Broker::new(journal);
        recovered
            .admit_external_tool("second-lane", "competing", &first.name, &first.input, &root)
            .await
            .unwrap();
        let reconciled: ActiveEffects = recovered.journal.read(&key).unwrap().unwrap();
        assert!(reconciled.external[&resource_id].settled);
        assert!(!reconciled.claims.contains_key(&resource_id));
        assert_eq!(
            recovered
                .admit_external_tool("apphost-lane", "original", &first.name, &first.input, &root)
                .await,
            Err(Error("native_effect_delivery_unconfirmed")),
            "the original AppHost call must not be forwarded a second time"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_failed_direct_receipt_commit_must_not_release_shared_writer() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let owner = broker
            .register_scoped("one", tools, false, Some(&root))
            .await
            .unwrap();
        let command = patch("file", "alpha", "ALPHA");
        let id = accepted(&broker, &owner, &command).await;
        let key = digest(&("mcp-turn-v1", &owner));
        let path = journal.root.join(format!("{key}.json"));
        let saved = journal.root.join("saved-direct-turn");
        fs::rename(&path, &saved).unwrap();
        fs::create_dir(&path).unwrap();
        let result = json!({"output":"patch applied"});
        assert!(broker
            .settle(&owner, &[(id.clone(), result.clone())])
            .await
            .is_err());
        let home = broker.shared_guard_home().unwrap();
        assert_eq!(
            SharedNativeGuard::offer_external(
                &root,
                Some(&home),
                "other",
                "competing",
                &command.name,
                &command.input
            )
            .err()
            .as_deref(),
            Some("native_resource_busy"),
            "a failed authoritative receipt commit cannot release its live writer"
        );
        fs::remove_dir(&path).unwrap();
        fs::rename(&saved, &path).unwrap();
        broker.settle(&owner, &[(id, result)]).await.unwrap();
        SharedNativeGuard::offer_external(
            &root,
            Some(&home),
            "other",
            "competing",
            &command.name,
            &command.input,
        )
        .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_persisted_direct_result_reconciles_after_restart_without_reexecution() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let owner = broker
            .register_scoped("one", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let command = patch("file", "alpha", "ALPHA");
        let id = accepted(&broker, &owner, &command).await;
        // Crash boundary: the authoritative result committed but the OS ledger
        // has not yet observed it. Do not offer the original effect again.
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&owner).unwrap();
            turn.calls.get_mut(&command.call_id).unwrap().result = Some(json!({"output":"done"}));
            broker.persist(&owner, turn).unwrap();
        }
        drop(broker);
        let recovered = Broker::new(journal);
        let other = recovered
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        assert_eq!(
            recovered.recorded_calls(&owner).unwrap()[0]["id"],
            id,
            "the original operation's journal and identity must survive restart"
        );
        let competing = patch("file", "alpha", "DIFFERENT");
        recovered
            .bind_execution(&other, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        recovered.enqueue(&other, &competing).await.unwrap();
        let ids = recovered.offer(&other).await.unwrap();
        assert_eq!(
            ids.len(),
            1,
            "a journaled completion must reconcile its physical claim before the next offer"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_persisted_continuation_exit_reconciles_after_restart() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let owner = broker
            .register_scoped("owner", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let exec = Arguments {
            call_id: "exec".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"sleep 300"}).to_string(),
            report: String::new(),
        };
        let exec_id = accepted(&broker, &owner, &exec).await;
        broker
            .settle(
                &owner,
                &[(
                    exec_id,
                    json!({"output":"Process running with session ID 83"}),
                )],
            )
            .await
            .unwrap();
        let poll = Arguments {
            call_id: "poll".into(),
            name: "functions.write_stdin".into(),
            input: json!({"session_id":83,"chars":"","yield_time_ms":1000}).to_string(),
            report: String::new(),
        };
        let poll_id = accepted(&broker, &owner, &poll).await;
        // Simulate a crash after the continuation's terminal result was
        // journaled, but before the physical process lease was settled.
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&owner).unwrap();
            turn.calls.get_mut(&poll.call_id).unwrap().result = Some(waited(&poll_id, Some(83), 0));
            broker.persist(&owner, turn).unwrap();
        }
        drop(broker);
        let recovered = Broker::new(journal);
        let other = recovered
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        let edit = patch("file", "alpha", "ALPHA");
        recovered
            .bind_execution(&other, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        recovered.enqueue(&other, &edit).await.unwrap();
        assert_eq!(
            recovered.offer(&other).await.unwrap().len(),
            1,
            "persisted exit of a continuation must release its original process claim"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_continuation_exit_in_later_integrated_turn_recovers_original_lease() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let first_turn = broker
            .register_scoped("integrated-work", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let original = Arguments {
            call_id: "start".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"sleep 300"}).to_string(),
            report: String::new(),
        };
        let id = accepted(&broker, &first_turn, &original).await;
        broker
            .settle(
                &first_turn,
                &[(id, json!({"output":"Process running with session ID 913"}))],
            )
            .await
            .unwrap();
        broker.retire(&first_turn, false).await.unwrap();
        let later_turn = broker
            .register_scoped("integrated-work", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let poll = Arguments {
            call_id: "stop-confirmation".into(),
            name: "functions.write_stdin".into(),
            input: json!({"session_id":913,"chars":"","yield_time_ms":1000}).to_string(),
            report: String::new(),
        };
        let poll_id = accepted(&broker, &later_turn, &poll).await;
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&later_turn).unwrap();
            turn.calls.get_mut(&poll.call_id).unwrap().result =
                Some(waited(&poll_id, Some(913), 0));
            broker.persist(&later_turn, turn).unwrap();
        }
        drop(broker);
        let recovered = Broker::new(journal);
        let other = recovered
            .register_scoped("other-work", tools, false, Some(&root))
            .await
            .unwrap();
        let competing = patch("file", "alpha", "ALPHA");
        recovered
            .bind_execution(&other, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        recovered.enqueue(&other, &competing).await.unwrap();
        assert_eq!(
            recovered.offer(&other).await.unwrap().len(),
            1,
            "a later integrated turn's exit must reconcile the original process claim"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_native_terminal_item_settles_a_retired_process_without_replaying_its_poll() {
        let (root, journal, tools) = fixture();
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("original", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let exec = Arguments {
            call_id: "start".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"owned uncertain process"}).to_string(),
            report: String::new(),
        };
        let facts = native_effect::hook_fixture(&root);
        broker
            .bind_execution(&token, Some(facts.clone()))
            .await
            .unwrap();
        let id = accepted(&broker, &token, &exec).await;
        let original = json!({"output":"Process running with session ID 56351"});
        broker
            .settle(&token, &[(id.clone(), original.clone())])
            .await
            .unwrap();
        let poll = Arguments {
            call_id: "poll".into(),
            name: "functions.write_stdin".into(),
            input: json!({"session_id":56351,"chars":""}).to_string(),
            report: String::new(),
        };
        broker.enqueue(&token, &poll).await.unwrap();
        broker.retire(&token, true).await.unwrap();
        drop(broker);
        let broker = Broker::new(journal.clone());
        let other = broker
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        let item = json!({"turn_id":facts.turn,"item":{"id":id,"type":"commandExecution","processId":"56351","status":"failed","exitCode":2}});
        for fault in [
            "other_owner",
            "other_call",
            "other_session",
            "active",
            "missing_exit",
            "unknown_exit",
            "timeout_exit",
        ] {
            let mut observed = item.clone();
            match fault {
                "other_owner" => {}
                "other_call" => observed["item"]["id"] = json!("unrelated"),
                "other_session" => observed["item"]["processId"] = json!("56352"),
                "active" => observed["item"]["status"] = json!("inProgress"),
                "missing_exit" => observed["item"]["exitCode"] = Value::Null,
                "unknown_exit" => observed["item"]["exitCode"] = json!(-1),
                "timeout_exit" => observed["item"]["exitCode"] = json!(124),
                _ => unreachable!(),
            }
            broker
                .settle_native_processes(
                    &[if fault == "other_owner" {
                        "other"
                    } else {
                        "original"
                    }],
                    &root,
                    &[observed],
                )
                .await
                .unwrap();
            assert!(
                broker.has_pending_processes(&["original"]).unwrap(),
                "{fault}"
            );
            assert_eq!(
                broker
                    .admit_external_tool(
                        "other",
                        "blocked",
                        "functions.exec_command",
                        &exec.input,
                        &root
                    )
                    .await,
                Err(Error("native_resource_busy"))
            );
        }
        broker
            .settle_native_processes(&["original"], &root, std::slice::from_ref(&item))
            .await
            .unwrap();
        assert!(!broker.has_pending_processes(&["original"]).unwrap());
        broker
            .settle_native_processes(&["original"], &root, &[item])
            .await
            .unwrap();
        let original_turn: Turn = journal
            .read(&digest(&("mcp-turn-v1", &token)))
            .unwrap()
            .unwrap();
        assert_eq!(
            original_turn.calls["start"].result,
            Some(original),
            "the real running-tool reply is unchanged"
        );
        assert!(!broker
            .recorded_calls(&token)
            .unwrap()
            .iter()
            .find(|call| call["id"] != id)
            .unwrap()["offered"]
            .as_bool()
            .unwrap());
        let next = accepted(&broker, &other, &exec).await;
        broker
            .settle(&other, &[(next.clone(), waited(&next, None, 0))])
            .await
            .unwrap();
        // LocalFunction reserves the same Native effect without an MCP token.
        // Its Native item carries the original call id, not the reservation id.
        broker
            .admit_external_tool_with_execution(
                "local-function",
                "native-call",
                &exec.name,
                &exec.input,
                &root,
                Some(&facts),
            )
            .await
            .unwrap();
        broker
            .settle_external_tools(
                "local-function",
                &[(
                    "native-call".into(),
                    json!({"output":"Process running with session ID 56351"}),
                )],
            )
            .await
            .unwrap();
        let local_item = json!({"turn_id":facts.turn,"item":{"id":"native-call","type":"commandExecution","processId":"56351","status":"completed","exitCode":0}});
        broker
            .settle_native_processes(&["other"], &root, std::slice::from_ref(&local_item))
            .await
            .unwrap();
        assert!(broker.has_pending_processes(&["local-function"]).unwrap());
        broker
            .settle_native_processes(&["local-function"], &root, &[local_item])
            .await
            .unwrap();
        assert!(!broker.has_pending_processes(&["local-function"]).unwrap());
        broker
            .admit_external_tool("other", "after-local-exit", &exec.name, &exec.input, &root)
            .await
            .unwrap();
        broker
            .settle_external_tools(
                "other",
                &[(
                    "after-local-exit".into(),
                    waited("after-local-exit", None, 0),
                )],
            )
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_overlap_conflict_and_original_receipt_are_preserved() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\nbeta\n").unwrap();
        let broker = Broker::new(journal);
        let one = broker
            .register_scoped("one", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let two = broker
            .register_scoped("two", tools, false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&two, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        let first = patch("file", "alpha", "ALPHA");
        let second = patch("file", "alpha", "changed");
        let id_first = accepted(&broker, &one, &first).await;
        broker.enqueue(&two, &second).await.unwrap();
        assert!(broker.offer(&two).await.unwrap().is_empty());
        fs::write(root.join("file"), "ALPHA\nbeta\n").unwrap();
        broker
            .settle(&one, &[(id_first, json!("original result"))])
            .await
            .unwrap();
        assert!(broker.offer(&two).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&two, second.clone()).await.unwrap()["error"],
            "native_resource_busy",
            "the original conflict is an exact pre-effect receipt"
        );
        let mut retry = second.clone();
        retry.call_id = "overlap-after-first-published".into();
        broker.enqueue(&two, &retry).await.unwrap();
        assert!(broker.offer(&two).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&two, retry).await.unwrap(),
            json!({"error":"native_resource_version_conflict","executed":false})
        );
        assert_eq!(
            broker.invoke(&one, first).await.unwrap(),
            json!("original result"),
            "identical call ID and payload reuses settled receipt"
        );
        assert_eq!(
            fs::read_to_string(root.join("file")).unwrap(),
            "ALPHA\nbeta\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn nc_unknown_opaque_and_crashed_writer_claim_are_not_forgotten() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal.clone());
        let one = broker
            .register_scoped("one", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let second = patch("file", "alpha", "replacement");
        let unknown = Arguments {
            call_id: "unknown-shell".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"touch file && git add file"}).to_string(),
            report: String::new(),
        };
        let opaque = accepted(&broker, &one, &unknown).await;
        let two = broker
            .register_scoped("two", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&two, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        broker.enqueue(&two, &second).await.unwrap();
        assert!(broker.offer(&two).await.unwrap().is_empty());
        broker
            .settle(&one, &[(opaque.clone(), waited(&opaque, None, 0))])
            .await
            .unwrap();
        assert!(
            broker.offer(&two).await.unwrap().is_empty(),
            "a conflicted original call cannot be dispatched after process exit"
        );
        let mut retry = second;
        retry.call_id = "after-opaque-exit".into();
        broker.enqueue(&two, &retry).await.unwrap();
        let offered = broker.offer(&two).await.unwrap();
        assert_eq!(offered.len(), 1);
        // Simulate process loss with a tool already offered but no result.
        drop(broker);
        let replacement = Broker::new(journal);
        let third = replacement
            .register_scoped("three", tools, false, Some(&root))
            .await
            .unwrap();
        let third_call = patch("file", "alpha", "other");
        replacement
            .bind_execution(&third, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        replacement.enqueue(&third, &third_call).await.unwrap();
        assert!(replacement.offer(&third).await.unwrap().is_empty());
        let stored: ActiveEffects = replacement
            .journal
            .read(&digest(&"native-effect-claims-v1"))
            .unwrap()
            .unwrap();
        assert!(stored
            .claims
            .contains_key(offered[0]["id"].as_str().unwrap()));
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn nc_parallel_compiler_session_polls_preserve_original_scopes_and_results() {
        let (root, journal, tools) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        let broker = Broker::new(journal.clone());
        let mut tokens = Vec::new();
        let mut originals = Vec::new();
        for (lane, session) in [("a", 51), ("b", 52)] {
            fs::write(
                root.join(format!("{lane}.c")),
                "int main(void){return 0;}\n",
            )
            .unwrap();
            let token = broker
                .register_scoped(lane, tools.clone(), false, Some(&root))
                .await
                .unwrap();
            broker
                .bind_execution(&token, Some(execution.clone()))
                .await
                .unwrap();
            let command = Arguments {
                call_id: format!("compile-{lane}"), name: "functions.exec_command".into(),
                input: json!({"cmd":format!("clang -O0 {lane}.c -o {lane}.out"),"workdir":root,"login":false}).to_string(),
                report: String::new(),
            };
            let id = accepted(&broker, &token, &command).await;
            let running = json!({"output":format!("Process running with session ID {session}")});
            broker
                .settle(&token, &[(id, running.clone())])
                .await
                .unwrap();
            tokens.push(token);
            originals.push((command, running));
        }
        let poll = |id: &str, session: u64, chars: &str| Arguments {
            call_id: id.into(),
            name: "functions.write_stdin".into(),
            input: json!({"session_id":session,"chars":chars,"yield_time_ms":1000}).to_string(),
            report: String::new(),
        };
        // The original compiler may have produced its output before the next
        // poll. Revalidating launch snapshots here would reject a valid result.
        for lane in ["a", "b"] {
            fs::write(root.join(format!("{lane}.out")), b"compiled output").unwrap();
        }
        let held: ActiveEffects = journal
            .read(&digest(&"native-effect-claims-v1"))
            .unwrap()
            .unwrap();
        assert_eq!(held.claims.len(), 2);
        let mut ambiguous: ActiveEffects = journal
            .read(&digest(&"native-effect-claims-v1"))
            .unwrap()
            .unwrap();
        let original = ambiguous
            .claims
            .values()
            .find(|claim| claim.lane == "a")
            .unwrap()
            .clone();
        ambiguous.claims.insert("duplicate-owner".into(), original);
        assert!(ambiguous.session_poll_scope("a", 51).is_none());
        for (id, session, chars) in [("foreign", 52, ""), ("unknown", 53, ""), ("write", 51, "x")] {
            let rejected = poll(id, session, chars);
            broker.enqueue(&tokens[0], &rejected).await.unwrap();
            assert!(broker.offer(&tokens[0]).await.unwrap().is_empty());
            assert_eq!(
                broker.invoke(&tokens[0], rejected).await.unwrap()["error"],
                "native_resource_busy"
            );
        }
        let external = poll("external-poll", 52, "");
        let mut unsafe_hooks = execution.clone();
        unsafe_hooks.hook_free_tools.clear();
        for (id, facts) in [
            ("missing-proof", None),
            ("unsafe-hooks", Some(&unsafe_hooks)),
        ] {
            assert_eq!(
                broker
                    .admit_external_tool_with_execution(
                        "b",
                        id,
                        &external.name,
                        &external.input,
                        &root,
                        facts
                    )
                    .await,
                Err(Error("native_execution_completion_unavailable"))
            );
        }
        broker
            .admit_external_tool_with_execution(
                "b",
                &external.call_id,
                &external.name,
                &external.input,
                &root,
                Some(&execution),
            )
            .await
            .unwrap();
        let first_poll = poll("poll-running", 51, "");
        let first_id = accepted(&broker, &tokens[0], &first_poll).await;
        let running = json!({"output":"Process running with session ID 51"});
        broker
            .settle(&tokens[0], &[(first_id, running.clone())])
            .await
            .unwrap();
        assert_eq!(
            broker.invoke(&tokens[0], first_poll).await.unwrap(),
            running
        );
        let status = broker.effect_status("a").unwrap();
        assert_eq!(status["unconfirmed_effects"], 2);
        assert_eq!(status["shared"]["unconfirmed_effects"], 2);
        assert!(status["shared"]["active"]
            .as_array()
            .unwrap()
            .iter()
            .all(|claim| claim["resource_kind"] == "bounded"));
        let terminal = json!({"output":"Process exited with code 0"});
        broker
            .settle_external_tools("b", &[(external.call_id, terminal.clone())])
            .await
            .unwrap();
        assert_eq!(
            broker.effect_status("a").unwrap()["shared"]["unconfirmed_effects"],
            1
        );
        let final_poll = poll("poll-exited", 51, "");
        let id = accepted(&broker, &tokens[0], &final_poll).await;
        broker
            .settle(&tokens[0], &[(id, terminal.clone())])
            .await
            .unwrap();
        assert_eq!(
            broker.invoke(&tokens[0], final_poll).await.unwrap(),
            terminal
        );
        for (token, (command, result)) in tokens.iter().zip(originals) {
            assert_eq!(broker.invoke(token, command).await.unwrap(), result);
        }
        assert_eq!(
            broker
                .invoke(&tokens[0], poll("write", 51, "x"))
                .await
                .unwrap()["error"],
            "native_resource_busy"
        );
        let status = broker.effect_status("a").unwrap();
        assert_eq!(status["unconfirmed_effects"], 0);
        assert_eq!(status["shared"]["unconfirmed_effects"], 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nc_opaque_async_process_keeps_claim_until_continuation_confirms_exit() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        let owner = broker
            .register_scoped("owner", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let other = broker
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        let command = Arguments {
            call_id: "started".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"sleep 300"}).to_string(),
            report: String::new(),
        };
        let command_id = accepted(&broker, &owner, &command).await;
        broker
            .settle(
                &owner,
                &[(
                    command_id.clone(),
                    json!({"output":"Process running with session ID 1200"}),
                )],
            )
            .await
            .unwrap();
        let pending = patch("file", "alpha", "ALPHA");
        broker
            .bind_execution(&other, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        broker.enqueue(&other, &pending).await.unwrap();
        assert!(broker.offer(&other).await.unwrap().is_empty());
        let poll = |id: &str| Arguments {
            call_id: id.into(),
            name: "functions.write_stdin".into(),
            input: json!({"session_id":1200,"chars":"","yield_time_ms":1000}).to_string(),
            report: String::new(),
        };
        let first_poll = accepted(&broker, &owner, &poll("poll-one")).await;
        broker
            .settle(
                &owner,
                &[(first_poll.clone(), session_running(&first_poll, 1200))],
            )
            .await
            .unwrap();
        assert!(
            broker.offer(&other).await.unwrap().is_empty(),
            "poll is not a process-exit confirmation"
        );
        let final_poll = accepted(&broker, &owner, &poll("poll-final")).await;
        broker
            .settle(
                &owner,
                &[(final_poll.clone(), waited(&final_poll, Some(1200), 0))],
            )
            .await
            .unwrap();
        assert!(
            broker.offer(&other).await.unwrap().is_empty(),
            "a rejected Native call must not run after a background session ends"
        );
        let mut retry = pending;
        retry.call_id = "after-session-exit".into();
        broker.enqueue(&other, &retry).await.unwrap();
        assert_eq!(broker.offer(&other).await.unwrap().len(), 1);
        assert!(!broker
            .journal
            .read::<ActiveEffects>(&digest(&"native-effect-claims-v1"))
            .unwrap()
            .unwrap()
            .claims
            .contains_key(&command_id));
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn nc_independent_mediated_callers_share_resource_claims() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        let home = broker.shared_guard_home().unwrap();
        let direct = patch("file", "alpha", "ALPHA");
        SharedNativeGuard::offer_external(
            &root,
            Some(&home),
            "mediated-work",
            "mediated-patch",
            &direct.name,
            &direct.input,
        )
        .unwrap();
        let token = broker
            .register_scoped("direct", tools, false, Some(&root))
            .await
            .unwrap();
        broker
            .bind_execution(&token, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        broker.enqueue(&token, &direct).await.unwrap();
        assert!(
            broker.offer(&token).await.unwrap().is_empty(),
            "Provider must see a mediated claim not in Broker's journal"
        );
        let other = broker
            .admit_external_tool("app-host", "same-file", &direct.name, &direct.input, &root)
            .await;
        assert_eq!(
            other,
            Err(Error("native_resource_busy")),
            "AppHost must see the independent mediated claim"
        );
        SharedNativeGuard::settle_external(
            &root,
            Some(&home),
            "mediated-work",
            "mediated-patch",
            &direct.name,
            &direct.input,
            &json!("patched"),
        )
        .unwrap();
        assert!(
            broker.offer(&token).await.unwrap().is_empty(),
            "the original rejected call must never be dispatched implicitly"
        );
        let mut retry = direct;
        retry.call_id = "after-independent-mediation".into();
        broker.enqueue(&token, &retry).await.unwrap();
        assert_eq!(broker.offer(&token).await.unwrap().len(), 1);
    }
    #[tokio::test]
    async fn nc_external_native_tool_and_mcp_tool_share_exact_resource_claims() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\nbeta\n").unwrap();
        fs::write(root.join("other"), "other\n").unwrap();
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("direct", tools, false, Some(&root))
            .await
            .unwrap();
        let direct = patch("file", "alpha", "ALPHA");
        let id = accepted(&broker, &token, &direct).await;
        let overlapping = patch("file", "beta", "BETA");
        assert_eq!(
            broker
                .admit_external_tool(
                    "app-host",
                    "upstream-overlap",
                    &overlapping.name,
                    &overlapping.input,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        let separate = patch("other", "other", "OTHER");
        broker
            .admit_external_tool(
                "app-host",
                "upstream-independent",
                &separate.name,
                &separate.input,
                &root,
            )
            .await
            .unwrap();
        assert_eq!(
            broker
                .admit_external_tool(
                    "app-host",
                    "upstream-independent",
                    &separate.name,
                    &separate.input,
                    &root
                )
                .await,
            Err(Error("native_effect_delivery_unconfirmed")),
            "an uncertain original tool offer is never resent"
        );
        fs::write(root.join("file"), "ALPHA\nbeta\n").unwrap();
        broker
            .settle(&token, &[(id, json!("native completed"))])
            .await
            .unwrap();
        broker
            .admit_external_tool(
                "app-host",
                "upstream-overlap",
                &overlapping.name,
                &overlapping.input,
                &root,
            )
            .await
            .unwrap();
        let restarted = Broker::new(journal);
        assert_eq!(
            restarted
                .admit_external_tool(
                    "app-host",
                    "upstream-overlap",
                    &overlapping.name,
                    &overlapping.input,
                    &root
                )
                .await,
            Err(Error("native_effect_delivery_unconfirmed"))
        );
        restarted
            .settle_external_tools(
                "app-host",
                &[
                    (
                        "upstream-overlap".into(),
                        json!({"output":"Process exited with code 0"}),
                    ),
                    (
                        "upstream-independent".into(),
                        json!({"output":"Process exited with code 0"}),
                    ),
                ],
            )
            .await
            .unwrap();
        assert!(restarted
            .journal
            .read::<ActiveEffects>(&digest(&"native-effect-claims-v1"))
            .unwrap()
            .unwrap()
            .claims
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn nc_external_long_running_native_process_release_requires_exit() {
        let (root, journal, _tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        broker
            .admit_external_tool(
                "owner",
                "command",
                "functions.exec_command",
                &json!({"cmd":"sleep 300"}).to_string(),
                &root,
            )
            .await
            .unwrap();
        broker
            .settle_external_tools(
                "owner",
                &[(
                    "command".into(),
                    json!({"output":"Process running with session ID 2200"}),
                )],
            )
            .await
            .unwrap();
        let update = patch("file", "alpha", "ALPHA");
        assert_eq!(
            broker
                .admit_external_tool("other", "write", &update.name, &update.input, &root)
                .await,
            Err(Error("native_resource_busy"))
        );
        let poll = json!({"session_id":2200,"chars":"","yield_time_ms":1000}).to_string();
        broker
            .admit_external_tool("owner", "poll", "functions.write_stdin", &poll, &root)
            .await
            .unwrap();
        broker
            .settle_external_tools("owner", &[("poll".into(), waited("poll", Some(2200), 0))])
            .await
            .unwrap();
        broker
            .admit_external_tool("other", "write", &update.name, &update.input, &root)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn nc_retired_mcp_turn_unreturned_command_settles_from_native_history() {
        let (root, journal, tools) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        let token = broker
            .register_scoped("owner", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let command = Arguments {
            call_id: "restart-cmd".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"printf ok"}).to_string(),
            report: "run command".into(),
        };
        let facts = native_effect::hook_fixture(&root);
        broker
            .bind_execution(&token, Some(facts.clone()))
            .await
            .unwrap();
        broker.enqueue(&token, &command).await.unwrap();
        let offered = broker.offer(&token).await.unwrap();
        let id = offered[0]["id"].as_str().unwrap().to_owned();
        broker.retire(&token, true).await.unwrap();
        assert!(broker.has_pending_processes(&["owner"]).unwrap());
        assert!(broker.active_claim_ids(&["owner"]).unwrap().contains(&id));
        let other_token = broker
            .register_scoped("other", tools.clone(), false, Some(&root))
            .await
            .unwrap();
        let write = patch("file", "alpha", "ALPHA");
        assert_eq!(
            broker
                .admit_external_tool("other", "ext-write", &write.name, &write.input, &root)
                .await,
            Err(Error("native_resource_busy"))
        );
        broker
            .bind_execution(&other_token, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        broker.enqueue(&other_token, &write).await.unwrap();
        assert!(broker.offer(&other_token).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&other_token, write.clone()).await.unwrap()["error"],
            "native_resource_busy"
        );
        for incomplete in [
            json!([{"turn_id":"t1","item":{"id":id,"type":"commandExecution","status":"inProgress","exitCode":0}}]),
            json!([{"turn_id":"t1","item":{"id":id,"type":"commandExecution","status":"failed","exitCode":-1}}]),
            json!([{"turn_id":"t1","item":{"id":id,"type":"commandExecution","status":"failed","exitCode":124}}]),
        ] {
            broker
                .settle_native_processes(&["owner"], &root, incomplete.as_array().unwrap())
                .await
                .unwrap();
            assert!(broker.has_pending_processes(&["owner"]).unwrap());
        }
        let committed = vec![json!({
            "turn_id": "t1",
            "item": {
                "id": id,
                "type": "commandExecution",
                "status": "completed",
                "exitCode": 0,
                "aggregatedOutput": "ok\n"
            }
        })];
        broker
            .settle_native_processes(&["owner"], &root, &committed)
            .await
            .unwrap();
        assert!(
            broker.has_pending_processes(&["owner"]).unwrap(),
            "public terminal item without the original result/proof cannot settle"
        );
        let (native, _) = gateway_calls(
            &id,
            &json!({"name":command.name,"input":command.input,"report":command.report}).to_string(),
            &tools,
        )
        .unwrap();
        let proof = json!({"schema":"codex-native-completion-v1","call_id":id,"session_id":null,
            "outcome":"wait_completed","exit_code":0,"hooks":"absent"});
        let evidence = crate::operator_native::CompletionEvidence::fixture(
            &facts,
            vec![
                json!({"type":"turn_context","payload":{"turn_id":facts.turn}}),
                json!({"type":"response_item","payload":native[0].output_item().unwrap()}),
                json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":id,
                "turn_id":facts.turn,"process_id":null,"exit_code":0,"status":"completed","native_completion":proof}}),
            ],
        );
        broker
            .settle_native_processes_with_evidence(&["owner"], &root, &committed, &evidence)
            .await
            .unwrap();
        assert!(!broker.has_pending_processes(&["owner"]).unwrap());
        assert!(broker.active_claim_ids(&["owner"]).unwrap().is_empty());
        assert!(
            broker.retained_result(&token, &command).unwrap().is_none(),
            "effect proof does not fabricate the missing original function reply"
        );
        let original = waited(&id, None, 0);
        broker
            .retire_native_results(&token, "owner", &[(id.clone(), original.clone())])
            .await
            .unwrap();
        assert_eq!(
            broker.retained_result(&token, &command).unwrap(),
            Some(original)
        );
        assert_eq!(
            broker.recorded_calls(&token).unwrap(),
            vec![json!({"id":id,"offered":true,"settled":true})]
        );
        let mut retry = write;
        retry.call_id = "write-after-settle".into();
        broker.enqueue(&other_token, &retry).await.unwrap();
        let offered_other = broker.offer(&other_token).await.unwrap();
        assert_eq!(offered_other.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn terminal_native_retirement_recovers_the_original_call_without_a_late_projection_overwrite(
    ) {
        for callback_first in [false, true] {
            let (root, journal, tools) = fixture();
            let broker = Broker::new(journal);
            let token = broker
                .register_scoped("owner", tools, false, Some(&root))
                .await
                .unwrap();
            let call = Arguments {
                call_id: "original".into(),
                name: "functions.exec_command".into(),
                input: json!({"cmd":"printf original"}).to_string(),
                report: "one original effect".into(),
            };
            broker
                .bind_execution(&token, Some(native_effect::hook_fixture(&root)))
                .await
                .unwrap();
            broker.enqueue(&token, &call).await.unwrap();
            let id = broker.offer(&token).await.unwrap()[0]["id"]
                .as_str()
                .unwrap()
                .to_owned();
            let proof = json!({"schema":"codex-native-completion-v1","call_id":id,"session_id":null,
                "outcome":"wait_completed","exit_code":0,"hooks":"absent"});
            let original = json!({"type":"function_call_output","call_id":id,
                "output":format!("Native completion: {proof}\nChunk ID: original\nWall time: 0.03 seconds\nProcess exited with code 0\nOutput:\n{}", "complete output\n".repeat(10000))});
            let projection = json!({"type":"function_call_output","call_id":id,
                "output":format!("Native completion: {proof}\nProcess exited with code 0\nOutput:\nmodel-truncated output")});
            assert_eq!(
                broker
                    .admit_external_tool("independent", "blocked", &call.name, &call.input, &root)
                    .await,
                Err(Error("native_resource_busy"))
            );
            if callback_first {
                broker
                    .settle(&token, &[(id.clone(), projection.clone())])
                    .await
                    .unwrap();
            }
            let waiting = {
                let broker = broker.clone();
                let token = token.clone();
                let call = call.clone();
                tokio::spawn(async move { broker.invoke(&token, call).await })
            };
            broker
                .retire_native_results(&token, "owner", &[(id.clone(), original.clone())])
                .await
                .unwrap();
            let expected = if callback_first {
                &projection
            } else {
                &original
            };
            assert_eq!(&waiting.await.unwrap().unwrap(), expected);
            assert_eq!(
                broker.retained_result(&token, &call).unwrap().as_ref(),
                Some(expected)
            );
            assert!(broker.active_claim_ids(&["owner"]).unwrap().is_empty());
            assert_eq!(
                broker
                    .settle(&token, &[(id.clone(), projection.clone())])
                    .await,
                Err(Error("mcp_turn_unavailable"))
            );
            assert_eq!(
                broker.retained_result(&token, &call).unwrap().as_ref(),
                Some(expected)
            );
            broker
                .admit_external_tool(
                    "independent",
                    "after-recovery",
                    &call.name,
                    &call.input,
                    &root,
                )
                .await
                .unwrap();
            assert_eq!(
                broker.recorded_calls(&token).unwrap(),
                vec![json!({"id":id,"offered":true,"settled":true})]
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn terminal_native_retirement_rejects_missing_results_and_foreign_lanes_before_mutation()
    {
        let (root, journal, tools) = fixture();
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("owner", tools, false, Some(&root))
            .await
            .unwrap();
        let call = Arguments {
            call_id: "original".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"printf original"}).to_string(),
            report: "original".into(),
        };
        broker
            .bind_execution(&token, Some(native_effect::hook_fixture(&root)))
            .await
            .unwrap();
        broker.enqueue(&token, &call).await.unwrap();
        let id = broker.offer(&token).await.unwrap()[0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let key = digest(&("mcp-turn-v1", &token));
        let before = journal.read::<Value>(&key).unwrap();
        assert_eq!(
            broker.retire_native_results(&token, "owner", &[]).await,
            Err(Error("mcp_result_missing"))
        );
        let result = json!({"type":"function_call_output","call_id":id,"output":"Process exited with code 0\nOutput:\noriginal"});
        assert_eq!(
            broker
                .retire_native_results(&token, "other", &[(id.clone(), result.clone())])
                .await,
            Err(Error("mcp_owner_conflict"))
        );
        assert_eq!(journal.read::<Value>(&key).unwrap(), before);
        assert!(broker.active_claim_ids(&["owner"]).unwrap().contains(&id));
        broker.retire(&token, true).await.unwrap();
        broker
            .retire_native_results(&token, "owner", &[(id, result.clone())])
            .await
            .unwrap();
        assert_eq!(broker.retained_result(&token, &call).unwrap(), Some(result));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn terminal_native_poll_receipt_survives_original_process_claim_settlement() {
        for observed_command_first in [false, true] {
            let (root, journal, tools) = fixture();
            let broker = Broker::new(journal);
            let token = broker
                .register_scoped("owner", tools, false, Some(&root))
                .await
                .unwrap();
            fs::write(root.join("source.c"), "int main(void){return 0;}\n").unwrap();
            let facts = native_effect::compiler_fixture(&root);
            broker
                .bind_execution(&token, Some(facts.clone()))
                .await
                .unwrap();
            let exec = Arguments {
                call_id: "original-exec".into(),
                name: "functions.exec_command".into(),
                input:
                    json!({"cmd":"clang -O0 source.c -o source.out","workdir":root,"login":false})
                        .to_string(),
                report: "original process".into(),
            };
            let exec_id = accepted(&broker, &token, &exec).await;
            let running = json!({"type":"function_call_output","call_id":exec_id,"output":"Process running with session ID 42\nOutput:\noriginal prefix\n"});
            broker
                .settle(&token, &[(exec_id.clone(), running.clone())])
                .await
                .unwrap();
            let poll = Arguments {
                call_id: "original-poll".into(),
                name: "functions.write_stdin".into(),
                input: json!({"session_id":42,"chars":""}).to_string(),
                report: "observe original process".into(),
            };
            let poll_id = accepted(&broker, &token, &poll).await;
            let expected_calls = broker.unreturned_native_calls(&token).unwrap();
            assert_eq!(expected_calls.len(), 1);
            assert_eq!(expected_calls[&poll_id]["call_id"], poll_id);
            assert_eq!(expected_calls[&poll_id]["arguments"], poll.input);
            assert!(broker.retained_result(&token, &poll).unwrap().is_none());
            if observed_command_first {
                broker.settle_native_processes(&["owner"], &root, &[json!({"turn_id":facts.turn,"item":{
                    "id":exec_id,"type":"commandExecution","status":"completed","processId":"42","exitCode":0,"aggregatedOutput":"original prefix\noriginal tail\n"
                }})]).await.unwrap();
                assert!(broker.active_claim_ids(&["owner"]).unwrap().is_empty());
                assert_eq!(
                    broker.unreturned_native_calls(&token).unwrap(),
                    expected_calls,
                    "receipt identity does not depend on an already settled process claim"
                );
            }
            let mut original = waited(&poll_id, Some(42), 0);
            original["output"] = json!(format!(
                "{}original tail\n",
                original["output"].as_str().unwrap()
            ));
            broker
                .retire_native_results(&token, "owner", &[(poll_id.clone(), original.clone())])
                .await
                .unwrap();
            assert_eq!(
                broker.retained_result(&token, &exec).unwrap(),
                Some(running)
            );
            assert_eq!(
                broker.retained_result(&token, &poll).unwrap(),
                Some(original)
            );
            assert!(broker.active_claim_ids(&["owner"]).unwrap().is_empty());
            assert!(broker.unreturned_native_calls(&token).unwrap().is_empty());
            broker
                .admit_external_tool(
                    "independent",
                    "after-poll-recovery",
                    &exec.name,
                    &exec.input,
                    &root,
                )
                .await
                .unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }
    async fn unproved_continuation_blocks_other_lane(external: bool) {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for chars in ["", "input"] {
                let (root, journal, tools) = fixture_with_process_aliases(exec_tool, poll_tool);
                let facts = native_effect::compiler_fixture(&root);
                for name in ["a.c", "b.c"] {
                    fs::write(root.join(name), "int main(void){return 0;}").unwrap();
                }
                let broker = Broker::new(journal.clone());
                let token = broker
                    .register_scoped("a", tools.clone(), false, Some(&root))
                    .await
                    .unwrap();
                broker
                    .bind_execution(&token, Some(facts.clone()))
                    .await
                    .unwrap();
                let exec = Arguments {
                    call_id: "start".into(),
                    name: exec_tool.into(),
                    input: json!({"cmd":"clang -O0 a.c -o a.out","workdir":root,"login":false})
                        .to_string(),
                    report: String::new(),
                };
                let id = accepted(&broker, &token, &exec).await;
                let original = json!({"output":"Process running with session ID 51\nOutput:\n"});
                broker
                    .settle(&token, &[(id.clone(), original.clone())])
                    .await
                    .unwrap();
                let mut unsafe_facts = facts.clone();
                if chars.is_empty() {
                    unsafe_facts.hook_free_tools.clear();
                }
                let poll = Arguments {
                    call_id: "unproved".into(),
                    name: poll_tool.into(),
                    input: json!({"session_id":51,"chars":chars}).to_string(),
                    report: String::new(),
                };
                if chars.is_empty() {
                    if external {
                        assert_eq!(
                            broker
                                .admit_external_tool_with_execution(
                                    "a",
                                    "unproved",
                                    &poll.name,
                                    &poll.input,
                                    &root,
                                    Some(&unsafe_facts)
                                )
                                .await,
                            Err(Error("native_execution_completion_unavailable"))
                        );
                    } else {
                        broker
                            .bind_execution(&token, Some(unsafe_facts))
                            .await
                            .unwrap();
                        broker.enqueue(&token, &poll).await.unwrap();
                        assert!(broker.offer(&token).await.unwrap().is_empty());
                        assert_eq!(
                            broker.retained_result(&token, &poll).unwrap().unwrap()["error"],
                            "native_execution_completion_unavailable"
                        );
                    }
                    let state: ActiveEffects = journal
                        .read(&digest(&"native-effect-claims-v1"))
                        .unwrap()
                        .unwrap();
                    assert_eq!(state.claims.len(), 1);
                    assert!(
                        matches!(state.claims[&id].effect.claim_scope(), Effect::Paths(_)),
                        "refused hooks never widen the original call"
                    );
                    let other = broker
                        .register_scoped("b", tools.clone(), false, Some(&root))
                        .await
                        .unwrap();
                    broker
                        .bind_execution(&other, Some(facts.clone()))
                        .await
                        .unwrap();
                    let independent = Arguments {
                        call_id: "unrelated-after-refusal".into(),
                        input: json!({"cmd":"clang -O0 b.c -o b.out","workdir":root,"login":false})
                            .to_string(),
                        ..exec.clone()
                    };
                    let other_id = accepted(&broker, &other, &independent).await;
                    broker
                        .settle(&other, &[(other_id.clone(), waited(&other_id, None, 0))])
                        .await
                        .unwrap();
                    broker
                        .bind_execution(&token, Some(facts.clone()))
                        .await
                        .unwrap();
                    let safe = Arguments {
                        call_id: "qualified-empty-poll".into(),
                        ..poll.clone()
                    };
                    let safe_id = accepted(&broker, &token, &safe).await;
                    broker
                        .settle(&token, &[(safe_id.clone(), waited(&safe_id, Some(51), 0))])
                        .await
                        .unwrap();
                    assert_eq!(
                        broker.retained_result(&token, &exec).unwrap(),
                        Some(original)
                    );
                    assert!(broker.active_claim_ids(&["a", "b"]).unwrap().is_empty());
                    fs::remove_dir_all(root).unwrap();
                    continue;
                }
                if external {
                    broker
                        .admit_external_tool_with_execution(
                            "a",
                            "unproved",
                            &poll.name,
                            &poll.input,
                            &root,
                            Some(&unsafe_facts),
                        )
                        .await
                        .unwrap();
                } else {
                    broker
                        .bind_execution(&token, Some(unsafe_facts))
                        .await
                        .unwrap();
                    accepted(&broker, &token, &poll).await;
                }
                let state: ActiveEffects = journal
                    .read(&digest(&"native-effect-claims-v1"))
                    .unwrap()
                    .unwrap();
                assert_eq!(state.claims[&id].effect.claim_scope(), &Effect::Opaque);
                assert_eq!(
                    broker.retained_result(&token, &exec).unwrap(),
                    Some(original.clone())
                );
                let (raw, _) = gateway_calls(
                    &id,
                    &json!({"name":exec.name,"input":exec.input,"report":exec.report}).to_string(),
                    &tools,
                )
                .unwrap();
                let proof = json!({"schema":"codex-native-completion-v1","call_id":id,"session_id":51,
                    "outcome":"wait_completed","exit_code":0,"hooks":"absent"});
                let evidence = crate::operator_native::CompletionEvidence::fixture(
                    &facts,
                    vec![
                        json!({"type":"turn_context","payload":{"turn_id":facts.turn}}),
                        json!({"type":"response_item","payload":raw[0].output_item().unwrap()}),
                        json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":id,
                        "turn_id":facts.turn,"process_id":"51","exit_code":0,"status":"completed",
                        "native_completion":proof}}),
                    ],
                );
                broker
                    .reconcile_stopped_receipts(
                        &["a"],
                        &[&token],
                        &BTreeMap::new(),
                        Some(&evidence),
                    )
                    .await
                    .unwrap();
                // An original late wait cannot clear a distinct pending invocation.
                assert_eq!(broker.effect_status("a").unwrap()["unconfirmed_effects"], 1);
                assert_eq!(
                    broker.retained_result(&token, &exec).unwrap(),
                    Some(original.clone())
                );
                drop(broker);
                let recovered = Broker::new(journal.clone());
                let other = recovered
                    .register_scoped("b", tools.clone(), false, Some(&root))
                    .await
                    .unwrap();
                recovered
                    .bind_execution(&other, Some(facts.clone()))
                    .await
                    .unwrap();
                let next = Arguments {
                    call_id: "independent".into(),
                    input: json!({"cmd":"clang -O0 b.c -o b.out","workdir":root,"login":false})
                        .to_string(),
                    ..exec.clone()
                };
                recovered.enqueue(&other, &next).await.unwrap();
                assert!(recovered.offer(&other).await.unwrap().is_empty());
                assert_eq!(
                    recovered.invoke(&other, next).await.unwrap()["error"],
                    "native_resource_busy"
                );
                let poll_id = if external {
                    "unproved".to_owned()
                } else {
                    digest(&(&token, &poll.call_id))
                };
                let proof = json!({"schema":"codex-native-completion-v1","call_id":poll_id,"session_id":51,
                    "outcome":"wait_completed","exit_code":0,"hooks":"absent"});
                let result = json!({"type":"function_call_output","call_id":poll_id,
                    "output":format!("Native completion: {proof}\nProcess exited with code 0\nOutput:\n")});
                if external {
                    recovered
                        .settle_external_tools("a", &[(poll_id, result)])
                        .await
                        .unwrap();
                } else {
                    recovered
                        .retire_native_results(&token, "a", &[(poll_id, result)])
                        .await
                        .unwrap();
                }
                assert!(recovered.active_claim_ids(&["a"]).unwrap().is_empty());
                let mut final_call = exec.clone();
                final_call.call_id = "after-exact-poll".into();
                final_call.input =
                    json!({"cmd":"clang -O0 b.c -o b.out","workdir":root,"login":false})
                        .to_string();
                accepted(&recovered, &other, &final_call).await;
                let saved: Turn = journal
                    .read(&digest(&("mcp-turn-v1", &token)))
                    .unwrap()
                    .unwrap();
                assert_eq!(saved.calls["start"].result, Some(original));
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
    #[tokio::test]
    async fn unproved_continuation_widens_mcp_scope_before_next_lane_and_restart() {
        unproved_continuation_blocks_other_lane(false).await;
    }
    #[tokio::test]
    async fn unproved_continuation_widens_external_scope_before_next_lane_and_restart() {
        unproved_continuation_blocks_other_lane(true).await;
    }

    #[tokio::test]
    async fn terminal_session_poll_after_observed_completion_does_not_block_on_a_sibling() {
        let (root, journal, tools) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        let broker = Broker::new(journal.clone());
        let mut tokens = Vec::new();
        let mut ids = Vec::new();
        for (lane, session) in [("a", 51), ("b", 52)] {
            fs::write(
                root.join(format!("{lane}.c")),
                "int main(void){return 0;}\n",
            )
            .unwrap();
            let token = broker
                .register_scoped(lane, tools.clone(), false, Some(&root))
                .await
                .unwrap();
            broker
                .bind_execution(&token, Some(execution.clone()))
                .await
                .unwrap();
            let command = Arguments {
                call_id: format!("compile-{lane}"), name: "functions.exec_command".into(),
                input: json!({"cmd":format!("clang -O0 {lane}.c -o {lane}.out"),"workdir":root,"login":false}).to_string(),
                report: String::new(),
            };
            let id = accepted(&broker, &token, &command).await;
            broker.settle(&token, &[(id.clone(), json!({"output":format!("Process running with session ID {session}\nOutput:\n")}))]).await.unwrap();
            tokens.push(token);
            ids.push(id);
        }
        // Native can publish terminal command history between a running poll
        // receipt and the next poll. This must retire the effect, while the
        // original session's remaining output is still available to its owner.
        broker.settle_native_processes(&["b"], &root, &[json!({"turn_id":execution.turn,"item":{
            "id":ids[1],"type":"commandExecution","status":"completed","processId":"52","exitCode":0,"aggregatedOutput":null
        }})]).await.unwrap();
        let before = broker.effect_status("b").unwrap();
        assert_eq!(before["unconfirmed_effects"], 1);
        assert_eq!(before["shared"]["unconfirmed_effects"], 1);
        let poll = Arguments {
            call_id: "remaining-output".into(),
            name: "functions.write_stdin".into(),
            input: json!({"session_id":52,"chars":"","yield_time_ms":1000}).to_string(),
            report: String::new(),
        };
        for (id, session, chars, target) in [
            ("foreign-completed", 52, "", &tokens[0]),
            ("unknown-completed", 53, "", &tokens[1]),
            ("write-completed", 52, "x", &tokens[1]),
        ] {
            let rejected = Arguments {
                call_id: id.into(),
                input: json!({"session_id":session,"chars":chars}).to_string(),
                ..poll.clone()
            };
            broker.enqueue(target, &rejected).await.unwrap();
            assert!(broker.offer(target).await.unwrap().is_empty());
            assert_eq!(
                broker.invoke(target, rejected).await.unwrap()["error"],
                "native_resource_busy"
            );
        }
        let mut owner_changed = execution.clone();
        owner_changed.start.push('0');
        let mut thread_changed = execution.clone();
        thread_changed.thread.push('x');
        let mut hooks_changed = execution.clone();
        hooks_changed.hook_free_tools.clear();
        for (id, facts) in [
            ("changed-owner", Some(owner_changed)),
            ("changed-thread", Some(thread_changed)),
            ("changed-hooks", Some(hooks_changed)),
            ("missing-facts", None),
        ] {
            broker.bind_execution(&tokens[1], facts).await.unwrap();
            let rejected = Arguments {
                call_id: id.into(),
                ..poll.clone()
            };
            broker.enqueue(&tokens[1], &rejected).await.unwrap();
            assert!(broker.offer(&tokens[1]).await.unwrap().is_empty());
            assert_eq!(
                broker.invoke(&tokens[1], rejected).await.unwrap()["error"],
                if matches!(id, "changed-hooks" | "missing-facts") {
                    "native_execution_completion_unavailable"
                } else {
                    "native_resource_busy"
                }
            );
        }
        // A valid follow-up turn of the same Native owner retains the original
        // receipt's turn identity. It does not transfer ownership to a new PID.
        let mut following = execution.clone();
        following.turn.push_str("-followup");
        broker
            .bind_execution(&tokens[1], Some(following.clone()))
            .await
            .unwrap();
        {
            let mut registry = broker.registry.lock().await;
            let state: ActiveEffects = journal
                .read(&digest(&"native-effect-claims-v1"))
                .unwrap()
                .unwrap();
            assert!(completed_session_poll(
                &registry, &state, "b", 52, &following
            ));
            let original = registry.turns[&tokens[1]].calls["compile-b"].clone();
            assert_eq!(
                original.process_origin.as_ref().unwrap().turn,
                execution.turn
            );
            let mut duplicated = original.clone();
            duplicated.id = "reused-session-owner".into();
            duplicated.completed_session = None;
            registry
                .turns
                .get_mut(&tokens[1])
                .unwrap()
                .calls
                .insert("later-launch".into(), duplicated);
            assert!(
                !completed_session_poll(&registry, &state, "b", 52, &following),
                "numeric session reuse cannot borrow the old completion proof"
            );
            registry
                .turns
                .get_mut(&tokens[1])
                .unwrap()
                .calls
                .remove("later-launch");
            registry
                .turns
                .get_mut(&tokens[1])
                .unwrap()
                .calls
                .get_mut("compile-b")
                .unwrap()
                .process_origin = None;
            assert!(
                !completed_session_poll(&registry, &state, "b", 52, &following),
                "legacy receipts without exact owner facts cannot acquire observation authority"
            );
            registry
                .turns
                .get_mut(&tokens[1])
                .unwrap()
                .calls
                .insert("compile-b".into(), original);
        }
        broker.enqueue(&tokens[1], &poll).await.unwrap();
        let queued = Arguments {
            call_id: "zz-queued-duplicate".into(),
            ..poll.clone()
        };
        broker.enqueue(&tokens[1], &queued).await.unwrap();
        let offered = broker.offer(&tokens[1]).await.unwrap();
        assert_eq!(offered.len(), 1, "verified completed own session must remain observable while an independent compiler runs");
        assert_eq!(
            broker.invoke(&tokens[1], queued).await.unwrap()["error"],
            "native_resource_busy"
        );
        let competing = Arguments {
            call_id: "pending-duplicate-poll".into(),
            ..poll.clone()
        };
        broker.enqueue(&tokens[1], &competing).await.unwrap();
        assert!(broker.offer(&tokens[1]).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&tokens[1], competing).await.unwrap()["error"],
            "native_resource_busy"
        );
        let id = offered[0]["id"].as_str().unwrap().to_owned();
        let output = json!({"type":"function_call_output","call_id":id,"output":"Process exited with code 0\nOutput:\nremaining original output\n"});
        broker
            .settle(&tokens[1], &[(id, output.clone())])
            .await
            .unwrap();
        assert_eq!(
            broker.invoke(&tokens[1], poll.clone()).await.unwrap(),
            output
        );
        let collected = Arguments {
            call_id: "already-collected".into(),
            ..poll
        };
        broker.enqueue(&tokens[1], &collected).await.unwrap();
        assert!(broker.offer(&tokens[1]).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&tokens[1], collected).await.unwrap()["error"],
            "native_resource_busy"
        );
        assert_eq!(
            broker.effect_status("b").unwrap()["shared"],
            before["shared"]
        );
        assert!(broker.active_claim_ids(&["a"]).unwrap().contains(&ids[0]));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn terminal_session_poll_reads_only_the_adopted_retired_chain_after_restart() {
        for completion_before_restart in [false, true] {
            let (root, journal, tools) = fixture();
            let execution = native_effect::compiler_fixture(&root);
            let broker = Broker::new(journal.clone());
            let mut tokens = Vec::new();
            let mut ids = Vec::new();
            let mut commands = Vec::new();
            let mut outputs = Vec::new();
            for (lane, session) in [("a", 51), ("b", 52)] {
                fs::write(
                    root.join(format!("{lane}.c")),
                    "int main(void){return 0;}\n",
                )
                .unwrap();
                let token = broker
                    .register_scoped(lane, tools.clone(), lane == "b", Some(&root))
                    .await
                    .unwrap();
                broker
                    .bind_execution(&token, Some(execution.clone()))
                    .await
                    .unwrap();
                let command = Arguments {
                    call_id: format!("compile-{lane}"), name: "functions.exec_command".into(),
                    input: json!({"cmd":format!("clang -O0 {lane}.c -o {lane}.out"),"workdir":root,"login":false}).to_string(),
                    report: String::new(),
                };
                let id = accepted(&broker, &token, &command).await;
                let output = json!({"output":format!("Process running with session ID {session}\nOutput:\n")});
                broker
                    .settle(&token, &[(id.clone(), output.clone())])
                    .await
                    .unwrap();
                tokens.push(token);
                ids.push(id);
                commands.push(command);
                outputs.push(output);
            }
            broker.retire(&tokens[1], false).await.unwrap();
            let last = broker
                .register_scoped("b", tools.clone(), true, Some(&root))
                .await
                .unwrap();
            broker.source_discovery(&tokens[1], &last).await.unwrap();
            broker.retire(&last, false).await.unwrap();
            // Exercise actual cache eviction, not a hand-built registry. Neither
            // the original receipt nor its retired successor remains cached.
            for _ in 0..=TURN_CACHE_CAPACITY {
                let other = broker
                    .register("other", BTreeMap::new(), false)
                    .await
                    .unwrap();
                broker.retire(&other, false).await.unwrap();
            }
            assert!(!broker.registry.lock().await.turns.contains_key(&tokens[1]));
            assert!(!broker.registry.lock().await.turns.contains_key(&last));
            let completion = json!({"turn_id":execution.turn,"item":{
                "id":ids[1],"type":"commandExecution","status":"completed","processId":"52","exitCode":0,"aggregatedOutput":null
            }});
            if completion_before_restart {
                broker
                    .settle_native_processes(&["b"], &root, std::slice::from_ref(&completion))
                    .await
                    .unwrap();
            }
            broker.close().await;
            let restored = Broker::new(journal.clone());
            if !completion_before_restart {
                restored
                    .settle_native_processes(&["b"], &root, &[completion])
                    .await
                    .unwrap();
            }
            assert_eq!(
                restored.retained_result(&tokens[1], &commands[1]).unwrap(),
                Some(outputs[1].clone())
            );
            let mut following = execution.clone();
            following.turn.push_str("-following");
            let poll = Arguments {
                call_id: "remaining-output".into(),
                name: "functions.write_stdin".into(),
                input: json!({"session_id":52,"chars":"","yield_time_ms":1000}).to_string(),
                report: String::new(),
            };
            // The same lane and Native identity alone cannot resurrect an old
            // source capability or inspect an unrelated chain on disk.
            let unrelated = restored
                .register_scoped("b", tools.clone(), true, Some(&root))
                .await
                .unwrap();
            restored
                .bind_execution(&unrelated, Some(following.clone()))
                .await
                .unwrap();
            restored.enqueue(&unrelated, &poll).await.unwrap();
            assert!(restored.offer(&unrelated).await.unwrap().is_empty());
            assert_eq!(
                restored.invoke(&unrelated, poll.clone()).await.unwrap()["error"],
                "native_resource_busy"
            );
            restored.retire(&unrelated, false).await.unwrap();
            assert_eq!(
                restored
                    .restore_source_discovery(&tokens[1], &last, "foreign")
                    .await,
                Err(Error("mcp_source_discovery_invalid"))
            );
            restored
                .restore_source_discovery(&tokens[1], &last, "b")
                .await
                .unwrap();
            let current = restored
                .register_scoped("b", tools.clone(), true, Some(&root))
                .await
                .unwrap();
            restored.source_discovery(&last, &current).await.unwrap();
            restored
                .bind_execution(&current, Some(following.clone()))
                .await
                .unwrap();
            for retired in [&tokens[1], &last] {
                assert_eq!(
                    restored.catalog(retired).await,
                    Err(Error("mcp_turn_unavailable"))
                );
                assert_eq!(
                    restored.offer(retired).await,
                    Err(Error("mcp_turn_unavailable"))
                );
                assert_eq!(
                    restored.enqueue(retired, &poll).await,
                    Err(Error("mcp_turn_unavailable"))
                );
            }
            let mut changed_owner = following.clone();
            changed_owner.start.push('0');
            let mut changed_thread = following.clone();
            changed_thread.thread.push('x');
            let mut unsafe_hooks = following.clone();
            unsafe_hooks.hook_free_tools.clear();
            for (id, facts) in [
                ("cold-changed-owner", Some(changed_owner)),
                ("cold-changed-thread", Some(changed_thread)),
                ("cold-unsafe-hooks", Some(unsafe_hooks)),
                ("cold-missing-facts", None),
            ] {
                restored.bind_execution(&current, facts).await.unwrap();
                let rejected = Arguments {
                    call_id: id.into(),
                    ..poll.clone()
                };
                restored.enqueue(&current, &rejected).await.unwrap();
                assert!(restored.offer(&current).await.unwrap().is_empty());
                assert_eq!(
                    restored.invoke(&current, rejected).await.unwrap()["error"],
                    if matches!(id, "cold-unsafe-hooks" | "cold-missing-facts") {
                        "native_execution_completion_unavailable"
                    } else {
                        "native_resource_busy"
                    }
                );
            }
            restored
                .bind_execution(&current, Some(following.clone()))
                .await
                .unwrap();
            let before = restored.effect_status("b").unwrap();
            assert_eq!(before["shared"]["unconfirmed_effects"], 1);
            restored.enqueue(&current, &poll).await.unwrap();
            let offered = restored.offer(&current).await.unwrap();
            assert_eq!(offered.len(), 1, "an adopted retired original must preserve uncollected output after cache eviction/restart");
            let duplicate = Arguments {
                call_id: "pending-duplicate".into(),
                ..poll.clone()
            };
            restored.enqueue(&current, &duplicate).await.unwrap();
            assert!(restored.offer(&current).await.unwrap().is_empty());
            assert_eq!(
                restored.invoke(&current, duplicate).await.unwrap()["error"],
                "native_resource_busy"
            );
            let id = offered[0]["id"].as_str().unwrap().to_owned();
            let terminal = json!({"type":"function_call_output","call_id":id,"output":"Process exited with code 0\nOutput:\noriginal uncollected output\n"});
            restored
                .settle(&current, &[(id, terminal.clone())])
                .await
                .unwrap();
            assert_eq!(
                restored.invoke(&current, poll.clone()).await.unwrap(),
                terminal
            );
            assert_eq!(
                restored.retained_result(&tokens[1], &commands[1]).unwrap(),
                Some(outputs[1].clone())
            );
            assert_eq!(
                restored.effect_status("b").unwrap()["shared"],
                before["shared"]
            );
            assert!(
                !restored
                    .registry
                    .lock()
                    .await
                    .turns
                    .contains_key(&tokens[1]),
                "readback must not reactivate or cache an old execution lease"
            );
            restored.retire(&current, false).await.unwrap();
            restored.close().await;
            // A second restart must read the collector too, rather than restoring
            // just the original and accidentally extending its expired proof.
            let collected = Broker::new(journal);
            collected
                .restore_source_discovery(&tokens[1], &current, "b")
                .await
                .unwrap();
            let next = collected
                .register_scoped("b", tools, true, Some(&root))
                .await
                .unwrap();
            collected.source_discovery(&current, &next).await.unwrap();
            collected
                .bind_execution(&next, Some(following))
                .await
                .unwrap();
            collected.enqueue(&next, &poll).await.unwrap();
            assert!(collected.offer(&next).await.unwrap().is_empty());
            assert_eq!(
                collected.invoke(&next, poll).await.unwrap()["error"],
                "native_resource_busy"
            );
            assert_eq!(
                collected
                    .retained_result(
                        &current,
                        &Arguments {
                            call_id: "remaining-output".into(),
                            name: "functions.write_stdin".into(),
                            input: json!({"session_id":52,"chars":"","yield_time_ms":1000})
                                .to_string(),
                            report: String::new()
                        }
                    )
                    .unwrap(),
                Some(terminal)
            );
            assert!(collected
                .active_claim_ids(&["a"])
                .unwrap()
                .contains(&ids[0]));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn terminal_session_poll_cold_chain_rejects_reuse_and_unsettled_collector() {
        for reused_session in [false, true] {
            let (root, journal, tools) = fixture();
            let execution = native_effect::compiler_fixture(&root);
            let broker = Broker::new(journal.clone());
            let mut tokens = Vec::new();
            let mut ids = Vec::new();
            for (lane, session) in [("a", 51), ("b", 52)] {
                fs::write(
                    root.join(format!("{lane}.c")),
                    "int main(void){return 0;}\n",
                )
                .unwrap();
                let token = broker
                    .register_scoped(lane, tools.clone(), lane == "b", Some(&root))
                    .await
                    .unwrap();
                broker
                    .bind_execution(&token, Some(execution.clone()))
                    .await
                    .unwrap();
                let command = Arguments {
                    call_id: format!("compile-{lane}"), name: "functions.exec_command".into(),
                    input: json!({"cmd":format!("clang -O0 {lane}.c -o {lane}.out"),"workdir":root,"login":false}).to_string(), report: String::new(),
                };
                let id = accepted(&broker, &token, &command).await;
                broker.settle(&token, &[(id.clone(), json!({"output":format!("Process running with session ID {session}\nOutput:\n")}))]).await.unwrap();
                tokens.push(token);
                ids.push(id);
            }
            broker.settle_native_processes(&["b"], &root, &[json!({"turn_id":execution.turn,"item":{
                "id":ids[1],"type":"commandExecution","status":"completed","processId":"52","exitCode":0,"aggregatedOutput":null
            }})]).await.unwrap();
            broker.retire(&tokens[1], false).await.unwrap();
            let last = broker
                .register_scoped("b", tools.clone(), true, Some(&root))
                .await
                .unwrap();
            broker.source_discovery(&tokens[1], &last).await.unwrap();
            let mut following = execution.clone();
            following.turn.push_str("-following");
            broker
                .bind_execution(&last, Some(following.clone()))
                .await
                .unwrap();
            let poll = Arguments {
                call_id: "remaining-output".into(),
                name: "functions.write_stdin".into(),
                input: json!({"session_id":52,"chars":""}).to_string(),
                report: String::new(),
            };
            if reused_session {
                let command = Arguments {
                    call_id: "later-launch".into(),
                    name: "functions.exec_command".into(),
                    input:
                        json!({"cmd":"clang -O0 b.c -o b-again.out","workdir":root,"login":false})
                            .to_string(),
                    report: String::new(),
                };
                let id = accepted(&broker, &last, &command).await;
                broker
                    .settle(
                        &last,
                        &[(
                            id.clone(),
                            json!({"output":"Process running with session ID 52\nOutput:\n"}),
                        )],
                    )
                    .await
                    .unwrap();
                broker.settle_native_processes(&["b"], &root, &[json!({"turn_id":following.turn,"item":{
                    "id":id,"type":"commandExecution","status":"completed","processId":"52","exitCode":0,"aggregatedOutput":null
                }})]).await.unwrap();
            } else {
                accepted(&broker, &last, &poll).await;
            }
            broker.close().await;
            let restored = Broker::new(journal);
            if reused_session {
                restored
                    .restore_source_discovery(&tokens[1], &last, "b")
                    .await
                    .unwrap();
                let current = restored
                    .register_scoped("b", tools, true, Some(&root))
                    .await
                    .unwrap();
                restored.source_discovery(&last, &current).await.unwrap();
                restored
                    .bind_execution(&current, Some(following))
                    .await
                    .unwrap();
                restored.enqueue(&current, &poll).await.unwrap();
                assert!(
                    restored.offer(&current).await.unwrap().is_empty(),
                    "numeric reuse in the retired chain cannot borrow the first session's proof"
                );
                assert_eq!(
                    restored.invoke(&current, poll).await.unwrap()["error"],
                    "native_resource_busy"
                );
            } else {
                assert_eq!(
                    restored
                        .restore_source_discovery(&tokens[1], &last, "b")
                        .await,
                    Err(Error("mcp_source_discovery_invalid")),
                    "a retired pending collector is still unresolved, never safe absence"
                );
                assert_eq!(
                    restored.catalog(&last).await,
                    Err(Error("mcp_turn_unavailable"))
                );
                assert!(restored
                    .recorded_calls(&last)
                    .unwrap()
                    .iter()
                    .any(|call| call["offered"] == true && call["settled"] == false));
            }
            assert!(restored.active_claim_ids(&["a"]).unwrap().contains(&ids[0]));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn terminal_session_poll_external_receipt_is_bound_and_expires_after_collection() {
        let (root, journal, _) = fixture();
        let execution = native_effect::compiler_fixture(&root);
        let broker = Broker::new(journal.clone());
        for (lane, session) in [("a", 51), ("b", 52)] {
            fs::write(
                root.join(format!("{lane}.c")),
                "int main(void){return 0;}\n",
            )
            .unwrap();
            let input = json!({"cmd":format!("clang -O0 {lane}.c -o {lane}.out"),"workdir":root,"login":false}).to_string();
            broker
                .admit_external_tool_with_execution(
                    lane,
                    "exec",
                    "functions.exec_command",
                    &input,
                    &root,
                    Some(&execution),
                )
                .await
                .unwrap();
            broker.settle_external_tools(lane, &[("exec".into(), json!({"output":format!("Process running with session ID {session}\nOutput:\n")}))]).await.unwrap();
        }
        broker.settle_native_processes(&["b"], &root, &[json!({"turn_id":execution.turn,"item":{
            "id":"exec","type":"commandExecution","status":"completed","processId":"52","exitCode":0,"aggregatedOutput":null
        }})]).await.unwrap();
        let input = json!({"session_id":52,"chars":""}).to_string();
        let mut unsafe_hooks = execution.clone();
        unsafe_hooks.hook_free_tools.clear();
        assert_eq!(
            broker
                .admit_external_tool_with_execution(
                    "b",
                    "unsafe",
                    "functions.write_stdin",
                    &input,
                    &root,
                    Some(&unsafe_hooks)
                )
                .await,
            Err(Error("native_execution_completion_unavailable"))
        );
        // External receipt proof survives a Provider instance restart without
        // recreating a claim, command, owner or result.
        let restored = Broker::new(journal);
        restored
            .admit_external_tool_with_execution(
                "b",
                "poll",
                "functions.write_stdin",
                &input,
                &root,
                Some(&execution),
            )
            .await
            .unwrap();
        assert_eq!(
            restored
                .admit_external_tool_with_execution(
                    "b",
                    "pending",
                    "functions.write_stdin",
                    &input,
                    &root,
                    Some(&execution)
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        restored
            .settle_external_tools(
                "b",
                &[(
                    "poll".into(),
                    json!({"output":"Process exited with code 0\nOutput:\noriginal remainder\n"}),
                )],
            )
            .await
            .unwrap();
        assert_eq!(
            restored
                .admit_external_tool_with_execution(
                    "b",
                    "collected",
                    "functions.write_stdin",
                    &input,
                    &root,
                    Some(&execution)
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        let status = restored.effect_status("a").unwrap();
        assert_eq!(status["unconfirmed_effects"], 1);
        assert_eq!(status["shared"]["unconfirmed_effects"], 1);
        assert!(restored
            .active_claim_ids(&["a"])
            .unwrap()
            .contains(&external_effect_id("a", "exec")));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn explicit_generic_origin_keeps_finite_tool_semantics_and_legacy_reads_stay_available() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            let (root, journal, tools) = fixture_with_process_aliases(exec_tool, poll_tool);
            let broker = Broker::new(journal.clone());
            let token = broker
                .register("generic-owner", tools, false)
                .await
                .unwrap();
            let original = Arguments {
                call_id: "original-generic".into(),
                name: exec_tool.into(),
                input: "{}".into(),
                report: String::new(),
            };
            broker.enqueue(&token, &original).await.unwrap();
            let offered = broker.offer(&token).await.unwrap();
            assert_eq!(offered.len(), 1);
            let id = offered[0]["id"].as_str().unwrap().to_owned();
            let result = json!({"output":"ordinary finite tool response"});
            broker
                .settle(&token, &[(id, result.clone())])
                .await
                .unwrap();
            assert_eq!(
                broker.effect_status("generic-owner").unwrap()["unconfirmed_effects"],
                0,
                "a generic tool name does not acquire Native process lifetime"
            );
            assert_eq!(
                broker.require_native_origin(&token),
                Err(Error("mcp_tool_origin_changed"))
            );
            let key = digest(&("mcp-turn-v1", &token));
            let before: Turn = journal.read(&key).unwrap().unwrap();
            assert_eq!(before.origin, crate::protocol::ToolOrigin::Generic);
            // An old record without provenance is readable, but cannot use that
            // absence to grant a fresh effect. Never rewrite its original call.
            {
                let mut old = serde_json::to_value(&before).unwrap();
                old.as_object_mut().unwrap().remove("origin");
                let old: Turn = serde_json::from_value(old).unwrap();
                assert_eq!(old.origin, crate::protocol::ToolOrigin::Native);
                broker.persist(&token, &old).unwrap();
                broker
                    .registry
                    .lock()
                    .await
                    .turns
                    .insert(token.clone(), old);
            }
            assert_eq!(
                broker.retained_result(&token, &original).unwrap(),
                Some(result.clone())
            );
            broker.enqueue(&token, &original).await.unwrap();
            assert!(broker.offer(&token).await.unwrap().is_empty());
            assert_eq!(
                broker.invoke(&token, original.clone()).await.unwrap(),
                result
            );
            let next = Arguments {
                call_id: "new-effect".into(),
                ..original.clone()
            };
            broker.enqueue(&token, &next).await.unwrap();
            assert!(broker.offer(&token).await.unwrap().is_empty());
            assert_eq!(
                broker.retained_result(&token, &next).unwrap().unwrap()["error"],
                "native_execution_completion_unavailable"
            );
            let after: Turn = journal.read(&key).unwrap().unwrap();
            assert_eq!(
                serde_json::to_value(&after.calls[&original.call_id]).unwrap(),
                serde_json::to_value(&before.calls[&original.call_id]).unwrap()
            );
            assert_eq!(
                broker.effect_status("generic-owner").unwrap()["unconfirmed_effects"],
                0
            );
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[cfg(test)]
#[path = "mcp_scope_settlement_tests.rs"]
mod scope_settlement_tests;
