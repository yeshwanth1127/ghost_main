//! Execution ticket schema - one state machine per tool call (Moltbot-style).
//! Restart-safe, auditable; permission flow: pending -> granted/denied -> running -> completed/failed.

use serde::{Deserialize, Serialize};

/// Canonicalized intent for a single tool call (permissions, UI, auditing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIntent {
    /// User-friendly description of the action.
    pub human_readable: String,
    /// How this aligns with the user goal (e.g. "create file", "read information").
    pub goal_alignment: String,
    /// Whether this action is irreversible (e.g. overwrite, delete).
    pub irreversible: bool,
    /// Risk factors (e.g. "system_path", "destructive_command").
    pub risk_factors: Vec<String>,
    /// Optional context (user goal, previous actions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ToolIntentContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIntentContext {
    pub user_goal: Option<String>,
    pub previous_actions: Option<Vec<String>>,
}

/// Execution ticket: single authoritative state for one tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTicket {
    pub ticket_id: String,
    pub run_id: String,
    pub step_id: String,
    pub capability: String,
    pub inputs: serde_json::Value,
    pub canonical_intent: ToolIntent,
    pub expected_outcome: String,
    pub permission_state: PermissionState,
    pub execution_state: ExecutionState,
    pub permission_id: Option<String>,
    pub execution_result: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: i64,
    pub permission_granted_at: Option<i64>,
    pub execution_started_at: Option<i64>,
    pub execution_completed_at: Option<i64>,
    pub retry_count: i32,
    pub max_retries: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Requested,
    Granted,
    Denied,
    AutoApproved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl PermissionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionState::Requested => "requested",
            PermissionState::Granted => "granted",
            PermissionState::Denied => "denied",
            PermissionState::AutoApproved => "auto_approved",
        }
    }
}

impl ExecutionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionState::Pending => "pending",
            ExecutionState::Running => "running",
            ExecutionState::Completed => "completed",
            ExecutionState::Failed => "failed",
            ExecutionState::Cancelled => "cancelled",
        }
    }
}
