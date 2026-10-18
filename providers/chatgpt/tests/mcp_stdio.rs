//! Existing tunnel stdio framing -> actual Rust HTTP broker, without an app.
use codex_chatgpt_provider::{
    config::Config,
    journal::{random_id, Journal},
    mcp::Broker,
    protocol::Request,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    time::{timeout, Duration},
};
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
async fn receive(input: &mut tokio::io::Lines<BufReader<tokio::io::DuplexStream>>) -> Value {
    serde_json::from_str(
        &timeout(Duration::from_secs(5), input.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap()
}
async fn send(input: &mut tokio::io::DuplexStream, row: Value) {
    input
        .write_all(format!("{row}\n").as_bytes())
        .await
        .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stdio_multiplexes_metadata_while_a_native_call_waits_and_detaches_without_replay() {
    use std::os::unix::fs::PermissionsExt;
    let root = Root(std::env::temp_dir().join(format!("pms-{}", &random_id().unwrap()[..8])));
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal);
    let request=Request::parse(json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":[{"role":"user","content":"Owned"}],"tools":[{"type":"function","name":"exec","parameters":{"type":"object"}}]}),&Config::default(),Some("owned")).unwrap();
    let token = broker.register("owned", request.tools).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = broker.clone().router();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let (mut input, reader) = tokio::io::duplex(65536);
    let (writer, output) = tokio::io::duplex(65536);
    let stdio = tokio::spawn(codex_chatgpt_provider::mcp_stdio::run(reader, writer, port));
    let mut output = BufReader::new(output).lines();
    send(&mut input,json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}})).await;
    let init = receive(&mut output).await;
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    let call = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"native_tool","arguments":{"turn_token":token,"call_id":"exact-action","name":"exec","input":"{}","report":"Run once"}}});
    send(&mut input, call.clone()).await;
    let offered = timeout(Duration::from_secs(5), async {
        loop {
            let v = broker.offer(&token).await.unwrap();
            if !v.is_empty() {
                break v[0].clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    send(&mut input, call.clone()).await;
    let duplicate = receive(&mut output).await;
    assert_eq!(duplicate["error"]["message"], "mcp_rpc_id_in_flight");
    send(&mut input, json!({"jsonrpc":"2.0","id":3,"method":"ping"})).await;
    assert_eq!(
        receive(&mut output).await["id"],
        3,
        "pending native result must not block metadata"
    );
    send(
        &mut input,
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}),
    )
    .await;
    // Cancellation is a peer lifecycle event. Wait for its local request slot
    // to retire; metadata continues and the broker has exactly one offered call.
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(broker.offer(&token).await.unwrap().is_empty());
    broker
        .settle(
            &token,
            &[(
                offered["id"].as_str().unwrap().into(),
                json!({"output":"native result"}),
            )],
        )
        .await
        .unwrap();
    let mut retry = call;
    retry["id"] = json!(4);
    send(&mut input, retry).await;
    let returned = receive(&mut output).await;
    assert_eq!(returned["id"], 4);
    assert_eq!(returned["result"]["isError"], false);
    assert!(returned.to_string().contains("native result"));
    input.write_all(b"{malformed\n").await.unwrap();
    assert_eq!(receive(&mut output).await["error"]["code"], -32700);
    drop(input);
    timeout(Duration::from_secs(3), stdio)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    broker.close().await;
    server.abort();
}
