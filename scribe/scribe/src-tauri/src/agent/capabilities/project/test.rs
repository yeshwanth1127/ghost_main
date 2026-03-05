// Project test - run tests with structured results

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub struct ProjectTest;

fn detect_test_command(path: &Path) -> Option<(&'static str, Vec<&'static str>)> {
    if path.join("package.json").exists() {
        return Some(("npm", vec!["test"]));
    }
    if path.join("Cargo.toml").exists() {
        return Some(("cargo", vec!["test"]));
    }
    if path.join("pytest.ini").exists() || path.join("pyproject.toml").exists() {
        return Some(("pytest", vec!["-v"]));
    }
    if path.join("requirements.txt").exists() {
        return Some(("python", vec!["-m", "pytest", "-v"]));
    }
    if path.join("go.mod").exists() {
        return Some(("go", vec!["test", "./..."]));
    }
    None
}

#[async_trait]
impl Capability for ProjectTest {
    fn name(&self) -> &'static str {
        "project.test"
    }

    fn description(&self) -> &'static str {
        "Run project tests, returns structured results (passed, failed, failures list)"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["process_execution"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["test_results"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Project root path"
                },
                "filter": {
                    "type": "string",
                    "description": "Optional test filter (e.g. test name pattern)"
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
        let filter = inputs.get("filter").and_then(|v| v.as_str());
        let root = Path::new(path);

        let (program, mut args) = detect_test_command(root)
            .unwrap_or(("npm", vec!["test"]));

        if let Some(f) = filter {
            match program {
                "cargo" => args = vec!["test", f],
                "pytest" | "python" => args.extend(["-k", f]),
                "go" => args = vec!["test", "-run", f, "./..."],
                _ => args.extend(["--", "--grep", f]),
            }
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
                "output": "running tests..."
            }),
        )
        .await?;

        let mut cmd = Command::new(program);
        cmd.args(&args).current_dir(root);
        let output = cmd.output()
            .map_err(|e| format!("Test run failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        let failures: Vec<Value> = if !success {
            vec![json!({
                "exit_code": exit_code,
                "stderr": stderr.to_string(),
                "stdout": stdout.to_string()
            })]
        } else {
            vec![]
        };

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

        let outcome = if success {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("Tests failed: {}", stderr))
        };

        let step_id = uuid::Uuid::new_v4().to_string();
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            if success {
                crate::agent::events::STEP_COMPLETED
            } else {
                crate::agent::events::STEP_FAILED
            },
            json!({
                "step_id": step_id,
                "completed_at": chrono::Utc::now(),
                "passed": success,
                "exit_code": exit_code
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome,
            artifacts: vec![json!({
                "passed": success,
                "failed": !success,
                "failures": failures,
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "exit_code": exit_code
            })],
            side_effects: vec!["process_execution".to_string()],
        })
    }
}
