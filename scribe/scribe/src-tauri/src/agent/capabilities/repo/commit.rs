// Repo commit capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub struct RepoCommit;

#[async_trait]
impl Capability for RepoCommit {
    fn name(&self) -> &'static str {
        "repo.commit"
    }

    fn description(&self) -> &'static str {
        "Create git commit with message"
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
        &["commit"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Repository path"
                },
                "message": {
                    "type": "string",
                    "description": "Commit message"
                }
            },
            "required": ["path", "message"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let path = inputs.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
        let message = inputs.get("message").and_then(|v| v.as_str()).unwrap_or("").trim();
        let mut missing = Vec::new();
        if path.is_empty() {
            missing.push("path".to_string());
        }
        if message.is_empty() {
            missing.push("message".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Commit in {}: {}", path, message),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let path = inputs["path"]
            .as_str()
            .ok_or_else(|| "Missing path".to_string())?;
        let message = inputs["message"]
            .as_str()
            .ok_or_else(|| "Missing message".to_string())?;
        let root = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": { "path": path, "message": message },
                "output": "committing..."
            }),
        )
        .await?;

        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .output()
            .map_err(|e| format!("Git add failed: {}", e))?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(format!("Git add failed: {}", stderr));
        }

        let commit_output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(root)
            .output()
            .map_err(|e| format!("Git commit failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        let exit_code = commit_output.status.code().unwrap_or(-1);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": "git",
                "exit_code": exit_code
            }),
        )
        .await?;

        let outcome = if commit_output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Git commit failed: {}", stderr))
        };

        let step_id = uuid::Uuid::new_v4().to_string();
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            if commit_output.status.success() {
                crate::agent::events::STEP_COMPLETED
            } else {
                crate::agent::events::STEP_FAILED
            },
            json!({
                "step_id": step_id,
                "completed_at": chrono::Utc::now()
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome,
            artifacts: vec![json!({
                "path": root.to_string_lossy().to_string(),
                "message": message,
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "exit_code": exit_code,
                "success": commit_output.status.success()
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}
