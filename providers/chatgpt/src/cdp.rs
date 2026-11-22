//! Single multiplexed Chrome DevTools Protocol (CDP) client over Chromium's
//! private `--remote-debugging-pipe` (NUL-delimited JSON-RPC on FD 3/4).
//!
//! Zero TCP debug listener, zero relay daemon.

use crate::error::{ProviderError, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const MAX_PENDING_REQUESTS: usize = 128;
const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(30);

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
}

impl CdpClient {
    pub fn from_pipe<R, W>(reader: R, mut writer: W, generation: String) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<CdpCommand>(MAX_PENDING_REQUESTS);
        let (in_tx, mut in_rx) = mpsc::channel::<Value>(MAX_PENDING_REQUESTS);

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
                        }
                    }
                }
            }
            reader_task.abort();
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
        }
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
