//! Session transcript (JSONL). Same format as ghost-gateway; ~/.ghost/sessions/<session_id>.jsonl

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const SESSION_VERSION: u32 = 1;

/// Ghost state dir: env GHOST_STATE_DIR or ~/.ghost
pub fn ghost_state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GHOST_STATE_DIR") {
        let home_str = dirs::home_dir()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_string());
        let expanded = dir.trim().replace('~', &home_str);
        return PathBuf::from(expanded);
    }
    dirs::home_dir()
        .map(|h| h.join(".ghost"))
        .unwrap_or_else(|| PathBuf::from(".ghost"))
}

fn sanitize_session_id(session_key: &str) -> String {
    let s: String = session_key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
        .take(200)
        .collect();
    if s.is_empty() {
        "default".to_string()
    } else {
        s
    }
}

pub fn resolve_transcript_path(session_key: &str) -> PathBuf {
    let dir = ghost_state_dir().join("sessions");
    let session_id = sanitize_session_id(session_key);
    dir.join(format!("{}.jsonl", session_id))
}

pub async fn ensure_transcript_file(path: &PathBuf, session_id: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let header = serde_json::json!({
        "type": "session",
        "version": SESSION_VERSION,
        "id": session_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "cwd": std::env::current_dir().unwrap_or_default().display().to_string()
    });
    let line = format!("{}\n", header);
    fs::write(path, line.as_bytes()).await?;
    tracing::debug!("[ghost-gateway] transcript created path={:?} session_id={}", path, session_id);
    Ok(())
}

pub async fn read_session_messages(path: &PathBuf, limit: u32) -> Vec<Value> {
    let limit = limit.min(1000);
    let content = match fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => {
            tracing::debug!("[ghost-gateway] transcript read: file missing path={:?}", path);
            return vec![];
        }
    };
    let mut messages = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if parsed.get("type").and_then(|t| t.as_str()) == Some("message") {
            if let Some(msg) = parsed.get("message") {
                messages.push(msg.clone());
            }
        }
    }
    if messages.len() <= limit as usize {
        messages
    } else {
        messages[messages.len() - limit as usize..].to_vec()
    }
}

pub async fn append_user_message(
    path: &PathBuf,
    session_id: &str,
    content: &str,
    create_if_missing: bool,
) -> Result<String> {
    if !path.exists() {
        if !create_if_missing {
            anyhow::bail!("transcript file not found");
        }
        ensure_transcript_file(path, session_id).await?;
    }
    let message_id = Uuid::new_v4().to_string()[..8].to_string();
    let now = chrono::Utc::now();
    let message_body = serde_json::json!({
        "role": "user",
        "content": [{"type": "text", "text": content}],
        "timestamp": now.timestamp_millis()
    });
    let entry = serde_json::json!({
        "type": "message",
        "id": message_id,
        "timestamp": now.to_rfc3339(),
        "message": message_body
    });
    let mut f = fs::OpenOptions::new().append(true).open(path).await?;
    f.write_all(format!("{}\n", entry).as_bytes()).await?;
    tracing::debug!("[ghost-gateway] append user message path={:?} message_id={}", path, message_id);
    Ok(message_id)
}

pub async fn append_assistant_message(
    path: &PathBuf,
    session_id: &str,
    message: &str,
    label: Option<&str>,
    create_if_missing: bool,
) -> Result<String> {
    if !path.exists() {
        if !create_if_missing {
            anyhow::bail!("transcript file not found");
        }
        ensure_transcript_file(path, session_id).await?;
    }
    let message_id = Uuid::new_v4().to_string()[..8].to_string();
    let now = chrono::Utc::now();
    let label_prefix = label.map(|l| format!("[{}]\n\n", l)).unwrap_or_default();
    let text = format!("{}{}", label_prefix, message);
    let message_body = serde_json::json!({
        "role": "assistant",
        "content": [{"type": "text", "text": text}],
        "timestamp": now.timestamp_millis(),
        "stopReason": "injected",
        "usage": {"input": 0, "output": 0, "totalTokens": 0}
    });
    let entry = serde_json::json!({
        "type": "message",
        "id": message_id,
        "timestamp": now.to_rfc3339(),
        "message": message_body
    });
    let mut f = fs::OpenOptions::new().append(true).open(path).await?;
    f.write_all(format!("{}\n", entry).as_bytes()).await?;
    tracing::debug!("[ghost-gateway] append assistant message path={:?} message_id={} label={:?}", path, message_id, label);
    Ok(message_id)
}
