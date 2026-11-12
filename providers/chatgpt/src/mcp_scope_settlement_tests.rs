//! Faults occur after the real result commit and before the exact shared release.
use super::*;
use crate::protocol::NativeScope;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
async fn fixture() -> (Root, Arc<Journal>, Arc<Broker>, String, Arguments) {
    fixture_for("view_image").await
}
async fn fixture_for(name: &str) -> (Root, Arc<Journal>, Arc<Broker>, String, Arguments) {
    let root = std::env::temp_dir().join(format!("scope-settlement-{}", random_id().unwrap()));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let root = Root(fs::canonicalize(root).unwrap());
    let journal = Arc::new(Journal::open(&root.0.join("journal")).unwrap());
    let broker = Broker::new(journal.clone());
    let definition = if name == "tool_search" {
        json!({"type":"tool_search"})
    } else {
        json!({"type":"function","name":name,"parameters":{"type":"object"}})
    };
    let request = crate::protocol::Request::parse(
        json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":"owned", "tools":[definition]}),
        &crate::config::Config::default(),
        Some("owned-thread"),
    )
    .unwrap();
    let token = broker
        .register_scoped("owned-lane", request.tools, false, Some(&root.0))
        .await
        .unwrap();
    let call = Arguments {
        call_id: "original-clock".into(),
        name: name.into(),
        input: if name == "tool_search" {
            json!({"query":"metadata","limit":1}).to_string()
        } else {
            "{}".into()
        },
        report: "owned".into(),
    };
    (root, journal, broker, token, call)
}
fn shared_path(broker: &Broker) -> PathBuf {
    broker
        .shared_guard_home()
        .unwrap()
        .join("shared-native-v1/claims.json")
}
fn claim_count(path: &Path) -> usize {
    serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap()["claims"]
        .as_object()
        .unwrap()
        .len()
}

// Recreate an already offered old-generation reservation, not a newly permitted
// unsupported call. The immutable identity comes from the original enqueue and
// the physical claim uses the existing exact Shared API in this private root.
async fn retained_legacy_opaque_offer(broker: &Broker, token: &str, call: &Arguments) -> String {
    let mut registry = broker.registry.lock().await;
    let turn = registry.turns.get_mut(token).unwrap();
    let invocation = &turn.calls[&call.call_id];
    let id = invocation.id.clone();
    let root = turn.workspace.as_deref().unwrap();
    SharedNativeGuard::offer_external_with_effect(
        root,
        broker.shared_guard_home().as_deref(),
        &turn.lane,
        &id,
        &call.name,
        &call.input,
        Effect::Opaque,
    )
    .unwrap();
    let key = digest(&"native-effect-claims-v1");
    let mut claims: ActiveEffects = broker.journal.read(&key).unwrap().unwrap_or_default();
    assert!(claims
        .claims
        .insert(
            id.clone(),
            ClaimedEffect {
                lane: turn.lane.clone(),
                effect: Effect::Opaque,
                session_id: None,
                turn_token: Some(token.into()),
                continuations: BTreeMap::new(),
                continuation_effect: None,
                process_hooks_absent: false,
            },
        )
        .is_none());
    broker.journal.write(&key, &claims).unwrap();
    let original = turn.calls.get_mut(&call.call_id).unwrap();
    original.offered = true;
    original.shared_reserved = Some(true);
    broker.persist(token, turn).unwrap();
    id
}

#[tokio::test]
async fn native_scope_committed_result_releases_original_opaque_claim_after_restart_or_retry() {
    for recovery in ["restart", "retry"] {
        for legacy_reader in [false, true] {
            let (root, journal, broker, token, call) = fixture().await;
            broker.enqueue(&token, &call).await.unwrap();
            // An older offered Read declaration was actually reserved as Opaque.
            // Restore that retained state without dispatching a new unsupported call.
            {
                let mut registry = broker.registry.lock().await;
                let turn = registry.turns.get_mut(&token).unwrap();
                turn.calls.get_mut(&call.call_id).unwrap().effect = Some(Effect::Read);
                broker.persist(&token, turn).unwrap();
            }
            let id = retained_legacy_opaque_offer(&broker, &token, &call).await;
            assert_eq!(broker.recorded_calls(&token).unwrap()[0]["offered"], true);
            let shared = shared_path(&broker);
            assert_eq!(claim_count(&shared), 1);
            let original_ledger = fs::read(&shared).unwrap();
            if legacy_reader {
                let mut registry = broker.registry.lock().await;
                let turn = registry.turns.get_mut(&token).unwrap();
                let mut old = serde_json::to_value(&*turn).unwrap();
                old["calls"][&call.call_id]
                    .as_object_mut()
                    .unwrap()
                    .remove("shared_reserved");
                *turn = serde_json::from_value(old).unwrap();
                broker.persist(&token, turn).unwrap();
            }
            let before: Turn = journal
                .read(&digest(&("mcp-turn-v1", &token)))
                .unwrap()
                .unwrap();
            let result = json!({"output":"original completed clock result"});
            let next = NativeScope {
                workspace: root.0.clone(),
                execution: Some(native_effect::hook_fixture(&root.0)),
            };
            let results = vec![(id.clone(), result.clone())];
            let failure = broker
                .settle_prepared_scoped(&token, &results, None, &[], Some(&next), || {
                    // Only this test's private shared ledger is faulted. Restore the exact
                    // original bytes below; no operation/effect is replayed by the test.
                    fs::write(&shared, b"{invalid owned fault").unwrap();
                    Ok(())
                })
                .await;
            fs::write(&shared, &original_ledger).unwrap();
            assert_eq!(failure, Err(Error("native_resource_shared_guard_failed")));
            let committed: Turn = journal
                .read(&digest(&("mcp-turn-v1", &token)))
                .unwrap()
                .unwrap();
            assert_eq!(committed.execution, next.execution);
            assert_eq!(committed.calls[&call.call_id].result, Some(result.clone()));
            assert_eq!(
                claim_count(&shared),
                1,
                "the fault really preceded shared release"
            );
            assert_eq!(
                committed.calls[&call.call_id].fingerprint,
                before.calls[&call.call_id].fingerprint
            );
            if recovery == "restart" {
                drop(broker);
                let recovered = Broker::new(journal.clone());
                recovered.reconcile_persisted_integrated_receipts().unwrap();
                assert_eq!(
                    recovered.retained_result(&token, &call).unwrap(),
                    Some(result.clone())
                );
                recovered.reconcile_persisted_integrated_receipts().unwrap();
            } else {
                broker
                    .settle_prepared_scoped(&token, &results, None, &[], Some(&next), || Ok(()))
                    .await
                    .unwrap();
                assert_eq!(
                    broker.retained_result(&token, &call).unwrap(),
                    Some(result.clone())
                );
            }
            assert_eq!(
                claim_count(&shared),
                0,
                "{recovery}, legacy reader={legacy_reader}: new Read facts cannot erase original Opaque settlement"
            );
            let state: ActiveEffects = journal
                .read(&digest(&"native-effect-claims-v1"))
                .unwrap()
                .unwrap();
            assert!(state.claims.is_empty());
        }
    }
}

