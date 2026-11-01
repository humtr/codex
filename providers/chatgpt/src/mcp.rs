//! Retained MCP invocations. This module never executes a tool or owns native
//! history. A detached HTTP peer may retry its exact logical call; it cannot
//! cause a second native action. Only the common app lane can offer/settle it.
use crate::{
    context::digest,
    journal::{random_id, Journal},
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
        "local_context"
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
    #[serde(default)]
    effect: Option<Effect>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Turn {
    lane: String,
    #[serde(default)]
    workspace: Option<PathBuf>,
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
}
#[derive(Clone, Serialize, Deserialize)]
struct ExternalEffectReceipt {
    fingerprint: String,
    name: String,
    continuation: Option<u64>,
    #[serde(default)]
    settled: bool,
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
fn settle_effect(
    claims: &mut ActiveEffects,
    id: &str,
    lane: &str,
    name: &str,
    continuation: Option<u64>,
    result: &Value,
) {
    let (running, exited) = native_process_result(result);
    if name == "functions.exec_command" && running.is_some() && !exited {
        if let Some(claim) = claims.claims.get_mut(id) {
            claim.session_id = running;
        }
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
impl Broker {
    /// Bounded, account-private status data. Never export paths, file contents,
    /// command text or authentication material from another work.
    pub(crate) fn effect_status(&self, lane: &str) -> Result<Value, Error> {
        let state: ActiveEffects = self
            .journal
            .read(&digest(&"native-effect-claims-v1"))?
            .unwrap_or_default();
        let entries = state
            .claims
            .iter()
            .map(|(id, held)| {
                json!({"call":id,"owner_work":held.lane,
                "resource_kind":match held.effect {
                    Effect::Read => "read",
                    Effect::Paths(_) => "bounded",
                    Effect::Opaque => "opaque",
                },
                "process_session":held.session_id,
                "owned":held.lane==lane,
                "pending_result":true})
            })
            .collect::<Vec<_>>();
        Ok(
            json!({"active":entries,"unconfirmed_effects":state.claims.len(),
            "external_call_receipts":state.external.len()}),
        )
    }
    /// Provider-owned AppHost tools also execute in upstream Native, but do
    /// not use MCP tool turns. Reserve before forwarding their actual Tool
    /// event; release only on the *next* verified native result. No re-exec on
    /// repeated/uncertain delivery IDs.
    pub(crate) async fn admit_external_tool(
        &self,
        lane: &str,
        id: &str,
        name: &str,
        input: &str,
        workspace: &Path,
    ) -> Result<(), Error> {
        let _registry = self.registry.lock().await;
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
        let effect = native_effect::classify(name, input, Some(&root));
        let continuation = continued_process(name, input);
        if state.claims.values().any(|held| {
            native_effect::conflicts(&effect, &held.effect)
                && !(continuation.is_some() && held.lane == lane && held.session_id == continuation)
        }) {
            return Err(Error("native_resource_busy"));
        }
        native_effect::can_offer(&effect, &root).map_err(Error)?;
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
                },
            );
        }
        state.external.insert(
            resource_id,
            ExternalEffectReceipt {
                fingerprint,
                name: name.into(),
                continuation,
                settled: false,
            },
        );
        self.journal.write(&key, &state)?;
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
        for (id, result) in results {
            let effect_id = external_effect_id(lane, id);
            if let Some(receipt) = state
                .external
                .get(&effect_id)
                .filter(|v| !v.settled)
                .cloned()
            {
                settle_effect(
                    &mut state,
                    &effect_id,
                    lane,
                    &receipt.name,
                    receipt.continuation,
                    result,
                );
                state.external.get_mut(&effect_id).unwrap().settled = true;
                changed = true;
            }
        }
        if changed {
            self.journal.write(&key, &state)?;
            self.changed.notify_waiters();
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
    /// Register before waiting. Cancelling this future only drops the observer.
    pub async fn enqueue(&self, token: &str, args: &Arguments) -> Result<(), Error> {
        if args.call_id.is_empty()
            || args.call_id.len() > 160
            || !args
                .call_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
            || args.input.len() > LIMIT
            || args.report.len() > 64 * 1024
        {
            return Err(Error("mcp_call_invalid"));
        }
        let fingerprint = digest(&(&args.name, &args.input, &args.report));
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
        let effect = native_effect::classify(&args.name, &args.input, turn.workspace.as_deref());
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
                if let Some(result) = &call.result {
                    return Ok(result.clone());
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
            if claims.claims.values().any(|held| {
                native_effect::conflicts(&effect, &held.effect)
                    && !(continuation.is_some()
                        && held.lane == next.lane
                        && held.session_id == continuation)
            }) {
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
            let continuing_owned = continuation.is_some_and(|session| {
                claims
                    .claims
                    .values()
                    .any(|held| held.lane == next.lane && held.session_id == Some(session))
            });
            if effect != Effect::Read && !continuing_owned {
                claims.claims.insert(
                    call.id.clone(),
                    ClaimedEffect {
                        lane: next.lane.clone(),
                        effect,
                        session_id: None,
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
        self.settle_prepared(token, results, catalog, || Ok(()))
            .await
    }
    /// Keep validation, preparation and commit in one admission section. A
    /// known validation rejection must not leave a newly prepared lane intent.
    pub(crate) async fn settle_prepared(
        &self,
        token: &str,
        results: &[(String, Value)],
        catalog: Option<BTreeMap<String, Tool>>,
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
        for (id, result) in results {
            if !seen.insert(id) {
                return Err(Error("mcp_result_invalid"));
            }
            let call = next
                .calls
                .values_mut()
                .find(|v| v.id == *id && v.offered)
                .ok_or(Error("mcp_result_invalid"))?;
            if let Some(old) = &call.result {
                if old != result {
                    return Err(Error("mcp_result_changed"));
                }
            } else {
                call.result = Some(result.clone());
            }
        }
        if let Some(catalog) = catalog {
            if next.tools != catalog && next.calls.values().any(|v| v.result.is_none()) {
                return Err(Error("mcp_pending_catalog_change"));
            }
            next.tools = catalog;
        }
        // All-or-nothing catalog/result validation and durable commit precede
        // waking the model, which may immediately discover/call the new tools.
        prepare()?;
        self.persist(token, &next)?;
        *turn = next;
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
                {"name":"native_tool","description":"Run one advertised native Codex tool and return its execution result.","inputSchema":{"type":"object","properties":{"turn_token":{"type":"string"},"call_id":{"type":"string"},"name":{"type":"string"},"input":{"type":"string"},"report":{"type":"string"}},"required":["turn_token","call_id","name","input","report"],"additionalProperties":false},"annotations":tool_annotations("native_tool")}
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
        broker.enqueue(token, call).await.unwrap();
        let offered = broker.offer(token).await.unwrap();
        assert_eq!(offered.len(), 1);
        offered[0]["id"].as_str().unwrap().to_owned()
    }

    #[tokio::test]
    async fn nc_distinct_writers_and_reads_are_parallel_nonoverlap_rebases() {
        let (root, journal, tools) = fixture();
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
        let first = patch("file", "alpha", "ALPHA");
        let second = patch("file", "beta", "BETA");
        let unrelated = patch("other", "separate", "SEPARATE");
        let id_first = accepted(&broker, &one, &first).await;
        broker.enqueue(&two, &second).await.unwrap();
        assert!(
            broker.offer(&two).await.unwrap().is_empty(),
            "same-file publication serializes, not creation"
        );
        let id_unrelated = accepted(&broker, &three, &unrelated).await;
        let read = Arguments {
            call_id: "read".into(),
            name: "functions.exec_command".into(),
            input: json!({"cmd":"pwd"}).to_string(),
            report: String::new(),
        };
        let read_id = accepted(&broker, &two, &read).await;
        broker
            .settle(&two, &[(read_id, json!("read"))])
            .await
            .unwrap();
        fs::write(root.join("file"), "ALPHA\nbeta\ngamma\n").unwrap();
        broker
            .settle(&one, &[(id_first, json!("first"))])
            .await
            .unwrap();
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
            broker.invoke(&two, second.clone()).await.unwrap(),
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
            .settle(&one, &[(opaque, json!("done"))])
            .await
            .unwrap();
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
        assert_eq!(broker.offer(&other).await.unwrap().len(), 1);
        assert!(broker
            .journal
            .read::<ActiveEffects>(&digest(&"native-effect-claims-v1"))
            .unwrap()
            .unwrap()
            .claims
            .get(&command_id)
            .is_none());
        fs::remove_dir_all(root).unwrap();
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
}
