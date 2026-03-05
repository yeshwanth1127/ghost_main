// Process kill capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ProcessKill;

#[async_trait]
impl Capability for ProcessKill {
    fn name(&self) -> &'static str {
        "process.kill"
    }

    fn description(&self) -> &'static str {
        "Kill a process by PID"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["process_execution"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
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
                "pid": {
                    "type": "integer",
                    "description": "Process ID to kill"
                }
            },
            "required": ["pid"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let pid = inputs.get("pid").and_then(|v| v.as_i64());
        if pid.is_none() || pid.unwrap_or(0) <= 0 {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["pid".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Kill process PID {}", pid.unwrap()),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let pid = inputs["pid"]
            .as_i64()
            .ok_or_else(|| "Missing pid".to_string())?;
        if pid <= 0 {
            return Err("Invalid PID".to_string());
        }

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "killing..."
            }),
        )
        .await?;

        #[cfg(unix)]
        let result = {
            use std::process::Command;
            Command::new("kill").arg(pid.to_string()).output()
        };

        #[cfg(windows)]
        let result = {
            use std::process::Command;
            Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output()
        };

        let output = result.map_err(|e| format!("Kill failed: {}", e))?;
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": "kill",
                "pid": pid,
                "exit_code": exit_code
            }),
        )
        .await?;

        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Kill failed: {}", stderr))
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
                "pid": pid,
                "killed": output.status.success(),
                "exit_code": exit_code
            })],
            side_effects: vec!["process_execution".to_string()],
        })
    }
}
