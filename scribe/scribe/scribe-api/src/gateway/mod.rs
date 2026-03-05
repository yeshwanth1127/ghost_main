//! Ghost gateway WebSocket (agent mode): connect handshake, chat.history, chat.send, chat.inject.
//! Same protocol as ghost-gateway; runs on the main server at GET /gateway.
//!
//! Role and permission naming aligned with Moltbot:
//! - Connect: params.role at top level, "operator" (desktop/UI) | "node" (channels). See moltbot gateway message-handler.
//! - Permission: event "tool.permission.requested", method "tool.permission.reply" (params: ticketId, granted). See moltbot permissions/manager, server-methods/tool-permission.

mod ollama;
mod transcript;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::Config;
use crate::gateway_state::GatewayState;
use crate::services::AppState;
use ollama::run_ollama_stream;
use transcript::{
    append_assistant_message, append_user_message, ensure_transcript_file, read_session_messages,
    resolve_transcript_path,
};

const PROTOCOL_VERSION: i64 = 1;
#[derive(Debug, Deserialize)]
struct RequestFrame {
    #[serde(rename = "type")]
    typ: String,
    id: String,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct ResponseFrame {
    #[serde(rename = "type")]
    typ: String,
    id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectParams {
    #[serde(rename = "minProtocol")]
    min_protocol: Option<i64>,
    #[serde(rename = "maxProtocol")]
    max_protocol: Option<i64>,
    client: Option<ConnectClient>,
    /// Moltbot-aligned: "operator" (desktop/UI) or "node" (channels). Default "node" for backward compat.
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectClient {
    id: Option<String>,
    version: Option<String>,
    platform: Option<String>,
}

async fn send_response(ws: &mut WebSocket, id: &str, ok: bool, payload: Option<Value>, err: Option<&str>) {
    let frame = ResponseFrame {
        typ: "res".to_string(),
        id: id.to_string(),
        ok,
        payload,
        error: err.map(String::from),
    };
    if let Ok(text) = serde_json::to_string(&frame) {
        let _ = ws.send(Message::Text(text)).await;
    }
}

/// GET /gateway/ping — returns 200 if this server has the gateway (for debugging).
pub async fn gateway_ping() -> impl IntoResponse {
    tracing::info!("[ghost-gateway] GET /gateway/ping");
    Json(json!({ "ok": true, "gateway": "ready" }))
}

pub async fn gateway_ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::info!("[ghost-gateway] WebSocket upgrade /gateway");
    let config = state.config.clone();
    let gateway_state = state.gateway_state.clone();
    ws.on_upgrade(move |socket| handle_gateway_socket(socket, config, gateway_state))
}

async fn handle_gateway_socket(mut socket: WebSocket, config: Config, gateway_state: GatewayState) {
    let conn_id = Uuid::new_v4().to_string();
    tracing::debug!("[ghost-gateway] WS connection conn_id={}", conn_id);

    let (stream_tx, mut stream_rx) = mpsc::channel::<String>(64);
    let mut connected = false;
    // Moltbot-aligned: "operator" = desktop, "node" = channels. Used for register/unregister and method routing.
    let mut conn_role: Option<String> = None;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let msg = match msg {
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) | None => break,
                    _ => continue,
                };

        let parsed: Value = match serde_json::from_str(&msg) {
            Ok(v) => v,
            Err(_) => {
                send_response(
                    &mut socket,
                    "",
                    false,
                    None,
                    Some("invalid JSON"),
                ).await;
                break;
            }
        };

        let req: RequestFrame = match serde_json::from_value::<RequestFrame>(parsed.clone()) {
            Ok(r) if r.typ == "req" && !r.id.is_empty() && !r.method.is_empty() => r,
            _ => {
                let id = parsed.get("id").and_then(|i| i.as_str()).map(String::from).unwrap_or_default();
                send_response(
                    &mut socket,
                    &id,
                    false,
                    None,
                    Some("first frame must be request"),
                ).await;
                break;
            }
        };

        if !connected {
            if req.method != "connect" {
                send_response(&mut socket, &req.id, false, None, Some("first request must be connect")).await;
                break;
            }
            let params: ConnectParams = match req.params.and_then(|p| serde_json::from_value(p).ok()) {
                Some(p) => p,
                None => {
                    send_response(&mut socket, &req.id, false, None, Some("invalid connect params")).await;
                    break;
                }
            };
            let min_p = params.min_protocol.unwrap_or(1);
            let max_p = params.max_protocol.unwrap_or(1);
            if max_p < 1 || min_p > 1 {
                send_response(&mut socket, &req.id, false, None, Some("protocol mismatch")).await;
                break;
            }
            let protocol = PROTOCOL_VERSION.min(max_p.max(min_p));
            let hello = serde_json::json!({
                "type": "hello-ok",
                "protocol": protocol,
                "server": {
                    "version": "1.0.0",
                    "connId": conn_id,
                    "host": config.host
                },
                "snapshot": {
                    "presence": [],
                    "health": { "ok": true, "ts": chrono::Utc::now().timestamp_millis() }
                }
            });
            send_response(&mut socket, &req.id, true, Some(hello), None).await;
            connected = true;
            let role_raw = params.role.as_deref().unwrap_or("node");
            let role = if role_raw == "operator" || role_raw == "node" {
                role_raw.to_string()
            } else {
                "node".to_string()
            };
            conn_role = Some(role.clone());
            if role == "operator" {
                gateway_state.register_desktop(stream_tx.clone()).await;
                tracing::debug!("[ghost-gateway] registered as operator (desktop) conn_id={}", conn_id);
            }
            tracing::debug!("[ghost-gateway] connect ok conn_id={} role={}", conn_id, role);
            continue;
        }

        if req.method == "connect" {
            send_response(&mut socket, &req.id, false, None, Some("connect is only valid as the first request")).await;
            continue;
        }

        let params = req.params.unwrap_or(Value::Object(serde_json::Map::new()));
        let params_obj = params.as_object().cloned().unwrap_or_default();

        match req.method.as_str() {
            "chat.history" => {
                let session_key: String = match params_obj
                    .get("sessionKey")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                {
                    Some(k) => k,
                    None => {
                        send_response(&mut socket, &req.id, false, None, Some("sessionKey required")).await;
                        continue;
                    }
                };
                let limit = params_obj
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200) as u32;
                let path = resolve_transcript_path(&session_key);
                let messages = read_session_messages(&path, limit).await;
                tracing::debug!("[ghost-gateway] chat.history session_key={} limit={} message_count={}", session_key, limit, messages.len());
                let payload = serde_json::json!({ "sessionKey": session_key, "messages": messages });
                send_response(&mut socket, &req.id, true, Some(payload), None).await;
            }
            "chat.send" => {
                let session_key_opt = params_obj.get("sessionKey").and_then(|v| v.as_str()).map(String::from);
                let message_opt = params_obj.get("message").and_then(|v| v.as_str()).map(String::from);
                let (session_key, message): (String, String) = match (session_key_opt, message_opt) {
                    (Some(k), Some(m)) => (k, m),
                    _ => {
                        send_response(&mut socket, &req.id, false, None, Some("sessionKey and message required")).await;
                        continue;
                    }
                };
                let message = message.trim();
                if message.is_empty() {
                    send_response(&mut socket, &req.id, false, None, Some("message required")).await;
                    continue;
                }
                let path = resolve_transcript_path(&session_key);
                let session_id: String = session_key
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
                    .take(200)
                    .collect();
                let session_id = if session_id.is_empty() { "default".to_string() } else { session_id };
                if let Err(e) = ensure_transcript_file(&path, &session_id).await {
                    tracing::debug!("[ghost-gateway] chat.send ensure failed session_key={} error={}", session_key, e);
                    send_response(&mut socket, &req.id, false, None, Some("failed to ensure transcript")).await;
                    continue;
                }
                let message_id = match append_user_message(&path, &session_id, message, true).await {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::debug!("[ghost-gateway] chat.send append failed session_key={} error={}", session_key, e);
                        send_response(&mut socket, &req.id, false, None, Some("failed to append message")).await;
                        continue;
                    }
                };
                let idempotency_key = params_obj.get("idempotencyKey").and_then(|v| v.as_str());
                let run_id = idempotency_key.unwrap_or(&message_id).to_string();
                tracing::debug!("[ghost-gateway] chat.send ok session_key={} message_id={} run_id={}", session_key, message_id, run_id);
                let payload = serde_json::json!({ "runId": run_id, "status": "started" });
                send_response(&mut socket, &req.id, true, Some(payload), None).await;
                let path_clone = path.clone();
                let stream_tx_clone = stream_tx.clone();
                let ollama_url = config.ollama_url.clone();
                let ollama_model = config.ollama_model.clone();
                tokio::spawn(async move {
                    run_ollama_stream(path_clone, session_id, run_id, ollama_url, ollama_model, stream_tx_clone).await;
                });
            }
            "chat.inject" => {
                let session_key_opt = params_obj.get("sessionKey").and_then(|v| v.as_str()).map(String::from);
                let message_opt = params_obj.get("message").and_then(|v| v.as_str()).map(String::from);
                let (session_key, message): (String, String) = match (session_key_opt, message_opt) {
                    (Some(k), Some(m)) => (k, m),
                    _ => {
                        send_response(&mut socket, &req.id, false, None, Some("sessionKey and message required")).await;
                        continue;
                    }
                };
                let label = params_obj.get("label").and_then(|v| v.as_str());
                let path = resolve_transcript_path(&session_key);
                let session_id: String = session_key
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
                    .take(200)
                    .collect();
                let session_id = if session_id.is_empty() { "default".to_string() } else { session_id };
                let message_id = match append_assistant_message(&path, &session_id, message.trim(), label, true).await {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::debug!("[ghost-gateway] chat.inject append failed session_key={} error={}", session_key, e);
                        send_response(&mut socket, &req.id, false, None, Some("failed to inject message")).await;
                        continue;
                    }
                };
                tracing::debug!("[ghost-gateway] chat.inject ok session_key={} message_id={} label={:?}", session_key, message_id, label);
                let payload = serde_json::json!({ "ok": true, "messageId": message_id });
                send_response(&mut socket, &req.id, true, Some(payload), None).await;
            }
            "agent.run" => {
                if conn_role.as_deref() != Some("node") {
                    tracing::debug!("[ghost-gateway] agent.run rejected (role not node) conn_id={}", conn_id);
                    send_response(&mut socket, &req.id, false, None, Some("agent.run only from node (channels)")).await;
                    continue;
                }
                let request_id = params_obj.get("requestId").and_then(|v| v.as_str()).map(String::from);
                let session_key = params_obj.get("sessionKey").and_then(|v| v.as_str()).map(String::from);
                let message = params_obj.get("message").and_then(|v| v.as_str()).map(String::from);
                let (request_id, session_key, message) = match (request_id, session_key, message) {
                    (Some(a), Some(b), Some(c)) if !a.is_empty() && !c.trim().is_empty() => (a, b, c),
                    _ => {
                        send_response(&mut socket, &req.id, false, None, Some("agent.run requires requestId, sessionKey, message")).await;
                        continue;
                    }
                };
                tracing::info!("[ghost-gateway] agent.run request_id={} session_key={} registered and forwarding to desktop", request_id, session_key);
                gateway_state.register_agent_request(&request_id, stream_tx.clone()).await;
                if let Some(desktop_tx) = gateway_state.get_desktop_tx().await {
                    let push = json!({
                        "type": "agent.run.request",
                        "requestId": request_id,
                        "sessionKey": session_key,
                        "message": message.trim()
                    });
                    if desktop_tx.send(push.to_string()).await.is_err() {
                        tracing::warn!("[ghost-gateway] agent.run failed to send to desktop (connection lost)");
                        let _ = stream_tx.send(json!({
                            "type": "agent.run.result",
                            "requestId": request_id,
                            "success": false,
                            "error": "Desktop connection lost"
                        }).to_string()).await;
                    } else {
                        tracing::info!("[ghost-gateway] agent.run.request sent to desktop request_id={}", request_id);
                    }
                } else {
                    tracing::warn!("[ghost-gateway] agent.run no desktop connected, sending error to channel request_id={}", request_id);
                    let res_push = json!({
                        "type": "agent.run.result",
                        "requestId": request_id,
                        "success": false,
                        "error": "No desktop connected"
                    });
                    let _ = stream_tx.send(res_push.to_string()).await;
                }
                send_response(&mut socket, &req.id, true, Some(json!({ "requestId": request_id })), None).await;
            }
            "agent.run.result" => {
                let rid = params_obj.get("requestId").and_then(|v| v.as_str()).unwrap_or("?");
                tracing::info!("[ghost-gateway] agent.run.result received request_id={} from desktop", rid);
                if conn_role.as_deref() != Some("operator") {
                    send_response(&mut socket, &req.id, false, None, Some("agent.run.result only from operator (desktop)")).await;
                    continue;
                }
                let request_id = params_obj.get("requestId").and_then(|v| v.as_str()).map(String::from);
                let request_id = match request_id {
                    Some(s) if !s.is_empty() => s,
                    _ => {
                        send_response(&mut socket, &req.id, false, None, Some("agent.run.result requires requestId")).await;
                        continue;
                    }
                };
                let success = params_obj.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                let summary = params_obj.get("summary").and_then(|v| v.as_str()).map(String::from);
                let error = params_obj.get("error").and_then(|v| v.as_str()).map(String::from);
                if let Some(channels_tx) = gateway_state.take_channels_for_agent_result(&request_id).await {
                    let push = json!({
                        "type": "agent.run.result",
                        "requestId": request_id,
                        "success": success,
                        "summary": summary,
                        "error": error
                    });
                    if channels_tx.send(push.to_string()).await.is_err() {
                        tracing::warn!("[ghost-gateway] agent.run.result failed to send to channels");
                    }
                } else {
                    tracing::warn!("[ghost-gateway] agent.run.result no channels connection for request_id={}", request_id);
                }
                send_response(&mut socket, &req.id, true, None, None).await;
            }
            "tool.permission.requested" => {
                if conn_role.as_deref() != Some("operator") {
                    tracing::warn!("[ghost-gateway] tool.permission.requested rejected: not from operator");
                    send_response(&mut socket, &req.id, false, None, Some("tool.permission.requested only from operator (desktop)")).await;
                    continue;
                }
                let request_id = params_obj.get("requestId").and_then(|v| v.as_str()).map(String::from);
                let run_id = params_obj.get("runId").and_then(|v| v.as_str()).map(String::from);
                let ticket_id = params_obj.get("ticketId").and_then(|v| v.as_str()).map(String::from);
                tracing::info!(
                    "[ghost-gateway] tool.permission.requested received from desktop request_id={:?} run_id={:?} ticket_id={:?}",
                    request_id, run_id, ticket_id
                );
                let (request_id, run_id, ticket_id) = match (request_id, run_id, ticket_id) {
                    (Some(a), Some(b), Some(c)) if !a.is_empty() && !b.is_empty() && !c.is_empty() => (a, b, c),
                    _ => {
                        tracing::warn!("[ghost-gateway] tool.permission.requested missing/invalid params");
                        send_response(&mut socket, &req.id, false, None, Some("tool.permission.requested requires requestId, runId, ticketId")).await;
                        continue;
                    }
                };
                gateway_state.store_permission_run_id(ticket_id.clone(), run_id.clone()).await;
                if let Some(channels_tx) = gateway_state.get_channels_tx_for_agent_request(&request_id).await {
                    let push = json!({
                        "type": "tool.permission.requested",
                        "requestId": request_id,
                        "runId": run_id,
                        "ticketId": ticket_id,
                        "humanReadable": params_obj.get("humanReadable").and_then(|v| v.as_str()),
                        "riskLevel": params_obj.get("riskLevel").and_then(|v| v.as_str()),
                        "irreversible": params_obj.get("irreversible").and_then(|v| v.as_bool()),
                        "riskFactors": params_obj.get("riskFactors")
                    });
                    if channels_tx.send(push.to_string()).await.is_err() {
                        tracing::warn!("[ghost-gateway] tool.permission.requested failed to send to channel request_id={}", request_id);
                    } else {
                        tracing::info!("[ghost-gateway] tool.permission.requested forwarded to channel request_id={} ticket_id={}", request_id, ticket_id);
                    }
                } else {
                    tracing::warn!("[ghost-gateway] tool.permission.requested no channel for request_id={} (run started from desktop UI?)", request_id);
                }
                send_response(&mut socket, &req.id, true, None, None).await;
            }
            "tool.input.requested" => {
                if conn_role.as_deref() != Some("operator") {
                    tracing::warn!("[ghost-gateway] tool.input.requested rejected: not from operator");
                    send_response(&mut socket, &req.id, false, None, Some("tool.input.requested only from operator (desktop)")).await;
                    continue;
                }
                let request_id = params_obj.get("requestId").and_then(|v| v.as_str()).map(String::from);
                let run_id = params_obj.get("runId").and_then(|v| v.as_str()).map(String::from);
                let input_request_id = params_obj.get("inputRequestId").and_then(|v| v.as_str()).map(String::from);
                let (request_id, run_id, input_request_id) = match (request_id, run_id, input_request_id) {
                    (Some(a), Some(b), Some(c)) if !a.is_empty() && !b.is_empty() && !c.is_empty() => (a, b, c),
                    _ => {
                        tracing::warn!("[ghost-gateway] tool.input.requested missing/invalid params");
                        send_response(&mut socket, &req.id, false, None, Some("tool.input.requested requires requestId, runId, inputRequestId")).await;
                        continue;
                    }
                };
                gateway_state.store_input_run_id(input_request_id.clone(), run_id.clone()).await;
                if let Some(channels_tx) = gateway_state.get_channels_tx_for_agent_request(&request_id).await {
                    let push = json!({
                        "type": "tool.input.requested",
                        "requestId": request_id,
                        "runId": run_id,
                        "inputRequestId": input_request_id,
                        "missingFields": params_obj.get("missingFields"),
                        "schema": params_obj.get("schema"),
                        "currentInputs": params_obj.get("currentInputs"),
                        "humanReadable": params_obj.get("humanReadable").and_then(|v| v.as_str())
                    });
                    if channels_tx.send(push.to_string()).await.is_err() {
                        tracing::warn!("[ghost-gateway] tool.input.requested failed to send to channel request_id={}", request_id);
                    } else {
                        tracing::info!("[ghost-gateway] tool.input.requested forwarded to channel request_id={} input_request_id={}", request_id, input_request_id);
                    }
                } else {
                    tracing::warn!("[ghost-gateway] tool.input.requested no channel for request_id={}", request_id);
                }
                send_response(&mut socket, &req.id, true, None, None).await;
            }
            "tool.permission.reply" => {
                if conn_role.as_deref() != Some("node") {
                    send_response(&mut socket, &req.id, false, None, Some("tool.permission.reply only from node (channels)")).await;
                    continue;
                }
                let ticket_id = params_obj
                    .get("ticketId")
                    .or_else(|| params_obj.get("ticket_id"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let granted = params_obj.get("granted").and_then(|v| v.as_bool()).unwrap_or(false);
                let ticket_id = match ticket_id {
                    Some(s) if !s.is_empty() => s,
                    _ => {
                        tracing::warn!("[ghost-gateway] tool.permission.reply missing ticketId");
                        send_response(&mut socket, &req.id, false, None, Some("tool.permission.reply requires ticketId")).await;
                        continue;
                    }
                };
                let run_id_opt = gateway_state.take_run_id_for_permission(&ticket_id).await;
                let desktop_tx_opt = gateway_state.get_desktop_tx().await;
                let has_run_id = run_id_opt.is_some();
                let has_desktop = desktop_tx_opt.is_some();
                if let (Some(run_id), Some(desktop_tx)) = (run_id_opt, desktop_tx_opt) {
                    let push = json!({
                        "type": "tool.permission.reply",
                        "runId": run_id,
                        "ticketId": ticket_id,
                        "granted": granted
                    });
                    if desktop_tx.send(push.to_string()).await.is_err() {
                        tracing::warn!("[ghost-gateway] tool.permission.reply failed to send to desktop");
                    } else {
                        tracing::info!("[ghost-gateway] tool.permission.reply forwarded to desktop ticket_id={} granted={}", ticket_id, granted);
                    }
                } else {
                    tracing::warn!(
                        "[ghost-gateway] tool.permission.reply no run_id or no desktop: ticket_id={} has_run_id={} has_desktop={}",
                        ticket_id,
                        has_run_id,
                        has_desktop
                    );
                }
                send_response(&mut socket, &req.id, true, None, None).await;
            }
            "tool.input.reply" => {
                if conn_role.as_deref() != Some("node") {
                    send_response(&mut socket, &req.id, false, None, Some("tool.input.reply only from node (channels)")).await;
                    continue;
                }
                let run_id = params_obj.get("runId").or_else(|| params_obj.get("run_id")).and_then(|v| v.as_str()).map(String::from);
                let input_request_id = params_obj.get("inputRequestId").or_else(|| params_obj.get("input_request_id")).and_then(|v| v.as_str()).map(String::from);
                let inputs = params_obj.get("inputs").cloned();
                let (run_id, input_request_id, inputs) = match (run_id, input_request_id, inputs) {
                    (Some(a), Some(b), Some(c)) if !a.is_empty() && !b.is_empty() => (a, b, c),
                    _ => {
                        tracing::warn!("[ghost-gateway] tool.input.reply missing runId, inputRequestId, or inputs");
                        send_response(&mut socket, &req.id, false, None, Some("tool.input.reply requires runId, inputRequestId, inputs")).await;
                        continue;
                    }
                };
                let desktop_tx_opt = gateway_state.get_desktop_tx().await;
                if let Some(desktop_tx) = desktop_tx_opt {
                    let push = json!({
                        "type": "tool.input.reply",
                        "runId": run_id,
                        "inputRequestId": input_request_id,
                        "inputs": inputs
                    });
                    if desktop_tx.send(push.to_string()).await.is_err() {
                        tracing::warn!("[ghost-gateway] tool.input.reply failed to send to desktop");
                    } else {
                        tracing::info!("[ghost-gateway] tool.input.reply forwarded to desktop input_request_id={}", input_request_id);
                    }
                } else {
                    tracing::warn!("[ghost-gateway] tool.input.reply no desktop connected");
                }
                let _ = gateway_state.take_run_id_for_input(&input_request_id).await;
                send_response(&mut socket, &req.id, true, None, None).await;
            }
            _ => {
                tracing::debug!("[ghost-gateway] unknown method method={}", req.method);
                send_response(
                    &mut socket,
                    &req.id,
                    false,
                    None,
                    Some(&format!("unknown method: {}", req.method)),
                ).await;
            }
        }
            }
            frame = stream_rx.recv() => {
                match frame {
                    Some(text) => {
                        let _ = socket.send(Message::Text(text)).await;
                    }
                    None => break,
                }
            }
        }
    }

    if conn_role.as_deref() == Some("operator") {
        gateway_state.unregister_desktop().await;
        tracing::debug!("[ghost-gateway] unregistered operator conn_id={}", conn_id);
    }
    tracing::debug!("[ghost-gateway] WS connection closed conn_id={}", conn_id);
}
