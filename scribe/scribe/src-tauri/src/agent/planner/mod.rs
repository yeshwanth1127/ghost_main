// Planner Module
//
// The Planner is the ONLY component that calls the LLM.
// It proposes the next action based on cognitive context.
//
// Rules:
// - Planner never executes, retries, evaluates, or mutates state
// - Planner output must be strict JSON
// - All Planner output is validated before use
// - All consequences happen via events (handled by Agent Loop)

pub mod context;
pub mod context_builder;
pub mod decision;
pub mod prompt;
pub mod validate;

use context::PlannerContext;
use decision::PlannerDecision;
use crate::agent::llm::ollama::OllamaClient;
use prompt::{build_user_prompt, SYSTEM_PROMPT};
use validate::validate_decision;

pub struct Planner {
    llm: OllamaClient,
}

impl Planner {
    pub fn new(model: &str) -> Self {
        Self {
            llm: OllamaClient::new(model),
        }
    }

    /// Decide the next action based on cognitive context
    /// Returns a validated PlannerDecision
    pub async fn decide_next(
        &self,
        ctx: PlannerContext,
        available_capabilities: &[String],
        registry: &crate::agent::capabilities::registry::CapabilityRegistry,
    ) -> Result<PlannerDecision, String> {
        let system = SYSTEM_PROMPT;
        let user = build_user_prompt(&ctx);

        // Call LLM
        let raw = self.llm.chat(system, &user).await?;

        // Parse JSON
        let decision: PlannerDecision = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid planner JSON: {}. Raw response: {}", e, raw))?;

        // Validate decision (including schema validation)
        validate_decision(&decision, available_capabilities, registry).await?;

        Ok(decision)
    }
}
