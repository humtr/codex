use codex_chatgpt_provider::cdp::{Client, Descriptor, Process};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};

// A synchronous app fixture still speaks the production preparation receipt.
// The delayed/lost-reply tests below exercise pending and observer-only paths.
fn bridge_reply(command: &Value, result: Value) -> Value {
    let Some(operation) = command.get("operation") else {
        return result;
    };
    let mut receipt = json!({"id":operation["id"],"generation":operation["generation"],
        "fingerprint":operation["fingerprint"],"owner":command["owner"],"revision":command["bridgeRevision"],
        "state":if operation["action"]=="acknowledge" {"acknowledged"} else {"settled"}});
    if operation["action"] != "acknowledge" {
        assert_eq!(operation["action"], "begin");
        receipt
            .as_object_mut()
            .unwrap()
            .extend(result.as_object().unwrap().clone());
    }
    json!({"value":receipt})
}
#[tokio::test]
async fn home_context_supports_programmatic_inference_without_a_visible_composer() {
    use codex_chatgpt_provider::app::{Bridge, Remote};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    for cold in [false, true] {
        let (native, peer) = tokio::io::duplex(65536);
        let (read, write) = tokio::io::split(native);
        let remote = Remote::pipe(
            Client::pipe(read, write, "home-fixture".into()),
            "owned-bridge".into(),
        );
        let peer = tokio::spawn(async move {
            let (read, mut write) = tokio::io::split(peer);
            let mut reader = BufReader::new(read);
            let mut discoveries = 0;
            loop {
                let mut bytes = Vec::new();
                assert!(reader.read_until(0, &mut bytes).await.unwrap() > 0);
                bytes.pop();
                let row: Value = serde_json::from_slice(&bytes).unwrap();
                let mut opened = false;
                let value = match row["method"].as_str().unwrap() {
                    "Target.getTargets" => {
                        discoveries += 1;
                        let target = if cold && discoveries == 1 {
                            "launcher"
                        } else {
                            "primary"
                        };
                        json!({"targetInfos":[{"targetId":target,"type":"page","url":"app://-/index.html"}]})
                    }
                    "Target.attachToTarget" => json!({"sessionId":row["params"]["targetId"]}),
                    "Target.detachFromTarget" => json!({}),
                    "Runtime.evaluate" => {
                        let expression = row["params"]["expression"].as_str().unwrap();
                        let result = if let Some((_, command)) =
                            expression.split_once("const command=")
                        {
                            let command: Value =
                                serde_json::from_str(command.split_once(";\n").unwrap().0).unwrap();
                            let result = match command["op"].as_str().unwrap() {
                                "identity" => {
                                    json!({"value":{"account":"owned-account","user":"owned-user","structure":"personal"}})
                                }
                                "init" => {
                                    json!({"value":{"ready":row["sessionId"]=="primary"}})
                                }
                                "open" => {
                                    assert_eq!(row["sessionId"], "primary");
                                    opened = true;
                                    json!({"value":{"localId":"owned-lane"}})
                                }
                                _ => panic!("opening may not submit inference or execute tools"),
                            };
                            bridge_reply(&command, result)
                        } else {
                            assert!(expression.contains("contenteditable"));
                            json!(false)
                        };
                        json!({"result":{"value":result}})
                    }
                    _ => panic!("app discovery must not navigate or type"),
                };
                write
                    .write_all(format!("{}\0", json!({"id":row["id"],"result":value})).as_bytes())
                    .await
                    .unwrap();
                if opened {
                    break;
                }
            }
        });
        if !cold {
            assert_eq!(
                remote.operation(json!({"op":"identity"})).await.unwrap()["account"],
                "owned-account"
            );
        }
        let opened = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            remote.operation(json!({"op":"open","lane":"owned-lane"})),
        )
        .await
        .expect("Home must not wait for a user-created composer")
        .unwrap();
        assert_eq!(opened["localId"], "owned-lane");
        peer.await.unwrap();
    }
}
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
                            assert!(text.contains("[contenteditable=\"true\"]"));
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
async fn primary_without_composer_and_multiple_window_disambiguation_are_separate() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let (native, peer) = tokio::io::duplex(65536);
    let (read, write) = tokio::io::split(native);
    let client = Client::pipe(read, write, "cold-start-fixture".into());
    let peer = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(peer);
        let mut reader = BufReader::new(read);
        let mut scans = 0;
        let mut detached = Vec::new();
        loop {
            let mut bytes = Vec::new();
            assert!(reader.read_until(0, &mut bytes).await.unwrap() > 0);
            bytes.pop();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    scans += 1;
                    let aux =
                        json!({"targetId":"launcher","type":"page","url":"app://-/index.html"});
                    let main = json!({"targetId":"main","type":"page","url":"app://-/index.html"});
                    json!({"targetInfos":if scans == 1 {json!([aux])} else {json!([aux,main])}})
                }
                "Target.attachToTarget" => json!({"sessionId":request["params"]["targetId"]}),
                "Runtime.evaluate" => {
                    let probe = request["params"]["expression"].as_str().unwrap();
                    assert!(probe.contains("contenteditable"));
                    assert!(
                        !probe.contains("Chat")
                            && !probe.contains("Work")
                            && !probe.contains(".click(")
                    );
                    json!({"result":{"value":request["sessionId"]=="main"}})
                }
                "Target.detachFromTarget" => {
                    detached.push(request["params"]["sessionId"].as_str().unwrap().to_owned());
                    json!({})
                }
                _ => panic!("cold discovery must only inspect existing surfaces"),
            };
            write
                .write_all(format!("{}\0", json!({"id":request["id"],"result":result})).as_bytes())
                .await
                .unwrap();
            if scans == 2 && detached.len() == 2 {
                break;
            }
        }
        assert_eq!(detached, vec!["launcher", "launcher"]);
    });
    let primary = client.app().await.unwrap();
    assert_eq!(
        primary
            .evaluate("!!document.querySelector('[contenteditable=true]')")
            .await
            .unwrap(),
        false
    );
    client
        .request(
            "Target.detachFromTarget",
            json!({"sessionId":"launcher"}),
            None,
        )
        .await
        .unwrap();
    assert!(client.app().await.is_ok());
    peer.await.unwrap();
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
                let mut finished = false;
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
                    "Runtime.evaluate"
                        if !request["params"]["expression"]
                            .as_str()
                            .unwrap()
                            .contains("const command=") =>
                    {
                        json!({"result":{"value":true}})
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
                                if command["operation"]["action"] == "acknowledge" {
                                    finished = identities == 2;
                                } else {
                                    identities += 1;
                                }
                                if identities == 1 {
                                    json!({"error":"app_auth_loading"})
                                } else {
                                    json!({"value":{"account":"owned-account","user":"owned-user","structure":"personal","present":false}})
                                }
                            }
                            _ => panic!("readiness must never allocate a lane or submit inference"),
                        };
                        json!({"result":{"value":bridge_reply(&command, value)}})
                    }
                    _ => panic!("readiness must not create/select targets"),
                };
                write
                    .write_all(
                        format!("{}\0", json!({"id":request["id"],"result":response})).as_bytes(),
                    )
                    .await
                    .unwrap();
                if ambiguous || finished {
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
            assert_eq!(peer.await.unwrap(), (2, 0, 2));
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
        loop {
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
                    if request["params"]["expression"]
                        .as_str()
                        .unwrap()
                        .starts_with("(()=>{const visible=")
                    {
                        json!({"result":{"value":true}})
                    } else {
                        json!({"result":{"value":{"ready":true}}})
                    }
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
            if request["method"] == "Runtime.evaluate"
                && request["params"]["expression"] == "({ready:true})"
            {
                break;
            }
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

#[tokio::test]
async fn descriptor_maintenance_borrows_the_acquired_connection_and_fences_a_changed_host() {
    use codex_chatgpt_provider::{
        app::{Bridge, Remote},
        journal::random_id,
        protocol::Error,
    };
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("cdp-borrow-{}", random_id().unwrap()));
    std::fs::create_dir(&root).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let process = Process::read(std::process::id()).unwrap();
    let mut descriptor = Descriptor {
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
    let path = root.join("desktop.json");
    std::fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let remote = Remote::new(path.clone(), "owned-fixture".into());
    assert!(matches!(
        remote.cdp_client().await,
        Err(Error("host_not_acquired"))
    ));
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        for _ in 0..5 {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                panic!("request expected")
            };
            let request: Value = serde_json::from_str(&text).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    json!({"targetInfos":[{"targetId":"primary","type":"page","url":"app://-/index.html"}]})
                }
                "Target.attachToTarget" => json!({"sessionId":"owned-session"}),
                "Runtime.evaluate" => {
                    let expression = request["params"]["expression"].as_str().unwrap();
                    let command: Value = serde_json::from_str(
                        expression
                            .split_once("const command=")
                            .unwrap()
                            .1
                            .split_once(";\n")
                            .unwrap()
                            .0,
                    )
                    .unwrap();
                    json!({"result":{"value":bridge_reply(&command,json!({"value":{"account":"fixture"}}))}})
                }
                "Runtime.getIsolateId" => json!({"id":"same-connection"}),
                _ => panic!("maintenance must not initialize or dispatch a model"),
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
    remote.operation(json!({"op":"identity"})).await.unwrap();
    let client = remote.cdp_client().await.unwrap();
    assert_eq!(
        client
            .request("Runtime.getIsolateId", json!({}), None)
            .await
            .unwrap()["id"],
        "same-connection"
    );
    server.await.unwrap();
    descriptor.endpoint = "http://127.0.0.1:9/".into();
    std::fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    assert!(matches!(
        remote.cdp_client().await,
        Err(Error("desktop_generation_changed"))
    ));
    std::fs::remove_dir_all(root).unwrap();
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
            loop {
                let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                    panic!("CDP frame required")
                };
                let request: Value = serde_json::from_str(&text).unwrap();
                let mut identity_replied = false;
                let result = match request["method"].as_str().unwrap() {
                    "Target.getTargets" => {
                        json!({"targetInfos":[{"targetId":"owned","type":"page","url":"app://-/index.html"}]})
                    }
                    "Target.attachToTarget" => json!({"sessionId":"owned-session"}),
                    "Runtime.evaluate"
                        if request["params"]["expression"]
                            .as_str()
                            .unwrap()
                            .starts_with("(()=>{const visible=") =>
                    {
                        json!({"result":{"value":true}})
                    }
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
                            "init" => panic!("identity observation must not install writer hooks"),
                            "identity" => {
                                identity_replied = command["operation"]["action"] == "acknowledge";
                                json!({"account":"fixture-account","user":"fixture-user","structure":"personal","present":false})
                            }
                            _ => panic!("identity reattachment must not submit work"),
                        };
                        json!({"result":{"value":bridge_reply(&command, json!({"value":value}))}})
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
                if identity_replied {
                    break;
                }
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

#[tokio::test]
async fn passive_cleanup_does_not_initialize_and_successful_disposal_reinitializes_on_next_open() {
    use codex_chatgpt_provider::app::{Bridge, Remote};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let (native, peer) = tokio::io::duplex(262144);
    let (read, write) = tokio::io::split(native);
    let remote = Remote::pipe(
        Client::pipe(read, write, "cleanup-fixture".into()),
        "owned".into(),
    );
    let task = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(peer);
        let mut reader = BufReader::new(read);
        let mut operations = Vec::new();
        loop {
            let mut bytes = Vec::new();
            assert!(reader.read_until(0, &mut bytes).await.unwrap() > 0);
            bytes.pop();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    json!({"targetInfos":[{"targetId":"main","type":"page","url":"app://-/index.html"}]})
                }
                "Target.attachToTarget" => json!({"sessionId":"main"}),
                "Runtime.evaluate"
                    if request["params"]["expression"]
                        .as_str()
                        .unwrap()
                        .starts_with("(()=>{const visible=") =>
                {
                    json!({"result":{"value":true}})
                }
                "Runtime.evaluate" => {
                    let expression = request["params"]["expression"].as_str().unwrap();
                    let command: Value = serde_json::from_str(
                        expression
                            .split_once("const command=")
                            .unwrap()
                            .1
                            .split_once(";\n")
                            .unwrap()
                            .0,
                    )
                    .unwrap();
                    assert!(command["bridgeRevision"].is_string());
                    let op = command["op"].as_str().unwrap().to_owned();
                    operations.push(op.clone());
                    json!({"result":{"value":{"value":match op.as_str() {
                        "init" => json!({"ready":true}),
                        "dispose" => json!({"disposed":true}),
                        "release" => json!({"released":true}),
                        _ => json!({"ready":true}),
                    }}}})
                }
                _ => panic!("unexpected CDP operation"),
            };
            write
                .write_all(format!("{}\0", json!({"id":request["id"],"result":result})).as_bytes())
                .await
                .unwrap();
            if operations.len() == 9 {
                break;
            }
        }
        operations
    });
    for op in [
        "snapshot", "stop", "release", "dispose", "open", "dispose", "open",
    ] {
        remote
            .operation(json!({"op":op,"lane":"one"}))
            .await
            .unwrap();
    }
    assert_eq!(
        task.await.unwrap(),
        ["snapshot", "stop", "release", "dispose", "init", "open", "dispose", "init", "open"]
    );
}

