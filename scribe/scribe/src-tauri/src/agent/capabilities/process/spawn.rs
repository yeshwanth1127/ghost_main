// Process spawn capability - safe process execution

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct ProcessSpawn;

#[async_trait]
impl Capability for ProcessSpawn {
    fn name(&self) -> &'static str {
        "process.spawn"
    }

    fn description(&self) -> &'static str {
        "Spawn a system process with structured arguments (no shell interpolation)"
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
        &["process_output"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "program": {
                    "type": "string",
                    "description": "Executable program name (e.g., 'git', 'npm', 'python')"
                },
                "args": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Array of command-line arguments (no shell interpolation)"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory (optional)"
                }
            },
            "required": ["program"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let program = inputs.get("program").and_then(|v| v.as_str()).unwrap_or("").trim();
        if program.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: vec!["program".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if self.requires_permission() {
            let reason = format!("Run process: {}", program);
            return PreflightResult::NeedsPermission(PermissionRequest { reason });
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let program = inputs["program"]
            .as_str()
            .ok_or_else(|| "Missing program argument".to_string())?;

        let args: Vec<String> = inputs["args"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let cwd = inputs["cwd"]
            .as_str()
            .map(String::from);

        // Emit fact event: TOOL_EXECUTED (BEFORE side effect)
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "program": program,
                "args": args,
                "cwd": cwd
            }),
        )
        .await?;

        // Build command (NO shell interpolation - safe)
        let mut cmd = Command::new(program);
        cmd.args(&args);
        
        if let Some(working_dir) = &cwd {
            cmd.current_dir(working_dir);
        }

        // Execute process
        let output = cmd.output()
            .map_err(|e| format!("Failed to spawn process {}: {}", program, e))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Emit fact event: PROCESS_COMPLETED
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": program,
                "args": args,
                "exit_code": exit_code,
                "stdout_len": stdout.len(),
                "stderr_len": stderr.len()
            }),
        )
        .await?;

        // Determine outcome
        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Process exited with code {}", exit_code))
        };

        // Emit projection event: STEP_COMPLETED or STEP_FAILED
        let step_id = uuid::Uuid::new_v4().to_string();
        if output.status.success() {
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
        } else {
            store::append_event(
                &ctx.app,
                &ctx.run_id,
                crate::agent::events::STEP_FAILED,
                json!({
                    "step_id": step_id,
                    "error": format!("Process exited with code {}", exit_code),
                    "stderr": stderr.to_string()
                }),
            )
            .await?;
        }

        // Create artifact with process output
        let artifact_json = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "kind": "process_output",
            "summary": format!("Executed {} with exit code {}", program, exit_code),
            "source_step": step_id,
            "created_at": chrono::Utc::now()
        });

        // Emit projection event: ARTIFACT_CREATED
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::ARTIFACT_CREATED,
            artifact_json.clone(),
        )
        .await?;

        Ok(CapabilityResult {
            outcome,
            artifacts: vec![json!({
                "program": program,
                "args": args,
                "exit_code": exit_code,
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "success": output.status.success()
            })],
            side_effects: vec!["process_execution".to_string()],
        })
    }
}
