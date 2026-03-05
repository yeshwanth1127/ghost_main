// Context Builder - Build Planner Context from Agent State
//
// This module builds the cognitive snapshot that the Planner uses.

use super::context::{PlannerContext, PlanSummary, BeliefSummary, LastStepSummary, CapabilitySummary};
use crate::agent::state::RunState;
use crate::agent::state::BeliefState;
use crate::agent::capabilities::registry::CapabilityRegistry;
use crate::agent::events::RunEvent;

/// Build planner context from agent state.
/// `routed_intent_hint`: optional hint from the embedding router (first planner call only).
pub async fn build_planner_context(
    run_state: &RunState,
    belief_state: &BeliefState,
    registry: &CapabilityRegistry,
    last_step_event: Option<&RunEvent>,
    routed_intent_hint: Option<String>,
) -> PlannerContext {
    // Build current plan summary
    let current_plan = run_state.plan.as_ref().map(|plan| PlanSummary {
        summary: plan.summary.clone(),
        steps: plan.steps.clone(),
        confidence: belief_state.plan_confidence,
    });

    // Build belief summary
    let belief_summary = BeliefSummary {
        known_failures: belief_state.known_failures.clone(),
        known_constraints: belief_state.known_constraints.clone(),
        plan_confidence: belief_state.plan_confidence,
    };

    // Build last step summary
    let last_step = last_step_event.and_then(|event| {
        if event.event_type == crate::agent::events::STEP_EVALUATED {
            let success = event.payload.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            let capability = event.payload.get("capability")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let reason = event.payload.get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            Some(LastStepSummary {
                capability,
                success,
                reason,
            })
        } else {
            None
        }
    });

    // Build capability summaries with input schemas
    let capability_descriptors = registry.get_all_descriptors().await;
    let capabilities: Vec<CapabilitySummary> = capability_descriptors
        .into_iter()
        .map(|desc| CapabilitySummary {
            name: desc.name.clone(),
            description: desc.description,
            risk: format!("{:?}", desc.risk_level),
            input_schema: Some(desc.input_schema), // Include full schema for LLM
        })
        .collect();

    // Extract lessons from belief state
    let lessons = belief_state.recent_lessons.clone();

    // Last N messages (role, content) so planner sees user replies to ask_user
    const MAX_RECENT_MESSAGES: usize = 20;
    let recent_messages: Vec<(String, String)> = run_state
        .messages
        .iter()
        .rev()
        .take(MAX_RECENT_MESSAGES)
        .map(|m| (m.role.clone(), m.content.clone()))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    PlannerContext {
        goal: run_state.goal.clone(),
        run_status: run_state.status.as_str().to_string(),
        current_plan,
        belief_state: belief_summary,
        last_step,
        lessons,
        recent_messages,
        capabilities,
        routed_intent_hint,
    }
}
