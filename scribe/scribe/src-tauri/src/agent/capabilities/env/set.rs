// Env set capability - update environment variable in .env file safely

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct EnvSet;

#[async_trait]
impl Capability for EnvSet {
    fn name(&self) -> &'static str {
        "env.set"
    }

    fn description(&self) -> &'static str {
        "Update environment variable in .env file safely"
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
                "path": {
                    "type": "string",
                    "description": "Path to .env file"
                },
                "key": {
                    "type": "string",
                    "description": "Variable name"
                },
                "value": {
                    "type": "string",
                    "description": "Variable value"
                }
            },
            "required": ["key", "value"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let key = inputs.get("key").and_then(|v| v.as_str()).unwrap_or("").trim();
        let value = inputs.get("value");
        let mut missing = Vec::new();
        if key.is_empty() {
            missing.push("key".to_string());
        }
        if value.is_none() {
            missing.push("value".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if key.contains('=') || key.contains('\n') {
            return PreflightResult::Reject("Invalid key: must not contain = or newline".to_string());
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Set env var: {}", key),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let path = inputs.get("path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(".env");
        let key = inputs["key"]
            .as_str()
            .ok_or_else(|| "Missing key".to_string())?;
        let value = inputs["value"]
            .as_str()
            .ok_or_else(|| "Missing value".to_string())?;

        let path_buf = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": { "path": path, "key": key },
                "output": "setting..."
            }),
        )
        .await?;

        let content = fs::read_to_string(path_buf)
            .unwrap_or_else(|_| String::new());

        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let key_eq = format!("{}=", key);
        let mut found = false;
        for line in &mut lines {
            let trimmed = line.trim();
            if trimmed.starts_with(&key_eq) || (trimmed.starts_with(key) && trimmed.len() > key.len() && trimmed.as_bytes()[key.len()] == b'=') {
                *line = format!("{}={}", key, value);
                found = true;
                break;
            }
        }
        if !found {
            if !lines.is_empty() && !lines.last().map(|s| s.is_empty()).unwrap_or(true) {
                lines.push(String::new());
            }
            lines.push(format!("{}={}", key, value));
        }

        fs::write(path_buf, lines.join("\n"))
            .map_err(|e| format!("Failed to write {}: {}", path_buf.display(), e))?;

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::FILE_WRITTEN,
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
                "key": key,
                "set": true
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}
