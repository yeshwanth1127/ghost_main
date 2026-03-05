// Process list capability - list running processes

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct ProcessList;

#[async_trait]
impl Capability for ProcessList {
    fn name(&self) -> &'static str {
        "process.list"
    }

    fn description(&self) -> &'static str {
        "List running processes"
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
        &["process_list"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "listing processes..."
            }),
        )
        .await?;

        #[cfg(windows)]
        let (stdout, exit_code) = {
            let output = Command::new("tasklist")
                .args(["/FO", "CSV", "/NH"])
                .output()
                .map_err(|e| format!("tasklist failed: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            (stdout, exit_code)
        };

        #[cfg(unix)]
        let (stdout, exit_code) = {
            let output = Command::new("ps")
                .args(["-eo", "pid,comm"])
                .output()
                .map_err(|e| format!("ps failed: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            (stdout, exit_code)
        };

        let processes: Vec<Value> = stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .enumerate()
            .take(100)
            .map(|(i, line)| json!({
                "index": i + 1,
                "line": line.trim()
            }))
            .collect();

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "exit_code": exit_code,
                "count": processes.len()
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
                "processes": processes,
                "raw": stdout,
                "count": processes.len()
            })],
            side_effects: vec!["disk_read".to_string()],
        })
    }
}