#[tokio::test]
async fn new_lane_open_revalidates_bridge_initialization_on_the_same_renderer() {
    use codex_chatgpt_provider::app::{Bridge, Remote};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let (native, peer) = tokio::io::duplex(65536);
    let (read, write) = tokio::io::split(native);
    let remote = Remote::pipe(
        Client::pipe(read, write, "same-renderer".into()),
        "owned".into(),
    );
    let peer = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(peer);
        let mut reader = BufReader::new(read);
        let mut operations = Vec::new();
        loop {
            let mut bytes = Vec::new();
            assert!(reader.read_until(0, &mut bytes).await.unwrap() > 0);
            bytes.pop();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    json!({"targetInfos":[{"targetId":"main","type":"page","url":"app://-/index.html"}]})
                }
                "Target.attachToTarget" => json!({"sessionId":"main"}),
                "Runtime.evaluate" => {
                    let expression = request["params"]["expression"].as_str().unwrap();
                    if expression.starts_with("(()=>{const visible=") {
                        json!({"result":{"value":true}})
                    } else {
                        let command: Value = serde_json::from_str(
                            expression
                                .split_once("const command=")
                                .unwrap()
                                .1
                                .split_once(";\n")
                                .unwrap()
                                .0,
                        )
                        .unwrap();
                        let op = command["op"].as_str().unwrap();
                        assert!(matches!(op, "init" | "open"));
                        operations.push(op.to_owned());
                        json!({"result":{"value":{"value":if op=="init"{json!({"ready":true})}else{json!({"localId":"new-owned-chat"})}}}})
                    }
                }
                _ => panic!("lane reentry must not submit inference"),
            };
            write
                .write_all(format!("{}\0", json!({"id":request["id"],"result":result})).as_bytes())
                .await
                .unwrap();
            if operations.iter().filter(|v| v.as_str() == "open").count() == 2 {
                break;
            }
        }
        assert_eq!(operations, vec!["init", "open", "init", "open"]);
    });
    for lane in ["account-one-lane", "account-two-lane"] {
        remote
            .operation(json!({"op":"open","lane":lane}))
            .await
            .unwrap();
    }
    peer.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn preparation_observes_one_original_promise_across_delays_lost_acks_and_dropped_observers() {
    use codex_chatgpt_provider::{
        app::{Bridge, Remote},
        protocol::Error,
    };
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    for scenario in [
        "slow_network",
        "lost_begin_ack",
        "lost_poll_ack",
        "dropped_observer",
        "unknown",
        "fresh_read",
    ] {
        let (native, peer) = tokio::io::duplex(262144);
        let (read, write) = tokio::io::split(native);
        let remote = Arc::new(Remote::pipe(
            Client::pipe(read, write, "original-host".into()),
            "owner".into(),
        ));
        let ready = Arc::new(AtomicBool::new(false));
        let begins = Arc::new(AtomicUsize::new(0));
        let (observed, mut observations) = tokio::sync::mpsc::unbounded_channel();
        let peer = {
            let ready = ready.clone();
            let begins = begins.clone();
            tokio::spawn(async move {
                let (read, mut write) = tokio::io::split(peer);
                let mut read = BufReader::new(read);
                let mut operation_id = None;
                let mut polls = 0;
                loop {
                    let mut bytes = Vec::new();
                    if read.read_until(0, &mut bytes).await.unwrap() == 0 {
                        break;
                    }
                    assert_eq!(bytes.pop(), Some(0));
                    let request: Value = serde_json::from_slice(&bytes).unwrap();
                    let mut signal = None;
                    let result = match request["method"].as_str().unwrap() {
                        "Target.getTargets" => {
                            json!({"targetInfos":[{"targetId":"original","type":"page","url":"app://-/index.html"}]})
                        }
                        "Target.attachToTarget" => json!({"sessionId":"original-session"}),
                        "Runtime.evaluate" => {
                            let expression = request["params"]["expression"].as_str().unwrap();
                            let command: Value = serde_json::from_str(
                                expression
                                    .split_once("const command=")
                                    .unwrap()
                                    .1
                                    .split_once(";\n")
                                    .unwrap()
                                    .0,
                            )
                            .unwrap();
                            assert_eq!(request["params"]["awaitPromise"], true);
                            let operation = &command["operation"];
                            assert!(
                                operation.is_object(),
                                "read preparation must have an owned receipt"
                            );
                            if command["op"] == "identity" {
                                bridge_reply(
                                    &command,
                                    json!({"value":{"account":"fixture-account","user":"fixture-user","structure":"personal"}}),
                                )
                            } else {
                                assert_eq!(command["op"], "project_read");
                                let id = operation["id"].as_str().unwrap().to_owned();
                                if let Some(original) = &operation_id {
                                    if scenario == "fresh_read" && operation["action"] == "begin" {
                                        assert_ne!(
                                            &id, original,
                                            "a new read caller owns a new observation"
                                        );
                                        operation_id = Some(id);
                                    } else {
                                        assert_eq!(&id, original);
                                    }
                                } else {
                                    operation_id = Some(id);
                                }
                                let mut receipt = json!({"id":operation["id"],"generation":operation["generation"],
                                    "fingerprint":operation["fingerprint"],"owner":command["owner"],"revision":command["bridgeRevision"]});
                                match operation["action"].as_str().unwrap() {
                                    "begin" => {
                                        assert_eq!(
                                            begins.fetch_add(1, Ordering::SeqCst),
                                            usize::from(
                                                scenario == "fresh_read"
                                                    && ready.load(Ordering::SeqCst)
                                            ),
                                            "never resend an app call"
                                        );
                                        signal = Some("begin");
                                        receipt["state"] = json!("pending");
                                        if scenario == "lost_begin_ack" {
                                            observed.send("begin").unwrap();
                                            continue;
                                        }
                                    }
                                    "observe" => {
                                        polls += 1;
                                        signal = Some("observe");
                                        if scenario == "lost_poll_ack" && polls == 1 {
                                            observed.send("observe").unwrap();
                                            continue;
                                        }
                                        receipt["state"] = json!(if scenario == "unknown"
                                            || scenario == "fresh_read"
                                                && begins.load(Ordering::SeqCst) == 1
                                        {
                                            "unknown"
                                        } else if ready.load(Ordering::SeqCst) {
                                            "settled"
                                        } else {
                                            "pending"
                                        });
                                        if receipt["state"] == "settled" {
                                            receipt["value"] = json!({"id":"g-p-original","name":"Codex","compatible":true});
                                        }
                                    }
                                    "acknowledge" => {
                                        receipt["state"] = json!("acknowledged");
                                    }
                                    _ => panic!("unexpected receipt action"),
                                }
                                json!({"value":receipt})
                            }
                        }
                        _ => panic!("read preparation may not initialize, submit or navigate"),
                    };
                    let result = if request["method"] == "Runtime.evaluate" {
                        json!({"result":{"value":result}})
                    } else {
                        result
                    };
                    write
                        .write_all(
                            format!("{}\0", json!({"id":request["id"],"result":result})).as_bytes(),
                        )
                        .await
                        .unwrap();
                    if let Some(signal) = signal {
                        observed.send(signal).unwrap();
                    }
                }
            })
        };
        let mut command = json!({"op":"project_read","project":"g-p-original","lane":"original-lane","nativeThread":"original-thread","preparation":"original-native-request"});
        if scenario == "fresh_read" {
            command.as_object_mut().unwrap().remove("preparation");
        }
        let mut work = {
            let remote = remote.clone();
            let command = command.clone();
            tokio::spawn(async move { remote.operation(command).await })
        };
        assert_eq!(observations.recv().await, Some("begin"));
        if scenario == "lost_begin_ack" {
            tokio::time::advance(std::time::Duration::from_secs(16)).await;
        }
        assert_eq!(observations.recv().await, Some("observe"));
        // The shared CDP client continues serving a genuinely independent call
        // while the original app's network Promise remains unsettled.
        assert_eq!(
            remote
                .operation(json!({"op":"identity","lane":"independent"}))
                .await
                .unwrap()["account"],
            "fixture-account"
        );
        match scenario {
            "lost_poll_ack" => {
                tokio::time::advance(std::time::Duration::from_secs(16)).await;
                assert_eq!(work.await.unwrap(), Err(Error("cdp_operation_timeout")));
                work = {
                    let remote = remote.clone();
                    let command = command.clone();
                    tokio::spawn(async move { remote.operation(command).await })
                };
                assert_eq!(observations.recv().await, Some("observe"));
            }
            "dropped_observer" => {
                work.abort();
                assert!(work.await.unwrap_err().is_cancelled());
                work = {
                    let remote = remote.clone();
                    let command = command.clone();
                    tokio::spawn(async move { remote.operation(command).await })
                };
                assert_eq!(observations.recv().await, Some("observe"));
            }
            "fresh_read" => {
                assert_eq!(work.await.unwrap(), Err(Error("app_operation_unavailable")));
                ready.store(true, Ordering::SeqCst);
                assert_eq!(
                    remote.operation(command).await.unwrap()["id"],
                    "g-p-original"
                );
                assert_eq!(begins.load(Ordering::SeqCst), 2);
                peer.abort();
                let _ = peer.await;
                continue;
            }
            "unknown" => {
                assert_eq!(work.await.unwrap(), Err(Error("app_operation_unavailable")));
                assert_eq!(
                    remote.operation(command).await,
                    Err(Error("app_operation_unavailable"))
                );
                assert_eq!(begins.load(Ordering::SeqCst), 1);
                peer.abort();
                let _ = peer.await;
                continue;
            }
            _ => {
                tokio::time::advance(std::time::Duration::from_secs(16)).await;
                tokio::task::yield_now().await;
                assert!(
                    !work.is_finished(),
                    "network duration is not a CDP command timeout"
                );
            }
        }
        ready.store(true, Ordering::SeqCst);
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), work)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result["id"], "g-p-original");
        assert_eq!(begins.load(Ordering::SeqCst), 1);
        peer.abort();
        let _ = peer.await;
    }
}

