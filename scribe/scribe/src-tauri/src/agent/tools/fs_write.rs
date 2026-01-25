// File write tool

use crate::agent::events::{FILE_WRITTEN, MESSAGE_APPENDED, TOOL_EXECUTED};
use crate::agent::state::run_state::{Artifact, ArtifactType};
use crate::agent::run::store;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;
use tauri::AppHandle;

/// Capability scope (simplified for now)
#[derive(Clone)]
pub struct CapabilityScope {
    pub allowed_paths: Vec<String>,
}

/// Write to a file and emit events
pub async fn fs_write(
    app: &AppHandle,
    run_id: &str,
    path: String,
    content: String,
    _scope: CapabilityScope, // TODO: Validate scope in Phase 3
) -> Result<Artifact, String> {
    let resolved_path = resolve_path_with_picker(app, &path).await?;
    let path_buf = Path::new(&resolved_path);

    // Compute content hash BEFORE writing
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let content_hash = format!("{:x}", hasher.finalize());

    // Emit fact event: TOOL_EXECUTED (BEFORE side effect)
    let _tool_event = store::append_event(
        app,
        run_id,
        TOOL_EXECUTED,
        json!({
            "tool_name": "fs_write",
            "args": {
                "path": resolved_path,
                "content_length": content.len()
            },
            "output": "writing..."
        }),
    )
    .await?;

    // Write file (side effect) - atomic write using temp file
    let temp_path = format!("{}.tmp", path_buf.display());
    fs::write(&temp_path, &content)
        .map_err(|e| format!("Failed to write temp file {}: {}", temp_path, e))?;
    
    // Atomic rename
    fs::rename(&temp_path, path_buf)
        .map_err(|e| format!("Failed to rename temp file to {}: {}", path_buf.display(), e))?;

    // Emit fact event: FILE_WRITTEN (after writing)
    store::append_event(
        app,
        run_id,
        FILE_WRITTEN,
        json!({
            "path": path_buf.to_string_lossy().to_string(),
            "content_hash": content_hash,
            "size": content.len()
        }),
    )
    .await?;

    // Emit projection event: STEP_COMPLETED
    let step_id = uuid::Uuid::new_v4().to_string();
    store::append_event(
        app,
        run_id,
        crate::agent::events::STEP_COMPLETED,
        json!({
            "step_id": step_id,
            "completed_at": chrono::Utc::now()
        }),
    )
    .await?;

    // Create artifact
    let artifact = Artifact {
        id: uuid::Uuid::new_v4().to_string(),
        kind: ArtifactType::File,
        location: path_buf.to_string_lossy().to_string(),
        summary: format!("Wrote file: {} ({} bytes)", path_buf.display(), content.len()),
        source_step: step_id,
        created_at: chrono::Utc::now(),
    };

    // Emit projection event: ARTIFACT_CREATED
    store::append_event(
        app,
        run_id,
        crate::agent::events::ARTIFACT_CREATED,
        json!({
            "id": artifact.id,
            "kind": "File",
            "location": artifact.location,
            "summary": artifact.summary,
            "source_step": artifact.source_step,
            "created_at": artifact.created_at
        }),
    )
    .await?;

    // Emit assistant message with file location
    store::append_event(
        app,
        run_id,
        MESSAGE_APPENDED,
        json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "role": "assistant",
            "content": format!("Created file at: {}", artifact.location),
            "created_at": chrono::Utc::now()
        }),
    )
    .await?;

    Ok(artifact)
}

async fn resolve_path_with_picker(app: &AppHandle, path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    let needs_picker = trimmed.is_empty() || Path::new(trimmed).is_relative();

    if !needs_picker {
        return Ok(trimmed.to_string());
    }

    let mut dialog = app.dialog().file();
    if !trimmed.is_empty() {
        if let Some(file_name) = Path::new(trimmed).file_name().and_then(|v| v.to_str()) {
            dialog = dialog.set_file_name(file_name.to_string());
        }
    }

    let (tx, rx) = oneshot::channel();
    dialog.save_file(move |picked| {
        let _ = tx.send(picked);
    });

    let picked = rx
        .await
        .map_err(|_| "Location picker failed".to_string())?;
    let picked = picked.ok_or("Location picker was cancelled")?;
    let picked_path = picked
        .into_path()
        .map_err(|e| format!("Failed to resolve picked path: {}", e))?;

    Ok(picked_path.to_string_lossy().to_string())
}
