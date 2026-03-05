// Belief State - Working Memory
//
// Beliefs are interpretations derived from events.
// They answer: "What do I currently believe about the world?"
//
// This is separate from RunState - it's a cognitive projection.
// Beliefs inform decisions but are not authoritative state.

use crate::agent::events::RunEvent;
use crate::agent::state::confidence::normalize_confidence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeliefState {
    /// Known constraints that limit what we can do
    pub known_constraints: Vec<String>,
    
    /// Known failure patterns we've encountered
    pub known_failures: Vec<String>,
    
    /// Current hypothesis about how to achieve the goal
    pub current_hypothesis: Option<String>,
    
    /// Confidence in the current plan (0.0 to 1.0)
    pub plan_confidence: f32,
    
    /// Recent lessons learned from reflections
    pub recent_lessons: Vec<String>,
}

impl BeliefState {
    pub fn new() -> Self {
        Self {
            known_constraints: Vec::new(),
            known_failures: Vec::new(),
            current_hypothesis: None,
            plan_confidence: 0.7, // Start with moderate confidence
            recent_lessons: Vec::new(),
        }
    }
}

/// Apply an event to update belief state
/// This is a pure reducer function - rebuilds beliefs from events
pub fn apply_event(belief: &mut BeliefState, event: &RunEvent) {
    match event.event_type.as_str() {
        crate::agent::events::STEP_EVALUATED => {
            // Update beliefs based on step evaluation
            // Skip if this is a planner failure (check failure_kind if available)
            let is_planner_failure = event.payload.get("failure_kind")
                .and_then(|v| v.as_str())
                .map(|k| k.contains("PlannerUnavailable") || k.contains("PlannerTimeout"))
                .unwrap_or(false);
            
            if is_planner_failure {
                // Planner failures don't affect plan confidence or known failures
                // They're infrastructure issues, not plan problems
                return;
            }
            
            if let Some(success) = event.payload.get("success").and_then(|v| v.as_bool()) {
                if !success {
                    // Step failed - learn from it
                    if let Some(reason) = event.payload.get("reason").and_then(|v| v.as_str()) {
                        let failure_pattern = format!(
                            "{}: {}",
                            event.payload.get("capability")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown"),
                            reason
                        );
                        if !belief.known_failures.contains(&failure_pattern) {
                            belief.known_failures.push(failure_pattern);
                        }
                    }
                }
                
                // Normalize confidence (Step 8 - prevent drift)
                let step_confidence = event.payload.get("confidence")
                    .and_then(|v| v.as_f64())
                    .map(|c| c as f32)
                    .unwrap_or(if success { 0.8 } else { 0.2 });
                
                // Get retry count from event (if available)
                let retry_count = event.payload.get("retry_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                
                // Use normalized confidence update
                belief.plan_confidence = normalize_confidence(
                    belief.plan_confidence,
                    success,
                    step_confidence,
                    retry_count,
                );
            }
        }
        crate::agent::events::PLAN_CREATED | crate::agent::events::PLAN_REVISED => {
            // Update hypothesis when plan changes
            if let Some(summary) = event.payload.get("summary").and_then(|v| v.as_str()) {
                belief.current_hypothesis = Some(summary.to_string());
            }
            // Reset confidence when plan is created/revised
            if event.event_type == crate::agent::events::PLAN_CREATED {
                belief.plan_confidence = 0.7;
            } else {
                // Plan revision suggests uncertainty
                belief.plan_confidence *= 0.9;
            }
        }
        crate::agent::events::RUN_REFLECTED => {
            // Learn from reflections
            if let Some(lessons) = event.payload.get("lessons_learned")
                .and_then(|v| v.as_array()) {
                for lesson in lessons {
                    if let Some(lesson_str) = lesson.as_str() {
                        if !belief.recent_lessons.contains(&lesson_str.to_string()) {
                            belief.recent_lessons.push(lesson_str.to_string());
                        }
                    }
                }
            }
            // Keep only recent lessons (last 10)
            if belief.recent_lessons.len() > 10 {
                belief.recent_lessons.drain(0..belief.recent_lessons.len() - 10);
            }
        }
        crate::agent::events::STEP_FAILED => {
            // Learn from step failures (but NOT planner failures)
            // Planner failures are handled separately and don't affect confidence
            if let Some(error) = event.payload.get("error").and_then(|v| v.as_str()) {
                let failure = format!("Step failed: {}", error);
                if !belief.known_failures.contains(&failure) {
                    belief.known_failures.push(failure);
                }
                belief.plan_confidence *= 0.85;
            }
        }
        crate::agent::events::PLANNER_FAILED => {
            // Planner failures are infrastructure issues, not plan failures
            // Do NOT reduce plan confidence
            // Do NOT add to known_failures (these are transient)
            // Just log for UI visibility
            // The UI should show: "Planner Status: ❌ Unreachable" without affecting plan
        }
        _ => {
            // Other events don't directly update beliefs
        }
    }
}

/// Load belief state by replaying events
pub fn load_belief_state(events: &[RunEvent]) -> BeliefState {
    let mut belief = BeliefState::new();
    for event in events {
        apply_event(&mut belief, event);
    }
    belief
}
