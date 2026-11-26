//! Single multiplexed Chrome DevTools Protocol (CDP) client over Chromium's
//! private `--remote-debugging-pipe` (NUL-delimited JSON-RPC on FD 3/4).
//!
//! Zero TCP debug listener, zero relay daemon.

use crate::error::{ProviderError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, Notify};

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const MAX_PENDING_REQUESTS: usize = 128;
const MAX_LANE_COMMITTED_EVENTS: usize = 4096;
const MAX_LANE_COMMITTED_BYTES: usize = 8 * 1024 * 1024;
const MAX_LANE_MUTABLE_IDS: usize = 256;
const MAX_LANE_MUTABLE_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(30);

pub const STREAM_BINDING: &str = "__termuxChatGptStream";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamItemKind {
    ReasoningSummary,
    Commentary,
    Answer,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEvent {
    Activity,
    Overflow,
    ItemUpsert {
        item_id: String,
        item_kind: StreamItemKind,
        text: String,
        #[serde(default)]
        committed: bool,
    },
    ItemRemove {
        item_id: String,
    },
    ToolCall {
        #[serde(rename = "call_id")]
        id: String,
        name: String,
        #[serde(default)]
        namespace: Option<String>,
        arguments: String,
        #[serde(default)]
        native_message_id: Option<String>,
        #[serde(default)]
        native_recipient: Option<String>,
    },
    TurnCompleted {
        #[serde(default)]
        chat_id: Option<String>,
        #[serde(default)]
        message_id: Option<String>,
    },
}

#[derive(Deserialize)]
struct BindingUpdate {
    lane_id: String,
    #[serde(flatten)]
    event: StreamEvent,
}

#[derive(Default)]
struct LaneQueue {
    committed: VecDeque<StreamEvent>,
    mutable_order: VecDeque<String>,
    mutable: HashMap<String, StreamEvent>,
    committed_bytes: usize,
    mutable_bytes: usize,
    overflowed: bool,
    activity: bool,
    closed: bool,
}

struct LaneShared {
    queue: Mutex<LaneQueue>,
    notify: Notify,
}

#[derive(Clone)]
pub struct LaneSender {
    shared: Arc<LaneShared>,
}

pub struct LaneReceiver {
    shared: Arc<LaneShared>,
}

pub fn lane_channel() -> (LaneSender, LaneReceiver) {
    let shared = Arc::new(LaneShared {
        queue: Mutex::new(LaneQueue::default()),
        notify: Notify::new(),
    });
    (
        LaneSender {
            shared: Arc::clone(&shared),
        },
        LaneReceiver { shared },
    )
}

impl LaneSender {
    /// Non-blocking enqueue so one stalled lane can never backpressure the shared CDP pipe.
    /// Mutable updates overwrite in-place by `item_id`; committed/terminal events queue losslessly.
    pub fn send_nonblocking(&self, event: StreamEvent) -> bool {
        let Ok(mut q) = self.shared.queue.lock() else {
            return false;
        };
        if q.closed {
            return false;
        }
        if q.overflowed {
            return false;
        }
        match event {
            StreamEvent::Activity => {
                q.activity = true;
            }
            StreamEvent::Overflow => {
                q.overflowed = true;
                q.closed = true;
            }
            StreamEvent::ItemUpsert {
                ref item_id,
                committed: false,
                ..
            } => {
                let event_bytes =
                    serde_json::to_vec(&event).map_or(usize::MAX, |bytes| bytes.len());
                let old_bytes = q
                    .mutable
                    .get(item_id)
                    .and_then(|old| serde_json::to_vec(old).ok())
                    .map_or(0, |bytes| bytes.len());
                let next_bytes = q
                    .mutable_bytes
                    .saturating_sub(old_bytes)
                    .saturating_add(event_bytes);
                if (!q.mutable.contains_key(item_id) && q.mutable.len() >= MAX_LANE_MUTABLE_IDS)
                    || next_bytes > MAX_LANE_MUTABLE_BYTES
                {
                    q.overflowed = true;
                    q.closed = true;
                    q.committed.push_back(StreamEvent::Overflow);
                    q.mutable.clear();
                    q.mutable_order.clear();
                    q.mutable_bytes = 0;
                    drop(q);
                    self.shared.notify.notify_one();
                    return false;
                }
                if !q.mutable.contains_key(item_id) {
                    q.mutable_order.push_back(item_id.clone());
                }
                q.mutable_bytes = next_bytes;
                q.mutable.insert(item_id.clone(), event);
            }
            StreamEvent::ItemUpsert {
                ref item_id,
                committed: true,
                ..
            }
            | StreamEvent::ItemRemove { ref item_id } => {
                if let Some(old) = q.mutable.remove(item_id) {
                    q.mutable_bytes = q
                        .mutable_bytes
                        .saturating_sub(serde_json::to_vec(&old).map_or(0, |bytes| bytes.len()));
                }
                q.mutable_order.retain(|id| id != item_id);
                let event_bytes =
                    serde_json::to_vec(&event).map_or(usize::MAX, |bytes| bytes.len());
                if q.committed.len() >= MAX_LANE_COMMITTED_EVENTS
                    || q.committed_bytes.saturating_add(event_bytes) > MAX_LANE_COMMITTED_BYTES
                {
                    q.overflowed = true;
                    q.closed = true;
                    q.committed.push_back(StreamEvent::Overflow);
                    drop(q);
                    self.shared.notify.notify_one();
                    return false;
                }
                q.committed_bytes += event_bytes;
                q.committed.push_back(event);
            }
            StreamEvent::ToolCall { .. } | StreamEvent::TurnCompleted { .. } => {
                let event_bytes =
                    serde_json::to_vec(&event).map_or(usize::MAX, |bytes| bytes.len());
                if q.committed.len() >= MAX_LANE_COMMITTED_EVENTS
                    || q.committed_bytes.saturating_add(event_bytes) > MAX_LANE_COMMITTED_BYTES
                {
                    q.overflowed = true;
                    q.closed = true;
                    q.committed.push_back(StreamEvent::Overflow);
                    drop(q);
                    self.shared.notify.notify_one();
                    return false;
                }
                q.committed_bytes += event_bytes;
                q.committed.push_back(event);
            }
        }
        drop(q);
        self.shared.notify.notify_one();
        true
    }

    pub fn close(&self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.closed = true;
        }
        self.shared.notify.notify_one();
    }
}

