//! Retained MCP invocations. This module never executes a tool or owns native
//! history. A detached HTTP peer may retry its exact logical call; it cannot
//! cause a second native action. Only the common app lane can offer/settle it.
use crate::{
    context::digest,
    journal::{random_id, Journal},
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
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify};

const LIMIT: usize = 16 * 1024 * 1024;
const TURN_CACHE_CAPACITY: usize = 256;
#[cfg(test)]
mod source_tests {
    use super::*;
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
        "native_catalog"
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
}
#[derive(Clone, Serialize, Deserialize)]
struct Turn {
    lane: String,
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
        let calls = next
            .calls
            .values_mut()
            .filter(|v| !v.offered && v.result.is_none())
            .take(32)
            .map(|v| {
                v.offered = true;
                json!({"id":v.id,"name":v.name,"input":v.input,"report":v.report})
            })
            .collect::<Vec<_>>();
        if !calls.is_empty() {
            self.persist(token, &next)?;
            *turn = next;
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
