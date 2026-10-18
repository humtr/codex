//! Actual TCP/WebSocket/private-RFB fixture; no app, account or model access.
use codex_chatgpt_provider::{
    config::Config,
    journal::random_id,
    protocol::{Error, Request},
    responses::Event,
    server::{self, Host, Service},
    viewer::Viewer,
};
use futures_util::{future::BoxFuture, SinkExt, StreamExt};
use std::{
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use tokio_util::sync::CancellationToken;
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct NoModel(AtomicUsize);
impl Host for NoModel {
    fn exchange(
        &self,
        _: Request,
        _: mpsc::Sender<Event>,
        _: CancellationToken,
    ) -> BoxFuture<'static, Result<(), Error>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(Error("fixture_no_inference")) })
    }
}
async fn http(address: std::net::SocketAddr, path: &str, headers: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n{headers}\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.read_to_end(&mut bytes),
    )
    .await
    .unwrap()
    .unwrap();
    String::from_utf8(bytes).unwrap()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn viewer_capability_assets_origin_rfb_and_shutdown_are_independent_of_responses_auth() {
    let root = Root(std::env::temp_dir().join(format!("pv-{}", &random_id().unwrap()[..8])));
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let assets = root.0.join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::create_dir(assets.join("core")).unwrap();
    std::fs::write(
        assets.join("vnc.html"),
        "<html><body>owned VNC</body></html>",
    )
    .unwrap();
    std::fs::write(assets.join("core/rfb.js"), "export default 'owned';").unwrap();
    std::fs::write(root.0.join("private.json"), "private outside assets").unwrap();
    symlink(root.0.join("private.json"), assets.join("leak.json")).unwrap();
    symlink(&root.0, assets.join("leak-directory")).unwrap();
    let socket = root.0.join("rfb");
    let rfb = tokio::net::UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let viewer = Viewer::new(socket.clone(), assets.clone(), address.port()).unwrap();
    let capability = viewer.capability.clone();
    let prefix = format!("/viewer/{capability}");
    let host = Arc::new(NoModel(AtomicUsize::new(0)));
    let service = Service::new(
        Config::default(),
        "owned_endpoint_token_012345678901234567890123456789".into(),
        host.clone(),
    )
    .unwrap()
    .with_viewer(viewer)
    .unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let task = tokio::spawn(async move {
        server::serve(listener, service, shutdown).await.unwrap();
    });
    let html = http(address, &format!("{prefix}/vnc.html"), "").await;
    assert!(html.starts_with("HTTP/1.1 200"));
    for expected in [
        "owned VNC",
        "owned_keyboard_auto",
        "owned_keyboard",
        "owned_scroll",
        "owned_escape",
        "owned_fullscreen",
        "cache-control: no-store",
        "referrer-policy: no-referrer",
    ] {
        assert!(html.contains(expected), "missing viewer/control header");
    }
    for path in [
        "../private.json",
        "%2e%2e/private.json",
        "leak.json",
        "leak-directory/private.json",
        "missing.js",
        "unknown.sh",
    ] {
        let reply = http(address, &format!("{prefix}/{path}"), "").await;
        assert!(
            reply.starts_with("HTTP/1.1 404"),
            "asset boundary failed: {path}"
        );
        assert!(!reply.contains("private outside assets"));
    }
    for file in ["mobile_controls.js", "mobile_controls.css", "core/rfb.js"] {
        assert!(http(address, &format!("{prefix}/{file}"), "")
            .await
            .starts_with("HTTP/1.1 200"));
    }
    let wrong = http(address, &format!("/viewer/{}/vnc.html", "0".repeat(48)), "").await;
    assert!(wrong.starts_with("HTTP/1.1 404"));
    let unauthorized = http(
        address,
        "/v1/responses",
        &format!("Authorization: Bearer {capability}\r\n"),
    )
    .await;
    assert!(unauthorized.starts_with("HTTP/1.1 401"));
    let url = format!("ws://{address}{prefix}/websock");
    let mut bad = url.clone().into_client_request().unwrap();
    bad.headers_mut()
        .insert("origin", "https://unowned.invalid".parse().unwrap());
    let bad = tokio_tungstenite::connect_async(bad)
        .await
        .expect_err("foreign viewer origin must fail");
    assert!(matches!(bad,tokio_tungstenite::tungstenite::Error::Http(ref v) if v.status()==403));
    let accepts = Arc::new(AtomicUsize::new(0));
    let seen = accepts.clone();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = rfb.accept().await.unwrap();
        seen.fetch_add(1, Ordering::SeqCst);
        socket.write_all(b"RFB 003.008\n").await.unwrap();
        let mut input = [0; 10];
        socket.read_exact(&mut input).await.unwrap();
        assert_eq!(&input, b"owned-keys");
        socket.write_all(b"ack").await.unwrap();
        let mut more = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            socket.read_to_end(&mut more),
        )
        .await
        .unwrap()
        .unwrap();
    });
    let mut request = url.into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap().into_data().as_ref(),
        b"RFB 003.008\n"
    );
    ws.send(Message::Binary(b"owned-keys".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap().into_data().as_ref(),
        b"ack"
    );
    stop.cancel();
    task.await.unwrap();
    peer.await.unwrap();
    assert_eq!(accepts.load(Ordering::SeqCst), 1);
    assert_eq!(host.0.load(Ordering::SeqCst), 0);
    assert!(Viewer::new(socket, root.0.join("absent-assets"), address.port()).is_err());
}
