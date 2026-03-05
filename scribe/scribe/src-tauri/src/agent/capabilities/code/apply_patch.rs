// Code apply_patch capability - apply unified diff patch

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, PermissionRequest, InputRequest, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct CodeApplyPatch;

#[async_trait]
impl Capability for CodeApplyPatch {
    fn name(&self) -> &'static str {
        "code.apply_patch"
    }

    fn description(&self) -> &'static str {
        "Apply a unified diff patch to files (safer than full-file overwrite)"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &["disk_write"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }

    fn requires_permission(&self) -> bool {
        true
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &["file"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo_path": {
                    "type": "string",
                    "description": "Repository/project root path"
                },
                "patch": {
                    "type": "string",
                    "description": "Unified diff patch content"
                }
            },
            "required": ["repo_path", "patch"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let repo_path = inputs.get("repo_path").and_then(|v| v.as_str()).unwrap_or("").trim();
        let patch = inputs.get("patch").and_then(|v| v.as_str()).unwrap_or("").trim();
        let mut missing = Vec::new();
        if repo_path.is_empty() {
            missing.push("repo_path".to_string());
        }
        if patch.is_empty() {
            missing.push("patch".to_string());
        }
        if !missing.is_empty() {
            return PreflightResult::NeedsInput(InputRequest {
                missing_fields: missing,
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        if self.requires_permission() {
            return PreflightResult::NeedsPermission(PermissionRequest {
                reason: "Apply patch to files".to_string(),
            });
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let repo_path = inputs["repo_path"]
            .as_str()
            .ok_or_else(|| "Missing repo_path argument".to_string())?;
        let patch_content = inputs["patch"]
            .as_str()
            .ok_or_else(|| "Missing patch argument".to_string())?;

        let root = Path::new(repo_path);

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": {
                    "repo_path": repo_path,
                    "patch_length": patch_content.len()
                },
                "output": "applying patch..."
            }),
        )
        .await?;

        let hunks = parse_unified_diff(patch_content)?;
        let mut modified_files: Vec<String> = Vec::new();

        // Group hunks by file
        let mut file_hunks: std::collections::HashMap<String, Vec<DiffHunk>> = std::collections::HashMap::new();
        for hunk in hunks {
            file_hunks.entry(hunk.file_path.clone()).or_default().push(hunk);
        }

        for (rel_path, file_hunk_list) in file_hunks {
            let file_path = root.join(&rel_path);
            let mut content = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

            // Apply hunks from bottom to top so line numbers don't shift
            let mut sorted = file_hunk_list;
            sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line));
            for hunk in &sorted {
                content = apply_hunk(&content, hunk)?;
            }
            fs::write(&file_path, &content)
                .map_err(|e| format!("Failed to write {}: {}", file_path.display(), e))?;
            modified_files.push(file_path.to_string_lossy().to_string());
        }

        for path in &modified_files {
            store::append_event(
                &ctx.app,
                &ctx.run_id,
                crate::agent::events::FILE_WRITTEN,
                json!({
                    "path": path
                }),
            )
            .await?;
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
                "modified_files": modified_files,
                "count": modified_files.len()
            })],
            side_effects: vec!["disk_write".to_string()],
        })
    }
}

struct DiffHunk {
    file_path: String,
    start_line: usize,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn parse_unified_diff(patch: &str) -> Result<Vec<DiffHunk>, String> {
    let mut hunks = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_old: Vec<String> = Vec::new();
    let mut current_new: Vec<String> = Vec::new();
    let mut current_start: usize = 0;

    for line in patch.lines() {
        if line.starts_with("--- ") {
            if let Some(path) = current_file.take() {
                if !current_old.is_empty() || !current_new.is_empty() {
                    hunks.push(DiffHunk {
                        file_path: path,
                        start_line: current_start,
                        old_lines: std::mem::take(&mut current_old),
                        new_lines: std::mem::take(&mut current_new),
                    });
                }
            }
            let path = line[4..].trim().split_whitespace().next().unwrap_or("").trim();
            let path = path.strip_prefix("a/").unwrap_or(path).to_string();
            current_file = Some(path);
        } else if line.starts_with("@@ ") {
            if let Some(ref path) = current_file {
                if !current_old.is_empty() || !current_new.is_empty() {
                    hunks.push(DiffHunk {
                        file_path: path.clone(),
                        start_line: current_start,
                        old_lines: std::mem::take(&mut current_old),
                        new_lines: std::mem::take(&mut current_new),
                    });
                }
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let old_info = parts[1];
                if let Some(comma) = old_info.find(',') {
                    current_start = old_info[1..comma].parse().unwrap_or(1);
                } else {
                    current_start = old_info[1..].parse().unwrap_or(1);
                }
            }
        } else if line.starts_with('-') && !line.starts_with("---") {
            current_old.push(line[1..].to_string());
        } else if line.starts_with('+') && !line.starts_with("+++") {
            current_new.push(line[1..].to_string());
        } else if line.starts_with(' ') {
            current_old.push(line[1..].to_string());
            current_new.push(line[1..].to_string());
        }
    }

    if let Some(path) = current_file {
        if !current_old.is_empty() || !current_new.is_empty() {
            hunks.push(DiffHunk {
                file_path: path,
                start_line: current_start,
                old_lines: current_old,
                new_lines: current_new,
            });
        }
    }

    if hunks.is_empty() {
        return Err("No valid hunks in patch".to_string());
    }
    Ok(hunks)
}

fn apply_hunk(content: &str, hunk: &DiffHunk) -> Result<String, String> {
    let lines: Vec<&str> = content.lines().collect();
    let start = hunk.start_line.saturating_sub(1);
    if start > lines.len() {
        return Err(format!("Hunk start line {} out of bounds (file has {} lines)", hunk.start_line, lines.len()));
    }

    let mut result: Vec<String> = lines[..start].iter().map(|s| s.to_string()).collect();
    for new_line in &hunk.new_lines {
        result.push(new_line.clone());
    }
    let skip = hunk.old_lines.len();
    let end = (start + skip).min(lines.len());
    for line in &lines[end..] {
        result.push((*line).to_string());
    }
    Ok(result.join("\n"))
}
