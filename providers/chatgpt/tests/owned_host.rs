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
    cleanup.stop();
    println!(
        "{}",
        json!({"accepted":true,"concurrentAuthOneService":true,"sameEndpointAfterRestart":true,"credentialRotated":true,"readonlyStatus":true,"chatSubmissions":0,"existingProfilesRead":false})
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
