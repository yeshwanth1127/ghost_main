// File read tool

use crate::agent::events::{FILE_READ, TOOL_EXECUTED};
use crate::agent::state::run_state::{Artifact, ArtifactType};
use crate::agent::run::store;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tauri::AppHandle;

/// Capability scope (simplified for now)
#[derive(Clone)]
pub struct CapabilityScope {
    pub allowed_paths: Vec<String>,
}

/// Read a file and emit events
pub async fn fs_read(
    app: &AppHandle,
    run_id: &str,
    path: String,
    _scope: CapabilityScope, // TODO: Validate scope in Phase 3
) -> Result<Artifact, String> {
    let path_buf = Path::new(&path);

    // Emit fact event: TOOL_EXECUTED (BEFORE side effect)
    let _tool_event = store::append_event(
        app,
        run_id,
        TOOL_EXECUTED,
        json!({
            "tool_name": "fs_read",
            "args": {
                "path": path
            },
            "output": "reading..."
        }),
    )
    .await?;

    // Read file (side effect)
    let content = fs::read_to_string(&path_buf)
        .map_err(|e| format!("Failed to read file {}: {}", path_buf.display(), e))?;

    // Compute content hash
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let content_hash = format!("{:x}", hasher.finalize());

    // Emit fact event: FILE_READ (after reading)
    store::append_event(
        app,
        run_id,
        FILE_READ,
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

    // Create artifact with file content preview for LLM context
    // Store a preview (first 500 chars) to help LLM understand what was read
    let content_preview = if content.len() > 500 {
        format!("{}... (truncated, {} bytes total)", &content[..500], content.len())
    } else {
        content.clone()
    };
    
    let artifact = Artifact {
        id: uuid::Uuid::new_v4().to_string(),
        kind: ArtifactType::File,
        location: path_buf.to_string_lossy().to_string(),
        summary: format!("Read file: {} ({} bytes). Content preview: {}", path_buf.display(), content.len(), content_preview),
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

    Ok(artifact)
}
