//! Ollama chat streaming: build messages from transcript, call /api/chat with stream, send run.chunk / run.think / run.done.

use futures_util::StreamExt;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc;

use super::transcript::{append_assistant_message, read_session_messages};

/// Run Ollama chat stream and send run.chunk / run.think / run.done to the channel.
pub async fn run_ollama_stream(
    path: PathBuf,
    session_id: String,
    run_id: String,
    ollama_url: String,
    ollama_model: String,
    stream_tx: mpsc::Sender<String>,
) {
    let transcript_messages = read_session_messages(&path, 100).await;
    let messages: Vec<serde_json::Value> = transcript_messages
        .into_iter()
        .filter_map(|msg| {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string();
            let content = msg
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|c| c.get("text").and_then(|t| t.as_str()))
                .unwrap_or("")
                .to_string();
            if content.is_empty() {
                None
            } else {
                Some(serde_json::json!({ "role": role, "content": content }))
            }
        })
        .collect();

    if messages.is_empty() {
        tracing::warn!("[ghost-gateway] ollama skipped: no messages in transcript");
        let _ = stream_tx
            .send(
                serde_json::json!({
                    "type": "run.error",
                    "runId": run_id,
                    "error": "No messages in transcript (cannot call Ollama with empty messages)"
                })
                .to_string(),
            )
            .await;
        return;
    }

    let body = serde_json::json!({
        "model": ollama_model,
        "messages": messages,
        "stream": true
    });

    let url = format!("{}/api/chat", ollama_url.trim_end_matches('/'));
    tracing::info!(
        "[ghost-gateway] ollama request url={} model={} message_count={}",
        url,
        ollama_model,
        messages.len()
    );
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("[ghost-gateway] ollama client build failed: {}", e);
            let _ = stream_tx
                .send(
                    serde_json::json!({
                        "type": "run.error",
                        "runId": run_id,
                        "error": format!("client build failed: {}", e)
                    })
                    .to_string(),
                )
                .await;
            return;
        }
    };

    let mut res = match client.post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[ghost-gateway] ollama request failed: {}", e);
            let _ = stream_tx
                .send(
                    serde_json::json!({
                        "type": "run.error",
                        "runId": run_id,
                        "error": format!("ollama request failed: {}", e)
                    })
                    .to_string(),
                )
                .await;
            return;
        }
    };

    // Retry once on 404 (Ollama can 404 briefly when model is loading or unavailable)
    if res.status().as_u16() == 404 {
        tracing::info!("[ghost-gateway] ollama got 404, retrying once after 500ms");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Ok(retry_res) = client.post(&url).json(&body).send().await {
            res = retry_res;
        }
    }

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_else(|_| String::new());
        let body_preview = body.chars().take(200).collect::<String>();
        tracing::warn!(
            "[ghost-gateway] ollama error status={} url={} model={} message_count={} body_preview={:?}",
            status,
            url,
            ollama_model,
            messages.len(),
            body_preview
        );
        let hint = if status.as_u16() == 404 {
            format!(
                "Ollama returned 404 - ensure model is pulled: run `ollama pull {}` and that Ollama is running",
                ollama_model
            )
        } else {
            format!(
                "ollama returned {} - check OLLAMA_URL (e.g. http://localhost:11434) and that Ollama is running",
                status
            )
        };
        let _ = stream_tx
            .send(
                serde_json::json!({
                    "type": "run.error",
                    "runId": run_id,
                    "error": hint
                })
                .to_string(),
            )
            .await;
        return;
    }

    let mut stream = res.bytes_stream();
    let mut full_content = String::new();
    let mut buf = Vec::<u8>::new();
    let mut stream_done = false;

    while !stream_done {
        let chunk = match stream.next().await {
            Some(Ok(b)) => b,
            Some(Err(e)) => {
                tracing::warn!("[ghost-gateway] ollama stream chunk error: {}", e);
                break;
            }
            None => break,
        };
        buf.extend_from_slice(&chunk);
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line).trim().to_string();
            if line.is_empty() {
                continue;
            }
            let parsed: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(err) = parsed.get("error").and_then(|e| e.as_str()) {
                let _ = stream_tx
                    .send(
                        serde_json::json!({
                            "type": "run.error",
                            "runId": run_id,
                            "error": err
                        })
                        .to_string(),
                    )
                    .await;
                return;
            }
            if parsed.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                stream_done = true;
            }
            if let Some(msg) = parsed.get("message").and_then(|m| m.as_object()) {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    if !content.is_empty() {
                        full_content.push_str(content);
                        let _ = stream_tx
                            .send(
                                serde_json::json!({
                                    "type": "run.chunk",
                                    "runId": run_id,
                                    "text": content
                                })
                                .to_string(),
                            )
                            .await;
                    }
                }
                if let Some(thinking) = msg.get("thinking").and_then(|t| t.as_str()) {
                    if !thinking.is_empty() {
                        let _ = stream_tx
                            .send(
                                serde_json::json!({
                                    "type": "run.think",
                                    "runId": run_id,
                                    "text": thinking
                                })
                                .to_string(),
                            )
                            .await;
                    }
                }
            }
        }
    }

    let content_to_save = full_content.trim();
    let message_id = match append_assistant_message(&path, &session_id, content_to_save, None, true).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!("[ghost-gateway] append assistant message failed: {}", e);
            let _ = stream_tx
                .send(
                    serde_json::json!({
                        "type": "run.error",
                        "runId": run_id,
                        "error": format!("failed to save: {}", e)
                    })
                    .to_string(),
                )
                .await;
            return;
        }
    };

    let _ = stream_tx
        .send(
            serde_json::json!({
                "type": "run.done",
                "runId": run_id,
                "messageId": message_id
            })
            .to_string(),
        )
        .await;
    tracing::debug!("[ghost-gateway] ollama run done run_id={} message_id={}", run_id, message_id);
}
