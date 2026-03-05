// Docker run capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct DockerRun;

#[async_trait]
impl Capability for DockerRun {
    fn name(&self) -> &'static str {
        "docker.run"
    }

    fn description(&self) -> &'static str {
        "Run Docker container from image"
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
        &["docker_container"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "string",
                    "description": "Docker image name"
                },
                "ports": {
                    "type": "object",
                    "description": "Port mapping (e.g. {\"8080\": \"80\"})"
                },
                "env": {
                    "type": "object",
                    "description": "Environment variables"
                }
            },
            "required": ["image"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let image = inputs.get("image").and_then(|v| v.as_str()).unwrap_or("").trim();
        if image.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["image".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Run Docker container: {}", image),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let image = inputs["image"]
            .as_str()
            .ok_or_else(|| "Missing image".to_string())?;
        let ports = inputs.get("ports").and_then(|v| v.as_object());
        let env = inputs.get("env").and_then(|v| v.as_object());

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "running..."
            }),
        )
        .await?;

        let mut args: Vec<String> = vec!["run".into(), "-d".into(), image.into()];
        if let Some(p) = ports {
            for (host, container) in p {
                if let Some(c) = container.as_str() {
                    args.push("-p".into());
                    args.push(format!("{}:{}", host, c));
                }
            }
        }
        if let Some(e) = env {
            for (k, v) in e {
                if let Some(s) = v.as_str() {
                    args.push("-e".into());
                    args.push(format!("{}={}", k, s));
                }
            }
        }

        let output = Command::new("docker")
            .args(&args)
            .output()
            .map_err(|e| format!("Docker run failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        let container_id = stdout.trim();

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": "docker",
                "exit_code": exit_code,
                "container_id": container_id
            }),
        )
        .await?;

        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Docker run failed: {}", stderr))
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
                "image": image,
                "container_id": container_id,
                "exit_code": exit_code,
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "success": output.status.success()
            })],
            side_effects: vec!["process_execution".to_string(), "network".to_string()],
        })
    }
}
