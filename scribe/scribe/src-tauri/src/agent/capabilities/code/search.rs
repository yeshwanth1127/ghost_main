// Code search capability - search for pattern/symbol in project

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct CodeSearch;

#[async_trait]
impl Capability for CodeSearch {
    fn name(&self) -> &'static str {
        "code.search"
    }

    fn description(&self) -> &'static str {
        "Search for pattern or symbol in project files"
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
        &["search_results"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Root path to search in"
                },
                "query": {
                    "type": "string",
                    "description": "Search pattern (regex supported)"
                },
                "file_types": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional file extensions to filter (e.g. [\"rs\", \"ts\"])"
                }
            },
            "required": ["path", "query"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let path = inputs.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
        let query = inputs.get("query").and_then(|v| v.as_str()).unwrap_or("").trim();
        let mut missing = Vec::new();
        if path.is_empty() {
            missing.push("path".to_string());
        }
        if query.is_empty() {
            missing.push("query".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        // Validate regex
        if Regex::new(query).is_err() {
            return PreflightResult::Reject("Invalid regex pattern".to_string());
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
            .ok_or_else(|| "Missing path argument".to_string())?;
        let query = inputs["query"]
            .as_str()
            .ok_or_else(|| "Missing query argument".to_string())?;
        let file_types: Vec<String> = inputs["file_types"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let re = Regex::new(query).map_err(|e| format!("Invalid regex: {}", e))?;
        let root = Path::new(path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": "searching..."
            }),
        )
        .await?;

        let mut matches: Vec<Value> = Vec::new();
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.file_name().map(|n| n.to_string_lossy().starts_with('.')).unwrap_or(false) {
                continue;
            }
            if !file_types.is_empty() {
                let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                if !file_types.iter().any(|t| t.trim_start_matches('.') == ext) {
                    continue;
                }
            }
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for (line_num, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    let line_no = line_num + 1;
                    matches.push(json!({
                        "file": path.to_string_lossy().to_string(),
                        "line": line_no,
                        "content": line
                    }));
                }
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

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            crate::agent::events::ARTIFACT_CREATED,
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "kind": "search_results",
                "summary": format!("Found {} matches for '{}'", matches.len(), query),
                "source_step": step_id,
                "created_at": chrono::Utc::now()
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({
                "matches": matches,
                "count": matches.len(),
                "query": query
            })],
            side_effects: vec!["disk_read".to_string()],
        })
    }
}
