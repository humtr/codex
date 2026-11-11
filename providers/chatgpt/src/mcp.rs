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
    result: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    native_input: Vec<Value>,
    #[serde(default)]
    effect: Option<Effect>,
}
impl Invocation {
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
}
#[derive(Clone, Serialize, Deserialize)]
struct ExternalCompletion {
    fingerprint: String,
    running: Option<u64>,
    exited: bool,
    #[serde(default)]
    not_spawned: bool,
}
impl ExternalCompletion {
    fn from_result(result: &Value) -> Self {
        let (running, exited) = native_process_result(result);
        let not_spawned = crate::native_claims::verified_exec_not_spawned(result);
        Self {
            fingerprint: digest(result),
            running,
            exited,
            not_spawned,
        }
    }

    fn marker(&self) -> Value {
        if let Some(session) = self.running {
            json!({"output":format!("Process running with session ID {session}")})
        } else if self.exited {
            json!({"output":"Process exited with code 0"})
        } else if self.not_spawned {
            json!({"error":"native_call_not_dispatched","executed":false})
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
fn settle_effect(
    claims: &mut ActiveEffects,
    id: &str,
    lane: &str,
    name: &str,
    continuation: Option<u64>,
    result: &Value,
) {
    let (running, exited) = native_process_result(result);
    let not_spawned =
        name == "functions.exec_command" && crate::native_claims::verified_exec_not_spawned(result);
    if name == "functions.exec_command" && running.is_some() && !exited && !not_spawned {
        if let Some(claim) = claims.claims.get_mut(id) {
            claim.session_id = running;
        }
        return;
    }
    // A response string without an observed Native exit is not proof that
    // a possibly backgrounded shell writer stopped. Keep BOTH the Provider
    // journal and the shared Provider claim until a real terminal receipt arrives.
    if name == "functions.exec_command" && !exited && !not_spawned {
        return;
    }
    if let Some(session) = continuation {
        if exited {
            claims
                .claims
                .retain(|_, held| !(held.lane == lane && held.session_id == Some(session)));
        }
    }
    claims.claims.remove(id);
}
/// These markers originate in the native tool's own public execution result.
/// An unrecognized or failed continuation does not establish process exit.
fn native_process_result(value: &Value) -> (Option<u64>, bool) {
    let output = value
        .get("output")
        .and_then(Value::as_str)
        .or_else(|| value.get("content").and_then(Value::as_str))
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    // The tool's status header precedes the "Output:" marker. Child process
    // stdout is untrusted and can itself print status-looking strings: never
    // allow that text to establish process exit or a new session lease.
    let status = output
        .split_once("\nOutput:\n")
        .map(|(head, _)| head)
        .or_else(|| output.split_once("\\nOutput:\\n").map(|(head, _)| head))
        .unwrap_or(&output);
    let session = status
        .find("Process running with session ID ")
        .and_then(|pos| {
            status[pos + "Process running with session ID ".len()..]
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|s| s.parse::<u64>().ok())
        });
    let exited = session.is_none()
        && (status.contains("Process exited with code ")
            || status.contains("Process exited with signal "));
    (session, exited)
}
fn continued_process(name: &str, input: &str) -> Option<u64> {
    (name == "functions.write_stdin")
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
                "resource_kind":match held.effect {
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
                    && turn
                        .calls
                        .values()
                        .any(|call| call.offered && call.result.is_none())
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
    /// on the stopped work's lanes whose call IDs never reached Native items.
    pub(crate) async fn reconcile_stopped_unexecuted(
        &self,
        lanes: &[&str],
        tokens: &[&str],
        committed_native_items: &BTreeSet<String>,
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let key = digest(&"native-effect-claims-v1");
        let state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let mut candidate_tokens: BTreeSet<String> =
            tokens.iter().map(|token| (*token).to_owned()).collect();
        for (token, turn) in &registry.turns {
            if lanes.contains(&turn.lane.as_str()) {
                candidate_tokens.insert(token.clone());
            }
        }
        for claim in state.claims.values() {
            if lanes.contains(&claim.lane.as_str()) {
                if let Some(token) = &claim.turn_token {
                    candidate_tokens.insert(token.clone());
                }
            }
        }
        let not_dispatched = json!({"error":"native_call_not_dispatched","executed":false});
        for token in candidate_tokens {
            let Some(mut turn) = registry.turns.get(&token).cloned().or(self
                .journal
                .read::<Turn>(&digest(&("mcp-turn-v1", &token)))?)
            else {
                continue;
            };
            if !lanes.contains(&turn.lane.as_str()) {
                continue;
            }
            let mut turn_changed = false;
            if !turn.retired {
                turn.retire(true);
                turn_changed = true;
            }
            for call in turn.calls.values_mut() {
                if call.offered
                    && call.result.is_none()
                    && !committed_native_items.contains(&call.id)
                {
                    call.result = Some(not_dispatched.clone());
                    turn_changed = true;
                }
            }
            if turn_changed {
                self.persist(&token, &turn)?;
                if let Some(cached) = registry.turns.get_mut(&token) {
                    *cached = turn;
                }
            }
        }
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let mut state_changed = false;
        let committed_external: BTreeSet<String> = lanes
            .iter()
            .flat_map(|lane| {
                committed_native_items
                    .iter()
                    .map(move |id| external_effect_id(lane, id))
            })
            .collect();
        for (effect_id, receipt) in &mut state.external {
            if !receipt.settled
                && receipt.continuation.is_none()
                && receipt.completion.is_none()
                && receipt.lane.as_deref().is_some_and(|l| lanes.contains(&l))
                && !committed_external.contains(effect_id)
            {
                receipt.completion = Some(ExternalCompletion::from_result(&not_dispatched));
                state_changed = true;
            }
        }
        if state_changed {
            self.journal.write(&key, &state)?;
        }
        self.reconcile_persisted_integrated_receipts()?;
        self.changed.notify_waiters();
        Ok(())
    }
    /// These entry points are used only at the Native-confirmed stopped-work
    /// boundary. The shared OS ledger is independent of the Provider mirror.
    pub(crate) fn has_stopped_shared_unexecuted(
        &self,
        lane: &str,
        workspace: &Path,
    ) -> Result<bool, Error> {
        SharedNativeGuard::has_stopped_unexecuted(
            workspace,
            self.shared_guard_home().as_deref(),
            lane,
        )
        .map_err(Self::shared_guard_error)
    }

    pub(crate) fn settle_stopped_shared_unexecuted(
        &self,
        lane: &str,
        workspace: &Path,
        native_item_ids: &BTreeSet<String>,
    ) -> Result<usize, Error> {
        SharedNativeGuard::settle_stopped_unexecuted(
            workspace,
            self.shared_guard_home().as_deref(),
            lane,
            native_item_ids,
        )
        .map_err(Self::shared_guard_error)
    }

    /// The caller has already persisted and independently verified a Native
    /// Stop: its thread is terminal/idle and has no owned background terminals.
    /// Unknown effects cannot be retired by Stop; only the original observed
    /// process-session reservations for that same Native thread are eligible.
    /// Release the shared ledger first, then the Provider mirror. A failed
    /// mirror commit keeps conservative claims that a later confirmed Stop or
    /// stopped-work claim can reconcile without executing a tool again.
    pub(crate) async fn settle_stopped_processes(
        &self,
        lane: &str,
        workspace: &Path,
    ) -> Result<(), Error> {
        let _registry = self.registry.lock().await;
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let processes = state
            .claims
            .iter()
            .filter(|(_, claim)| claim.lane == lane)
            .filter_map(|(id, claim)| claim.session_id.map(|session| (id.clone(), session)))
            .collect::<BTreeMap<_, _>>();
        if processes.is_empty() {
            return Ok(());
        }
        SharedNativeGuard::release_observed_processes(
            workspace,
            self.shared_guard_home().as_deref(),
            lane,
            &processes,
        )
        .map_err(Self::shared_guard_error)?;
        state.claims.retain(|id, _| !processes.contains_key(id));
        self.journal.write(&key, &state)?;
        Ok(())
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
    /// Native's committed command end is independent of an app's next poll.
    /// These items are read through the verified original Native work binding;
    /// they never replace its original MCP invocation/result.
    pub(crate) async fn settle_native_processes(
        &self,
        lanes: &[&str],
        workspace: &Path,
        history: &[Value],
    ) -> Result<(), Error> {
        let mut registry = self.registry.lock().await;
        let key = digest(&"native-effect-claims-v1");
        let mut state: ActiveEffects = self.journal.read(&key)?.unwrap_or_default();
        let mut ended = BTreeMap::<String, BTreeMap<String, u64>>::new();
        let mut retired_settled = false;
        for (id, claim) in &state.claims {
            if !lanes.contains(&claim.lane.as_str()) {
                continue;
            }
            if let Some(session) = claim.session_id {
                if history.iter().any(|row| {
                    let item = &row["item"];
                    item["type"] == "commandExecution"
                        && item["id"].as_str().is_some_and(|native_id| {
                            native_id == id
                                || state.external.contains_key(id)
                                    && external_effect_id(&claim.lane, native_id) == *id
                        })
                        && item["processId"]
                            .as_str()
                            .and_then(|value| value.parse::<u64>().ok())
                            == Some(session)
                        && matches!(item["status"].as_str(), Some("completed" | "failed"))
                        // Native uses -1 for unconfirmed errors and 124 for its
                        // timeout path, which can request termination before wait.
                        // Neither sentinel independently establishes process exit.
                        && item["exitCode"].as_i64().is_some_and(|code| code >= 0 && code != 124)
                }) {
                    ended
                        .entry(claim.lane.clone())
                        .or_default()
                        .insert(id.clone(), session);
                }
                continue;
            }
            let Some(token) = claim.turn_token.as_deref() else {
                continue;
            };
            let Some(result) = history
                .iter()
                .find(|row| row["item"]["id"].as_str() == Some(id.as_str()))
                .and_then(|row| native_history_item_result(&row["item"]))
            else {
                continue;
            };
            let mut turn = match registry.turns.get(token).cloned() {
                Some(turn) => turn,
                None => match self
                    .journal
                    .read::<Turn>(&digest(&("mcp-turn-v1", token)))?
                {
                    Some(turn) => turn,
                    None => continue,
                },
            };
            if !turn.retired || turn.lane != claim.lane {
                continue;
            }
            let mut updated = false;
            for call in turn.calls.values_mut() {
                if call.id == *id && call.offered && call.result.is_none() {
                    call.result = Some(result.clone());
                    updated = true;
                }
            }
            if updated {
                self.persist(token, &turn)?;
                if registry.turns.contains_key(token) {
                    registry.turns.insert(token.to_owned(), turn);
                }
                retired_settled = true;
            }
        }
        for (lane, sessions) in &ended {
            SharedNativeGuard::release_observed_processes(
                workspace,
                self.shared_guard_home().as_deref(),
                lane,
                sessions,
            )
            .map_err(Self::shared_guard_error)?;
        }
        if !ended.is_empty() {
            state.claims.retain(|id, claim| {
                !ended
                    .get(&claim.lane)
                    .is_some_and(|processes| processes.contains_key(id))
            });
            self.journal.write(&key, &state)?;
            self.changed.notify_waiters();
        }
        drop(registry);
        if retired_settled {
            self.reconcile_persisted_integrated_receipts()?;
            self.changed.notify_waiters();
        }
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
        let facts = matches!(name, "apply_patch" | "functions.apply_patch")
            .then(|| native_effect::hook_fixture(workspace));
        self.admit_external_tool_with_execution(lane, id, name, input, workspace, facts.as_ref())
            .await
    }

    pub(crate) async fn admit_external_tool_with_execution(
        &self,
        lane: &str,
        id: &str,
        name: &str,
        input: &str,
        workspace: &Path,
        execution: Option<&native_effect::Execution>,
    ) -> Result<(), Error> {
        let _registry = self.registry.lock().await;
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
        let effect = native_effect::classify_with_execution(name, input, Some(&root), execution);
        let read_only = effect == Effect::Read;
        let continuation = continued_process(name, input);
        if state.claims.values().any(|held| {
            native_effect::conflicts(&effect, &held.effect)
                && !(continuation.is_some() && held.lane == lane && held.session_id == continuation)
        }) {
            return Err(Error("native_resource_busy"));
        }
        native_effect::can_offer(&effect, &root).map_err(Error)?;
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
            SharedNativeGuard::offer_external_with_effect(
                &root,
                self.shared_guard_home().as_deref(),
                lane,
                &resource_id,
                name,
                input,
                effect.clone(),
            )
            .map_err(Self::shared_guard_error)?;
        }
        let continuing_owned = continuation.is_some_and(|session| {
            state
                .claims
                .values()
                .any(|held| held.lane == lane && held.session_id == Some(session))
        });
        if effect != Effect::Read && !continuing_owned {
            state.claims.insert(
                resource_id.clone(),
                ClaimedEffect {
                    lane: lane.into(),
                    effect,
                    session_id: None,
                    turn_token: None,
                    continuations: BTreeMap::new(),
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
            if let Some(receipt) = state.external.get_mut(&effect_id).filter(|v| !v.settled) {
                // This trusted original lane and delivery ID also identifies
                // receipts created before durable lane ownership was recorded.
                // Never guess a lane from an arbitrary orphaned journal entry.
                if receipt.lane.is_none() {
                    receipt.lane = Some(lane.to_owned());
                    changed = true;
                }
                let completion = ExternalCompletion::from_result(result);
                if receipt
                    .completion
                    .as_ref()
                    .is_some_and(|old| old.fingerprint != completion.fingerprint)
                {
                    return Err(Error("native_effect_result_changed"));
                }
                if receipt.completion.is_none() {
                    receipt.completion = Some(completion);
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
            let Some(result) = call.result.as_ref() else {
                continue;
            };
            if let Some(workspace) = turn
                .workspace
                .as_deref()
                .filter(|_| call.effect != Some(Effect::Read))
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
            settle_effect(
                &mut state,
                &id,
                &lane,
                &call.name,
                continued_process(&call.name, &call.input),
                result,
            );
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
            if call.name != "functions.write_stdin"
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
                .filter(|_| call.effect != Some(Effect::Read))
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
                (!receipt.settled)
                    .then(|| {
                        receipt
                            .completion
                            .as_ref()
                            .map(|proof| (id.clone(), receipt.clone(), proof.marker()))
                    })
                    .flatten()
            })
            .collect();
        for (id, receipt, result) in completed {
            let Some(lane) = receipt.lane.as_deref() else {
                // Historical receipts without a verified lane cannot be
                // silently released after a restart.
                continue;
            };
            if let Some(root) = receipt.workspace.as_deref().filter(|_| !receipt.read_only) {
                SharedNativeGuard::settle_external_receipt(
                    root,
                    self.shared_guard_home().as_deref(),
                    lane,
                    &id,
                    &receipt.name,
                    receipt.continuation,
                    &result,
                )
                .map_err(Self::shared_guard_error)?;
            }
            settle_effect(
                &mut state,
                &id,
                lane,
                &receipt.name,
                receipt.continuation,
                &result,
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
    pub async fn register(
        &self,
        lane: &str,
        tools: BTreeMap<String, Tool>,
        source_bound: bool,
    ) -> Result<String, Error> {
        self.register_scoped(lane, tools, source_bound, None).await
    }
    /// Capture the physical root once at Native work admission, not from
    /// untrusted tool arguments. A missing root can admit only opaque writes.
    pub async fn register_scoped(
        &self,
        lane: &str,
        tools: BTreeMap<String, Tool>,
        source_bound: bool,
        workspace: Option<&Path>,
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
        let mut old = old.into_owned();
        old.discovery = Some(next.into());
        self.persist(previous, &old)?;
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
                result: None,
                native_input: Vec::new(),
                effect: Some(effect),
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
        let turns = &mut registry.turns;
        let turn = turns
            .get_mut(token)
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
                let effect = call.effect.clone().unwrap_or(Effect::Opaque);
                let continuation = continued_process(&call.name, &call.input);
                // The call's authoritative turn still says it was not offered.
                // Repair a crash between a previous shared reservation and turn
                // commit, including its local non-authoritative claim mirror.
                if claims.claims.remove(&call.id).is_some() {
                    claims_changed = true;
                }
                if claims.claims.values().any(|held| {
                    native_effect::conflicts(&effect, &held.effect)
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
                if let Effect::Paths(_) = &effect {
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
                    match SharedNativeGuard::offer_external_with_effect(
                        workspace,
                        self.shared_guard_home().as_deref(),
                        &next.lane,
                        &call.id,
                        &call.name,
                        &call.input,
                        effect.clone(),
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
                            if let std::collections::btree_map::Entry::Vacant(item) =
                                held.continuations.entry(call.id.clone())
                            {
                                item.insert(token.to_owned());
                                claims_changed = true;
                            }
                        }
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
                        },
                    );
                    claims_changed = true;
                }
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
            *turn = next;
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
                if call.effect != Some(Effect::Read) {
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
                settle_effect(
                    &mut claims,
                    id,
                    &turn.lane,
                    &call.name,
                    continued_process(&call.name, &call.input),
                    result,
                );
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
                        token,
                        DiscoveryAdmission {
                            lane: lane.into(),
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
        assert!(!state.claims.contains_key("first-call"));
    }

    #[tokio::test]
    async fn nc_confirmed_stop_releases_original_session_and_preserves_unconfirmed_effects() {
        let (root, journal, _) = fixture();
        fs::write(root.join("file"), "alpha\n").unwrap();
        let broker = Broker::new(journal);
        let running = r#"{"cmd":"sleep 30; printf result > file"}"#;
        broker
            .admit_external_tool("owner", "run", "functions.exec_command", running, &root)
            .await
            .unwrap();
        broker
            .settle_external_tools(
                "owner",
                &[(
                    "run".into(),
                    json!({"output":"Process running with session ID 83\nOutput:\n"}),
                )],
            )
            .await
            .unwrap();
        let contested = r#"{"cmd":"printf other > file"}"#;
        assert_eq!(
            broker
                .admit_external_tool(
                    "other",
                    "blocked",
                    "functions.exec_command",
                    contested,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        broker
            .settle_stopped_processes("other", &root)
            .await
            .unwrap();
        assert_eq!(
            broker
                .admit_external_tool(
                    "other",
                    "blocked",
                    "functions.exec_command",
                    contested,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        broker
            .settle_stopped_processes("owner", &root)
            .await
            .unwrap();
        assert_eq!(
            broker.effect_status("owner").unwrap()["unconfirmed_effects"],
            0
        );
        broker
            .admit_external_tool(
                "other",
                "blocked",
                "functions.exec_command",
                contested,
                &root,
            )
            .await
            .unwrap();
        broker
            .settle_external_tools(
                "other",
                &[(
                    "blocked".into(),
                    json!({"output":"Process exited with code 0\nOutput:\n"}),
                )],
            )
            .await
            .unwrap();

        let (_, _, catalog) = fixture();
        let token = broker
            .register_scoped("lane_app_owner", catalog, false, Some(&root))
            .await
            .unwrap();
        let unexecuted_call = Arguments {
            call_id: "unexecuted-mcp-call".into(),
            name: "functions.exec_command".into(),
            input: r#"{"cmd":"pwd"}"#.into(),
            report: "probe".into(),
        };
        let unexecuted_id = accepted(&broker, &token, &unexecuted_call).await;
        let mut owners = BTreeMap::new();
        owners.insert("lane_app_owner".into(), "work-owner".into());
        let status = broker
            .effect_status_with_owners(&["work-owner", "lane_app_owner"], &owners)
            .unwrap();
        assert_eq!(status["unconfirmed_effects"], 1);
        assert_eq!(status["active"][0]["owner_work"], "work-owner");
        assert_eq!(status["active"][0]["owned"], true);
        let other_status = broker
            .effect_status_with_owners(&["other"], &owners)
            .unwrap();
        assert_eq!(other_status["active"][0]["owner_work"], "work-owner");
        assert_eq!(other_status["active"][0]["owned"], false);
        assert_eq!(
            broker
                .admit_external_tool(
                    "other",
                    "blocked-by-mcp",
                    "functions.exec_command",
                    contested,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        // If Native thread items contain the offered call ID, Stop must not
        // release it as unexecuted.
        let mut committed = BTreeSet::new();
        committed.insert(unexecuted_id.clone());
        broker
            .reconcile_stopped_unexecuted(&["work-owner", "lane_app_owner"], &[&token], &committed)
            .await
            .unwrap();
        assert_eq!(
            broker.effect_status("work-owner").unwrap()["unconfirmed_effects"],
            1
        );
        assert_eq!(
            broker
                .admit_external_tool(
                    "other",
                    "blocked-by-mcp",
                    "functions.exec_command",
                    contested,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );
        // Once Native thread items at a confirmed Stop boundary prove the
        // offered call ID never reached Native, reconcile it as not dispatched
        // and release the conflicting reservation.
        broker
            .reconcile_stopped_unexecuted(
                &["work-owner", "lane_app_owner"],
                &[&token],
                &BTreeSet::new(),
            )
            .await
            .unwrap();
        assert_eq!(
            broker.effect_status("work-owner").unwrap()["unconfirmed_effects"],
            0
        );
        let recorded = broker.recorded_calls(&token).unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0]["id"], unexecuted_id);
        assert_eq!(recorded[0]["settled"], true);
        broker
            .admit_external_tool(
                "other",
                "unblocked-after-stop",
                "functions.exec_command",
                contested,
                &root,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn nc_confirmed_stop_releases_orphaned_physical_claim_after_mirror_settled() {
        let (root, journal, catalog) = fixture();
        fs::write(root.join("file"), "original\n").unwrap();
        let broker = Broker::new(journal.clone());
        let token = broker
            .register_scoped("stopped-work", catalog, false, Some(&root))
            .await
            .unwrap();
        let call = Arguments {
            call_id: "orphaned-shell".into(),
            name: "functions.exec_command".into(),
            input: r#"{"cmd":"printf updated > file"}"#.into(),
            report: "bounded fixture".into(),
        };
        let id = accepted(&broker, &token, &call).await;
        broker
            .settle(
                &token,
                &[(
                    id.clone(),
                    json!({"output":"connection closed before terminal proof"}),
                )],
            )
            .await
            .unwrap();

        // Simulate the observed crash/recovery split: the original call
        // result is durable and the Provider mirror has no active writer,
        // but the OS-shared journal still contains the exact opaque claim.
        let key = digest(&"native-effect-claims-v1");
        let mut mirror: ActiveEffects = journal.read(&key).unwrap().unwrap();
        assert!(mirror.claims.remove(&id).is_some());
        journal.write(&key, &mirror).unwrap();
        assert_eq!(
            broker.effect_status("stopped-work").unwrap()["unconfirmed_effects"],
            0
        );
        let competing = r#"{"cmd":"printf other > file"}"#;
        assert_eq!(
            broker
                .admit_external_tool(
                    "other-work",
                    "before",
                    "functions.exec_command",
                    competing,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );

        // An observed Native item is evidence that the original call may
        // have executed; a stopped work must not free its reservation.
        broker
            .reconcile_stopped_unexecuted(
                &["stopped-work"],
                &[&token],
                &BTreeSet::from([id.clone()]),
            )
            .await
            .unwrap();
        assert_eq!(
            broker
                .admit_external_tool(
                    "other-work",
                    "still-blocked",
                    "functions.exec_command",
                    competing,
                    &root
                )
                .await,
            Err(Error("native_resource_busy"))
        );

        // The physical OS ledger is not the Provider's active-claim mirror.
        // The original call ID seen in Native keeps the physical writer held.
        assert_eq!(
            broker
                .settle_stopped_shared_unexecuted(
                    "stopped-work",
                    &root,
                    &BTreeSet::from([id.clone()])
                )
                .unwrap(),
            0
        );

        // A verified Native Stop with no jobs and complete item history
        // absent the original call ID allows exact physical reconciliation,
        // even after the Provider mirror and saved call result have settled.
        broker
            .reconcile_stopped_unexecuted(&["stopped-work"], &[&token], &BTreeSet::new())
            .await
            .unwrap();
        assert!(broker
            .has_stopped_shared_unexecuted("stopped-work", &root)
            .unwrap());
        assert_eq!(
            broker
                .settle_stopped_shared_unexecuted("stopped-work", &root, &BTreeSet::new())
                .unwrap(),
            1
        );
        assert_eq!(broker.recorded_calls(&token).unwrap()[0]["settled"], true);
        broker
            .admit_external_tool(
                "other-work",
                "after-confirmed-stop",
                "functions.exec_command",
                competing,
                &root,
            )
            .await
            .unwrap();
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
        if call.name == "functions.apply_patch" {
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
            Err(Error("native_resource_busy"))
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
            Err(Error("native_resource_busy"))
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
    async fn normalized_startup_failure_settles_only_its_original_effect() {
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
            0
        );
        // Duplicate and late receipt delivery cannot offer the command again.
        broker
            .settle(&token, &[(id.clone(), failure.clone())])
            .await
            .unwrap();
        assert_eq!(broker.invoke(&token, exec.clone()).await.unwrap(), failure);
        assert!(broker.offer(&token).await.unwrap().is_empty());
        let mut next = exec.clone();
        next.call_id = "next".into();
        next.input = json!({"cmd":"printf next"}).to_string();
        let next_id = accepted(&broker, &token, &next).await;
        // A returned error without the handler's pre-spawn envelope is unknown.
        let unknown =
            json!({"id":next_id,"name":exec.name,"output":"connection lost after dispatch"});
        broker.settle(&token, &[(next_id, unknown)]).await.unwrap();
        assert_eq!(
            broker.effect_status("startup-work").unwrap()["unconfirmed_effects"],
            1
        );
        broker.settle(&token, &[(id, failure)]).await.unwrap();
        assert_eq!(
            broker.effect_status("startup-work").unwrap()["unconfirmed_effects"],
            1
        );
        let other = broker
            .register_scoped("other-work", tools, false, Some(&root))
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
            ExternalCompletion::from_result(&json!({"output":"patch applied"})),
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
        let _poll_id = accepted(&broker, &owner, &poll).await;
        // Simulate a crash after the continuation's terminal result was
        // journaled, but before the physical process lease was settled.
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&owner).unwrap();
            turn.calls.get_mut(&poll.call_id).unwrap().result =
                Some(json!({"output":"Process exited with code 0"}));
            broker.persist(&owner, turn).unwrap();
        }
        drop(broker);
        let recovered = Broker::new(journal);
        let other = recovered
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        let edit = patch("file", "alpha", "ALPHA");
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
        accepted(&broker, &later_turn, &poll).await;
        {
            let mut registry = broker.registry.lock().await;
            let turn = registry.turns.get_mut(&later_turn).unwrap();
            turn.calls.get_mut(&poll.call_id).unwrap().result =
                Some(json!({"output":"Process exited with code 0"}));
            broker.persist(&later_turn, turn).unwrap();
        }
        drop(broker);
        let recovered = Broker::new(journal);
        let other = recovered
            .register_scoped("other-work", tools, false, Some(&root))
            .await
            .unwrap();
        let competing = patch("file", "alpha", "ALPHA");
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
        let item = json!({"turn_id":"original-native-turn","item":{"id":id,"type":"commandExecution","processId":"56351","status":"failed","exitCode":2}});
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
            .settle(
                &other,
                &[(next, json!({"output":"Process exited with code 0"}))],
            )
            .await
            .unwrap();
        // LocalFunction reserves the same Native effect without an MCP token.
        // Its Native item carries the original call id, not the reservation id.
        broker
            .admit_external_tool(
                "local-function",
                "native-call",
                &exec.name,
                &exec.input,
                &root,
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
        let local_item = json!({"item":{"id":"native-call","type":"commandExecution","processId":"56351","status":"completed","exitCode":0}});
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
                    json!({"output":"Process exited with code 0"}),
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
        broker.enqueue(&two, &second).await.unwrap();
        assert!(broker.offer(&two).await.unwrap().is_empty());
        broker
            .settle(
                &one,
                &[(
                    opaque,
                    json!({"output":
                "Chunk ID: terminal\nProcess exited with code 0\nOutput:\ndone"}),
                )],
            )
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
                &[(
                    first_poll,
                    json!({"output":"Process running with session ID 1200"}),
                )],
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
                &[(final_poll, json!({"output":"Process exited with code 0"}))],
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
            .settle_external_tools(
                "owner",
                &[(
                    "poll".into(),
                    json!({"output":"Process exited with code 0"}),
                )],
            )
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
        broker.enqueue(&token, &command).await.unwrap();
        let offered = broker.offer(&token).await.unwrap();
        let id = offered[0]["id"].as_str().unwrap().to_owned();
        broker.retire(&token, true).await.unwrap();
        assert!(broker.has_pending_processes(&["owner"]).unwrap());
        assert!(broker.active_claim_ids(&["owner"]).unwrap().contains(&id));
        let other_token = broker
            .register_scoped("other", tools, false, Some(&root))
            .await
            .unwrap();
        let write = patch("file", "alpha", "ALPHA");
        assert_eq!(
            broker
                .admit_external_tool("other", "ext-write", &write.name, &write.input, &root)
                .await,
            Err(Error("native_resource_busy"))
        );
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
        assert!(!broker.has_pending_processes(&["owner"]).unwrap());
        assert!(broker.active_claim_ids(&["owner"]).unwrap().is_empty());
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
            broker.enqueue(&token, &call).await.unwrap();
            let id = broker.offer(&token).await.unwrap()[0]["id"]
                .as_str()
                .unwrap()
                .to_owned();
            let original = json!({"type":"function_call_output","call_id":id,
                "output":format!("Chunk ID: original\nWall time: 0.03 seconds\nProcess exited with code 0\nOutput:\n{}", "complete output\n".repeat(10000))});
            let projection = json!({"type":"function_call_output","call_id":id,
                "output":"Process exited with code 0\nOutput:\nmodel-truncated output"});
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
            let exec = Arguments {
                call_id: "original-exec".into(),
                name: "functions.exec_command".into(),
                input: json!({"cmd":"long-running-command"}).to_string(),
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
                broker.settle_native_processes(&["owner"], &root, &[json!({"turn_id":"original-turn","item":{
                    "id":exec_id,"type":"commandExecution","status":"completed","processId":"42","exitCode":0,"aggregatedOutput":"original prefix\noriginal tail\n"
                }})]).await.unwrap();
                assert!(broker.active_claim_ids(&["owner"]).unwrap().is_empty());
                assert_eq!(
                    broker.unreturned_native_calls(&token).unwrap(),
                    expected_calls,
                    "receipt identity does not depend on an already settled process claim"
                );
            }
            let original = json!({"type":"function_call_output","call_id":poll_id,"output":"Chunk ID: original-poll\nWall time: 1.00 seconds\nProcess exited with code 0\nOutput:\noriginal tail\n"});
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
}
