// Artifact extraction and summarization

use crate::agent::state::run_state::{Artifact, ArtifactType};
use crate::agent::state::run_state::RunState;

/// Extract artifact from tool output
pub fn extract_artifact(
    tool_name: &str,
    tool_output: &serde_json::Value,
    step_id: &str,
) -> Artifact {
    let artifact_id = uuid::Uuid::new_v4().to_string();
    
    match tool_name {
        "fs_read" | "fs_write" => {
            let location = tool_output
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            let size = tool_output
                .get("size")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            Artifact {
                id: artifact_id,
                kind: ArtifactType::File,
                location,
                summary: format!("{}: {} bytes", tool_name, size),
                source_step: step_id.to_string(),
                created_at: chrono::Utc::now(),
            }
        }
        _ => {
            Artifact {
                id: artifact_id,
                kind: ArtifactType::Text,
                location: "unknown".to_string(),
                summary: format!("Tool output from {}", tool_name),
                source_step: step_id.to_string(),
                created_at: chrono::Utc::now(),
            }
        }
    }
}

/// Generate summary for artifact (simple heuristic for now)
/// In the future, this could use a cheap LLM call
pub fn summarize_artifact(artifact: &Artifact) -> String {
    match artifact.kind {
        ArtifactType::File => {
            format!("File: {}", artifact.location)
        }
        ArtifactType::Directory => {
            format!("Directory: {}", artifact.location)
        }
        ArtifactType::Text => {
            artifact.summary.clone()
        }
        ArtifactType::Image => {
            format!("Image: {}", artifact.location)
        }
    }
}

/// Get relevant artifacts for observation
/// Filters artifacts by simple keyword matching
pub fn get_relevant_artifacts(
    run_state: &RunState,
    keywords: &[String],
) -> Vec<Artifact> {
    if keywords.is_empty() {
        // Return all artifacts if no keywords
        return run_state.artifacts.clone();
    }

    run_state
        .artifacts
        .iter()
        .filter(|artifact| {
            keywords.iter().any(|keyword| {
                artifact.location.to_lowercase().contains(&keyword.to_lowercase())
                    || artifact.summary.to_lowercase().contains(&keyword.to_lowercase())
            })
        })
        .cloned()
        .collect()
}
