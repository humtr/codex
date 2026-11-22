//! Abstract `DriverBackend` trait + real CDP-backed `CdpDriver` executing
//! the clean-room `page_driver.js` inside the owned Linux ChatGPT AppHost.

use crate::config::ProviderConfig;
use crate::error::{ProviderError, Result};
use crate::host::OwnedHost;
use crate::viewer::{Viewer, ViewerSlot};
use serde_json::{json, Value};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub const PAGE_DRIVER_JS: &str = include_str!("page_driver.js");

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait DriverBackend: Send + Sync {
    fn host_running(&self) -> BoxFuture<'_, bool>;
    fn status(&self) -> BoxFuture<'_, Result<Value>>;
    fn list_chats(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn read_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn rename_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn list_projects(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn read_project(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn create_project(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn send_message(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn observe_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn stop_generation(&self) -> BoxFuture<'_, Result<Value>>;
    fn viewer_info(&self) -> BoxFuture<'_, Result<Value>>;
    fn restart_host(&self) -> BoxFuture<'_, Result<Value>>;
}

struct LiveHostState {
    host: Option<OwnedHost>,
    session_id: Option<String>,
}

pub struct CdpDriver {
    config: ProviderConfig,
    home: PathBuf,
    listen_port: u16,
    viewer_slot: ViewerSlot,
    state: Mutex<LiveHostState>,
}

impl CdpDriver {
    pub fn new(
        config: ProviderConfig,
        home: PathBuf,
        listen_port: u16,
        viewer_slot: ViewerSlot,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            home,
            listen_port,
            viewer_slot,
            state: Mutex::new(LiveHostState {
                host: None,
                session_id: None,
            }),
        })
    }

    async fn ensure_ready_and_call(
        &self,
        method: &str,
        arg: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let mut guard = self.state.lock().await;
        if guard.host.is_none() {
            let rootfs = self.config.resolved_rootfs(&self.home);
            let profile = self.config.profile_dir(&self.home);
            let host = OwnedHost::start(&rootfs, &profile).await?;
            if let Some(ref assets) = self.config.host.viewer_assets {
                if let Ok(viewer) =
                    Viewer::new(host.viewer_socket(), assets.clone(), self.listen_port)
                {
                    if let Ok(mut slot) = self.viewer_slot.write() {
                        *slot = Some(viewer);
                    }
                }
            }
            guard.host = Some(host);
            guard.session_id = None;
        }
        let client = guard.host.as_ref().unwrap().client.clone();
        let sid = match &guard.session_id {
            Some(s) => s.clone(),
            None => {
                let s = client.attach_chatgpt_page().await?;
                guard.session_id = Some(s.clone());
                s
            }
        };
        let arg_json = serde_json::to_string(&arg)
            .map_err(|e| ProviderError::internal("driver_encode_failed", e.to_string()))?;
        // Ensure page_driver.js is installed and invoke the requested method atomically
        // in one evaluation so Electron page reloads cannot drop the global between calls.
        let expr = format!(
            "(async () => {{\
                if (!globalThis.__termuxChatGptDriver || globalThis.__termuxChatGptDriver.version !== 1) {{\
                    {PAGE_DRIVER_JS};\
                }}\
                return await globalThis.__termuxChatGptDriver.{method}({arg_json});\
            }})()"
        );
        let result = client.evaluate_json(&sid, &expr, timeout).await;
        match result {
            Ok(val) => Ok(val),
            Err(err) => {
                guard.session_id = None;
                Err(err)
            }
        }
    }
}

impl DriverBackend for CdpDriver {
    fn host_running(&self) -> BoxFuture<'_, bool> {
        Box::pin(async move { self.state.lock().await.host.is_some() })
    }

    fn status(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let guard = self.state.lock().await;
            if guard.host.is_none() {
                let rootfs = self.config.resolved_rootfs(&self.home);
                let profile = self.config.profile_dir(&self.home);
                return Ok(json!({
                    "state": "ok",
                    "host_running": false,
                    "rootfs_ready": rootfs.join("usr/bin/chatgpt").is_file(),
                    "rootfs": rootfs.display().to_string(),
                    "profile_dir": profile.display().to_string(),
                    "auth_state": "host_stopped",
                    "models": self.config.model.available,
                    "default_model": self.config.model.default,
                }));
            }
            drop(guard);
            let (rootfs_str, host_gen) = {
                let g = self.state.lock().await;
                match g.host.as_ref() {
                    Some(h) => (
                        h.rootfs.display().to_string(),
                        h.client.generation.to_string(),
                    ),
                    None => (String::new(), String::new()),
                }
            };
            let mut st = self
                .ensure_ready_and_call("status", json!({}), Duration::from_secs(20))
                .await?;
            st["host_running"] = json!(true);
            st["rootfs"] = json!(rootfs_str);
            st["host_generation"] = json!(host_gen);
            st["default_model"] = json!(self.config.model.default);
            Ok(st)
        })
    }

    fn list_chats(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("listChats", params, Duration::from_secs(20))
                .await
        })
    }

    fn read_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("readChat", params, Duration::from_secs(20))
                .await
        })
    }

    fn rename_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("renameChat", params, Duration::from_secs(20))
                .await
        })
    }

    fn list_projects(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("listProjects", params, Duration::from_secs(20))
                .await
        })
    }

    fn read_project(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("readProject", params, Duration::from_secs(20))
                .await
        })
    }

    fn create_project(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("createProject", params, Duration::from_secs(20))
                .await
        })
    }

    fn send_message(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let wait_ms = params
                .get("wait_ms")
                .and_then(Value::as_u64)
                .unwrap_or(30_000)
                .min(120_000);
            self.ensure_ready_and_call(
                "sendMessage",
                params,
                Duration::from_millis(wait_ms + 15_000),
            )
            .await
        })
    }

    fn observe_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("observeChat", params, Duration::from_secs(20))
                .await
        })
    }

    fn stop_generation(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("stopGeneration", json!({}), Duration::from_secs(15))
                .await
        })
    }

    fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let url = self
                .viewer_slot
                .read()
                .ok()
                .and_then(|g| g.as_ref().map(|v| v.url()));
            Ok(json!({
                "state": "ok",
                "available": url.is_some(),
                "url": url
            }))
        })
    }

    fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let mut guard = self.state.lock().await;
            if let Ok(mut slot) = self.viewer_slot.write() {
                *slot = None;
            }
            if let Some(mut old) = guard.host.take() {
                old.shutdown().await;
            }
            guard.session_id = None;
            let rootfs = self.config.resolved_rootfs(&self.home);
            let profile = self.config.profile_dir(&self.home);
            let host = OwnedHost::start(&rootfs, &profile).await?;
            let display = host.display;
            if let Some(ref assets) = self.config.host.viewer_assets {
                if let Ok(viewer) =
                    Viewer::new(host.viewer_socket(), assets.clone(), self.listen_port)
                {
                    if let Ok(mut slot) = self.viewer_slot.write() {
                        *slot = Some(viewer);
                    }
                }
            }
            guard.host = Some(host);
            Ok(json!({
                "state": "ok",
                "restarted": true,
                "display": display
            }))
        })
    }
}
