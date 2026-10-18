//! Optional tunnel stdio framing; the existing Rust service remains the sole
//! broker. No app startup, model call, native execution or delivery retry here.
use crate::protocol::Error;
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::{
    io::{
        AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
        BufReader,
    },
    sync::Mutex,
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
const LIMIT: usize = 16 * 1024 * 1024;
#[derive(Default)]
struct Framing {
    output: Vec<u8>,
    overflow: bool,
}
async fn line(
    input: &mut (impl AsyncBufRead + Unpin),
    framing: &mut Framing,
) -> Result<Option<Vec<u8>>, Error> {
    // State survives select cancellation when another RPC finishes midway
    // through a fragmented input line. Already consumed bytes must not vanish.
    let Framing { output, overflow } = framing;
    loop {
        let bytes = input
            .fill_buf()
            .await
            .map_err(|_| Error("mcp_stdin_failed"))?;
        if bytes.is_empty() {
            return if *overflow {
                *overflow = false;
                Err(Error("mcp_request_too_large"))
            } else if output.is_empty() {
                Ok(None)
            } else {
                Ok(Some(std::mem::take(output)))
            };
        }
        let end = bytes.iter().position(|b| *b == b'\n');
        let count = end.map_or(bytes.len(), |v| v + 1);
        if !*overflow {
            if output.len() + count > LIMIT {
                *overflow = true;
                output.clear();
            } else {
                output.extend_from_slice(&bytes[..count]);
            }
        }
        input.consume(count);
        if end.is_some() {
            return if *overflow {
                *overflow = false;
                Err(Error("mcp_request_too_large"))
            } else {
                Ok(Some(std::mem::take(output)))
            };
        }
    }
}
async fn forward(port: u16, row: &Value) -> Result<Option<Value>, Error> {
    let body = row.to_string();
    let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|_| Error("mcp_provider_unavailable"))?;
    socket.write_all(format!("POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.map_err(|_|Error("mcp_delivery_unknown"))?;
    let mut bytes = Vec::new();
    socket
        .take((2 * LIMIT + 8193) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error("mcp_delivery_unknown"))?;
    if bytes.len() > 2 * LIMIT + 8192 {
        return Err(Error("mcp_response_too_large"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| Error("mcp_response_invalid"))?;
    let (headers, body) = text
        .split_once("\r\n\r\n")
        .filter(|(headers, _)| headers.len() <= 8192)
        .ok_or(Error("mcp_response_invalid"))?;
    if row.get("id").is_none() && headers.starts_with("HTTP/1.1 202 ") {
        return Ok(None);
    }
    if !headers.starts_with("HTTP/1.1 200 ") {
        return Err(Error("mcp_provider_rejected"));
    }
    let reply: Value = serde_json::from_str(body).map_err(|_| Error("mcp_response_invalid"))?;
    if reply["jsonrpc"] != "2.0" || reply["id"] != row["id"] {
        return Err(Error("mcp_response_invalid"));
    }
    Ok(Some(reply))
}
async fn send(output: &Arc<Mutex<impl AsyncWrite + Unpin>>, row: Value) -> Result<(), Error> {
    let mut output = output.lock().await;
    output
        .write_all(format!("{row}\n").as_bytes())
        .await
        .map_err(|_| Error("mcp_stdout_failed"))?;
    output.flush().await.map_err(|_| Error("mcp_stdout_failed"))
}
fn failure(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
pub async fn run(
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin + Send + 'static,
    port: u16,
) -> Result<(), Error> {
    let mut input = BufReader::new(input);
    let mut framing = Framing::default();
    let output = Arc::new(Mutex::new(output));
    let mut active = BTreeMap::<String, CancellationToken>::new();
    let mut tasks = JoinSet::new();
    loop {
        let next = tokio::select! {
            done=tasks.join_next(),if !tasks.is_empty()=>{match done{Some(Ok((key,result)))=>{active.remove(&key);result?;},Some(Err(_))=>return Err(Error("mcp_forward_failed")),None=>()}continue;},
            row=line(&mut input,&mut framing)=>row,
        };
        let bytes = match next {
            Ok(Some(bytes)) => bytes,
            Ok(None) => break,
            Err(Error("mcp_request_too_large")) => {
                send(
                    &output,
                    failure(Value::Null, -32600, "mcp_request_too_large"),
                )
                .await?;
                continue;
            }
            Err(error) => return Err(error),
        };
        let row: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => {
                send(&output, failure(Value::Null, -32700, "mcp_json_invalid")).await?;
                continue;
            }
        };
        if row["jsonrpc"] != "2.0" || !row["method"].is_string() {
            send(&output, failure(Value::Null, -32600, "mcp_request_invalid")).await?;
            continue;
        }
        if row["method"] == "notifications/cancelled" {
            if let Some(cancel) = active.get(&row["params"]["requestId"].to_string()) {
                cancel.cancel();
            }
            continue;
        }
        let id = row.get("id").cloned();
        if id
            .as_ref()
            .is_some_and(|v| !v.is_string() && !v.is_number())
        {
            send(&output, failure(Value::Null, -32600, "mcp_request_invalid")).await?;
            continue;
        }
        if id.is_none() {
            continue;
        }
        let key = id.as_ref().unwrap().to_string();
        if active.contains_key(&key) {
            send(
                &output,
                failure(id.unwrap(), -32600, "mcp_rpc_id_in_flight"),
            )
            .await?;
            continue;
        }
        if active.len() >= 64 {
            send(&output, failure(id.unwrap(), -32000, "mcp_observer_limit")).await?;
            continue;
        }
        let cancel = CancellationToken::new();
        active.insert(key.clone(), cancel.clone());
        let output = output.clone();
        tasks.spawn(async move {
            let result=tokio::select!{
                _=cancel.cancelled()=>Ok(()),
                reply=tokio::time::timeout(Duration::from_secs(610),forward(port,&row))=>match reply {
                    Ok(Ok(Some(value)))=>send(&output,value).await,
                    Ok(Ok(None))=>Ok(()),
                    other=>{let error=match other{Ok(Err(error))=>error, _=>Error("mcp_observer_timeout")};send(&output,failure(row["id"].clone(),-32000,error.0)).await},
                },
            };(key,result)
        });
    }
    // EOF/cancellation detaches HTTP observers; it never stops or reissues a
    // native action. A fresh exact logical retry can collect its stored result.
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fragmented_input_survives_a_cancelled_reader_between_rpc_completions() {
        let (mut input, read) = tokio::io::duplex(1024);
        let mut read = BufReader::new(read);
        let mut framing = Framing::default();
        input.write_all(b"{\"jsonrpc\":").await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), line(&mut read, &mut framing))
                .await
                .is_err()
        );
        assert_eq!(framing.output, b"{\"jsonrpc\":");
        input
            .write_all(b"\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
            .await
            .unwrap();
        let row: Value =
            serde_json::from_slice(&line(&mut read, &mut framing).await.unwrap().unwrap()).unwrap();
        assert_eq!(row["method"], "ping");
        assert!(framing.output.is_empty());
    }
}
