// Filesystem delete capability - delete file or directory

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct FilesystemDelete;

#[async_trait]
impl Capability for FilesystemDelete {
    fn name(&self) -> &'static str {
        "filesystem.delete"
    }

    fn description(&self) -> &'static str {
        "Delete file or directory (recursive for directories)"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["disk_write"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }

    fn requires_permission(&self) -> bool {
        true
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &[]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to file or directory to delete"
                }
            },
            "required": ["path"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let path = inputs.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
        if path.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["path".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if self.requires_permission() {
            return PreflightResult::NeedsPermission(PermissionRequest {
                reason: format!("Delete: {}", path),
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

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "deleting..."
            }),
        )
        .await?;

        if path_buf.is_dir() {
            fs::remove_dir_all(path_buf)
                .map_err(|e| format!("Failed to delete directory {}: {}", path_buf.display(), e))?;
        } else {
            fs::remove_file(path_buf)
                .map_err(|e| format!("Failed to delete file {}: {}", path_buf.display(), e))?;
        }

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::FILE_DELETED,
            json!({
                "path": path_buf.to_string_lossy().to_string()
            }),
        )
        .await?;

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

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "path": path_buf.to_string_lossy().to_string(),
                "deleted": true
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}