impl LaneReceiver {
    fn pop_locked(q: &mut LaneQueue) -> Option<StreamEvent> {
        if let Some(ev) = q.committed.pop_front() {
            q.committed_bytes = q
                .committed_bytes
                .saturating_sub(serde_json::to_vec(&ev).map_or(0, |bytes| bytes.len()));
            return Some(ev);
        }
        while let Some(id) = q.mutable_order.pop_front() {
            if let Some(ev) = q.mutable.remove(&id) {
                q.mutable_bytes = q
                    .mutable_bytes
                    .saturating_sub(serde_json::to_vec(&ev).map_or(0, |bytes| bytes.len()));
                return Some(ev);
            }
        }
        if q.activity {
            q.activity = false;
            return Some(StreamEvent::Activity);
        }
        None
    }

    pub fn try_recv(&mut self) -> std::result::Result<StreamEvent, mpsc::error::TryRecvError> {
        let Ok(mut q) = self.shared.queue.lock() else {
            return Err(mpsc::error::TryRecvError::Disconnected);
        };
        if let Some(ev) = Self::pop_locked(&mut q) {
            return Ok(ev);
        }
        if q.closed {
            Err(mpsc::error::TryRecvError::Disconnected)
        } else {
            Err(mpsc::error::TryRecvError::Empty)
        }
    }

