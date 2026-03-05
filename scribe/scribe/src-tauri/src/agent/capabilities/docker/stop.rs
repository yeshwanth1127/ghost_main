// Docker stop capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct DockerStop;

#[async_trait]
impl Capability for DockerStop {
    fn name(&self) -> &'static str {
        "docker.stop"
    }

    fn description(&self) -> &'static str {
        "Stop a running Docker container"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["process_execution"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
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
                "container_id": {
                    "type": "string",
                    "description": "Container ID or name"
                }
            },
            "required": ["container_id"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let id = inputs.get("container_id").and_then(|v| v.as_str()).unwrap_or("").trim();
        if id.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["container_id".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Stop Docker container: {}", id),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let container_id = inputs["container_id"]
            .as_str()
            .ok_or_else(|| "Missing container_id".to_string())?;

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "stopping..."
            }),
        )
        .await?;

        let output = Command::new("docker")
            .args(["stop", container_id])
            .output()
            .map_err(|e| format!("Docker stop failed: {}", e))?;

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
            CapabilityOutcome::Failure(format!("Docker stop failed: {}", stderr))
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
                "container_id": container_id,
                "stopped": output.status.success(),
                "exit_code": exit_code
            })],
            side_effects: vec!["process_execution".to_string()],
        })
    }
}
