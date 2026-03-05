// Filesystem write capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::{FILE_WRITTEN, MESSAGE_APPENDED, TOOL_EXECUTED};
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

pub struct FilesystemWrite;

#[async_trait]
impl Capability for FilesystemWrite {
    fn name(&self) -> &'static str {
        "filesystem.write"
    }

    fn description(&self) -> &'static str {
        "Write content to a file on disk"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["disk_write"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// No separate permission step: user already confirmed path and content in the Input Request dialog.
    /// Writing proceeds immediately after they submit path + content.
    fn requires_permission(&self) -> bool {
        false
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["file"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full file path including file name and extension (e.g. C:\\Users\\me\\Documents\\myfile.txt). Relative paths open the save dialog."
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let path = inputs.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
        let content = inputs.get("content").and_then(|v| v.as_str()).unwrap_or("").trim();
        let mut missing = Vec::new();
        // Ask for path when empty or relative (so Telegram/channel can collect path instead of opening picker on desktop)
        if path.is_empty() || Path::new(path).is_relative() {
            missing.push("path".to_string());
        }
        // Ask for content when missing or empty/whitespace (user must provide what to write)
        if content.is_empty() {
            missing.push("content".to_string());
        }
        // If path looks like a directory (ends with separator, or no file name/extension), ask for path again as full file path
        if !path.is_empty() && Path::new(path).is_absolute() {
            let path_buf = Path::new(path);
            let has_file_name = path_buf.file_name().is_some();
            let looks_like_dir = path.ends_with(std::path::MAIN_SEPARATOR) || path.ends_with('/')
                || !has_file_name;
            if looks_like_dir {
                missing.push("path".to_string());
            }
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if self.requires_permission() {
            let reason = format!("Write file at {}", path);
            return PreflightResult::NeedsPermission(PermissionRequest { reason });
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let path = inputs["path"]
            .as_str()
            .ok_or_else(|| "Missing path argument".to_string())?;
        
        let content = inputs["content"]
            .as_str()
            .ok_or_else(|| "Missing content argument".to_string())?;

        // Resolve path (may trigger save dialog if relative)
        let resolved_path = resolve_path_with_picker(&ctx.app, path).await?;
        let path_buf = Path::new(&resolved_path);

        // Compute content hash BEFORE writing
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());

        // Emit fact event: TOOL_EXECUTED (BEFORE side effect)
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": {
                    "path": resolved_path,
                    "content_length": content.len()
                },
                "output": "writing..."
            }),
        )
        .await?;

        // Write file (side effect) - atomic write using temp file
        let temp_path = format!("{}.tmp", path_buf.display());
        fs::write(&temp_path, content)
            .map_err(|e| format!("Failed to write temp file {}: {}", temp_path, e))?;
        
        // Atomic rename
        fs::rename(&temp_path, path_buf)
            .map_err(|e| format!("Failed to rename temp file to {}: {}", path_buf.display(), e))?;

        // Emit fact event: FILE_WRITTEN (after writing)
        store::append_event(
            &ctx.app,
            &ctx.run_id,
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
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::STEP_COMPLETED,
            json!({
                "step_id": step_id,
                "completed_at": chrono::Utc::now()
            }),
        )
        .await?;

        // Create artifact
        let artifact_json = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "kind": "File",
            "location": path_buf.to_string_lossy().to_string(),
            "summary": format!("Wrote file: {} ({} bytes)", path_buf.display(), content.len()),
            "source_step": step_id,
            "created_at": chrono::Utc::now()
        });

        // Emit projection event: ARTIFACT_CREATED
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::ARTIFACT_CREATED,
            artifact_json.clone(),
        )
        .await?;

        // Emit assistant message with file location
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            MESSAGE_APPENDED,
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "role": "assistant",
                "content": format!("Created file at: {}", path_buf.to_string_lossy()),
                "created_at": chrono::Utc::now()
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "path": path_buf.to_string_lossy().to_string(),
                "content_hash": content_hash,
                "size": content.len()
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}

async fn resolve_path_with_picker(app: &tauri::AppHandle, path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    // Absolute path: use as-is (no picker)
    if !trimmed.is_empty() && Path::new(trimmed).is_absolute() {
        return Ok(trimmed.to_string());
    }
    // Relative path: resolve against cwd so Telegram/channel-provided paths work without a picker
    if !trimmed.is_empty() {
        let cwd = std::env::current_dir().map_err(|e| format!("Failed to get cwd: {}", e))?;
        let resolved = cwd.join(trimmed);
        return Ok(resolved.to_string_lossy().to_string());
    }
    // Empty path: open save dialog (desktop only; used when user picks location in UI)
    let (tx, rx) = oneshot::channel();
    app.dialog().file().save_file(move |picked| {
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