    pub async fn recv(&mut self) -> Option<StreamEvent> {
        loop {
            let notified = self.shared.notify.notified();
            {
                let Ok(mut q) = self.shared.queue.lock() else {
                    return None;
                };
                if let Some(ev) = Self::pop_locked(&mut q) {
                    return Some(ev);
                }
                if q.closed {
                    return None;
                }
            }
            notified.await;
        }
    }
}

impl Drop for LaneReceiver {
    fn drop(&mut self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.closed = true;
        }
    }
}

type StreamRoutes = Arc<Mutex<BTreeMap<(String, String), LaneSender>>>;

pub struct StreamRoute {
    routes: StreamRoutes,
    key: (String, String),
}

impl Drop for StreamRoute {
    fn drop(&mut self) {
        if let Ok(mut routes) = self.routes.lock() {
            if let Some(sender) = routes.remove(&self.key) {
                sender.close();
            }
        }
    }
}

struct CdpCommand {
    method: String,
    params: Value,
    session_id: Option<String>,
    reply: oneshot::Sender<Result<Value>>,
}

#[derive(Clone)]
pub struct CdpClient {
    tx: mpsc::Sender<CdpCommand>,
    pub generation: Arc<str>,
    streams: StreamRoutes,
}

impl CdpClient {
    pub fn from_pipe<R, W>(reader: R, mut writer: W, generation: String) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<CdpCommand>(MAX_PENDING_REQUESTS);
        let (in_tx, mut in_rx) = mpsc::channel::<Value>(MAX_PENDING_REQUESTS);

