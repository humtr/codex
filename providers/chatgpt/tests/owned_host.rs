//! Real Android app launch without model sampling or account/profile migration.
use codex_chatgpt_provider::{
    config::HostConfig,
    host::OwnedHost,
    journal::{random_id, Journal},
};
use serde_json::json;
struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct ServiceCleanup(std::path::PathBuf);
impl ServiceCleanup {
    fn stop(&self) {
        let key = codex_chatgpt_provider::context::digest(&"termux-codex-responses-service-v1");
        if let Ok(Some(value)) = Journal::inspect(&self.0.join("state"), &key) {
            if let Ok(process) = serde_json::from_value::<codex_chatgpt_provider::cdp::Process>(
                value["process"].clone(),
            ) {
                if process.validate().is_ok() {
                    unsafe {
                        libc::kill(process.pid as i32, libc::SIGTERM);
                    }
                    let until = std::time::Instant::now() + std::time::Duration::from_secs(12);
                    while process.validate().is_ok() && std::time::Instant::now() < until {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    assert!(
                        process.validate().is_err(),
                        "owned candidate must stop before fixture removal"
                    );
                }
            }
        }
    }
}
impl Drop for ServiceCleanup {
    fn drop(&mut self) {
        self.stop();
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CHATGPT_PROVIDER_ROOTFS and Termux Xvnc/proot"]
async fn native_auth_bootstraps_one_rust_service_and_reclaims_endpoint_without_chat() {
    use codex_chatgpt_provider::{config::Config, runtime};
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let root = std::env::temp_dir().join(format!("pa-{}", &random_id().unwrap()[..8]));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let cleanup = ServiceCleanup(root.clone());
    let config = root.join("config/codex/providers");
    std::fs::create_dir_all(&config).unwrap();
    let mut settings = Config::default();
    settings.host.rootfs = Some(
        std::env::var_os("CHATGPT_PROVIDER_ROOTFS")
            .expect("qualified rootfs required")
            .into(),
    );
    settings.host.viewer_assets =
        std::env::var_os("CHATGPT_PROVIDER_VIEWER_ASSETS").map(Into::into);
    let file = config.join("chatgpt.toml");
    std::fs::write(&file, settings.render().unwrap()).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    let authenticate = || {
        let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"));
        cmd.arg("token")
            .arg("--state")
            .arg(root.join("state"))
            .env("HOME", &root)
            .env("XDG_CONFIG_HOME", root.join("config"));
        cmd
    };
    let (first, sibling) = tokio::join!(authenticate().output(), authenticate().output());
    let first = first.unwrap();
    let sibling = sibling.unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        sibling.status.success(),
        "{}",
        String::from_utf8_lossy(&sibling.stderr)
    );
    assert!(
        first.stdout.len() == 49 && first.stdout == sibling.stdout,
        "concurrent auth must reuse one service credential"
    );
    let status = runtime::status(&root.join("state")).await.unwrap();
    let absent = runtime::control(
        &root.join("state"),
        "codex_host",
        json!({"action":"status"}),
    )
    .await
    .unwrap();
    assert_eq!(absent["is_error"], false);
    assert_eq!(absent["result"]["host"]["present"], false);
    assert!(
        !root.join("state/desktop").exists(),
        "authentication must not acquire an app"
    );
    // Acquisition is explicit. A fresh logged-out profile can report unavailable
    // app/auth context, while still retaining the owned host for login/reuse.
    let ensured = runtime::control(
        &root.join("state"),
        "codex_host",
        json!({"action":"ensure"}),
    )
    .await
    .unwrap();
    if ensured["is_error"] == true {
        let cause = ensured["result"]["error"].as_str().unwrap();
        assert!(
            matches!(
                cause,
                "app_scope_unavailable"
                    | "app_auth_loading"
                    | "app_auth_context_unavailable"
                    | "app_target_unavailable"
                    | "app_bindings_unavailable"
            ),
            "unexpected fresh-profile readiness: {cause}"
        );
    } else {
        assert_eq!(ensured["result"]["inference_submitted"], false);
    }
    assert!(root.join("state/desktop/home").is_dir());
    assert_eq!(
        runtime::status(&root.join("state")).await.unwrap()["process"],
        status["process"]
    );
    let viewer_capability = if settings.host.viewer_assets.is_some() {
        use futures_util::StreamExt;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let key = codex_chatgpt_provider::context::digest(&"termux-codex-responses-service-v1");
        let record = Journal::inspect(&root.join("state"), &key)
            .unwrap()
            .unwrap();
        let capability = record["viewer_capability"]
            .as_str()
            .expect("owned viewer must be configured");
        let port = record["port"].as_u64().unwrap();
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port as u16))
            .await
            .unwrap();
        stream.write_all(format!("GET /viewer/{capability}/vnc.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await.unwrap();
        let html = String::from_utf8(bytes).unwrap();
        assert!(html.starts_with("HTTP/1.1 200") && html.contains("owned_keyboard_auto"));
        let mut request = format!("ws://127.0.0.1:{port}/viewer/{capability}/websock")
            .into_client_request()
            .unwrap();
        request.headers_mut().insert(
            "origin",
            format!("http://127.0.0.1:{port}").parse().unwrap(),
        );
        let (mut ws, _) = tokio_tungstenite::connect_async(request)
            .await
            .expect("private viewer handshake");
        let first = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(first.into_data().as_ref(), b"RFB 003.008\n");
        ws.close(None).await.unwrap();
        Some(capability.to_owned())
    } else {
        None
    };
    runtime::stop(&root.join("state")).await.unwrap();
    cleanup.stop();
    assert!(root.join("state/desktop/home").is_dir());
    assert!(
        runtime::status(&root.join("state")).await.is_err(),
        "status must not start a service"
    );
    let next = authenticate().output().await.unwrap();
    assert!(
        next.status.success(),
        "{}",
        String::from_utf8_lossy(&next.stderr)
    );
    assert!(
        first.stdout != next.stdout,
        "restart must rotate credential"
    );
    let restarted = runtime::status(&root.join("state")).await.unwrap();
    assert_eq!(status["base_url"], restarted["base_url"]);
    assert_ne!(status["process"], restarted["process"]);
    if let Some(capability) = viewer_capability {
        let key = codex_chatgpt_provider::context::digest(&"termux-codex-responses-service-v1");
        let record = Journal::inspect(&root.join("state"), &key)
            .unwrap()
            .unwrap();
        assert_ne!(
            record["viewer_capability"].as_str(),
            Some(capability.as_str()),
            "restart must revoke the previous viewer capability"
        );
    }
    runtime::stop(&root.join("state")).await.unwrap();
    cleanup.stop();
    println!(
        "{}",
        json!({"accepted":true,"concurrentAuthOneService":true,"authenticationAcquiredNoHost":true,"explicitHostEnsure":true,"freshProfileReadinessError":ensured["result"]["error"],"sameEndpointAfterRestart":true,"credentialRotated":true,"readonlyStatus":true,"actualPrivateRfbViewer":settings.host.viewer_assets.is_some(),"physicalUiObserved":false,"chatSubmissions":0,"existingProfilesRead":false})
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CHATGPT_PROVIDER_ROOTFS and Termux Xvnc/proot"]
async fn official_app_boots_over_rust_private_pipe_and_shuts_down_without_chat() {
    let root = std::env::temp_dir().join(format!("ph-{}", &random_id().unwrap()[..8]));
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let _cleanup = Cleanup(root.clone());
    let journal = Journal::open(&root).unwrap();
    let config = HostConfig {
        rootfs: Some(
            std::env::var_os("CHATGPT_PROVIDER_ROOTFS")
                .expect("qualified official app rootfs required")
                .into(),
        ),
        ..Default::default()
    };
    let mut host = OwnedHost::start(&config, &root).await.unwrap();
    assert_eq!(
        OwnedHost::start(&config, &root).await.err(),
        Some(codex_chatgpt_provider::protocol::Error("host_profile_busy"))
    );
    let version = host
        .client
        .request("Browser.getVersion", json!({}), None)
        .await
        .unwrap();
    assert!(version["product"]
        .as_str()
        .is_some_and(|v| v.contains("Chrome")));
    let until = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    let app = loop {
        match host.client.app().await {
            Ok(app) => break app,
            Err(_) if tokio::time::Instant::now() < until => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await
            }
            Err(error) => panic!("app target unavailable: {error}"),
        }
    };
    loop {
        if app
            .evaluate("({protocol:location.protocol})")
            .await
            .unwrap()["protocol"]
            == "app:"
        {
            break;
        }
        if tokio::time::Instant::now() >= until {
            host.shutdown().await;
            panic!("app document failed to commit");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    host.shutdown().await;
    assert!(host
        .client
        .request("Browser.getVersion", json!({}), None)
        .await
        .is_err());
    drop(host);
    drop(journal);
    assert!(
        root.join("desktop/home").is_dir(),
        "shutdown must preserve profile/authentication"
    );
    std::fs::remove_dir_all(&root).unwrap();
    println!(
        "{}",
        json!({"accepted":true,"browser":version["product"],"privateRustPipe":true,"appTarget":true,"chatSubmissions":0,"existingProfilesRead":false,"testProfileRemoved":true})
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "explicit owned Android app crash/restart; fresh profile, zero Chat"]
async fn official_app_loss_releases_pipe_and_restarts_preserving_the_owned_profile() {
    use std::os::unix::fs::DirBuilderExt;
    use std::time::Duration;
    let root = std::env::temp_dir().join(format!("gl-{}", &random_id().unwrap()[..8]));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let _cleanup = Cleanup(root.clone());
    let _journal = Journal::open(&root).unwrap();
    let config = HostConfig {
        rootfs: Some(
            std::env::var_os("CHATGPT_PROVIDER_ROOTFS")
                .expect("qualified app rootfs")
                .into(),
        ),
        ..Default::default()
    };
    let mut first = OwnedHost::start(&config, &root).await.unwrap();
    first
        .client
        .request("Browser.getVersion", json!({}), None)
        .await
        .unwrap();
    let sentinel = root.join("desktop/home/owned-retention-sentinel");
    std::fs::write(&sentinel, b"preserve-owned-profile-on-loss").unwrap();
    // Close the actual child browser through its owned private pipe. This is a
    // transport-loss test, not a claim of Android reboot or model cancellation.
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        first.client.request("Browser.close", json!({}), None),
    )
    .await;
    let until = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            first.client.request("Browser.getVersion", json!({}), None),
        )
        .await;
        if result.is_err() || result.unwrap().is_err() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < until,
            "owned pipe failed to observe host loss"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        OwnedHost::start(&config, &root).await.err(),
        Some(codex_chatgpt_provider::protocol::Error("host_profile_busy"))
    );
    first.shutdown().await;
    drop(first);
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"preserve-owned-profile-on-loss"
    );
    let mut second = OwnedHost::start(&config, &root).await.unwrap();
    second
        .client
        .request("Browser.getVersion", json!({}), None)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"preserve-owned-profile-on-loss"
    );
    second.shutdown().await;
    drop(second);
    println!(
        "{}",
        json!({"accepted":true,"actualPrivatePipeLoss":true,"explicitRestart":true,"profilePreserved":true,"competingOwnerRejected":true,"androidReboot":false,"chatSubmissions":0})
    );
}
