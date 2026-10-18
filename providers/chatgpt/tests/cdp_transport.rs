use codex_chatgpt_provider::cdp::{Client, Descriptor, Process};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};

#[tokio::test]
async fn one_authenticated_relay_multiplexes_calls_and_reattaches_only_unique_owned_app() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let process = Process::read(std::process::id()).unwrap();
    let descriptor = Descriptor {
        version: 1,
        kind: "termux-electron-pipe".into(),
        endpoint: format!(
            "http://127.0.0.1:{}/",
            listener.local_addr().unwrap().port()
        ),
        token: "a".repeat(43),
        host: process.clone(),
        browser: process,
    };
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        // tungstenite's callback fixes the by-value HTTP error type.
        #[allow(clippy::result_large_err)]
        fn admit(
            request: &Request,
            response: Response,
        ) -> Result<Response, tokio_tungstenite::tungstenite::handshake::server::ErrorResponse>
        {
            assert_eq!(request.uri().path(), "/cdp");
            assert_eq!(
                request.headers()["authorization"],
                format!("Bearer {}", "a".repeat(43))
            );
            assert!(!request.headers().contains_key("origin"));
            Ok(response)
        }
        let mut socket = accept_hdr_async(socket, admit).await.unwrap();
        let mut requests = Vec::<Value>::new();
        for _ in 0..2 {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                panic!("expected request")
            };
            requests.push(serde_json::from_str(&text).unwrap());
        }
        for request in requests.iter().rev() {
            socket
                .send(Message::Text(
                    json!({"id":request["id"],"result":{"echo":request["params"]["value"]}})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
        for _ in 0..3 {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                panic!("expected app request")
            };
            let request: Value = serde_json::from_str(&text).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    json!({"targetInfos":[{"targetId":"owned","type":"page","url":"app://-/index.html"},{"targetId":"foreign","type":"page","url":"https://private.invalid/"}]})
                }
                "Target.attachToTarget" => {
                    assert_eq!(request["params"]["targetId"], "owned");
                    json!({"sessionId":"owned-session"})
                }
                "Runtime.evaluate" => {
                    assert_eq!(request["sessionId"], "owned-session");
                    json!({"result":{"value":{"ready":true}}})
                }
                _ => panic!("unexpected CDP operation"),
            };
            socket
                .send(Message::Text(
                    json!({"id":request["id"],"result":result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
    });
    let client = Client::connect(&descriptor).await.unwrap();
    let (first, second) = tokio::join!(
        client.request("Fixture.echo", json!({"value":"first"}), None),
        client.request("Fixture.echo", json!({"value":"second"}), None)
    );
    assert_eq!(first.unwrap()["echo"], "first");
    assert_eq!(second.unwrap()["echo"], "second");
    let app = client.app().await.unwrap();
    assert_eq!(app.evaluate("({ready:true})").await.unwrap()["ready"], true);
    server.await.unwrap();
}

#[test]
fn descriptor_rejects_remote_endpoints_and_stale_process_identity() {
    let process = Process::read(std::process::id()).unwrap();
    let mut d = Descriptor {
        version: 1,
        kind: "termux-electron-pipe".into(),
        endpoint: "http://127.0.0.1:12345/".into(),
        token: "a".repeat(43),
        host: process.clone(),
        browser: process,
    };
    d.validate().unwrap();
    let original = d.generation();
    d.token = "b".repeat(43);
    assert_ne!(d.generation(), original);
    for endpoint in [
        "http://remote.invalid:12345/",
        "http://127.0.0.1:12345/path",
        "http://127.0.0.1:0/",
        "http://user@127.0.0.1:12345/",
    ] {
        d.endpoint = endpoint.into();
        assert!(d.validate().is_err());
    }
    d.endpoint = "http://127.0.0.1:12345/".into();
    d.browser.start = "not-current".into();
    assert!(d.validate().is_err());
}

#[tokio::test]
async fn private_pipe_preserves_fragmented_frames_out_of_order_ids_and_disconnects_pending_calls() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let (client, peer) = tokio::io::duplex(65536);
    let (read, write) = tokio::io::split(client);
    let client = Client::pipe(read, write, "owned-pipe-fixture".into());
    let server = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(peer);
        let mut read = BufReader::new(read);
        let mut requests = Vec::new();
        for _ in 0..2 {
            let mut bytes = Vec::new();
            read.read_until(0, &mut bytes).await.unwrap();
            bytes.pop();
            requests.push(serde_json::from_slice::<Value>(&bytes).unwrap());
        }
        for request in requests.iter().rev() {
            let text = json!({"id":request["id"],"result":{"echo":request["params"]["value"]}})
                .to_string();
            let half = text.len() / 2;
            write.write_all(&text.as_bytes()[..half]).await.unwrap();
            tokio::task::yield_now().await;
            write.write_all(&text.as_bytes()[half..]).await.unwrap();
            write.write_all(&[0]).await.unwrap();
        }
        let mut bytes = Vec::new();
        read.read_until(0, &mut bytes).await.unwrap();
        // Closing a partial frame never fabricates a successful reply.
        write.write_all(b"{\"id\":").await.unwrap();
    });
    let (first, second) = tokio::join!(
        client.request("Fixture.echo", json!({"value":"first"}), None),
        client.request("Fixture.echo", json!({"value":"second"}), None)
    );
    assert_eq!(first.unwrap()["echo"], "first");
    assert_eq!(second.unwrap()["echo"], "second");
    assert!(client
        .request("Fixture.lost", json!({}), None)
        .await
        .is_err());
    server.await.unwrap();
}