#[tokio::test]
async fn a_failed_old_observer_cannot_remove_a_new_host_connection_or_rebegin_its_operation() {
    use codex_chatgpt_provider::{
        app::{Bridge, Remote},
        journal::random_id,
        protocol::Error,
    };
    use std::{os::unix::fs::PermissionsExt, sync::Arc};
    let root = std::env::temp_dir().join(format!("cdp-generation-race-{}", random_id().unwrap()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("host.json");
    let old_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let new_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let process = Process::read(std::process::id()).unwrap();
    let mut descriptor = Descriptor {
        version: 1,
        kind: "termux-electron-pipe".into(),
        endpoint: format!(
            "http://127.0.0.1:{}/",
            old_listener.local_addr().unwrap().port()
        ),
        token: "a".repeat(43),
        host: process.clone(),
        browser: process,
    };
    std::fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let remote = Arc::new(Remote::new(path.clone(), "original-owner".into()));
    let (arrived, waiting) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let old_peer = tokio::spawn(async move {
        let (socket, _) = old_listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        let mut arrived = Some(arrived);
        let mut released = Some(released);
        loop {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                panic!("CDP request expected")
            };
            let request: Value = serde_json::from_str(&text).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    json!({"targetInfos":[{"targetId":"old-target","type":"page","url":"app://-/index.html"}]})
                }
                "Target.attachToTarget" => json!({"sessionId":"old-session"}),
                "Runtime.evaluate" => {
                    arrived.take().unwrap().send(()).unwrap();
                    released.take().unwrap().await.unwrap();
                    socket
                        .send(Message::Text(
                            json!({"id":request["id"],"error":{"code":-32000}})
                                .to_string()
                                .into(),
                        ))
                        .await
                        .unwrap();
                    break;
                }
                _ => panic!("old observer may not dispatch work"),
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
    let command =
        json!({"op":"identity","lane":"old-lane","preparation":"original-native-request"});
    let old = {
        let remote = remote.clone();
        let command = command.clone();
        tokio::spawn(async move { remote.operation(command).await })
    };
    waiting.await.unwrap();
    descriptor.endpoint = format!(
        "http://127.0.0.1:{}/",
        new_listener.local_addr().unwrap().port()
    );
    descriptor.token = "b".repeat(43);
    std::fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let new_peer = tokio::spawn(async move {
        let (socket, _) = new_listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        loop {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                panic!("CDP request expected")
            };
            let request: Value = serde_json::from_str(&text).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => {
                    json!({"targetInfos":[{"targetId":"new-target","type":"page","url":"app://-/index.html"}]})
                }
                "Target.attachToTarget" => json!({"sessionId":"new-session"}),
                "Runtime.evaluate" => {
                    let expression = request["params"]["expression"].as_str().unwrap();
                    let command: Value = serde_json::from_str(
                        expression
                            .split_once("const command=")
                            .unwrap()
                            .1
                            .split_once(";\n")
                            .unwrap()
                            .0,
                    )
                    .unwrap();
                    assert_eq!(
                        command["lane"], "new-lane",
                        "a lost operation cannot begin in the replacement host"
                    );
                    json!({"result":{"value":bridge_reply(&command,json!({"value":{"account":"new-account","user":"new-user","structure":"personal"}}))}})
                }
                "Runtime.getIsolateId" => json!({"id":"new-connection"}),
                _ => panic!("replacement observation must remain read-only"),
            };
            socket
                .send(Message::Text(
                    json!({"id":request["id"],"result":result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            if request["method"] == "Runtime.getIsolateId" {
                break;
            }
        }
    });
    assert_eq!(
        remote
            .operation(json!({"op":"identity","lane":"new-lane"}))
            .await
            .unwrap()["account"],
        "new-account"
    );
    release.send(()).unwrap();
    assert_eq!(old.await.unwrap(), Err(Error("cdp_operation_rejected")));
    assert_eq!(
        remote.operation(command).await,
        Err(Error("app_operation_identity_changed"))
    );
    let current = remote.cdp_client().await.unwrap();
    assert_eq!(current.generation.as_ref(), descriptor.generation());
    assert_eq!(
        current
            .request("Runtime.getIsolateId", json!({}), None)
            .await
            .unwrap()["id"],
        "new-connection"
    );
    old_peer.await.unwrap();
    new_peer.await.unwrap();
    drop(remote);
    std::fs::remove_dir_all(root).unwrap();
}
