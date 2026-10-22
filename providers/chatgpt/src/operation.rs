//! Logical mutation receipts and concurrent-observer admission. This is transport
//! recovery metadata, not an executor, scheduler or canonical conversation store.
use crate::{journal::Journal, protocol::Error};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{watch, Mutex};

type Outcome = Option<Result<Value, Error>>;
struct Observer {
    fingerprint: String,
    result: watch::Receiver<Outcome>,
}

#[derive(Default)]
pub(crate) struct Operations {
    active: Mutex<BTreeMap<String, Observer>>,
}
impl Operations {
    pub async fn run(
        self: &Arc<Self>,
        key: String,
        fingerprint: String,
        action: BoxFuture<'static, Result<Value, Error>>,
    ) -> Result<Value, Error> {
        let mut result = {
            let mut active = self.active.lock().await;
            if let Some(observer) = active.get(&key) {
                if observer.fingerprint != fingerprint {
                    return Err(Error("operator_call_changed"));
                }
                observer.result.clone()
            } else {
                if active.len() >= 4096 {
                    return Err(Error("operator_operation_limit"));
                }
                let (sender, receiver) = watch::channel(None);
                active.insert(
                    key.clone(),
                    Observer {
                        fingerprint,
                        result: receiver.clone(),
                    },
                );
                let operations = self.clone();
                // Concurrent observers share this result, including a timeout.
                // A later retry consults the durable operation receipt afresh.
                // Dropping an observer cannot cancel an admitted mutation.
                tokio::spawn(async move {
                    let result = action.await;
                    let mut active = operations.active.lock().await;
                    active.remove(&key);
                    sender.send_replace(Some(result));
                });
                receiver
            }
        };
        loop {
            if let Some(result) = result.borrow().clone() {
                return result;
            }
            result
                .changed()
                .await
                .map_err(|_| Error("operator_operation_reconciliation_required"))?;
        }
    }
}

/// Absence permits a new dispatch. An intent without a result never does.
pub(crate) fn recover(
    journal: &Journal,
    key: &str,
    fingerprint: &str,
    unknown: &'static str,
) -> Result<Option<Value>, Error> {
    let Some(previous) = journal.read::<Value>(key)? else {
        return Ok(None);
    };
    if previous["fingerprint"] != fingerprint {
        return Err(Error("operator_call_changed"));
    }
    previous
        .get("result")
        .cloned()
        .map(Some)
        .ok_or(Error(unknown))
}

pub(crate) fn intent(journal: &Journal, key: &str, fingerprint: &str) -> Result<(), Error> {
    intent_with_context(journal, key, fingerprint, Value::Null)
}
pub(crate) fn intent_with_context(
    journal: &Journal,
    key: &str,
    fingerprint: &str,
    context: Value,
) -> Result<(), Error> {
    if let Some(previous) = journal.read::<Value>(key)? {
        if previous["fingerprint"] != fingerprint {
            return Err(Error("operator_call_changed"));
        }
        return Err(Error("operator_operation_reconciliation_required"));
    }
    journal.write(key, &json!({"fingerprint":fingerprint,"context":context}))
}

pub(crate) fn settle(
    journal: &Journal,
    key: &str,
    fingerprint: &str,
    result: &Value,
) -> Result<(), Error> {
    let mut previous = journal
        .read::<Value>(key)?
        .ok_or(Error("operator_operation_intent_missing"))?;
    if previous["fingerprint"] != fingerprint {
        return Err(Error("operator_call_changed"));
    }
    if previous.get("result").is_some_and(|old| old != result) {
        return Err(Error("operator_operation_result_changed"));
    }
    previous["result"] = result.clone();
    journal.write(key, &previous)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::digest, journal::random_id};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Root(std::path::PathBuf);
    impl Root {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("fo-{}", random_id().unwrap())))
        }
    }
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn observers_share_one_effect_and_cancellation_does_not_cancel_it() {
        let operations = Arc::new(Operations::default());
        let effects = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let o = operations.clone();
        let e = effects.clone();
        let ready = entered.clone();
        let done = release.clone();
        let first = tokio::spawn(async move {
            o.run(
                "key".into(),
                "input".into(),
                Box::pin(async move {
                    e.fetch_add(1, Ordering::SeqCst);
                    ready.notify_one();
                    done.notified().await;
                    Ok(json!({"native_result":"original"}))
                }),
            )
            .await
        });
        entered.notified().await;
        assert_eq!(
            operations
                .run(
                    "key".into(),
                    "changed".into(),
                    Box::pin(async { panic!("changed input dispatched") })
                )
                .await,
            Err(Error("operator_call_changed"))
        );
        let mut observers = Vec::new();
        for _ in 0..8 {
            let o = operations.clone();
            observers.push(async move {
                o.run(
                    "key".into(),
                    "input".into(),
                    Box::pin(async { panic!("duplicate dispatched") }),
                )
                .await
            });
        }
        // Admit all observers before publishing the original result.
        let (outcomes, ()) = tokio::join!(
            biased;
            futures_util::future::join_all(observers),
            async { first.abort(); release.notify_one(); }
        );
        for outcome in outcomes {
            assert_eq!(outcome.unwrap(), json!({"native_result":"original"}));
        }
        assert_eq!(effects.load(Ordering::SeqCst), 1);
        assert!(operations.active.lock().await.is_empty());
    }

    #[test]
    fn restart_preserves_unknown_intent_and_exact_results_with_context() {
        let root = Root::new();
        let key = digest(&"logical");
        {
            let journal = Journal::open(&root.0).unwrap();
            assert_eq!(recover(&journal, &key, "input", "unknown").unwrap(), None);
            intent_with_context(&journal, &key, "input", json!({"round":"original"})).unwrap();
        }
        let journal = Journal::open(&root.0).unwrap();
        assert_eq!(
            recover(&journal, &key, "input", "unknown"),
            Err(Error("unknown"))
        );
        assert_eq!(
            recover(&journal, &key, "changed", "unknown"),
            Err(Error("operator_call_changed"))
        );
        assert_eq!(
            intent(&journal, &key, "input"),
            Err(Error("operator_operation_reconciliation_required"))
        );
        let result = json!({"receipt":"original"});
        settle(&journal, &key, "input", &result).unwrap();
        settle(&journal, &key, "input", &result).unwrap();
        assert_eq!(
            recover(&journal, &key, "input", "unknown").unwrap(),
            Some(result)
        );
        assert_eq!(
            journal.read::<Value>(&key).unwrap().unwrap()["context"],
            json!({"round":"original"})
        );
        assert_eq!(
            settle(&journal, &key, "input", &json!({"changed":true})),
            Err(Error("operator_operation_result_changed"))
        );
        assert_eq!(
            recover(&journal, &digest(&"new logical id"), "input", "unknown").unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn completed_failure_is_not_a_stale_cached_wait() {
        let operations = Arc::new(Operations::default());
        assert_eq!(
            operations
                .run(
                    "key".into(),
                    "input".into(),
                    Box::pin(async { Err(Error("confirmation_timeout")) })
                )
                .await,
            Err(Error("confirmation_timeout"))
        );
        assert_eq!(
            operations
                .run(
                    "key".into(),
                    "input".into(),
                    Box::pin(async { Ok(json!({"settled":true})) })
                )
                .await
                .unwrap(),
            json!({"settled":true})
        );
    }
}
