//! Type 1 OpenAI-compatible `/v1/models` and `/v1/responses` HTTP handlers.
//!
//! Orchestrates `catalog`, `session`, `turn`, and `stream` so each user turn
//! dispatches one prompt-box-identical `POST /f/conversation` without
//! `local_function_names` while ChatGPT executes registered remote MCP
//! connectors server-side inside that single turn.

#[cfg(test)]
use crate::catalog::CatalogModel;
pub use crate::catalog::{
    build_codex_model_catalog, fallback_catalog_models, new_live_model_set,
    resolve_dispatch_model_and_effort, set_catalog_parallel_tool_calls, sync_catalog_from_status,
    sync_model_catalog_file, LiveModelSet,
};
pub use crate::session::ResponseThreadMap;

use crate::cdp::lane_channel;
use crate::config::{random_hex, HistoryConfig, McpConfig, ModelConfig};
use crate::driver::DriverBackend;
use crate::error::Result;
use crate::session::{wire_uuid, ThreadLease};
use crate::stream::{
    estimate_tokens, format_sse_frame, send_chunk, send_frames, CancelTurnGuard, MpscSseStream,
    ResponsesEncoder, TurnAssembly, RESPONSES_WAIT_MS, SSE_IDLE, SSE_KEEPALIVE_INTERVAL, SSE_LIMIT,
};
use crate::turn::{
    build_utility_wire_messages, derive_auto_project_name, extract_workspace_cwd,
    is_background_utility_request, TurnPlan, TurnStrategy,
};
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn handle_models(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    catalog_path: Option<&Path>,
    live_models: Option<&LiveModelSet>,
    parallel_tool_calls: bool,
) -> Json<Value> {
    let mut ids: Vec<String> = model_cfg.available.clone();
    if let Ok(st) = driver.status().await {
        if st.get("state").and_then(Value::as_str) == Some("ok") {
            sync_catalog_from_status(
                catalog_path,
                &st,
                model_cfg,
                live_models,
                parallel_tool_calls,
            );
            let live = crate::catalog::catalog_models_from_status(&st);
            if !live.is_empty() {
                let catalog = build_codex_model_catalog(&live, model_cfg, parallel_tool_calls);
                let visible: Vec<String> = catalog["models"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|entry| entry.get("visibility").and_then(Value::as_str) == Some("list"))
                    .filter_map(|entry| entry.get("slug").and_then(Value::as_str))
                    .map(ToOwned::to_owned)
                    .collect();
                if !visible.is_empty() {
                    ids = visible;
                }
            }
        }
    }
    let data: Vec<Value> = ids
        .into_iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "created": 0,
                "owned_by": "chatgpt-desktop"
            })
        })
        .collect();
    Json(json!({ "object": "list", "data": data }))
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub async fn handle_responses(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    history_cfg: &HistoryConfig,
    threads: Arc<ResponseThreadMap>,
    session_hint: Option<String>,
    live_models: Option<&LiveModelSet>,
    body: Value,
) -> Result<Response> {
    handle_responses_with_metadata(
        driver,
        model_cfg,
        history_cfg,
        &McpConfig::default(),
        threads,
        session_hint,
        None,
        live_models,
        body,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_responses_with_metadata(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    history_cfg: &HistoryConfig,
    mcp_cfg: &McpConfig,
    threads: Arc<ResponseThreadMap>,
    session_hint: Option<String>,
    turn_metadata: Option<Value>,
    live_models: Option<&LiveModelSet>,
    body: Value,
) -> Result<Response> {
    let requested_model = body
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(&model_cfg.default)
        .to_owned();
    let requested_effort = body
        .get("reasoning")
        .and_then(|r| r.get("effort"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    let known_slugs = live_models
        .and_then(|lock| lock.read().ok().map(|guard| guard.clone()))
        .unwrap_or_else(|| {
            model_cfg
                .available
                .iter()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        });
    let utility = is_background_utility_request(turn_metadata.as_ref(), &body);
    let (mut model, mut effort) = resolve_dispatch_model_and_effort(
        &requested_model,
        requested_effort.as_deref(),
        &known_slugs,
    );
    if utility {
        if let Some(instant) = known_slugs.iter().find(|s| s.ends_with("-instant")) {
            model = instant.clone();
            effort = None;
        } else if !known_slugs.is_empty()
            && !known_slugs.contains(&model)
            && !matches!(model.as_str(), "auto" | "research")
        {
            model = model_cfg.default.clone();
            effort = None;
        }
    }
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(true);
    let resp_id = format!("resp_{}", random_hex(8)?);
    let lane_id = resp_id.clone();

    let mut encoder = ResponsesEncoder::new(resp_id.clone(), requested_model);
    encoder.input_tokens = estimate_tokens(&json!([body["input"], body["instructions"]]));

    if utility {
        let util_messages = build_utility_wire_messages(&body)?;
        let (util_project_id, temporary) = if let Some(pid) = history_cfg
            .utility_project_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            (Some(pid.to_owned()), false)
        } else if let Some(pname) = history_cfg
            .utility_project_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let resolved = threads
                .resolve_project_by_name(&driver, pname, Some("project_v2"))
                .await;
            let is_temp = resolved.is_none();
            (resolved, is_temp)
        } else {
            (None, true)
        };
        let parent = wire_uuid()?;
        let send_payload = json!({
            "lane_id": lane_id,
            "chat_id": null,
            "project_id": util_project_id,
            "temporary": temporary,
            "prompt": "",
            "model": model,
            "reasoning_effort": effort,
            "native_candidate": {
                "messages": util_messages,
                "parent_message_id": parent
            },
            "wait_ms": RESPONSES_WAIT_MS
        });
        return dispatch_responses_turn(
            driver,
            threads,
            None,
            None,
            None,
            None,
            send_payload,
            lane_id,
            encoder,
            stream,
        )
        .await;
    }

    let mut session_key = session_hint
        .or_else(|| {
            body.get("prompt_cache_key")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty() && s.len() <= 512)
                .map(ToOwned::to_owned)
        })
        .filter(|key| key.len() <= 512)
        .map(|key| format!("session:{key}"));
    if session_key.is_none() {
        if let Some(prev_id) = body.get("previous_response_id").and_then(Value::as_str) {
            session_key = threads
                .current_response_key(prev_id)
                .or_else(|| Some(format!("response:{prev_id}")));
        }
    }
    let lease = session_key
        .as_deref()
        .map(|key| threads.lease(key))
        .transpose()?;

    let mut prior = session_key.as_deref().and_then(|key| threads.get(key));
    let expected_account = prior.as_ref().and_then(|e| e.account_id.clone());
    let cwd = extract_workspace_cwd(&body, turn_metadata.as_ref());
    let active_connectors = if mcp_cfg.bridge_guidance && mcp_cfg.preferred_connectors.is_empty() {
        threads.active_mcp_connectors(&driver).await
    } else {
        Vec::new()
    };

    let plan = TurnPlan::prepare(
        &body,
        prior.as_ref(),
        &resp_id,
        TurnStrategy::McpSingleTurn,
        cwd.as_deref(),
        &active_connectors,
        mcp_cfg,
    )?;
    if plan.fresh_handoff {
        prior = None;
    }

    let effective_project_id = if let Some(entry) = &prior {
        entry.project_id.clone()
    } else if let Some(explicit) = turn_metadata
        .as_ref()
        .and_then(|m| m.get("project_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            history_cfg
                .project_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
    {
        Some(explicit.to_owned())
    } else if let Some(pname) = history_cfg
        .project_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        threads.resolve_project_by_name(&driver, pname, None).await
    } else if history_cfg.auto_project {
        let auto_name = derive_auto_project_name(cwd.as_deref());
        threads
            .resolve_project_by_name(&driver, &auto_name, None)
            .await
    } else {
        None
    };

    let send_payload = json!({
        "lane_id": lane_id,
        "chat_id": prior.as_ref().map(|entry| &entry.chat_id),
        "expected_account_id": expected_account,
        "project_id": effective_project_id,
        "prompt": "",
        "model": model,
        "reasoning_effort": effort,
        "is_continuation": prior.is_some(),
        "native_candidate": {
            "messages": plan.messages,
            "parent_message_id": plan.parent
        },
        "wait_ms": RESPONSES_WAIT_MS
    });

    dispatch_responses_turn(
        driver,
        threads,
        lease,
        session_key,
        Some(plan),
        effective_project_id,
        send_payload,
        lane_id,
        encoder,
        stream,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_responses_turn(
    driver: Arc<dyn DriverBackend>,
    threads: Arc<ResponseThreadMap>,
    lease: Option<ThreadLease>,
    session_key: Option<String>,
    plan: Option<TurnPlan>,
    effective_project_id: Option<String>,
    send_payload: Value,
    lane_id: String,
    mut encoder: ResponsesEncoder,
    stream: bool,
) -> Result<Response> {
    if !stream {
        let _lease = lease;
        let mut cancel_guard = CancelTurnGuard::new(Arc::clone(&driver), lane_id);
        let driver_res = driver.send_message(send_payload).await?;
        let mut assembly = TurnAssembly::default();
        assembly.reconcile_settled(&driver_res)?;
        cancel_guard.disarm();
        let (_, final_resp) = encoder.finish(&mut assembly, &driver_res);
        if let Some(plan) = &plan {
            plan.commit(
                &threads,
                session_key.as_deref(),
                &driver_res,
                &final_resp["output"],
                effective_project_id,
            )?;
        }
        return Ok((StatusCode::OK, Json(final_resp)).into_response());
    }

    let (tx, rx) = mpsc::channel::<String>(32);
    for frame in encoder.start() {
        let _ = tx.try_send(format_sse_frame(&frame));
    }
    tokio::spawn(async move {
        let _lease = lease;
        let mut cancel = CancelTurnGuard::new(Arc::clone(&driver), lane_id);
        let (event_tx, mut event_rx) = lane_channel();
        let mut assembly = TurnAssembly::default();
        let mut turn = driver.stream_message(send_payload, event_tx);
        let mut keepalive = tokio::time::interval_at(
            tokio::time::Instant::now() + SSE_KEEPALIVE_INTERVAL,
            SSE_KEEPALIVE_INTERVAL,
        );
        let idle = tokio::time::sleep(SSE_IDLE);
        let limit = tokio::time::sleep(SSE_LIMIT);
        tokio::pin!(idle, limit);
        let mut event_open = true;
        let outcome = loop {
            tokio::select! {
                biased;
                _ = tx.closed() => {
                    cancel.cancel_now().await;
                    return;
                }
                _ = &mut limit => break Err(crate::error::ProviderError::unavailable(
                    "chatgpt_stream_limit",
                    "response exceeded two hours",
                )),
                _ = &mut idle => break Err(crate::error::ProviderError::unavailable(
                    "chatgpt_stream_idle",
                    "no upstream event within five minutes",
                )),
                event = event_rx.recv(), if event_open => {
                    match event {
                        Some(event) => {
                            idle.as_mut().reset(tokio::time::Instant::now() + SSE_IDLE);
                            assembly.apply_event(event);
                            let frames = encoder.flush_ready(&mut assembly);
                            if !frames.is_empty() && !send_frames(&tx, frames).await {
                                cancel.cancel_now().await;
                                return;
                            }
                        }
                        None => event_open = false,
                    }
                }
                result = &mut turn => break result,
                _ = keepalive.tick() => {
                    if !send_chunk(&tx, ": keepalive\n\n".to_owned()).await {
                        cancel.cancel_now().await;
                        return;
                    }
                }
            }
        };
        let result = (|| -> Result<Vec<Value>> {
            let settled = outcome?;
            while let Ok(event) = event_rx.try_recv() {
                assembly.apply_event(event);
            }
            match assembly.reconcile_settled(&settled) {
                Ok(()) => {
                    cancel.disarm();
                }
                Err(err) => {
                    if err.code != "chatgpt_response_in_progress" {
                        cancel.disarm();
                    }
                    return Err(err);
                }
            }
            let (frames, final_resp) = encoder.finish(&mut assembly, &settled);
            if let Some(plan) = &plan {
                plan.commit(
                    &threads,
                    session_key.as_deref(),
                    &settled,
                    &final_resp["output"],
                    effective_project_id,
                )?;
            }
            Ok(frames)
        })();
        let frames = match result {
            Ok(frames) => frames,
            Err(err) => vec![encoder.failed(err)],
        };
        send_frames(&tx, frames).await;
    });

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/event-stream; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(MpscSseStream { rx }),
    )
        .into_response())
}

use axum::body::Body;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdp::{StreamEvent, StreamItemKind};
    use crate::turn::{format_mcp_bridge_directive, TurnPlan, TurnStrategy};
    use std::collections::HashSet;

    #[test]
    fn model_catalog_sets_1m_context_and_800k_auto_compact() {
        let models = vec![
            CatalogModel {
                slug: "gpt-6-2".into(),
                title: Some("GPT-6.2".into()),
                description: Some("Flagship".into()),
                max_tokens: Some(52_815),
                tags: vec![],
            },
            CatalogModel {
                slug: "gpt-6-2-thinking".into(),
                title: Some("GPT-6.2".into()),
                description: None,
                max_tokens: Some(262_144),
                tags: vec![],
            },
            CatalogModel {
                slug: "gpt-6.2-sol-wm".into(),
                title: Some("GPT-6.2 Sol".into()),
                description: None,
                max_tokens: Some(262_144),
                tags: vec![],
            },
        ];
        let catalog = build_codex_model_catalog(&models, &ModelConfig::default(), false);
        let list = catalog["models"].as_array().unwrap();
        let entry = list.iter().find(|m| m["slug"] == "gpt-6-2").unwrap();
        assert_eq!(entry["visibility"], "list");
        assert_eq!(entry["context_window"], 1_000_000);
        assert_eq!(entry["effective_context_window_percent"], 95);
        assert_eq!(entry["auto_compact_token_limit"], 800_000);
        assert_eq!(entry["supports_parallel_tool_calls"], false);
        assert_eq!(entry["supports_reasoning_summary_parameter"], true);
        let wm = list.iter().find(|m| m["slug"] == "gpt-6.2-sol-wm").unwrap();
        assert_eq!(wm["visibility"], "hide");
    }

    #[test]
    fn resolve_dispatch_routes_effort_variants() {
        let known: HashSet<String> = ["gpt-6-2", "gpt-6-2-instant", "gpt-6-2-thinking"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect();
        assert_eq!(
            resolve_dispatch_model_and_effort("gpt-6-2", Some("high"), &known),
            ("gpt-6-2-thinking".to_owned(), Some("high".to_owned()))
        );
        assert_eq!(
            resolve_dispatch_model_and_effort("gpt-6-2", Some("none"), &known),
            ("gpt-6-2-instant".to_owned(), None)
        );
    }

    #[test]
    fn utility_detection_excludes_compaction_and_normal_turns() {
        let compact = json!({
            "instructions": "Produce a summary of its thinking process and compact the conversation.",
            "input": [{"role": "user", "content": "hello"}]
        });
        assert!(!is_background_utility_request(None, &compact));

        let title = json!({
            "instructions": "Generate a concise, single-line task title.",
            "input": [{"role": "user", "content": "fix bug"}]
        });
        assert!(is_background_utility_request(None, &title));
    }

    #[test]
    fn turn_plan_injects_mcp_bridge_and_hides_codex_preambles() {
        let body = json!({
            "instructions": "You are Codex.",
            "input": [
                {"role": "user", "content": "<environment_context><cwd>/home/u/prj/codex</cwd></environment_context>"},
                {"role": "user", "content": "Run git status"}
            ]
        });
        let cwd = extract_workspace_cwd(&body, None);
        assert_eq!(cwd.as_deref(), Some("/home/u/prj/codex"));
        let active = vec!["TermuxShell".to_owned(), "TermuxChatGPT".to_owned()];
        let plan = TurnPlan::prepare(
            &body,
            None,
            "resp_1",
            TurnStrategy::McpSingleTurn,
            cwd.as_deref(),
            &active,
            &McpConfig::default(),
        )
        .unwrap();
        assert_eq!(plan.messages.len(), 3);
        assert_eq!(plan.messages[0]["hidden"], true);
        assert!(plan.messages[1].get("hidden").is_none());
        let dev = plan.messages[2]["parts"][0].as_str().unwrap();
        assert!(dev.contains("You are Codex."));
        assert!(dev.contains("[Termux MCP Single-Turn Execution Mode]"));
        assert!(dev.contains("TermuxShell, TermuxChatGPT"));
        assert!(dev.contains("/home/u/prj/codex"));
        assert_eq!(plan.messages[2]["one_turn"], true);
    }

    #[test]
    fn turn_plan_rejects_input_image_and_retired_tool_outputs() {
        let img = json!({"input": [{"role": "user", "content": [{"type": "input_image", "image_url": "data:image/png;base64,AA=="}]}]});
        let err = TurnPlan::prepare(
            &img,
            None,
            "resp_img",
            TurnStrategy::McpSingleTurn,
            None,
            &[],
            &McpConfig::default(),
        )
        .err()
        .unwrap();
        assert_eq!(err.code, "native_content_unsupported");

        let tool_out =
            json!({"input": [{"type": "function_call_output", "call_id": "c1", "output": "ok"}]});
        let err = TurnPlan::prepare(
            &tool_out,
            None,
            "resp_tool",
            TurnStrategy::McpSingleTurn,
            None,
            &[],
            &McpConfig::default(),
        )
        .err()
        .unwrap();
        assert_eq!(err.code, "local_tools_retired");
    }

    #[test]
    fn mcp_bridge_directive_respects_disabled_flag_and_preferred_list() {
        let disabled = McpConfig {
            bridge_guidance: false,
            preferred_connectors: vec![],
        };
        assert!(format_mcp_bridge_directive(Some("/tmp"), &[], &disabled).is_none());

        let pref = McpConfig {
            bridge_guidance: true,
            preferred_connectors: vec!["TermuxShell".into(), "TermuxCodex".into()],
        };
        let text = format_mcp_bridge_directive(Some("/repo"), &[], &pref).unwrap();
        assert!(text.contains("TermuxShell, TermuxCodex"));
        assert!(text.contains("/repo"));
    }

    #[test]
    fn stream_encoder_streams_append_only_reasoning_and_answer_with_usage() {
        let mut assembly = TurnAssembly::default();
        let mut enc = ResponsesEncoder::new("resp_s1".into(), "gpt-6-2".into());
        enc.input_tokens = 20;
        let _ = enc.start();

        assembly.apply_event(StreamEvent::ItemUpsert {
            item_id: "r1".into(),
            item_kind: StreamItemKind::ReasoningSummary,
            text: "Running TermuxShell".into(),
            committed: false,
            streamable: true,
        });
        let f1 = enc.flush_ready(&mut assembly);
        assert!(f1
            .iter()
            .any(|f| f["type"] == "response.reasoning_summary_text.delta"));

        assembly.apply_event(StreamEvent::ItemUpsert {
            item_id: "r1".into(),
            item_kind: StreamItemKind::ReasoningSummary,
            text: "Running TermuxShell: git status".into(),
            committed: true,
            streamable: true,
        });
        let f2 = enc.flush_ready(&mut assembly);
        assert!(f2
            .iter()
            .any(|f| f["type"] == "response.reasoning_summary_text.done"));

        assembly
            .reconcile_settled(&json!({
                "state": "settled",
                "items": [{"id": "a1", "kind": "answer", "text": "Clean tree."}],
                "usage": {"input_tokens": 100, "output_tokens": 25}
            }))
            .unwrap();
        let (_, final_resp) = enc.finish(
            &mut assembly,
            &json!({"usage": {"input_tokens": 100, "output_tokens": 25}}),
        );
        assert_eq!(final_resp["usage"]["total_tokens"], 125);
        assert_eq!(final_resp["parallel_tool_calls"], false);
        assert_eq!(final_resp["output"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn thread_store_accepts_legacy_v3_file_with_pending_calls() {
        let dir = std::env::temp_dir().join(format!("cgp-sess-{}", random_hex(6).unwrap()));
        std::fs::create_dir_all(&dir).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = dir.join("response_threads.json");
        let zero_hash = vec![0u8; 32];
        let one_hash = vec![1u8; 32];
        let legacy = json!({
            "schema_version": 3,
            "entries": [[
                "session:s1",
                {
                    "chat_id": "chat-1",
                    "account_id": "acct-1",
                    "project_id": "proj-1",
                    "input_len": 2,
                    "prefix_sha256": zero_hash,
                    "native": {
                        "response_id": "resp_old",
                        "remote_head": "msg_1",
                        "pending_calls": [{"call_id": "c1", "message_id": "m1", "recipient": "local.exec", "is_custom": false, "name": "exec"}],
                        "instructions_sha256": one_hash,
                        "request_input_len": 1,
                        "request_input_sha256": zero_hash
                    }
                }
            ]]
        });
        std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let store = ResponseThreadMap::open(&dir).unwrap();
        let entry = store.get("session:s1").unwrap();
        assert_eq!(entry.chat_id, "chat-1");
        assert_eq!(entry.native.unwrap().remote_head.as_deref(), Some("msg_1"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
