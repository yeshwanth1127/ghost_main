// Filesystem read capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::{FILE_READ, TOOL_EXECUTED};
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub struct FilesystemRead;

#[async_trait]
impl Capability for FilesystemRead {
    fn name(&self) -> &'static str {
        "filesystem.read"
    }

    fn description(&self) -> &'static str {
        "Read contents of a file from disk"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["disk_read"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn requires_permission(&self) -> bool {
        true
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
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let path = inputs.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
        if path.is_empty() {
            let schema = self.input_schema();
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["path".to_string()],
                schema,
                current_inputs: inputs.clone(),
            });
        }
        if self.requires_permission() {
            return PreflightResult::NeedsPermission(PermissionRequest {
                reason: format!("Read file: {}", path),
            });
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

        let path_buf = Path::new(path);

        // Emit fact event: TOOL_EXECUTED (BEFORE side effect)
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
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
            &ctx.app,
            &ctx.run_id,
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
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::STEP_COMPLETED,
            json!({
                "step_id": step_id,
                "completed_at": chrono::Utc::now()
            }),
        )
        .await?;

        // Create artifact with file content preview for LLM context
        let content_preview = if content.len() > 500 {
            format!("{}... (truncated, {} bytes total)", &content[..500], content.len())
        } else {
            content.clone()
        };

        let artifact_json = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "kind": "File",
            "location": path_buf.to_string_lossy().to_string(),
            "summary": format!("Read file: {} ({} bytes). Content preview: {}", path_buf.display(), content.len(), content_preview),
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

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "path": path_buf.to_string_lossy().to_string(),
                "content": content,
                "content_hash": content_hash,
                "size": content.len()
            })],
            side_effects: vec!["disk_read".to_string()],
        })
    }
}
