// Code modify_block capability - modify specific line range in file

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct CodeModifyBlock;

#[async_trait]
impl Capability for CodeModifyBlock {
    fn name(&self) -> &'static str {
        "code.modify_block"
    }

    fn description(&self) -> &'static str {
        "Modify a specific line range in a file (safer than full overwrite)"
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
        &["file"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "Path to the file"
                },
                "start_line": {
                    "type": "integer",
                    "description": "1-based start line (inclusive)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "1-based end line (inclusive)"
                },
                "replacement": {
                    "type": "string",
                    "description": "Content to replace the block with"
                }
            },
            "required": ["file", "start_line", "end_line", "replacement"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let file = inputs.get("file").and_then(|v| v.as_str()).unwrap_or("").trim();
        let start = inputs.get("start_line").and_then(|v| v.as_i64()).unwrap_or(0);
        let end = inputs.get("end_line").and_then(|v| v.as_i64()).unwrap_or(0);
        let mut missing = Vec::new();
        if file.is_empty() {
            missing.push("file".to_string());
        }
        if start < 1 {
            missing.push("start_line".to_string());
        }
        if end < 1 {
            missing.push("end_line".to_string());
        }
        if !inputs.get("replacement").map(|v| v.is_string()).unwrap_or(false) {
            missing.push("replacement".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if start > end {
            return PreflightResult::Reject("start_line must be <= end_line".to_string());
        }
        if self.requires_permission() {
            return PreflightResult::NeedsPermission(PermissionRequest {
                reason: format!("Modify lines {}-{} in {}", start, end, file),
            });
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let file = inputs["file"]
            .as_str()
            .ok_or_else(|| "Missing file argument".to_string())?;
        let start_line = inputs["start_line"]
            .as_i64()
            .ok_or_else(|| "Missing start_line".to_string())? as usize;
        let end_line = inputs["end_line"]
            .as_i64()
            .ok_or_else(|| "Missing end_line".to_string())? as usize;
        let replacement = inputs["replacement"]
            .as_str()
            .ok_or_else(|| "Missing replacement".to_string())?;

        let path_buf = Path::new(file);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": {
                    "file": file,
                    "start_line": start_line,
                    "end_line": end_line,
                    "replacement_length": replacement.len()
                },
                "output": "modifying..."
            }),
        )
        .await?;

        let content = fs::read_to_string(path_buf)
            .map_err(|e| format!("Failed to read file {}: {}", path_buf.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        if start_line > lines.len() || end_line > lines.len() {
            return Err(format!("Line range {}-{} out of bounds (file has {} lines)", start_line, end_line, lines.len()));
        }

        let before: String = lines[..start_line - 1].join("\n");
        let after: String = if end_line < lines.len() {
            let rest = &lines[end_line..];
            if rest.is_empty() {
                String::new()
            } else {
                format!("\n{}", rest.join("\n"))
            }
        } else {
            String::new()
        };

        let new_content = if before.is_empty() {
            format!("{}{}", replacement, after)
        } else {
            format!("{}\n{}{}", before, replacement, after)
        };

        fs::write(path_buf, new_content)
            .map_err(|e| format!("Failed to write file {}: {}", path_buf.display(), e))?;

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::FILE_WRITTEN,
            json!({
                "path": path_buf.to_string_lossy().to_string(),
                "modified_lines": format!("{}-{}", start_line, end_line)
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

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::ARTIFACT_CREATED,
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "kind": "File",
                "location": path_buf.to_string_lossy().to_string(),
                "summary": format!("Modified lines {}-{}", start_line, end_line),
                "source_step": step_id,
                "created_at": chrono::Utc::now()
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "path": path_buf.to_string_lossy().to_string(),
                "start_line": start_line,
                "end_line": end_line,
                "modified": true
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}
