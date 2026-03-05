// Port kill capability - kill process using port

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct PortKill;

fn find_pid_by_port(port: u16) -> Result<Option<u32>, String> {
    #[cfg(windows)]
    {
        let output = Command::new("netstat")
            .args(["-ano"])
            .output()
            .map_err(|e| format!("netstat failed: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let port_str = format!(":{}", port);
        for line in stdout.lines() {
            if line.contains(&port_str) && (line.contains("LISTENING") || line.contains("ESTABLISHED")) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pid_str) = parts.last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        return Ok(Some(pid));
                    }
                }
            }
        }
        Ok(None)
    }

    #[cfg(unix)]
    {
        let output = Command::new("lsof")
            .args(["-i", &format!(":{}", port), "-t"])
            .output()
            .map_err(|e| format!("lsof failed: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let pid_str = stdout.trim();
        if pid_str.is_empty() {
            return Ok(None);
        }
        if let Ok(pid) = pid_str.parse::<u32>() {
            return Ok(Some(pid));
        }
        Ok(None)
    }
}

#[async_trait]
impl Capability for PortKill {
    fn name(&self) -> &'static str {
        "port.kill"
    }

    fn description(&self) -> &'static str {
        "Kill process using a specific port"
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
                "port": {
                    "type": "integer",
                    "description": "Port number (e.g. 3000, 5173)"
                }
            },
            "required": ["port"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let port = inputs.get("port").and_then(|v| v.as_i64());
        if port.is_none() || port.unwrap_or(0) <= 0 || port.unwrap_or(0) > 65535 {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["port".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Kill process on port {}", port.unwrap()),
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let port = inputs["port"]
            .as_i64()
            .ok_or_else(|| "Missing port".to_string())? as u16;

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "finding and killing..."
            }),
        )
        .await?;

        let pid = find_pid_by_port(port)?
            .ok_or_else(|| format!("No process found using port {}", port))?;

        #[cfg(unix)]
        let output = Command::new("kill").arg(pid.to_string()).output();
        #[cfg(windows)]
        let output = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output();

        let output = output.map_err(|e| format!("Kill failed: {}", e))?;
        let exit_code = output.status.code().unwrap_or(-1);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": "kill",
                "pid": pid,
                "port": port,
                "exit_code": exit_code
            }),
        )
        .await?;

        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Failed to kill PID {}", pid))
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
                "port": port,
                "pid": pid,
                "killed": output.status.success(),
                "exit_code": exit_code
            })],
            side_effects: vec!["process_execution".to_string()],
        })
    }
}
