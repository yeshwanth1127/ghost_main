// Env read capability - read .env file

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct EnvRead;

#[async_trait]
impl Capability for EnvRead {
    fn name(&self) -> &'static str {
        "env.read"
    }

    fn description(&self) -> &'static str {
        "Read .env file contents"
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
        &["env_vars"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to .env file (default: .env in project root)"
                }
            },
            "required": []
        })
    }

    fn preflight(&self, _inputs: &Value) -> PreflightResult {
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let path = inputs.get("path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(".env");

        let path_buf = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "reading..."
            }),
        )
        .await?;

        let content = fs::read_to_string(path_buf)
            .map_err(|e| format!("Failed to read {}: {}", path_buf.display(), e))?;

        let mut vars: Vec<Value> = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                vars.push(json!({
                    "key": key,
                    "value": value,
                    "masked": value.len() > 4 && (key.to_lowercase().contains("secret") || key.to_lowercase().contains("password") || key.to_lowercase().contains("key"))
                }));
            }
        }

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
                "path": path_buf.to_string_lossy().to_string(),
                "vars": vars,
                "count": vars.len()
            })],
            side_effects: vec!["disk_read".to_string()],
        })
    }
}
