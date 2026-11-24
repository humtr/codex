//! Abstract `DriverBackend` trait + real CDP-backed `CdpDriver` executing
//! the clean-room `page_driver.js` inside the owned Linux ChatGPT AppHost.

use crate::cdp::{LaneSender, STREAM_BINDING};
use crate::config::ProviderConfig;
use crate::error::{ProviderError, Result};
use crate::host::OwnedHost;
use crate::viewer::{Viewer, ViewerSlot};
use serde_json::{json, Value};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{watch, Mutex};

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
    fn stream_message(&self, params: Value, _events: LaneSender) -> BoxFuture<'_, Result<Value>> {
        self.send_message(params)
    }
    fn observe_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn stop_generation(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn viewer_info(&self) -> BoxFuture<'_, Result<Value>>;
    fn restart_host(&self) -> BoxFuture<'_, Result<Value>>;
    fn list_connectors(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn register_connector(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
    fn refresh_connector(&self, params: Value) -> BoxFuture<'_, Result<Value>>;
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
    host_running: RwLock<Option<Arc<AtomicBool>>>,
    stopping: watch::Sender<bool>,
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
            host_running: RwLock::new(None),
            stopping: watch::Sender::new(false),
            state: Mutex::new(LiveHostState {
                host: None,
                session_id: None,
            }),
        })
    }

    fn running(&self) -> bool {
        self.host_running.read().is_ok_and(|state| {
            state
                .as_ref()
                .is_some_and(|value| value.load(Ordering::Acquire))
        })
    }

    pub fn begin_shutdown(&self) {
        self.stopping.send_replace(true);
    }

    async fn while_serving<T>(&self, action: impl Future<Output = Result<T>>) -> Result<T> {
        let mut stopping = self.stopping.subscribe();
        tokio::select! {
            biased;
            _ = stopping.wait_for(|&value| value) => Err(ProviderError::unavailable(
                "provider_shutting_down", "provider is shutting down",
            )),
            result = action => result,
        }
    }

    fn set_host(&self, guard: &mut LiveHostState, host: OwnedHost) {
        if let Some(assets) = self.config.resolved_viewer_assets(&self.home) {
            if let Ok(viewer) = Viewer::new(
                host.viewer_socket(),
                assets,
                self.listen_port,
                host.running.clone(),
            ) {
                if let Ok(mut slot) = self.viewer_slot.write() {
                    *slot = Some(viewer);
                }
            }
        }
        if let Ok(mut running) = self.host_running.write() {
            *running = Some(host.running.clone());
        }
        guard.host = Some(host);
        guard.session_id = None;
    }

    async fn stop_host(&self, guard: &mut LiveHostState) -> Result<()> {
        if let Ok(mut slot) = self.viewer_slot.write() {
            *slot = None;
        }
        if let Ok(mut running) = self.host_running.write() {
            *running = None;
        }
        guard.session_id = None;
        let Some(host) = guard.host.as_mut() else {
            return Ok(());
        };
        let result = host.shutdown().await;
        if !result
            .as_ref()
            .is_err_and(|error| error.code == "host_cleanup_pending")
        {
            guard.host.take();
        }
        result
    }

    /// Ensure the owned host & page session are attached under `self.state`, then
    /// release the mutex BEFORE invoking `evaluate_json` so concurrent `/healthz`,
    /// `/control/observe`, `/control/stop`, `/control/status`, and `SIGTERM` shutdown
    /// never block behind a long-running `sendMessage` turn.
    async fn ensure_ready_and_call(
        &self,
        method: &str,
        arg: Value,
        timeout: Duration,
    ) -> Result<Value> {
        self.while_serving(self.call_host(method, arg, timeout, None))
            .await
    }

    async fn call_host(
        &self,
        method: &str,
        arg: Value,
        timeout: Duration,
        events: Option<LaneSender>,
    ) -> Result<Value> {
        let (client, sid) = {
            let mut guard = self.state.lock().await;
            if guard.host.is_some() && !self.running() {
                self.stop_host(&mut guard).await?;
            }
            if guard.host.is_none() {
                let rootfs = self.config.resolved_rootfs(&self.home);
                let profile = self.config.profile_dir(&self.home);
                let host = OwnedHost::start(&rootfs, &profile).await?;
                self.set_host(&mut guard, host);
            }
            let client = guard.host.as_ref().unwrap().client.clone();
            let (sid, freshly_attached) = match &guard.session_id {
                Some(s) => (s.clone(), false),
                None => {
                    let s = client.attach_chatgpt_page().await?;
                    client
                        .request("Runtime.enable", json!({}), Some(s.clone()))
                        .await?;
                    client
                        .request(
                            "Runtime.addBinding",
                            json!({"name": STREAM_BINDING}),
                            Some(s.clone()),
                        )
                        .await?;
                    guard.session_id = Some(s.clone());
                    (s, true)
                }
            };
            if freshly_attached {
                // Wait for the initial `about:blank` -> `app://-/index.html` document commit
                // before injecting `page_driver.js`, avoiding transient
                // "Execution context was destroyed" errors during cold boot without ever
                // replaying a mutation.
                let ready_deadline = tokio::time::Instant::now() + Duration::from_secs(15);
                while tokio::time::Instant::now() < ready_deadline {
                    match client
                        .evaluate_json(
                            &sid,
                            "(document.readyState === 'complete' && location.protocol !== 'about:')",
                            Duration::from_secs(3),
                        )
                        .await
                    {
                        Ok(Value::Bool(true)) => break,
                        Ok(_) => {}
                        Err(e) if e.code == "cdp_disconnected" => {
                            let _ = self.stop_host(&mut guard).await;
                            return Err(e);
                        }
                        Err(_) => {}
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
            (client, sid)
        };

        let _route = if let Some(events) = events {
            let lane = arg["lane_id"]
                .as_str()
                .ok_or_else(|| ProviderError::internal("cdp_stream_lane", "missing stream lane"))?;
            Some(client.route_stream(&sid, lane, events)?)
        } else {
            None
        };
        let arg_json = serde_json::to_string(&arg)
            .map_err(|e| ProviderError::internal("driver_encode_failed", e.to_string()))?;
        // Ensure page_driver.js is installed and invoke the requested method atomically
        // in one evaluation so Electron page reloads cannot drop the global between calls.
        let expr = format!(
            "(async () => {{\
                if (!globalThis.__termuxChatGptDriver) {{\
                    {PAGE_DRIVER_JS};\
                }}\
                return await globalThis.__termuxChatGptDriver.{method}({arg_json});\
            }})()"
        );
        let result = client.evaluate_json(&sid, &expr, timeout).await;
        match result {
            Ok(val) => Ok(val),
            Err(err) => {
                // Re-acquire lock only for cleanup and verify matching host generation so a
                // stale failure can never tear down a newly restarted host.
                let mut guard = self.state.lock().await;
                let same_generation = guard
                    .host
                    .as_ref()
                    .is_some_and(|h| Arc::ptr_eq(&h.client.generation, &client.generation));
                if same_generation {
                    if err.code == "cdp_disconnected" {
                        let _ = self.stop_host(&mut guard).await;
                    } else if err.code != "cdp_timeout"
                        && guard.session_id.as_deref() == Some(sid.as_str())
                    {
                        guard.session_id = None;
                    }
                }
                Err(err)
            }
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.begin_shutdown();
        let mut guard = self.state.lock().await;
        self.stop_host(&mut guard).await
    }
}

impl DriverBackend for CdpDriver {
    fn host_running(&self) -> BoxFuture<'_, bool> {
        Box::pin(async move { self.running() })
    }

    fn status(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let running_meta = {
                let guard = self.state.lock().await;
                guard.host.as_ref().filter(|_| self.running()).map(|h| {
                    (
                        h.rootfs.display().to_string(),
                        h.client.generation.to_string(),
                    )
                })
            };
            let Some((rootfs_str, host_gen)) = running_meta else {
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
            };
            let mut st = self
                .ensure_ready_and_call("status", json!({}), Duration::from_secs(30))
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
                .min(300_000);
            self.ensure_ready_and_call(
                "sendMessage",
                params,
                Duration::from_millis(wait_ms + 45_000),
            )
            .await
        })
    }

    fn stream_message(
        &self,
        mut params: Value,
        events: LaneSender,
    ) -> BoxFuture<'_, Result<Value>> {
        params["push_events"] = json!(true);
        Box::pin(self.while_serving(self.call_host(
            "sendMessage",
            params,
            Duration::from_secs(2 * 60 * 60 + 30),
            Some(events),
        )))
    }

    fn observe_chat(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("observeChat", params, Duration::from_secs(20))
                .await
        })
    }

    fn stop_generation(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("stopGeneration", params, Duration::from_secs(15))
                .await
        })
    }

    fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let url = self
                .viewer_slot
                .read()
                .ok()
                .and_then(|g| g.as_ref().filter(|v| v.running()).map(|v| v.url()));
            Ok(json!({
                "state": "ok",
                "available": url.is_some(),
                "url": url
            }))
        })
    }

    fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
        Box::pin(self.while_serving(async {
            let mut guard = self.state.lock().await;
            self.stop_host(&mut guard).await?;
            let rootfs = self.config.resolved_rootfs(&self.home);
            let profile = self.config.profile_dir(&self.home);
            let host = OwnedHost::start(&rootfs, &profile).await?;
            let display = host.display;
            self.set_host(&mut guard, host);
            Ok(json!({
                "state": "ok",
                "restarted": true,
                "display": display
            }))
        }))
    }

    fn list_connectors(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("listConnectors", params, Duration::from_secs(45))
                .await
        })
    }

    fn register_connector(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("registerConnector", params, Duration::from_secs(90))
                .await
        })
    }

    fn refresh_connector(&self, params: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            self.ensure_ready_and_call("refreshConnector", params, Duration::from_secs(90))
                .await
        })
    }
}
