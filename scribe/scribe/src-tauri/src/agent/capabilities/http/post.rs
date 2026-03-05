// HTTP POST capability

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct HttpPost;

#[async_trait]
impl Capability for HttpPost {
    fn name(&self) -> &'static str {
        "http.post"
    }

    fn description(&self) -> &'static str {
        "Perform HTTP POST request with body"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["network"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn requires_permission(&self) -> bool {
        true
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["http_response"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to POST to"
                },
                "body": {
                    "type": "string",
                    "description": "Request body (JSON or plain text)"
                },
                "headers": {
                    "type": "object",
                    "description": "Optional request headers"
                }
            },
            "required": ["url", "body"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let url = inputs.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
        let body = inputs.get("body").and_then(|v| v.as_str());
        let mut missing = Vec::new();
        if url.is_empty() {
            missing.push("url".to_string());
        }
        if body.is_none() {
            missing.push("body".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return PreflightResult::Reject("URL must start with http:// or https://".to_string());
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let url = inputs["url"]
            .as_str()
            .ok_or_else(|| "Missing url".to_string())?;
        let body = inputs["body"]
            .as_str()
            .ok_or_else(|| "Missing body".to_string())?;
        let headers = inputs.get("headers");

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": { "url": url, "body_length": body.len() },
                "output": "posting..."
            }),
        )
        .await?;

        let mut req = reqwest::Client::new().post(url).body(body.to_string());
        if let Some(h) = headers.and_then(|v| v.as_object()) {
            for (k, v) in h {
                if let Some(s) = v.as_str() {
                    req = req.header(k.as_str(), s);
                }
            }
        }
        if !headers.and_then(|h| h.get("Content-Type")).is_some() {
            req = req.header("Content-Type", "application/json");
        }

        let resp = req.send().await
            .map_err(|e| format!("HTTP POST failed: {}", e))?;
        let status = resp.status().as_u16();
        let resp_body = resp.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let step_id = uuid::Uuid::new_v4().to_string();
        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::STEP_COMPLETED,
            json!({
                "step_id": step_id,
                "completed_at": chrono::Utc::now(),
                "status": status
            }),
        )
        .await?;

        let outcome = if (200..300).contains(&status) {
            CapabilityOutcome::Success
        } else {
            CapabilityOutcome::Failure(format!("HTTP {}: {}", status, &resp_body[..resp_body.len().min(200)]))
        };

        Ok(CapabilityResult {
            outcome,
            artifacts: vec![json!({
                "url": url,
                "status": status,
                "body": resp_body,
                "success": (200..300).contains(&status)
            })],
            side_effects: vec!["network".to_string()],
        })
    }
}
