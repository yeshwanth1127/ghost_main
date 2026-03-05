// Project install_dependencies - detect package manager and run install

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub struct ProjectInstallDependencies;

fn detect_package_manager(path: &Path) -> Option<(&'static str, &'static [&'static str])> {
    if path.join("package.json").exists() {
        return Some(("npm", &["install"][..]));
    }
    if path.join("Cargo.toml").exists() {
        return Some(("cargo", &["fetch"][..]));
    }
    if path.join("requirements.txt").exists() {
        return Some(("pip", &["install", "-r", "requirements.txt"][..]));
    }
    if path.join("pyproject.toml").exists() {
        return Some(("pip", &["install", "-e", "."][..]));
    }
    if path.join("go.mod").exists() {
        return Some(("go", &["mod", "download"][..]));
    }
    if path.join("yarn.lock").exists() {
        return Some(("yarn", &["install"][..]));
    }
    if path.join("pnpm-lock.yaml").exists() {
        return Some(("pnpm", &["install"][..]));
    }
    None
}

#[async_trait]
impl Capability for ProjectInstallDependencies {
    fn name(&self) -> &'static str {
        "project.install_dependencies"
    }

    fn description(&self) -> &'static str {
        "Install project dependencies (detects npm/pip/cargo/go)"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["process_execution", "network"]
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
                "package_manager": {
                    "type": "string",
                    "description": "Override: npm, pip, cargo, go, yarn, pnpm"
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
            return PreflightResult::Reject(format!("Path does not exist or is not a directory: {}", path));
        }
        if inputs.get("package_manager").is_none() {
            if detect_package_manager(p).is_none() {
                return PreflightResult::Reject("Could not detect package manager. Specify package_manager explicitly.".to_string());
            }
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let path = inputs["path"]
            .as_str()
            .ok_or_else(|| "Missing path".to_string())?;
        let root = Path::new(path);

        let (program, args): (&str, Vec<&str>) = if let Some(pm) = inputs.get("package_manager").and_then(|v| v.as_str()) {
            match pm {
                "npm" => ("npm", vec!["install"]),
                "yarn" => ("yarn", vec!["install"]),
                "pnpm" => ("pnpm", vec!["install"]),
                "pip" => ("pip", vec!["install", "-r", "requirements.txt"]),
                "cargo" => ("cargo", vec!["fetch"]),
                "go" => ("go", vec!["mod", "download"]),
                _ => return Err(format!("Unknown package_manager: {}", pm)),
            }
        } else {
            let (prog, a) = detect_package_manager(root)
                .ok_or("Could not detect package manager")?;
            (prog, a.to_vec())
        };

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "program": program,
                "args": args,
                "output": "installing..."
            }),
        )
        .await?;

        let output = Command::new(program)
            .args(&args)
            .current_dir(root)
            .output()
            .map_err(|e| format!("Failed to run {}: {}", program, e))?;

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
            CapabilityOutcome::Failure(format!("Exit code {}: {}", exit_code, stderr))
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
                "exit_code": exit_code,
                "stderr": stderr.to_string()
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
            side_effects: vec!["process_execution".to_string(), "network".to_string()],
        })
    }
}
