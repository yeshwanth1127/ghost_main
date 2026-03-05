// Project run_dev_server - start dev server (non-blocking, streams logs)

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct ProjectRunDevServer;

fn detect_dev_command(path: &Path) -> Option<(&'static str, Vec<&'static str>)> {
    if path.join("package.json").exists() {
        return Some(("npm", vec!["run", "dev"]));
    }
    if path.join("Cargo.toml").exists() {
        return Some(("cargo", vec!["run"]));
    }
    if path.join("manage.py").exists() {
        return Some(("python", vec!["manage.py", "runserver"]));
    }
    if path.join("go.mod").exists() {
        return Some(("go", vec!["run", "."]));
    }
    None
}

#[async_trait]
impl Capability for ProjectRunDevServer {
    fn name(&self) -> &'static str {
        "project.run_dev_server"
    }

    fn description(&self) -> &'static str {
        "Start development server (runs in background, streams logs)"
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
                "port": {
                    "type": "integer",
                    "description": "Optional port (e.g. 3000, 5173)"
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
        if detect_dev_command(p).is_none() {
            return PreflightResult::Reject("Could not detect dev server command".to_string());
        }
        PreflightResult::NeedsPermission(PermissionRequest {
            reason: format!("Run dev server at {}", path),
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
        let port = inputs.get("port").and_then(|v| v.as_i64());
        let root = Path::new(path);

        let (program, mut args) = detect_dev_command(root)
            .ok_or("Could not detect dev server command")?;

        let port_str = port.map(|p| p.to_string());
        if let Some(ref p) = port_str {
            match program {
                "npm" => args.extend(["--", "--port", p.as_str()]),
                _ => {}
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
                "output": "starting dev server..."
            }),
        )
        .await?;

        let mut cmd = Command::new(program);
        cmd.args(&args)
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(p) = port {
            cmd.env("PORT", p.to_string());
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start dev server: {}", e))?;

        let pid = child.id();
        std::mem::forget(child); // Keep process running in background
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::PROCESS_COMPLETED,
            json!({
                "program": program,
                "pid": pid,
                "status": "started"
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
                "completed_at": chrono::Utc::now(),
                "pid": pid,
                "note": "Dev server running in background. Use process.kill to stop."
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "program": program,
                "args": args,
                "pid": pid,
                "path": root.to_string_lossy().to_string(),
                "port": port,
                "status": "running"
            })],
            side_effects: vec!["process_execution".to_string(), "network".to_string()],
        })
    }
}
