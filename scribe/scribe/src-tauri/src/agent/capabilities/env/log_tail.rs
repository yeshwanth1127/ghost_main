// Log tail capability - tail file in streaming mode

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct LogTail;

#[async_trait]
impl Capability for LogTail {
    fn name(&self) -> &'static str {
        "log.tail"
    }

    fn description(&self) -> &'static str {
        "Read last N lines of a file (tail-like, for logs)"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["disk_read"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["log_content"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to log file"
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of lines to read from end (default: 100)"
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
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let path = inputs["path"]
            .as_str()
            .ok_or_else(|| "Missing path".to_string())?;
        let lines = inputs.get("lines").and_then(|v| v.as_i64()).unwrap_or(100) as usize;
        let path_buf = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "tailing..."
            }),
        )
        .await?;

        let content = fs::read_to_string(path_buf)
            .map_err(|e| format!("Failed to read {}: {}", path_buf.display(), e))?;

        let all_lines: Vec<&str> = content.lines().collect();
        let start = all_lines.len().saturating_sub(lines);
        let tail_lines: Vec<&str> = all_lines[start..].to_vec();
        let tail_content = tail_lines.join("\n");

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::FILE_READ,
            json!({
                "path": path_buf.to_string_lossy().to_string(),
                "lines_read": tail_lines.len()
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
                "lines": tail_lines.len(),
                "content": tail_content,
                "total_lines": all_lines.len()
            })],
            side_effects: vec!["disk_read".to_string()],
        })
    }
}
