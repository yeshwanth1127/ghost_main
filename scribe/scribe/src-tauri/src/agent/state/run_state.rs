// ⚠️ IMPORTANT: RunState is a DERIVED PROJECTION, not authoritative state.
// 
// - Events in the database are the ONLY source of truth
// - RunState is reconstructed by replaying events
// - This is a VIEW for convenience, not the model
// - Future projections may exist (timeline, audit, debug views)
// - NEVER mutate RunState directly - always emit events

use crate::agent::events::{RunEvent, *};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RunStatus {
    Pending,
    Running,
    WaitingPermission,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RunStatus::Pending => "pending",
            RunStatus::Running => "running",
            RunStatus::WaitingPermission => "waiting_permission",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => RunStatus::Pending,
            "running" => RunStatus::Running,
            "waiting_permission" => RunStatus::WaitingPermission,
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            "cancelled" => RunStatus::Cancelled,
            _ => RunStatus::Pending,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub tool_name: Option<String>,
    pub status: StepStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Started,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub scope: serde_json::Value, // PermissionScope serialized
    pub reason: String,
    pub risk_score: f32,
    pub scope_type: String, // "once" | "run"
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub decision: Option<PermissionDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub granted: bool,
    pub decided_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub kind: ArtifactType,
    pub location: String,
    pub summary: String, // Contains content preview for files, description for other types
    pub source_step: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    // Note: Full content is not stored in artifact to keep state lightweight
    // Artifacts serve as memory markers - the LLM uses summaries to decide what to do next
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    File,
    Directory,
    Text,
    Image,
}

impl std::fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactType::File => write!(f, "File"),
            ArtifactType::Directory => write!(f, "Directory"),
            ArtifactType::Text => write!(f, "Text"),
            ArtifactType::Image => write!(f, "Image"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub id: String,
    pub goal: String,
    pub status: RunStatus,
    pub messages: Vec<Message>,
    pub steps: Vec<Step>,
    pub permissions: Vec<PermissionRequest>,
    pub artifacts: Vec<Artifact>,
}

impl RunState {
    pub fn new(id: String, goal: String) -> Self {
        Self {
            id,
            goal,
            status: RunStatus::Pending,
            messages: Vec::new(),
            steps: Vec::new(),
            permissions: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

/// Pure reducer function - applies events to state
/// This is a projection builder, not authoritative state
pub fn apply_event(state: &mut RunState, event: &RunEvent) {
    match event.event_type.as_str() {
        RUN_CREATED => {
            // Run already created, just ensure status is set
            if state.status == RunStatus::Pending {
                // Already correct
            }
        }
        RUN_STATUS_CHANGED => {
            // PROJECTION: This is derived from other events
            if let Some(status_str) = event.payload.get("status").and_then(|v: &serde_json::Value| v.as_str()) {
                state.status = RunStatus::from_str(status_str);
            }
        }
        MESSAGE_APPENDED => {
            if let Ok(message) = serde_json::from_value::<Message>(event.payload.clone()) {
                state.messages.push(message);
            }
        }
        STEP_STARTED => {
            // PROJECTION: This is derived from tool execution
            if let Ok(step) = serde_json::from_value::<Step>(event.payload.clone()) {
                state.steps.push(step);
            }
        }
        STEP_COMPLETED => {
            // PROJECTION: This is derived from successful tool execution
            if let Some(step_id) = event.payload.get("step_id").and_then(|v: &serde_json::Value| v.as_str()) {
                if let Some(step) = state.steps.iter_mut().find(|s| s.id == step_id) {
                    step.status = StepStatus::Completed;
                    if let Some(completed_at) = event.payload.get("completed_at") {
                        if let Ok(dt) = serde_json::from_value::<chrono::DateTime<chrono::Utc>>(
                            completed_at.clone(),
                        ) {
                            step.completed_at = Some(dt);
                        }
                    }
                }
            }
        }
        STEP_FAILED => {
            // PROJECTION: This is derived from failed tool execution
            if let Some(step_id) = event.payload.get("step_id").and_then(|v: &serde_json::Value| v.as_str()) {
                if let Some(step) = state.steps.iter_mut().find(|s| s.id == step_id) {
                    step.status = StepStatus::Failed;
                    if let Some(error) = event.payload.get("error").and_then(|v: &serde_json::Value| v.as_str()) {
                        step.error = Some(error.to_string());
                    }
                }
            }
        }
        PERMISSION_REQUESTED => {
            if let Ok(permission) =
                serde_json::from_value::<PermissionRequest>(event.payload.clone())
            {
                state.permissions.push(permission);
            }
        }
        PERMISSION_DECISION => {
            if let Some(permission_id) = event
                .payload
                .get("permission_id")
                .and_then(|v: &serde_json::Value| v.as_str())
            {
                if let Some(permission) = state
                    .permissions
                    .iter_mut()
                    .find(|p| p.id == permission_id)
                {
                    if let Ok(decision) =
                        serde_json::from_value::<PermissionDecision>(event.payload.clone())
                    {
                        permission.decision = Some(decision);
                    }
                }
            }
        }
        ARTIFACT_CREATED => {
            // PROJECTION: This is derived from tool output
            if let Ok(artifact) = serde_json::from_value::<Artifact>(event.payload.clone()) {
                state.artifacts.push(artifact);
            }
        }
        _ => {
            // Unknown event types are ignored (no panic)
            // This allows future event types without breaking existing reducers
        }
    }
}
