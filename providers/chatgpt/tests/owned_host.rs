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
