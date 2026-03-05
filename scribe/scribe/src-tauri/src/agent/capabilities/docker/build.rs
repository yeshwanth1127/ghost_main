// Docker build capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub struct DockerBuild;

#[async_trait]
impl Capability for DockerBuild {
    fn name(&self) -> &'static str {
        "docker.build"
    }

    fn description(&self) -> &'static str {
        "Build Docker image from Dockerfile"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["process_execution", "network"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }

    fn requires_permission(&self) -> bool {
        true
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["docker_image"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to directory containing Dockerfile"
                },
                "tag": {
                    "type": "string",
                    "description": "Image tag (e.g. myapp:latest)"
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
        let p = Path::new(path);
        if !p.exists() || !p.is_dir() {
            return PreflightResult::Reject(format!("Path does not exist: {}", path));
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Build Docker image at {}", path),
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
        let tag = inputs.get("tag").and_then(|v| v.as_str()).unwrap_or("latest");
        let root = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "building..."
            }),
        )
        .await?;

        let mut cmd = Command::new("docker");
        cmd.args(["build", "-t", tag, "."]).current_dir(root);
        let output = cmd.output()
            .map_err(|e| format!("Docker build failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": "docker",
                "exit_code": exit_code
            }),
        )
        .await?;

        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Docker build failed: {}", stderr))
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
                "tag": tag,
                "path": root.to_string_lossy().to_string(),
                "exit_code": exit_code,
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "success": output.status.success()
            })],
            side_effects: vec!["process_execution".to_string(), "network".to_string()],
        })
    }
}
