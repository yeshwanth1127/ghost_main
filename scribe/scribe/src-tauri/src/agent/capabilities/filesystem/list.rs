// Filesystem list capability - list directory contents

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct FilesystemList;

#[async_trait]
impl Capability for FilesystemList {
    fn name(&self) -> &'static str {
        "filesystem.list"
    }

    fn description(&self) -> &'static str {
        "List directory contents (files and subdirectories)"
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
        &["directory_listing"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
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
                reason: format!("List directory: {}", path),
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
                "output": "listing..."
            }),
        )
        .await?;

        let entries = fs::read_dir(path_buf)
            .map_err(|e| format!("Failed to list directory {}: {}", path_buf.display(), e))?;

        let mut items: Vec<Value> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let is_dir = path.is_dir();
            let metadata = entry.metadata().ok();
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            items.push(json!({
                "name": name,
                "path": path.to_string_lossy().to_string(),
                "is_dir": is_dir,
                "size": size
            }));
        }

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

        let artifact_json = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "kind": "directory_listing",
            "location": path_buf.to_string_lossy().to_string(),
            "summary": format!("Listed {} items in {}", items.len(), path_buf.display()),
            "source_step": step_id,
            "created_at": chrono::Utc::now()
        });

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::ARTIFACT_CREATED,
            artifact_json,
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "path": path_buf.to_string_lossy().to_string(),
                "items": items,
                "count": items.len()
            })],
            side_effects: vec!["disk_read".to_string()],
        })
    }
}
