// Planner Context - Cognitive Snapshot
//
// The LLM does not see raw state.
// It sees a structured cognitive snapshot.
//
// This is what the agent "knows" at decision time.

use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct PlannerContext {
    pub goal: String,
    pub run_status: String,

    pub current_plan: Option<PlanSummary>,
    pub belief_state: BeliefSummary,

    pub last_step: Option<LastStepSummary>,
    pub lessons: Vec<String>,

    /// Recent conversation (assistant questions, user answers) so planner sees clarifications
    pub recent_messages: Vec<(String, String)>,
    pub capabilities: Vec<CapabilitySummary>,
    /// Optional hint from the embedding router (e.g. "file_operation") when confidence was 0.55–0.75.
    pub routed_intent_hint: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct PlanSummary {
    pub summary: String,
    pub steps: Vec<String>,
    pub confidence: f32,
}

#[derive(Clone, Serialize)]
pub struct BeliefSummary {
    pub known_failures: Vec<String>,
    pub known_constraints: Vec<String>,
    pub plan_confidence: f32,
}

#[derive(Clone, Serialize)]
pub struct LastStepSummary {
    pub capability: String,
    pub success: bool,
    pub reason: String,
}

#[derive(Clone, Serialize)]
pub struct CapabilitySummary {
    pub name: String,
    pub description: String,
    pub risk: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}