#[tokio::test]
async fn native_scope_completed_known_read_does_not_consult_an_unrelated_broken_shared_ledger() {
    for name in ["view_image", "tool_search"] {
        let (root, _journal, broker, token, call) = fixture_for(name).await;
        broker
            .bind_execution(&token, Some(native_effect::hook_fixture(&root.0)))
            .await
            .unwrap();
        broker.enqueue(&token, &call).await.unwrap();
        let offered = broker.offer(&token).await.unwrap();
        assert_eq!(offered.len(), 1);
        let shared = shared_path(&broker);
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(shared.parent().unwrap())
            .unwrap();
        fs::write(&shared, b"{unrelated owned corruption").unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o600)).unwrap();
        let before = fs::read(&shared).unwrap();
        let next = NativeScope {
            workspace: root.0.clone(),
            execution: None,
        };
        let result = json!({"output":"known original read result"});
        let results = [(
            offered[0]["id"].as_str().unwrap().to_owned(),
            result.clone(),
        )];
        for _ in 0..2 {
            broker
                .settle_prepared_scoped(&token, &results, None, &[], Some(&next), || Ok(()))
                .await
                .unwrap();
            assert_eq!(
                broker.retained_result(&token, &call).unwrap(),
                Some(result.clone())
            );
            assert_eq!(fs::read(&shared).unwrap(), before);
        }
    }
}

#[tokio::test]
async fn native_scope_legacy_intrinsic_read_releases_its_actual_opaque_reservation() {
    for old_effect in [None, Some(Effect::Opaque)] {
        for old_reader in [false, true] {
            let (root, journal, broker, token, call) = fixture_for("tool_search").await;
            broker.enqueue(&token, &call).await.unwrap();
            {
                let mut registry = broker.registry.lock().await;
                let turn = registry.turns.get_mut(&token).unwrap();
                turn.calls.get_mut(&call.call_id).unwrap().effect = old_effect.clone();
                broker.persist(&token, turn).unwrap();
            }
            let offered = broker.offer(&token).await.unwrap();
            assert_eq!(offered.len(), 1);
            let id = offered[0]["id"].as_str().unwrap().to_owned();
            let shared = shared_path(&broker);
            assert_eq!(claim_count(&shared), 1);
            {
                let mut registry = broker.registry.lock().await;
                let turn = registry.turns.get_mut(&token).unwrap();
                assert_eq!(turn.calls[&call.call_id].shared_reserved, Some(true));
                if old_reader {
                    turn.calls.get_mut(&call.call_id).unwrap().shared_reserved = None;
                    broker.persist(&token, turn).unwrap();
                }
            }
            let result = json!({"output":"original tool search metadata"});
            let reserved = fs::read(&shared).unwrap();
            assert_eq!(
                SharedNativeGuard::settle_external(
                    &root.0,
                    broker.shared_guard_home().as_deref(),
                    "owned-lane",
                    &id,
                    &call.name,
                    r#"{"query":"changed-original-input"}"#,
                    &result,
                ),
                Err("native_effect_identity_changed".into())
            );
            assert_eq!(fs::read(&shared).unwrap(), reserved);
            let next = NativeScope {
                workspace: root.0.clone(),
                execution: None,
            };
            let results = [(id, result.clone())];
            broker
                .settle_prepared_scoped(&token, &results, None, &[], Some(&next), || Ok(()))
                .await
                .unwrap();
            assert_eq!(
                claim_count(&shared),
                0,
                "legacy effect {old_effect:?}, old reader={old_reader}"
            );
            assert_eq!(
                broker.retained_result(&token, &call).unwrap(),
                Some(result.clone())
            );
            drop(broker);
            let restored = Broker::new(journal);
            restored.reconcile_persisted_integrated_receipts().unwrap();
            assert_eq!(
                restored.retained_result(&token, &call).unwrap(),
                Some(result)
            );
            assert_eq!(claim_count(&shared), 0);
        }
    }
}