        let streams = StreamRoutes::default();
        let routes = Arc::clone(&streams);
        let reader_task = tokio::spawn(async move {
            let mut br = BufReader::new(reader);
            loop {
                let mut frame = Vec::new();
                let read_res = (&mut br)
                    .take((MAX_FRAME_BYTES + 1) as u64)
                    .read_until(0, &mut frame)
                    .await;
                match read_res {
                    Ok(0) => break,
                    Ok(_) if frame.len() > MAX_FRAME_BYTES => break,
                    Ok(_) => {
                        if frame.last() == Some(&0) {
                            frame.pop();
                        }
                        if frame.is_empty() {
                            continue;
                        }
                        let Ok(val) = serde_json::from_slice::<Value>(&frame) else {
                            break;
                        };
                        if in_tx.send(val).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        tokio::spawn(async move {
            let mut seq = 0u64;
            let mut pending: BTreeMap<u64, oneshot::Sender<Result<Value>>> = BTreeMap::new();
            loop {
                tokio::select! {
                    cmd = rx.recv() => {
                        let Some(cmd) = cmd else { break };
                        pending.retain(|_, s| !s.is_closed());
                        if pending.len() >= MAX_PENDING_REQUESTS {
                            let _ = cmd.reply.send(Err(ProviderError::unavailable("cdp_queue_full", "too many in-flight CDP requests")));
                            continue;
                        }
                        seq += 1;
                        let mut body = json!({
                            "id": seq,
                            "method": cmd.method,
                            "params": cmd.params
                        });
                        if let Some(sid) = cmd.session_id {
                            body["sessionId"] = json!(sid);
                        }
                        let mut raw = body.to_string().into_bytes();
                        if raw.len() > MAX_FRAME_BYTES {
                            let _ = cmd.reply.send(Err(ProviderError::bad_request("cdp_request_too_large", "CDP frame exceeds 16 MiB")));
                            continue;
                        }
                        raw.push(0);
                        pending.insert(seq, cmd.reply);
                        if writer.write_all(&raw).await.is_err() || writer.flush().await.is_err() {
                            break;
                        }
                    }
                    msg = in_rx.recv() => {
                        let Some(msg) = msg else { break };
                        if let Some(id) = msg.get("id").and_then(Value::as_u64) {
                            if let Some(reply) = pending.remove(&id) {
                                if let Some(err) = msg.get("error") {
                                    let detail = err.get("message").and_then(Value::as_str).unwrap_or("CDP error").to_owned();
                                    let _ = reply.send(Err(ProviderError::unavailable("cdp_operation_rejected", detail)));
                                } else {
                                    let _ = reply.send(Ok(msg.get("result").cloned().unwrap_or(Value::Null)));
                                }
                            }
                        } else if msg["method"] == "Runtime.bindingCalled"
                            && msg["params"]["name"] == STREAM_BINDING
                        {
                            let sid = msg["sessionId"].as_str();
                            let payload = msg["params"]["payload"].as_str()
                                .filter(|data| data.len() <= 512 * 1024);
                            if let (Some(sid), Some(payload)) = (sid, payload) {
                                if let Ok(update) = serde_json::from_str::<BindingUpdate>(payload) {
                                    if let Ok(routes) = routes.lock() {
                                        if let Some(sender) = routes.get(&(sid.to_owned(), update.lane_id)) {
                                            sender.send_nonblocking(update.event);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            reader_task.abort();
            if let Ok(mut active_routes) = routes.lock() {
                for sender in active_routes.values() {
                    sender.close();
                }
                active_routes.clear();
            }
            for (_, reply) in pending {
                let _ = reply.send(Err(ProviderError::unavailable(
                    "cdp_disconnected",
                    "CDP pipe closed before response arrived",
                )));
            }
        });

        Self {
            tx,
            generation: generation.into(),
            streams,
        }
    }

    pub fn route_stream(
        &self,
        session: &str,
        lane: &str,
        sender: LaneSender,
    ) -> Result<StreamRoute> {
        let key = (session.to_owned(), lane.to_owned());
        let mut routes = self.streams.lock().map_err(|_| {
            ProviderError::internal("cdp_stream_routes", "CDP stream route lock poisoned")
        })?;
        if routes.contains_key(&key) {
            return Err(ProviderError::internal(
                "cdp_stream_duplicate",
                "duplicate CDP stream route",
            ));
        }
        routes.insert(key.clone(), sender);
        Ok(StreamRoute {
            routes: Arc::clone(&self.streams),
            key,
        })
    }

    pub async fn request(
        &self,
        method: &str,
        params: Value,
        session_id: Option<String>,
    ) -> Result<Value> {
        self.request_with_timeout(method, params, session_id, DEFAULT_RPC_TIMEOUT)
            .await
    }

    pub async fn request_with_timeout(
        &self,
        method: &str,
        params: Value,
        session_id: Option<String>,
        timeout: Duration,
    ) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(CdpCommand {
                method: method.to_owned(),
                params,
                session_id,
                reply: tx,
            })
            .await
            .map_err(|_| ProviderError::unavailable("cdp_disconnected", "CDP channel closed"))?;
        tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| ProviderError::unavailable("cdp_timeout", format!("{method} timed out")))?
            .map_err(|_| {
                ProviderError::unavailable("cdp_disconnected", "CDP reply channel dropped")
            })?
    }

    /// Find the primary ChatGPT page target and attach a flattened CDP session.
    pub async fn attach_chatgpt_page(&self) -> Result<String> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        let target_id = loop {
            let res = self.request("Target.getTargets", json!({}), None).await?;
            let infos = res
                .get("targetInfos")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    ProviderError::unavailable(
                        "cdp_targets_invalid",
                        "Target.getTargets missing array",
                    )
                })?;
            let target = infos.iter().find(|t| {
                t.get("type").and_then(Value::as_str) == Some("page")
                    && t.get("url").and_then(Value::as_str).is_some_and(|u| {
                        u.starts_with("app://")
                            || u.contains("chatgpt.com")
                            || u.starts_with("https://")
                    })
            });
            if let Some(tid) = target
                .and_then(|t| t.get("targetId"))
                .and_then(Value::as_str)
            {
                break tid.to_owned();
            }
            if tokio::time::Instant::now() >= deadline {
                if let Some(fallback_tid) = infos
                    .iter()
                    .find(|t| t.get("type").and_then(Value::as_str) == Some("page"))
                    .and_then(|t| t.get("targetId"))
                    .and_then(Value::as_str)
                {
                    break fallback_tid.to_owned();
                }
                return Err(ProviderError::unavailable(
                    "cdp_page_not_found",
                    "no ChatGPT renderer page target",
                ));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        };
        let attached = self
            .request(
                "Target.attachToTarget",
                json!({ "targetId": target_id, "flatten": true }),
                None,
            )
            .await?;
        attached
            .get("sessionId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                ProviderError::unavailable(
                    "cdp_session_missing",
                    "attachToTarget missing sessionId",
                )
            })
    }

    /// Evaluate a JS expression (supporting Promises) inside a page session and return JSON.
    pub async fn evaluate_json(
        &self,
        session_id: &str,
        expression: &str,
        timeout: Duration,
    ) -> Result<Value> {
        let res = self
            .request_with_timeout(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "awaitPromise": true,
                    "returnByValue": true
                }),
                Some(session_id.to_owned()),
                timeout,
            )
            .await?;
        if let Some(exc) = res.get("exceptionDetails") {
            let desc = exc
                .get("exception")
                .and_then(|e| e.get("description"))
                .and_then(Value::as_str)
                .or_else(|| exc.get("text").and_then(Value::as_str))
                .unwrap_or("JS evaluation exception");
            return Err(ProviderError::unavailable("page_driver_exception", desc));
        }
        Ok(res
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[test]
    fn lane_mailbox_bounds_mutable_ids_and_signals_overflow() {
        let (tx, mut rx) = lane_channel();
        for index in 0..MAX_LANE_MUTABLE_IDS {
            assert!(tx.send_nonblocking(StreamEvent::ItemUpsert {
                item_id: format!("mutable-{index}"),
                item_kind: StreamItemKind::Commentary,
                text: "x".to_owned(),
                committed: false,
            }));
        }
        assert!(!tx.send_nonblocking(StreamEvent::ItemUpsert {
            item_id: "overflow".to_owned(),
            item_kind: StreamItemKind::Commentary,
            text: "x".to_owned(),
            committed: false,
        }));
        assert!(matches!(rx.try_recv(), Ok(StreamEvent::Overflow)));
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Disconnected)
        ));
    }

    #[tokio::test]
    async fn cdp_binding_routes_only_matching_session_and_lane() {
        let (pipe, mut peer) = tokio::io::duplex(8192);
        let (reader, writer) = tokio::io::split(pipe);
        let client = CdpClient::from_pipe(reader, writer, "test".to_owned());
        let (tx, mut rx) = lane_channel();
        let _route = client.route_stream("session", "lane", tx).unwrap();
        for (session, lane, text) in [
            ("wrong", "lane", "wrong session"),
            ("session", "wrong", "wrong lane"),
            ("session", "lane", "visible"),
        ] {
            let msg = json!({
                "method":"Runtime.bindingCalled", "sessionId":session, "params":{
                    "name":STREAM_BINDING, "payload":json!({
                        "kind":"item_upsert",
                        "lane_id":lane,
                        "item_id":"thought:1",
                        "item_kind":"reasoning_summary",
                        "text":text,
                        "committed":true
                    }).to_string()
                }
            });
            peer.write_all(format!("{msg}\0").as_bytes()).await.unwrap();
        }
        let update = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap();
        assert!(matches!(
            update,
            Some(StreamEvent::ItemUpsert {
                item_kind: StreamItemKind::ReasoningSummary,
                committed: true,
                ref text,
                ..
            }) if text == "visible"
        ));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn lane_mailbox_coalesces_mutable_preserves_committed_and_isolates_lanes() {
        let (tx_a, mut rx_a) = lane_channel();
        let (tx_b, mut rx_b) = lane_channel();

        for i in 0..1000 {
            assert!(tx_a.send_nonblocking(StreamEvent::ItemUpsert {
                item_id: "answer:1".to_owned(),
                item_kind: StreamItemKind::Answer,
                text: format!("draft-{i}"),
                committed: false,
            }));
        }
        assert!(tx_a.send_nonblocking(StreamEvent::ItemUpsert {
            item_id: "thought:1".to_owned(),
            item_kind: StreamItemKind::ReasoningSummary,
            text: "committed thought".to_owned(),
            committed: true,
        }));
        assert!(tx_a.send_nonblocking(StreamEvent::ItemUpsert {
            item_id: "answer:1".to_owned(),
            item_kind: StreamItemKind::Answer,
            text: "final".to_owned(),
            committed: true,
        }));
        assert!(tx_a.send_nonblocking(StreamEvent::TurnCompleted {
            chat_id: Some("c1".to_owned()),
            message_id: Some("m1".to_owned()),
        }));
        assert!(tx_b.send_nonblocking(StreamEvent::ItemUpsert {
            item_id: "answer:b".to_owned(),
            item_kind: StreamItemKind::Answer,
            text: "sibling lane ok".to_owned(),
            committed: true,
        }));
        tx_a.close();
        tx_b.close();

        // Lane B receives immediately without waiting on Lane A.
        assert_eq!(
            rx_b.recv().await,
            Some(StreamEvent::ItemUpsert {
                item_id: "answer:b".to_owned(),
                item_kind: StreamItemKind::Answer,
                text: "sibling lane ok".to_owned(),
                committed: true,
            })
        );
        assert_eq!(rx_b.recv().await, None);

        // Lane A delivers committed events losslessly and drops stale mutable drafts.
        let mut drained = Vec::new();
        while let Some(ev) = rx_a.recv().await {
            drained.push(ev);
        }
        assert_eq!(
            drained,
            vec![
                StreamEvent::ItemUpsert {
                    item_id: "thought:1".to_owned(),
                    item_kind: StreamItemKind::ReasoningSummary,
                    text: "committed thought".to_owned(),
                    committed: true,
                },
                StreamEvent::ItemUpsert {
                    item_id: "answer:1".to_owned(),
                    item_kind: StreamItemKind::Answer,
                    text: "final".to_owned(),
                    committed: true,
                },
                StreamEvent::TurnCompleted {
                    chat_id: Some("c1".to_owned()),
                    message_id: Some("m1".to_owned()),
                },
            ]
        );
    }

    #[tokio::test]
    async fn cdp_pipe_roundtrip_and_page_evaluate() {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let (cr, cw) = tokio::io::split(client_io);
        let (sr, mut sw) = tokio::io::split(server_io);

        let srv = tokio::spawn(async move {
            let mut br = BufReader::new(sr);
            for _ in 0..3 {
                let mut frame = Vec::new();
                if br.read_until(0, &mut frame).await.unwrap() == 0 {
                    break;
                }
                frame.pop();
                let req: Value = serde_json::from_slice(&frame).unwrap();
                let id = req["id"].as_u64().unwrap();
                let method = req["method"].as_str().unwrap();
                let resp = match method {
                    "Target.getTargets" => json!({
                        "id": id,
                        "result": {
                            "targetInfos": [{"targetId": "T1", "type": "page", "url": "https://chatgpt.com/"}]
                        }
                    }),
                    "Target.attachToTarget" => json!({"id": id, "result": {"sessionId": "S1"}}),
                    "Runtime.evaluate" => {
                        json!({"id": id, "result": {"result": {"value": {"state": "ok", "n": 42}}}})
                    }
                    _ => json!({"id": id, "result": {}}),
                };
                let mut out = resp.to_string().into_bytes();
                out.push(0);
                sw.write_all(&out).await.unwrap();
                sw.flush().await.unwrap();
            }
        });

        let client = CdpClient::from_pipe(cr, cw, "gen-test".to_owned());
        let sid = client.attach_chatgpt_page().await.unwrap();
        assert_eq!(sid, "S1");
        let val = client
            .evaluate_json(&sid, "({state:'ok', n:42})", Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(val["n"], 42);
        srv.await.unwrap();
    }
}
