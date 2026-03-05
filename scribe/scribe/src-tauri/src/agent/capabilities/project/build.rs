// Project build - run build command

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub struct ProjectBuild;

fn detect_build_command(path: &Path) -> Option<(&'static str, Vec<&'static str>)> {
    if path.join("package.json").exists() {
        return Some(("npm", vec!["run", "build"]));
    }
    if path.join("Cargo.toml").exists() {
        return Some(("cargo", vec!["build", "--release"]));
    }
    if path.join("Makefile").exists() {
        return Some(("make", vec![]));
    }
    if path.join("go.mod").exists() {
        return Some(("go", vec!["build", "./..."]));
    }
    None
}

#[async_trait]
impl Capability for ProjectBuild {
    fn name(&self) -> &'static str {
        "project.build"
    }

    fn description(&self) -> &'static str {
        "Build the project (dev or prod mode)"
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
        &["process_output"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Project root path"
                },
                "mode": {
                    "type": "string",
                    "description": "dev or prod (default: prod)",
                    "enum": ["dev", "prod"]
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
        if detect_build_command(p).is_none() {
            return PreflightResult::Reject("Could not detect build system (package.json, Cargo.toml, Makefile, go.mod)".to_string());
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Build project at {}", path),
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
        let mode = inputs.get("mode").and_then(|v| v.as_str()).unwrap_or("prod");
        let root = Path::new(path);

        let (program, mut args) = detect_build_command(root)
            .ok_or("Could not detect build system")?;

        if program == "cargo" && mode == "dev" {
            args = vec!["build"];
        } else if program == "npm" && mode == "dev" {
            args = vec!["run", "build", "--", "--mode", "development"];
        }

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "program": program,
                "args": args,
                "output": "building..."
            }),
        )
        .await?;

        let mut cmd = Command::new(program);
        cmd.args(&args).current_dir(root);
        let output = cmd.output()
            .map_err(|e| format!("Build failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": program,
                "exit_code": exit_code
            }),
        )
        .await?;

        let outcome = if output.status.success() {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Build failed: {}", stderr))
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
                "completed_at": chrono::Utc::now(),
                "exit_code": exit_code
            }),
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
