// Planner Decision - Strict JSON Output
//
// The LLM is only allowed to propose one next action.
// This enum represents all valid decision types.
// Optional fields allow minimal LLM output (e.g. capability + inputs only).

use serde::Deserialize;

fn default_confidence() -> f32 {
    0.8
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action")]
pub enum PlannerDecision {
    #[serde(rename = "invoke_capability")]
    InvokeCapability {
        capability: String,
        #[serde(default)]
        intent: Option<String>,
        #[serde(default)]
        expected_outcome: Option<String>,
        #[serde(default)]
        inputs: Option<serde_json::Value>,
        #[serde(default = "default_confidence")]
        confidence: f32,
        #[serde(default)]
        reason: Option<String>,
    },

    #[serde(rename = "revise_plan")]
    RevisePlan {
        reason: String,
        summary: String,
        steps: Vec<String>,
    },

    #[serde(rename = "ask_user")]
    AskUser {
        #[serde(default)]
        question: Option<String>,
        #[serde(default)]
        reason: Option<String>,
    },

    #[serde(rename = "finish")]
    Finish {
        reason: String,
    },
}
