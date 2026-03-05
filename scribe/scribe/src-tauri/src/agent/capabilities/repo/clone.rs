// Repo clone capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub struct RepoClone;

#[async_trait]
impl Capability for RepoClone {
    fn name(&self) -> &'static str {
        "repo.clone"
    }

    fn description(&self) -> &'static str {
        "Clone a git repository"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["process_execution", "network", "disk_write"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn requires_permission(&self) -> bool {
        true
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["repository"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Repository URL"
                },
                "path": {
                    "type": "string",
                    "description": "Local path to clone into"
                },
                "branch": {
                    "type": "string",
                    "description": "Optional branch to checkout"
                }
            },
            "required": ["url", "path"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let url = inputs.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
        let path = inputs.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
        let mut missing = Vec::new();
        if url.is_empty() {
            missing.push("url".to_string());
        }
        if path.is_empty() {
            missing.push("path".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Clone {} to {}", url, path),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let url = inputs["url"]
            .as_str()
            .ok_or_else(|| "Missing url".to_string())?;
        let path = inputs["path"]
            .as_str()
            .ok_or_else(|| "Missing path".to_string())?;
        let branch = inputs.get("branch").and_then(|v| v.as_str());
        let target = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "cloning..."
            }),
        )
        .await?;

        let mut cmd = Command::new("git");
        cmd.args(["clone", url, path]);
        if let Some(b) = branch {
            cmd.arg("-b").arg(b);
        }
        let output = cmd.output()
            .map_err(|e| format!("Git clone failed: {}", e))?;

        let _stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

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

        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Git clone failed: {}", stderr))
        };

        let step_id = uuid::Uuid::new_v4().to_string();
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            if output.status.success() {
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
                "url": url,
                "path": target.to_string_lossy().to_string(),
                "branch": branch,
                "exit_code": exit_code,
                "success": output.status.success()
            })],
            side_effects: vec!["process_execution".to_string(), "network".to_string(), "disk_write".to_string()],
        })
    }
}
