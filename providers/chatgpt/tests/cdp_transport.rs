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
async fn auxiliary_index_pages_are_readonly_probed_without_choosing_an_ambiguous_main() {
    use codex_chatgpt_provider::protocol::Error;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    // 0: one main; 1: two main surfaces; 2: startup not ready; 3: lost renderer.
    for case in 0..4 {
        let (native, peer) = tokio::io::duplex(65536);
        let (read, write) = tokio::io::split(native);
        let client = Client::pipe(read, write, "owned-index-fixture".into());
        let peer = tokio::spawn(async move {
            let (read, mut write) = tokio::io::split(peer);
            let mut reader = BufReader::new(read);
            let mut detached = Vec::new();
            let mut probes = 0;
            loop {
                let mut bytes = Vec::new();
                reader.read_until(0, &mut bytes).await.unwrap();
                assert_eq!(bytes.pop(), Some(0));
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                let result = match request["method"].as_str().unwrap() {
                    "Target.getTargets" => json!({"targetInfos":[
                        {"targetId":"aux","type":"page","url":"app://-/index.html"},
                        {"targetId":"main","type":"page","url":"app://-/index.html"},
                        {"targetId":"foreign","type":"page","url":"https://private.invalid/"}]}),
                    "Target.attachToTarget" => {
                        let id = request["params"]["targetId"].as_str().unwrap();
                        assert!(matches!(id, "aux" | "main"));
                        json!({"sessionId":id})
                    }
                    "Runtime.evaluate" => {
                        let text = request["params"]["expression"].as_str().unwrap();
                        if text == "verify_selected" {
                            assert_eq!(request["sessionId"], "main");
                            json!({"result":{"value":true}})
                        } else {
                            assert!(text.contains("labels.has('Chat')&&labels.has('Work')"));
                            assert!(!text.contains(".click(") && !text.contains("fetch("));
                            probes += 1;
                            if case == 3 && request["sessionId"] == "main" {
                                json!({"exceptionDetails":{"text":"fixture renderer unavailable"}})
                            } else {
                                json!({"result":{"value":case==1||case==0&&request["sessionId"]=="main"}})
                            }
                        }
                    }
                    "Target.detachFromTarget" => {
                        detached.push(request["params"]["sessionId"].as_str().unwrap().to_owned());
                        json!({})
                    }
                    _ => panic!("selection must not create, navigate or activate a target"),
                };
                write
                    .write_all(
                        format!("{}\0", json!({"id":request["id"],"result":result})).as_bytes(),
                    )
                    .await
                    .unwrap();
                if case == 0
                    && request["method"] == "Runtime.evaluate"
                    && request["params"]["expression"] == "verify_selected"
                    || case != 0 && detached.len() == 2
                {
                    break;
                }
            }
            assert_eq!(probes, 2);
            if case == 0 {
                assert_eq!(detached, vec!["aux"]);
            } else {
                assert_eq!(detached, vec!["aux", "main"]);
            }
        });
        match (case, client.app().await) {
            (0, Ok(app)) => assert_eq!(app.evaluate("verify_selected").await.unwrap(), true),
            (1, Err(e)) => assert_eq!(e, Error("unique_app_target_required")),
            (2, Err(e)) => assert_eq!(e, Error("app_target_unavailable")),
            (3, Err(e)) => assert_eq!(e, Error("app_bridge_evaluation_failed")),
            _ => panic!("incorrect main target admission"),
        }
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn first_owned_admission_waits_for_readonly_startup_and_rejects_ambiguous_targets() {
    use codex_chatgpt_provider::{
        app::{Bridge, Remote},
        protocol::Error,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    for ambiguous in [false, true] {
        let (native, peer) = tokio::io::duplex(65536);
        let (read, write) = tokio::io::split(native);
        let remote = Remote::pipe(
            Client::pipe(read, write, "owned-pipe-generation".into()),
            "owned-bridge".into(),
        );
        let peer = tokio::spawn(async move {
            let (read, mut write) = tokio::io::split(peer);
            let mut reader = BufReader::new(read);
            let (mut targets, mut inits, mut identities) = (0, 0, 0);
            loop {
                let mut bytes = Vec::new();
                reader.read_until(0, &mut bytes).await.unwrap();
                assert_eq!(bytes.pop(), Some(0));
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                let response = match request["method"].as_str().unwrap() {
                    "Target.getTargets" => {
                        targets += 1;
                        let target =
                            json!({"targetId":"owned","type":"page","url":"app://-/index.html"});
                        json!({"targetInfos":if ambiguous{json!([target.clone(),target])}else if targets==1{json!([])}else{json!([target])}})
                    }
                    "Target.attachToTarget" => {
                        assert!(!ambiguous);
                        json!({"sessionId":"owned-session"})
                    }
                    "Runtime.evaluate" => {
                        let command = request["params"]["expression"]
                            .as_str()
                            .unwrap()
                            .split_once("const command=")
                            .unwrap()
                            .1
                            .split_once(";\n")
                            .unwrap()
                            .0;
                        let command: Value = serde_json::from_str(command).unwrap();
                        let value = match command["op"].as_str().unwrap() {
                            "init" => {
                                inits += 1;
                                json!({"value":{"ready":inits>=2}})
                            }
                            "identity" => {
                                identities += 1;
                                if identities == 1 {
                                    json!({"error":"app_auth_loading"})
                                } else {
                                    json!({"value":{"account":"owned-account","user":"owned-user","structure":"personal","present":false}})
                                }
                            }
                            _ => panic!("readiness must never allocate a lane or submit inference"),
                        };
                        json!({"result":{"value":value}})
                    }
                    _ => panic!("readiness must not create/select targets"),
                };
                write
                    .write_all(
                        format!("{}\0", json!({"id":request["id"],"result":response})).as_bytes(),
                    )
                    .await
                    .unwrap();
                if ambiguous || identities == 2 {
                    break;
                }
            }
            (targets, inits, identities)
        });
        let result = remote
            .operation(json!({"op":"identity","lane":"not-opened"}))
            .await;
        if ambiguous {
            assert_eq!(result, Err(Error("unique_app_target_required")));
            assert_eq!(peer.await.unwrap(), (1, 0, 0));
        } else {
            assert_eq!(result.unwrap()["account"], "owned-account");
            assert_eq!(peer.await.unwrap(), (2, 2, 2));
        }
    }
}

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

#[tokio::test]
async fn remote_reattaches_verified_host_for_identity_without_submitting_work() {
    use codex_chatgpt_provider::{
        app::{Bridge, Remote},
        journal::random_id,
    };
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let root = std::env::temp_dir().join(format!("remote-{}", random_id().unwrap()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let descriptor = root.join("descriptor.json");
    let remote = Remote::new(descriptor.clone(), random_id().unwrap());
    let mut previous = None;
    for index in 0..2 {
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let process = Process::read(std::process::id()).unwrap();
        let d = Descriptor {
            version: 1,
            kind: "termux-electron-pipe".into(),
            endpoint: format!("http://127.0.0.1:{}/", socket.local_addr().unwrap().port()),
            token: if index == 0 {
                "a".repeat(43)
            } else {
                "b".repeat(43)
            },
            host: process.clone(),
            browser: process,
        };
        std::fs::write(&descriptor, serde_json::to_vec(&d).unwrap()).unwrap();
        std::fs::set_permissions(&descriptor, std::fs::Permissions::from_mode(0o600)).unwrap();
        let peer = tokio::spawn(async move {
            let (socket, _) = socket.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            for _ in 0..4 {
                let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                    panic!("CDP frame required")
                };
                let request: Value = serde_json::from_str(&text).unwrap();
                let result = match request["method"].as_str().unwrap() {
                    "Target.getTargets" => {
                        json!({"targetInfos":[{"targetId":"owned","type":"page","url":"app://-/index.html"}]})
                    }
                    "Target.attachToTarget" => json!({"sessionId":"owned-session"}),
                    "Runtime.evaluate" => {
                        let expression = request["params"]["expression"].as_str().unwrap();
                        let command = expression
                            .split_once("const command=")
                            .unwrap()
                            .1
                            .split_once(";\n")
                            .unwrap()
                            .0;
                        let command: Value = serde_json::from_str(command).unwrap();
                        let value = match command["op"].as_str().unwrap() {
                            "init" => json!({"ready":true}),
                            "identity" => {
                                json!({"account":"fixture-account","user":"fixture-user","structure":"personal","present":false})
                            }
                            _ => panic!("identity reattachment must not submit work"),
                        };
                        json!({"result":{"value":{"value":value}}})
                    }
                    _ => panic!("unexpected CDP method"),
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
        let identity = remote
            .operation(json!({"op":"identity","lane":"fixture-lane"}))
            .await
            .unwrap();
        assert_eq!(identity["account"], "fixture-account");
        assert_eq!(identity["generation"], d.generation());
        if let Some(previous) = &previous {
            assert_ne!(previous, &identity["generation"]);
        }
        previous = Some(identity["generation"].clone());
        peer.await.unwrap();
    }
    drop(remote);
    std::fs::remove_dir_all(root).unwrap();
}
