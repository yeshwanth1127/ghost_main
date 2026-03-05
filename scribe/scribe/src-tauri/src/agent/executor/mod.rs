// Executor - Centralized execution control
//
// The Executor is the single choke point for all capability execution.
// It owns:
// - Capability execution
// - Outcome evaluation
// - Event emission (STEP_EVALUATED, FAILURE_ANALYZED)
//
// It does NOT own:
// - Retry logic (that's policy, handled by Agent Loop)
// - Recovery decisions (that's policy, handled by Agent Loop)
// - Timeout handling (future: can be added here)
//
// LLMs NEVER touch this layer - it's pure mechanics.
// Policy decisions happen in Agent Loop, not here.

pub mod failure;

use crate::agent::capabilities::{CapabilityContext, CapabilityOutcome};
use crate::agent::capabilities::registry::CapabilityRegistry;
use crate::agent::run::store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

/// Request to execute a capability
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub run_id: String,
    pub step_id: String,
    pub capability: String,
    pub intent: String,
    pub expected_outcome: String,
    pub inputs: Value,
    pub retry_count: u32, // How many times we've retried (0 = first attempt)
}

/// Result of execution (normalized)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub confidence: f32,
    pub reason: String,
    pub failure_kind: Option<failure::FailureKind>,
}

/// Executor - central execution control
/// This is a stateless executor that uses the registry from app state
pub struct Executor;

impl Executor {
    /// Execute a capability and evaluate the outcome
    /// This is the single point where execution happens
    pub async fn execute(
        app: &AppHandle,
        req: ExecutionRequest,
    ) -> Result<ExecutionResult, String> {
        // Get capability registry from app state
        let registry = app
            .try_state::<CapabilityRegistry>()
            .ok_or("Capability registry not initialized")?;
        
        // Get capability from registry
        let capability = registry
            .get(&req.capability)
            .await
            .ok_or_else(|| format!("Unknown capability: {}", req.capability))?;

        // Load current state for capability context
        let current_state = store::load_run_state(app, &req.run_id).await?;

        // Create capability context
        let ctx = CapabilityContext {
            app: app.clone(),
            run_id: req.run_id.clone(),
            state: current_state,
        };

        // Execute capability (this emits TOOL_EXECUTED, domain events, STEP_COMPLETED, ARTIFACT_CREATED)
        let capability_result = capability.execute(ctx, req.inputs.clone()).await;

        // Evaluate outcome
        let (success, confidence, reason, failure_kind) = match &capability_result {
            Ok(result) => {
                match &result.outcome {
                    CapabilityOutcome::Success => {
                        // Check if outcome matches expectation
                        let expected_lower = req.expected_outcome.to_lowercase();
                        let matches_expectation = expected_lower.contains("success")
                            || expected_lower.contains("available")
                            || expected_lower.contains("created")
                            || expected_lower.contains("completed");

                        if matches_expectation {
                            (
                                true,
                                0.85,
                                format!("Outcome matches expectation: {}", req.expected_outcome),
                                None,
                            )
                        } else {
                            (
                                true,
                                0.7,
                                "Operation succeeded but outcome differs from expectation".to_string(),
                                None,
                            )
                        }
                    }
                    CapabilityOutcome::Partial => {
                        (
                            false,
                            0.5,
                            "Partial success - may need follow-up action".to_string(),
                            Some(failure::FailureKind::ExecutionError),
                        )
                    }
                    CapabilityOutcome::Failure(err) => {
                        // Classify failure
                        let failure_kind = failure::classify_failure(err);
                        let reason = format!("Execution failed: {}", err);
                        (
                            false,
                            0.2,
                            reason,
                            Some(failure_kind),
                        )
                    }
                }
            }
            Err(e) => {
                // Execution error
                let failure_kind = failure::classify_failure(e);
                (
                    false,
                    0.1,
                    format!("Execution error: {}", e),
                    Some(failure_kind),
                )
            }
        };

        // Emit STEP_EVALUATED event (judgment, not fact)
        // Executor evaluates single execution attempt
        // retry_count is provided by Agent Loop (policy layer)
        store::append_event(
            app,
            &req.run_id,
            crate::agent::events::STEP_EVALUATED,
            json!({
                "step_id": req.step_id,
                "capability": req.capability,
                "intent": req.intent,
                "expected_outcome": req.expected_outcome,
                "actual_outcome": if success {
                    "Operation completed successfully"
                } else {
                    &reason
                },
                "success": success,
                "confidence": confidence,
                "reason": reason,
                "failure_kind": failure_kind.as_ref().map(|k| format!("{:?}", k)),
                "retry_count": req.retry_count, // For confidence normalization
            }),
        )
        .await?;

        // If failure occurred, emit failure.analyzed event
        if let Some(kind) = &failure_kind {
            store::append_event(
                app,
                &req.run_id,
                crate::agent::events::FAILURE_ANALYZED,
                json!({
                    "step_id": req.step_id,
                    "kind": format!("{:?}", kind),
                    "reason": reason,
                }),
            )
            .await?;
        }

        Ok(ExecutionResult {
            success,
            confidence,
            reason,
            failure_kind,
        })
    }
}
