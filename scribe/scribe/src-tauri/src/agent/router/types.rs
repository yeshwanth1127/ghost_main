// Router types: route decision, direct commands, environment context

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result of routing a user goal: direct execution, ask user, or defer to planner.
#[derive(Debug, Clone)]
pub enum RouteDecision {
    /// Execute via fast path (single capability, no LLM).
    Direct(DirectCommand),
    /// Ask user for clarification (ambiguous or missing input).
    AskUser(Clarification),
    /// Defer to LLM planner (multi-step, ambiguous, or unsupported).
    DeferToPlanner,
}

/// Full result from the router: decision plus optional intent hint and metadata for logging.
#[derive(Debug, Clone)]
pub struct RouterResult {
    pub decision: RouteDecision,
    /// When deferring to planner, optional hint (e.g. "file_operation") for 0.55–0.75 confidence.
    pub intent_hint: Option<String>,
    /// Predicted intent name for logging.
    pub predicted_intent: String,
    /// Similarity score for logging.
    pub confidence: f32,
    /// "direct" | "llm" | "llm_with_hint" for logging.
    pub final_route: String,
    /// When final_route is "llm" and confidence is 0: "feature_disabled" | "embedding_unavailable" for UI.
    pub fallback_reason: Option<String>,
}

/// Commands that map 1:1 to a single capability and can be executed without the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DirectCommand {
    WriteFile {
        path: String,
        content: String,
        #[serde(default)]
        mode: WriteMode,
    },
    ReadFile {
        path: String,
    },
    ListFiles {
        path: String,
    },
    CreateFile {
        path: String,
    },
    RunCommand {
        cmd: String,
        args: Vec<String>,
    },
}

/// Write semantics (for future use; capability may only support overwrite today).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteMode {
    #[default]
    Overwrite,
    Create,
    Patch,
}

/// Environment context available to the router (e.g. working directory).
#[derive(Debug, Clone)]
pub struct EnvContext {
    pub working_directory: String,
}

/// Suggestion for the user when the goal is ambiguous.
#[derive(Debug, Clone)]
pub struct Clarification {
    pub message: String,
    pub suggested_commands: Vec<String>,
}
