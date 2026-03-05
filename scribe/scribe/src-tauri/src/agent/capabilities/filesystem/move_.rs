// Filesystem move capability - move or rename file

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct FilesystemMove;

#[async_trait]
impl Capability for FilesystemMove {
    fn name(&self) -> &'static str {
        "filesystem.move"
    }

    fn description(&self) -> &'static str {
        "Move or rename file/directory"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["disk_write"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
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
                "from": {
                    "type": "string",
                    "description": "Source path"
                },
                "to": {
                    "type": "string",
                    "description": "Destination path"
                }
            },
            "required": ["from", "to"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let from = inputs.get("from").and_then(|v| v.as_str()).unwrap_or("").trim();
        let to = inputs.get("to").and_then(|v| v.as_str()).unwrap_or("").trim();
        let mut missing = Vec::new();
        if from.is_empty() {
            missing.push("from".to_string());
        }
        if to.is_empty() {
            missing.push("to".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if self.requires_permission() {
            return PreflightResult::NeedsPermission(PermissionRequest {
                reason: format!("Move {} to {}", from, to),
            });
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let from = inputs["from"]
            .as_str()
            .ok_or_else(|| "Missing from argument".to_string())?;
        let to = inputs["to"]
            .as_str()
            .ok_or_else(|| "Missing to argument".to_string())?;

        let from_path = Path::new(from);
        let to_path = Path::new(to);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "moving..."
            }),
        )
        .await?;

        fs::rename(from_path, to_path)
            .map_err(|e| format!("Failed to move {} to {}: {}", from_path.display(), to_path.display(), e))?;

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

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::ARTIFACT_CREATED,
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "kind": "File",
                "location": to_path.to_string_lossy().to_string(),
                "summary": format!("Moved from {} to {}", from_path.display(), to_path.display()),
                "source_step": step_id,
                "created_at": chrono::Utc::now()
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "from": from_path.to_string_lossy().to_string(),
                "to": to_path.to_string_lossy().to_string()
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}
