// Agent loop orchestrator

use crate::agent::events::RunEvent;
use crate::agent::executor::{Executor, ExecutionRequest};
use crate::agent::router::{route_goal_with_result, run_direct_path, EnvContext, RouteDecision};
use crate::agent::executor::failure::{recovery_for, RecoveryAction};
use crate::agent::execution_ticket::{
    create_ticket, mark_execution_completed, mark_execution_failed, mark_execution_started,
    mark_permission_denied, mark_permission_granted, PermissionState, ExecutionState as TicketExecutionState,
};
use crate::agent::intent::canonicalize_tool_intent;
use crate::agent::permissions::is_auto_approved;
use crate::agent::run::store;
use crate::agent::state::run_state::RunState;
use crate::agent::state::{CONFIDENCE_LOW, CONFIDENCE_CRITICAL};
use crate::agent::state::plan_revision::{RevisionState, should_allow_revision, record_revision, record_failure, record_success};
use crate::agent::planner::{Planner, context_builder};
use crate::agent::planner::decision::PlannerDecision;
use crate::agent::planner::prompt::{build_user_prompt, SYSTEM_PROMPT};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_util::sync::CancellationToken;
use tauri::{AppHandle, Manager};

/// Observation model - data collection only, no reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub run_state: RunState,
    pub recent_events: Vec<RunEvent>,
    pub environment: EnvironmentSnapshot,
    pub last_decision_event_id: Option<i64>, // Track last decision point for determinism
    pub belief_state: Option<crate::agent::state::BeliefState>, // Working memory
    pub recent_reflections: Vec<RunEvent>, // Lessons learned from previous runs
}

/// Environment snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub current_directory: String,
    pub working_directory: String,
}

/// Decision from LLM - capability-based action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Decision {
    /// New capability-based format
    CapabilityBased {
        #[serde(rename = "action_type")]
        action_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        capability: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inputs: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_outcome: Option<String>,
        confidence: f32,
        reason: String,
    },
    /// Legacy format for backward compatibility
    Legacy {
        action: String,
        args: serde_json::Value,
        confidence: f32,
        reason: String,
    },
}

impl Decision {
    pub fn action_type(&self) -> &str {
        match self {
            Decision::CapabilityBased { action_type, .. } => action_type,
            Decision::Legacy { action, .. } => {
                // Map legacy actions to new action types
                match action.as_str() {
                    "fs_read" | "fs_write" => "invoke_capability",
                    _ => action,
                }
            }
        }
    }

    pub fn capability(&self) -> Option<String> {
        match self {
            Decision::CapabilityBased { capability, .. } => capability.clone(),
            Decision::Legacy { action, .. } => {
                // Map legacy actions to capability names
                match action.as_str() {
                    "fs_read" => Some("filesystem.read".to_string()),
                    "fs_write" => Some("filesystem.write".to_string()),
                    _ => None,
                }
            }
        }
    }

    pub fn intent(&self) -> Option<&String> {
        match self {
            Decision::CapabilityBased { intent, .. } => intent.as_ref(),
            Decision::Legacy { .. } => None,
        }
    }

    pub fn inputs(&self) -> Option<&serde_json::Value> {
        match self {
            Decision::CapabilityBased { inputs, .. } => inputs.as_ref(),
            Decision::Legacy { args, .. } => Some(args),
        }
    }

    pub fn expected_outcome(&self) -> Option<&String> {
        match self {
            Decision::CapabilityBased { expected_outcome, .. } => expected_outcome.as_ref(),
            Decision::Legacy { .. } => None,
        }
    }

    pub fn confidence(&self) -> f32 {
        match self {
            Decision::CapabilityBased { confidence, .. } => *confidence,
            Decision::Legacy { confidence, .. } => *confidence,
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Decision::CapabilityBased { reason, .. } => reason,
            Decision::Legacy { reason, .. } => reason,
        }
    }

    pub fn to_capability_based(self) -> Self {
        match self {
            Decision::Legacy { action, args, confidence, reason } => {
                let (capability, intent, expected_outcome) = match action.as_str() {
                    "fs_read" => (
                        Some("filesystem.read".to_string()),
                        Some(format!("Read file: {}", args.get("path").and_then(|v| v.as_str()).unwrap_or(""))),
                        Some("File contents available for inspection".to_string()),
                    ),
                    "fs_write" => (
                        Some("filesystem.write".to_string()),
                        Some("Write content to file".to_string()),
                        Some("File written successfully".to_string()),
                    ),
                    _ => (None, None, None),
                };

                Decision::CapabilityBased {
                    action_type: if capability.is_some() {
                        "invoke_capability".to_string()
                    } else {
                        action
                    },
                    capability,
                    intent,
                    inputs: Some(args),
                    expected_outcome,
                    confidence,
                    reason,
                }
            }
            other => other,
        }
    }
}

/// Observe current state for decision making
/// Uses a single DB pass (load_run_state_and_events) to avoid loading events twice per iteration.
async fn observe_state(
    app: &AppHandle,
    run_id: &str,
    last_decision_event_id: Option<i64>,
) -> Result<Observation, String> {
    // 1. Load RunState and all events in one pass (was: load_run_state + load_run_events = 2 event loads)
    let (run_state, all_events) = store::load_run_state_and_events(app, run_id).await?;

    // 2. Events since last decision (deterministic by event ID, NOT by count)
    let recent_events: Vec<RunEvent> = if let Some(last_id) = last_decision_event_id {
        all_events
            .iter()
            .filter(|e| e.id > last_id)
            .cloned()
            .collect()
    } else {
        all_events.clone()
    };

    // 3. Load belief state (working memory)
    let belief_state = crate::agent::state::load_belief_state(&all_events);

    // 4. Load recent reflections from other runs (learning)
    // TODO: Load from all runs, not just current run
    let recent_reflections: Vec<RunEvent> = all_events
        .iter()
        .filter(|e| e.event_type == crate::agent::events::RUN_REFLECTED)
        .cloned()
        .collect();

    // 5. Collect environment snapshot
    let current_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    
    let environment = EnvironmentSnapshot {
        current_directory: current_dir.clone(),
        working_directory: current_dir,
    };

    // 6. Include relevant artifacts in observation
    // Extract keywords from goal for artifact filtering (currently unused but kept for future filtering)
    let _goal_keywords: Vec<String> = run_state.goal
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();
    
    // Get relevant artifacts (artifacts are already in run_state, filtered here for observation)
    // The artifacts in run_state are already available, so we just use them
    // In a more sophisticated implementation, we could filter by relevance

    Ok(Observation {
        run_state,
        recent_events,
        environment,
        last_decision_event_id,
        belief_state: Some(belief_state),
        recent_reflections,
    })
}

/// Decide next action using Planner (Ollama LLM)
/// This is the ONLY LLM integration point in the system.
/// `routed_intent_hint`: optional hint from embedding router (first planner call only).
async fn decide_next(
    app: &AppHandle,
    observation: &Observation,
    routed_intent_hint: Option<String>,
) -> Result<Decision, String> {
    // Get capability registry
    let registry = app
        .try_state::<crate::agent::capabilities::registry::CapabilityRegistry>()
        .ok_or("Capability registry not initialized")?;
    
    // Get belief state
    let belief_state = observation.belief_state.as_ref()
        .ok_or("Belief state not available")?;
    
    // Get last step event (for context)
    let last_step_event = observation.recent_events
        .iter()
        .find(|e| e.event_type == crate::agent::events::STEP_EVALUATED);
    
    // Build planner context (cognitive snapshot)
    let planner_ctx = context_builder::build_planner_context(
        &observation.run_state,
        belief_state,
        &registry,
        last_step_event,
        routed_intent_hint,
    ).await;
    
    // Get available capability names for validation
    let available_caps = registry.list().await;
    
    // Build exact prompts that will be sent to the LLM (for visibility)
    let user_prompt = build_user_prompt(&planner_ctx);
    let system_prompt = SYSTEM_PROMPT.to_string();

    // Emit event so UI/logs can show the exact prompt on every LLM call
    let run_id = observation.run_state.id.clone();
    let _ = store::append_event(
        app,
        &run_id,
        crate::agent::events::PLANNER_PROMPT_SENT,
        json!({
            "system_prompt": system_prompt,
            "user_prompt": user_prompt,
            "sent_at": chrono::Utc::now(),
        }),
    )
    .await;

    tracing::info!(
        "[planner] prompt sent run_id={} user_prompt_len={} system_prompt_len={}",
        run_id,
        user_prompt.len(),
        system_prompt.len()
    );
    // Exact prompt text at debug level (RUST_LOG=scribe=debug to see full prompts)
    tracing::debug!(
        run_id = %run_id,
        system_prompt = %system_prompt,
        user_prompt = %user_prompt,
        "[planner] exact prompt sent to LLM"
    );

    // Create planner (model from OLLAMA_MODEL env, or default llama3.2 to match gateway)
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
    let planner = Planner::new(&ollama_model);

    // Call planner with retry logic (max 2 retries)
    // Planner handles validation internally, but we retry on network/parsing errors
    let mut retries = 0;
    let max_retries = 2;
    let planner_decision = loop {
        match planner.decide_next(planner_ctx.clone(), &available_caps, &registry).await {
            Ok(decision) => break decision,
            Err(e) => {
                retries += 1;
                if retries >= max_retries {
                    return Err(format!("Planner failed after {} retries: {}", max_retries, e));
                }
                // Continue to retry
            }
        }
    };
    
    // Convert PlannerDecision to Decision (for backward compatibility with existing loop logic)
    let decision = match planner_decision {
        PlannerDecision::InvokeCapability {
            capability,
            intent,
            expected_outcome,
            inputs,
            confidence,
            reason,
        } => {
            let intent_str = intent.unwrap_or_else(|| format!("Invoke {}", capability));
            let outcome_str = expected_outcome.unwrap_or_else(|| "Operation completed".to_string());
            let inputs_val = inputs.unwrap_or_else(|| json!({}));
            let reason_str = reason.unwrap_or_default();
            Decision::CapabilityBased {
                action_type: "invoke_capability".to_string(),
                capability: Some(capability),
                intent: Some(intent_str),
                inputs: Some(inputs_val),
                expected_outcome: Some(outcome_str),
                confidence,
                reason: reason_str,
            }
        }
        PlannerDecision::RevisePlan { reason, summary, steps } => {
            // Emit plan revision event
            if let Some(ref plan) = observation.run_state.plan {
                let _ = store::append_event(
                    app,
                    &observation.run_state.id,
                    crate::agent::events::PLAN_REVISED,
                    json!({
                        "plan_id": plan.id,
                        "reason": reason,
                        "summary": summary,
                        "steps": steps,
                    }),
                ).await;
            }
            // Plan revision handled above, continue loop
            // Return a no-op decision that will cause loop to continue
            Decision::CapabilityBased {
                action_type: "continue".to_string(),
                capability: None,
                intent: None,
                inputs: None,
                expected_outcome: None,
                confidence: 0.5,
                reason: "Plan revised, continuing".to_string(),
            }
        }
        PlannerDecision::AskUser { question, reason } => {
            let question_str = question.unwrap_or_else(|| "What details should I use? (e.g. file path, content)".to_string());
            let reason_str = reason.unwrap_or_default();
            Decision::CapabilityBased {
                action_type: "ask_user".to_string(),
                capability: None,
                intent: Some(question_str.clone()),
                inputs: Some(json!({ "question": question_str })),
                expected_outcome: None,
                confidence: 0.5,
                reason: reason_str,
            }
        }
        PlannerDecision::Finish { reason } => {
            Decision::CapabilityBased {
                action_type: "finish".to_string(),
                capability: None,
                intent: None,
                inputs: None,
                expected_outcome: None,
                confidence: 1.0,
                reason,
            }
        }
    };
    
    Ok(decision)
}

/// Main agent loop.
/// `initial_intent_hint`: optional hint from embedding router (used on first planner call only).
pub async fn run_loop(
    app: AppHandle,
    run_id: String,
    cancel: CancellationToken,
    initial_intent_hint: Option<String>,
) -> Result<(), String> {
    // On start/resume, detect last decision point for restart safety
    let events = store::load_run_events(&app, &run_id).await?;
    let mut last_decision_event_id: Option<i64> = events
        .iter()
        .rev()
        .find(|e| e.event_type == crate::agent::events::DECISION_MADE)
        .map(|e| e.id);

    // Create initial plan if one doesn't exist
    let run_state = store::load_run_state(&app, &run_id).await?;
    if run_state.plan.is_none() {
        // Create initial plan
        let plan_id = uuid::Uuid::new_v4().to_string();
        let _ = store::append_event(
            &app,
            &run_id,
            crate::agent::events::PLAN_CREATED,
            json!({
                "plan_id": plan_id,
                "goal": run_state.goal,
                "steps": vec!["Analyze goal", "Execute actions", "Verify completion"],
                "summary": format!("Initial plan to achieve: {}", run_state.goal),
                "created_at": chrono::Utc::now()
            }),
        )
        .await;
    }

    // Track revision state for dampening (Step 9 - prevent oscillation)
    let mut revision_state = RevisionState::new();
    // Consecutive planner failures; after MAX_PLANNER_FAILURES we fail the run (Ollama likely down).
    let mut planner_failure_count: u32 = 0;
    // Intent hint from router: used only on first planner call.
    let mut routed_intent_hint_for_turn: Option<String> = initial_intent_hint;

    'main_loop: loop {
        // Check for cancellation
        if cancel.is_cancelled() {
            // Emit cancellation event
            let _ = store::append_event(
                &app,
                &run_id,
                crate::agent::events::RUN_STATUS_CHANGED,
                json!({
                    "status": "cancelled"
                }),
            )
            .await;
            return Ok(());
        }

        // Observe current state
        let observation = observe_state(&app, &run_id, last_decision_event_id).await?;

        // Check if run is terminal
        if observation.run_state.status.is_terminal() {
            break;
        }

        // Confidence-driven behavior (Step 8)
        if let Some(ref belief) = observation.belief_state {
            if belief.plan_confidence < CONFIDENCE_CRITICAL {
                // Critical low confidence - ask user for help
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::MESSAGE_APPENDED,
                    json!({
                        "id": uuid::Uuid::new_v4().to_string(),
                        "role": "assistant",
                        "content": format!(
                            "Plan confidence is critically low ({:.2}%). Should I continue or revise the approach?",
                            belief.plan_confidence * 100.0
                        ),
                        "created_at": chrono::Utc::now()
                    }),
                )
                .await;
                // Continue but with extreme caution
            } else if belief.plan_confidence < CONFIDENCE_LOW {
                // Low confidence - proceed with caution
                // (Future: could limit multi-step execution)
            }
        }

        // Decide next action with retry (skip retries on parse errors - same model will repeat).
        // Race against cancel so Stop run is respected while waiting for LLM.
        let mut decision_attempt = 0;
        let max_decision_attempts = 3;
        let hint_this_turn = routed_intent_hint_for_turn.take();
        let decision = loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = store::append_event(
                        &app,
                        &run_id,
                        crate::agent::events::RUN_STATUS_CHANGED,
                        json!({ "status": "cancelled" }),
                    )
                    .await;
                    return Ok(());
                }
                result = decide_next(&app, &observation, hint_this_turn.clone()) => match result {
                Ok(d) => break d,
                Err(e) => {
                    let is_parse_error = e.contains("Invalid planner JSON");
                    if is_parse_error {
                        decision_attempt = max_decision_attempts;
                    } else {
                        decision_attempt += 1;
                    }
                    if decision_attempt >= max_decision_attempts {
                        // Classify planner failure
                        let failure_kind = crate::agent::executor::failure::classify_failure(&e);
                        let is_planner_failure = crate::agent::executor::failure::is_planner_failure(&failure_kind);
                        
                        eprintln!(
                            "[run_loop] decision failed run_id={} attempts={} error={} failure_kind={:?} is_planner_failure={}",
                            run_id, decision_attempt, e, failure_kind, is_planner_failure
                        );
                        
                        // Emit PLANNER_FAILED event (not STEP_FAILED) for planner failures
                        // This prevents poisoning belief state
                        let event_type = if is_planner_failure {
                            crate::agent::events::PLANNER_FAILED
                        } else {
                            crate::agent::events::STEP_FAILED
                        };
                        
                        let _ = store::append_event(
                            &app,
                            &run_id,
                            event_type,
                            json!({
                                "error": format!("Decision failed after {} attempts: {}", max_decision_attempts, e),
                                "failure_kind": format!("{:?}", failure_kind),
                                "is_planner_failure": is_planner_failure,
                                "step_id": uuid::Uuid::new_v4().to_string()
                            }),
                        )
                        .await;
                        
                        // For planner failures: retry a few times then fail the run (Ollama may be down)
                        if is_planner_failure {
                            // Cap consecutive planner failures so we don't loop forever
                            const MAX_PLANNER_FAILURES: u32 = 6;
                            planner_failure_count += 1;
                            if planner_failure_count >= MAX_PLANNER_FAILURES {
                                eprintln!(
                                    "[run_loop] too many planner failures ({}), marking run as failed run_id={}",
                                    planner_failure_count, run_id
                                );
                                let _ = store::append_event(
                                    &app,
                                    &run_id,
                                    crate::agent::events::RUN_STATUS_CHANGED,
                                    json!({
                                        "status": "failed",
                                        "reason": format!("Planner (Ollama) failed {} times. Is Ollama running? Error: {}", planner_failure_count, e)
                                    }),
                                )
                                .await;
                                return Err(format!(
                                    "Planner failed {} times. Is Ollama running at OLLAMA_URL? Error: {}",
                                    planner_failure_count, e
                                ));
                            }
                            // Wait 5 seconds before retrying (planner might be starting up)
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue 'main_loop; // Retry on next iteration
                        } else {
                            // Non-planner failure - ask user or fail gracefully
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            continue 'main_loop;
                        }
                    }
                    // Retry with backoff
                    let delay_ms = 200 * decision_attempt;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
            }
        };

        // Emit DECISION_MADE event (for restart safety)
        let decision_event = store::append_event(
            &app,
            &run_id,
            crate::agent::events::DECISION_MADE,
            json!({
                "action_type": decision.action_type(),
                "capability": decision.capability(),
                "intent": decision.intent(),
                "inputs": decision.inputs(),
                "expected_outcome": decision.expected_outcome(),
                "reason": decision.reason()
            }),
        )
        .await?;
        last_decision_event_id = Some(decision_event.id);

        // Handle decision
        match decision.action_type() {
            "invoke_capability" => {
                // Get capability from registry
                let capability_name = decision.capability()
                    .ok_or("Missing capability name")?;
                let capability_name_str = capability_name.clone();
                
                let registry = app
                    .try_state::<crate::agent::capabilities::registry::CapabilityRegistry>()
                    .ok_or("Capability registry not initialized")?;
                
                let capability = registry.get(&capability_name).await
                    .ok_or_else(|| format!("Capability not found: {}", capability_name))?;

                // Emit step.started projection event
                let step_id = uuid::Uuid::new_v4().to_string();
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::STEP_STARTED,
                    json!({
                        "step_id": step_id,
                        "capability": capability_name_str,
                        "intent": decision.intent(),
                        "expected_outcome": decision.expected_outcome(),
                        "started_at": chrono::Utc::now()
                    }),
                )
                .await;

                // Check for missing required inputs BEFORE permission check
                let mut inputs = decision.inputs().cloned().unwrap_or_else(|| json!({}));
                let schema = capability.input_schema();
                let missing_inputs = check_missing_inputs(&inputs, &schema);
                
                if !missing_inputs.is_empty() {
                    // Request missing inputs from user
                    let input_request_id = uuid::Uuid::new_v4().to_string();
                    
                    // Emit INPUT_REQUESTED event
                    let _ = store::append_event(
                        &app,
                        &run_id,
                        crate::agent::events::INPUT_REQUESTED,
                        json!({
                            "input_request_id": input_request_id,
                            "capability": capability_name_str,
                            "intent": decision.intent(),
                            "missing_fields": missing_inputs,
                            "schema": schema,
                            "current_inputs": inputs,
                            "requested_at": chrono::Utc::now()
                        }),
                    )
                    .await;
                    
                    // Emit projection event: RUN_STATUS_CHANGED
                    let _ = store::append_event(
                        &app,
                        &run_id,
                        crate::agent::events::RUN_STATUS_CHANGED,
                        json!({
                            "status": "waiting_input"
                        }),
                    )
                    .await;
                    
                    // Create channel for input response
                    let (tx, mut rx) = mpsc::channel::<serde_json::Value>(1);
                    
                    // Register channel with run manager
                    let manager = app
                        .try_state::<RunManager>()
                        .ok_or("Run manager not initialized")?;
                    manager.register_input_channel(input_request_id.clone(), tx).await;
                    
                    // Block loop and wait for input response; respect cancel (Stop run)
                    match tokio::select! {
                        _ = cancel.cancelled() => None,
                        msg = rx.recv() => msg,
                    } {
                        Some(provided_inputs) => {
                            // Merge provided inputs with existing inputs
                            if let (Some(merged_obj), Some(provided_obj)) = (inputs.as_object_mut(), provided_inputs.as_object()) {
                                for (key, value) in provided_obj {
                                    merged_obj.insert(key.clone(), value.clone());
                                }
                            }
                            
                            // Emit INPUT_PROVIDED event
                            let _ = store::append_event(
                                &app,
                                &run_id,
                                crate::agent::events::INPUT_PROVIDED,
                                json!({
                                    "input_request_id": input_request_id,
                                    "provided_inputs": provided_inputs,
                                    "merged_inputs": inputs,
                                    "provided_at": chrono::Utc::now()
                                }),
                            )
                            .await;
                            
                            // Update run status back to running
                            let _ = store::append_event(
                                &app,
                                &run_id,
                                crate::agent::events::RUN_STATUS_CHANGED,
                                json!({
                                    "status": "running"
                                }),
                            )
                            .await;
                            
                            // Continue with execution using merged inputs
                        }
                        None => {
                            // Cancelled or channel closed - emit cancelled and exit
                            let _ = store::append_event(
                                &app,
                                &run_id,
                                crate::agent::events::RUN_STATUS_CHANGED,
                                json!({ "status": "cancelled" }),
                            )
                            .await;
                            return Ok(());
                        }
                    }
                }
                
                // Execution ticket (Moltbot-style): one state machine per tool call, restart-safe, auditable
                let ticket_id = uuid::Uuid::new_v4().to_string();
                let canonical_intent = canonicalize_tool_intent(
                    capability_name_str.as_str(),
                    &inputs,
                    Some(&observation.run_state.goal),
                    decision.intent().map(|s| s.as_str()),
                    capability.risk_level(),
                );
                let expected_outcome_str = decision.expected_outcome()
                    .unwrap_or(&"Operation completed".to_string())
                    .clone();
                let auto_approved = is_auto_approved(
                    capability_name_str.as_str(),
                    capability.risk_level(),
                    &canonical_intent,
                );
                let needs_permission = capability.requires_permission() && !auto_approved;

                let (perm_state, exec_state) = if needs_permission {
                    (PermissionState::Requested, TicketExecutionState::Pending)
                } else {
                    (PermissionState::AutoApproved, TicketExecutionState::Pending)
                };

                if let Err(e) = create_ticket(
                    &app,
                    &ticket_id,
                    &run_id,
                    &step_id,
                    capability_name_str.as_str(),
                    &inputs,
                    &canonical_intent,
                    &expected_outcome_str,
                    perm_state,
                    exec_state,
                )
                .await
                {
                    tracing::warn!("[run_loop] create_ticket failed: {}", e);
                    continue;
                }

                if needs_permission {
                    // Request permission: emit PERMISSION_REQUESTED with ticket_id and canonical intent
                    let _ = store::append_event(
                        &app,
                        &run_id,
                        crate::agent::events::PERMISSION_REQUESTED,
                        json!({
                            "permission_id": ticket_id,
                            "ticket_id": ticket_id,
                            "capability": capability_name_str,
                            "scope": capability_name_str,
                            "reason": decision.intent().map(|s| s.clone()).unwrap_or_else(|| format!("Execute {}", capability_name_str)),
                            "scope_type": "once",
                            "risk_score": match capability.risk_level() {
                                crate::agent::capabilities::RiskLevel::Low => 0.2,
                                crate::agent::capabilities::RiskLevel::Medium => 0.5,
                                crate::agent::capabilities::RiskLevel::High => 0.8,
                                crate::agent::capabilities::RiskLevel::Critical => 1.0,
                            },
                            "requested_at": chrono::Utc::now(),
                            "canonical_intent": {
                                "human_readable": canonical_intent.human_readable,
                                "goal_alignment": canonical_intent.goal_alignment,
                                "irreversible": canonical_intent.irreversible,
                                "risk_factors": canonical_intent.risk_factors,
                            }
                        }),
                    )
                    .await;

                    let _ = store::append_event(
                        &app,
                        &run_id,
                        crate::agent::events::RUN_STATUS_CHANGED,
                        json!({ "status": "waiting_permission" }),
                    )
                    .await;

                    let (tx, mut rx) = mpsc::channel::<bool>(1);
                    let manager = app
                        .try_state::<RunManager>()
                        .ok_or("Run manager not initialized")?;
                    manager.register_permission_channel(ticket_id.clone(), tx).await;

                    match tokio::select! {
                        _ = cancel.cancelled() => None,
                        msg = rx.recv() => msg,
                    } {
                        Some(granted) => {
                            if !granted {
                                let _ = mark_permission_denied(&app, &ticket_id).await;
                                continue;
                            }
                            if let Err(e) = mark_permission_granted(&app, &ticket_id, &ticket_id).await {
                                tracing::warn!("[run_loop] mark_permission_granted failed: {}", e);
                                continue;
                            }
                        }
                        None => {
                            // Cancelled or channel closed
                            let _ = mark_permission_denied(&app, &ticket_id).await;
                            let _ = store::append_event(
                                &app,
                                &run_id,
                                crate::agent::events::RUN_STATUS_CHANGED,
                                json!({ "status": "cancelled" }),
                            )
                            .await;
                            return Ok(());
                        }
                    }
                } else {
                    if let Err(e) = mark_execution_started(&app, &ticket_id).await {
                        tracing::warn!("[run_loop] mark_execution_started failed: {}", e);
                    }
                }

                // Use Executor for centralized execution (Step 6)
                // inputs variable already contains merged inputs if we requested them
                let intent = decision.intent().unwrap_or(&"Execute capability".to_string()).clone();
                let expected_outcome = decision.expected_outcome()
                    .unwrap_or(&"Operation completed".to_string())
                    .clone();
                
                // Execute with retry logic (policy decision in Agent Loop)
                let mut attempt = 0;
                let max_attempts = 3;
                let mut last_result: Option<crate::agent::executor::ExecutionResult> = None;
                
                loop {
                    // Execute capability (Executor emits STEP_EVALUATED with retry_count)
                    let exec_result = Executor::execute(
                        &app,
                        ExecutionRequest {
                            run_id: run_id.clone(),
                            step_id: step_id.clone(),
                            capability: capability_name_str.clone(),
                            intent: intent.clone(),
                            expected_outcome: expected_outcome.clone(),
                            inputs: inputs.clone(),
                            retry_count: attempt, // Pass retry count for confidence normalization
                        },
                    ).await;
                    
                    match exec_result {
                        Ok(result) => {
                            last_result = Some(result.clone());
                            
                            // Update revision state
                            // Planner failures should NOT count toward plan revision
                            if result.success {
                                record_success(&mut revision_state);
                            } else {
                                // Only count non-planner failures
                                if crate::agent::state::plan_revision::should_count_failure(result.failure_kind.as_ref()) {
                                    record_failure(&mut revision_state);
                                }
                            }
                            
                            if result.success {
                                // Success - break retry loop
                                break;
                            } else {
                                // Failure - determine recovery action (policy decision in Agent Loop)
                                let recovery = if let Some(ref failure_kind) = result.failure_kind {
                                    recovery_for(failure_kind)
                                } else {
                                    RecoveryAction::Retry
                                };
                                
                                match recovery {
                                    RecoveryAction::WaitAndRetry => {
                                        // Planner failures - wait longer and retry
                                        // Don't increment attempt count (these are infrastructure issues)
                                        // Wait 5 seconds before retrying
                                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                        // Continue loop to retry (don't break)
                                    }
                                    RecoveryAction::Retry => {
                                        attempt += 1;
                                        if attempt >= max_attempts {
                                            // Max retries reached
                                            break;
                                        }
                                        // Retry with exponential backoff
                                        let delay_ms = 100 * (1 << (attempt - 1));
                                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                                    }
                                    RecoveryAction::RevisePlan => {
                                        // Check if revision is allowed (dampening - Step 9)
                                        let current_confidence = observation.belief_state
                                            .as_ref()
                                            .map(|b| b.plan_confidence)
                                            .unwrap_or(0.5);
                                        
                                        let allow_revision = should_allow_revision(
                                            &revision_state,
                                            &step_id,
                                            current_confidence,
                                            true, // has_failure
                                            2,    // min_steps_between
                                            5,    // cooldown_seconds
                                        );
                                        
                                        if allow_revision {
                                            // Trigger plan revision
                                            let run_state = store::load_run_state(&app, &run_id).await?;
                                            if let Some(ref plan) = run_state.plan {
                                                let _ = store::append_event(
                                                    &app,
                                                    &run_id,
                                                    crate::agent::events::PLAN_REVISED,
                                                    json!({
                                                        "plan_id": plan.id,
                                                        "reason": format!("Step failed: {}", result.reason),
                                                        "adjustment": "Revising plan based on failure",
                                                        "summary": format!("Revised: {}", plan.summary),
                                                    }),
                                                )
                                                .await;
                                                
                                                // Record revision
                                                record_revision(&mut revision_state, step_id.clone());
                                            }
                                        }
                                        break; // Don't retry, let loop replan
                                    }
                                    RecoveryAction::AskUser => {
                                        // Ask user for help
                                        let _ = store::append_event(
                                            &app,
                                            &run_id,
                                            crate::agent::events::MESSAGE_APPENDED,
                                            json!({
                                                "id": uuid::Uuid::new_v4().to_string(),
                                                "role": "assistant",
                                                "content": format!("Need help: {}. How should I proceed?", result.reason),
                                                "created_at": chrono::Utc::now()
                                            }),
                                        )
                                        .await;
                                        break; // Wait for user response
                                    }
                                    RecoveryAction::Abort => {
                                        // Abort the run
                                        let _ = store::append_event(
                                            &app,
                                            &run_id,
                                            crate::agent::events::RUN_STATUS_CHANGED,
                                            json!({
                                                "status": "failed",
                                                "reason": format!("Aborted due to: {}", result.reason)
                                            }),
                                        )
                                        .await;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Executor error
                            attempt += 1;
                            if attempt >= max_attempts {
                                let _ = store::append_event(
                                    &app,
                                    &run_id,
                                    crate::agent::events::STEP_FAILED,
                                    json!({
                                        "step_id": step_id,
                                        "error": format!("Executor failed after {} attempts: {}", max_attempts, e)
                                    }),
                                )
                                .await;
                                break;
                            }
                            let delay_ms = 100 * (1 << (attempt - 1));
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                }

                // Update execution ticket with outcome (Moltbot-style audit trail)
                match &last_result {
                    Some(result) => {
                        let result_json = serde_json::json!({
                            "success": result.success,
                            "confidence": result.confidence,
                            "reason": result.reason,
                        })
                        .to_string();
                        if result.success {
                            let _ = mark_execution_completed(&app, &ticket_id, &result_json).await;
                        } else {
                            let _ = mark_execution_failed(&app, &ticket_id, &result.reason).await;
                        }
                    }
                    None => {
                        let _ = mark_execution_failed(
                            &app,
                            &ticket_id,
                            "Execution did not complete (retries exhausted or aborted)",
                        )
                        .await;
                    }
                }

                // Check if we should trigger plan revision based on failure + low confidence
                // (Requires multiple signals to prevent oscillation)
                if let Some(ref result) = last_result {
                    if !result.success {
                        let belief = observation.belief_state.as_ref();
                        let current_confidence = belief.map(|b| b.plan_confidence).unwrap_or(0.5);
                        let has_low_confidence = current_confidence < 0.3;
                        
                        // Require both failure AND low confidence (multiple signals)
                        if has_low_confidence {
                            // Check if revision is allowed (dampening)
                            let allow_revision = should_allow_revision(
                                &revision_state,
                                &step_id,
                                current_confidence,
                                true, // has_failure
                                2,    // min_steps_between
                                5,    // cooldown_seconds
                            );
                            
                            if allow_revision {
                                // Automatic plan revision (Step 9)
                                let run_state = store::load_run_state(&app, &run_id).await?;
                                if let Some(ref plan) = run_state.plan {
                                    let _ = store::append_event(
                                        &app,
                                        &run_id,
                                        crate::agent::events::PLAN_REVISED,
                                        json!({
                                            "plan_id": plan.id,
                                            "reason": "Low confidence after failure",
                                            "adjustment": "Adding validation and error handling steps",
                                            "summary": format!("Revised plan: {}", plan.summary),
                                        }),
                                    )
                                    .await;
                                    
                                    // Record revision
                                    record_revision(&mut revision_state, step_id.clone());
                                }
                            }
                        }
                    }
                }
            }
            "ask_permission" => {
                // Emit fact event: PERMISSION_REQUESTED
                let permission_id = uuid::Uuid::new_v4().to_string();
                let inputs = decision.inputs().cloned().unwrap_or_else(|| json!({}));
                let scope_type = inputs.get("scope_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("once")
                    .to_string();
                
                // Build reason - use inputs reason or fallback to intent
                let reason_json = if let Some(reason) = inputs.get("reason") {
                    Some(reason.clone())
                } else if let Some(intent) = decision.intent() {
                    Some(json!(intent))
                } else {
                    None
                };
                
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::PERMISSION_REQUESTED,
                    json!({
                        "permission_id": permission_id,
                        "scope": inputs.get("scope"),
                        "reason": reason_json,
                        "scope_type": scope_type,
                        "risk_score": inputs.get("risk_score").and_then(|v| v.as_f64()).unwrap_or(0.5),
                        "requested_at": chrono::Utc::now()
                    }),
                )
                .await;

                // Emit projection event: RUN_STATUS_CHANGED
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::RUN_STATUS_CHANGED,
                    json!({
                        "status": "waiting_permission"
                    }),
                )
                .await;

                // Create channel for permission decision
                let (tx, mut rx) = mpsc::channel::<bool>(1);
                
                // Register channel with run manager
                let manager = app
                    .try_state::<RunManager>()
                    .ok_or("Run manager not initialized")?;
                manager.register_permission_channel(permission_id.clone(), tx).await;

                // Block loop and wait for permission decision
                match rx.recv().await {
                    Some(granted) => {
                        if !granted {
                            // Permission denied - replan
                            continue;
                        }
                        // Permission granted - continue with loop
                        // Status will be updated by reply_permission command
                    }
                    None => {
                        // Channel closed - treat as denied
                        continue;
                    }
                }
            }
            "finish" => {
                // Evaluate the run before completing
                let run_state = store::load_run_state(&app, &run_id).await?;
                let all_events = store::load_run_events(&app, &run_id).await?;
                
                // Count successful vs failed steps
                let successful_steps = run_state.steps.iter()
                    .filter(|s| s.status == crate::agent::state::run_state::StepStatus::Completed)
                    .count();
                let failed_steps = run_state.steps.iter()
                    .filter(|s| s.status == crate::agent::state::run_state::StepStatus::Failed)
                    .count();
                let total_steps = run_state.steps.len();
                
                // Determine success criteria
                let criteria_met: Vec<String> = vec![];
                let criteria_failed: Vec<String> = vec![];
                
                // Check if goal was achieved (simplified - check for artifacts)
                let has_artifacts = !run_state.artifacts.is_empty();
                let final_success = has_artifacts && failed_steps == 0;
                let final_confidence = if total_steps > 0 {
                    successful_steps as f32 / total_steps as f32
                } else {
                    0.5
                };
                
                // Emit RUN_EVALUATED
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::RUN_EVALUATED,
                    json!({
                        "goal": run_state.goal,
                        "success": final_success,
                        "confidence": final_confidence,
                        "criteria_met": if has_artifacts { vec!["artifacts_created"] } else { vec![] },
                        "criteria_failed": if failed_steps > 0 { vec!["steps_failed"] } else { vec![] },
                        "reason": if final_success {
                            format!("Run completed successfully with {} artifact(s)", run_state.artifacts.len())
                        } else {
                            format!("Run completed with {} failed step(s)", failed_steps)
                        }
                    }),
                )
                .await;
                
                // Emit RUN_REFLECTED (learning)
                let what_worked: Vec<String> = if successful_steps > 0 {
                    vec!["Steps executed successfully".to_string()]
                } else {
                    vec![]
                };
                let what_failed: Vec<String> = if failed_steps > 0 {
                    vec![format!("{} step(s) failed", failed_steps)]
                } else {
                    vec![]
                };
                let lessons_learned: Vec<String> = vec![];
                let future_adjustments: Vec<String> = if failed_steps > 0 {
                    vec!["Retry failed steps with different approach".to_string()]
                } else {
                    vec![]
                };
                
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::RUN_REFLECTED,
                    json!({
                        "what_worked": what_worked,
                        "what_failed": what_failed,
                        "lessons_learned": lessons_learned,
                        "future_adjustments": future_adjustments,
                    }),
                )
                .await;
                
                // Emit run.status_changed -> Completed
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::RUN_STATUS_CHANGED,
                    json!({
                        "status": "completed"
                    }),
                )
                .await;
                break;
            }
            "ask_user" => {
                let question = decision.inputs()
                    .and_then(|v| v.get("question").and_then(|q| q.as_str().map(String::from)))
                    .unwrap_or_else(|| "Question".to_string());
                let reason = decision.reason().to_string();
                let request_id = uuid::Uuid::new_v4().to_string();

                // Emit ask_user.requested so UI/gateway can show the question and wait for reply
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::ASK_USER_REQUESTED,
                    json!({
                        "request_id": request_id,
                        "question": question,
                        "reason": reason,
                        "created_at": chrono::Utc::now()
                    }),
                )
                .await;

                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::RUN_STATUS_CHANGED,
                    json!({ "status": "waiting_ask_user" }),
                )
                .await;

                // Show question in chat
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::MESSAGE_APPENDED,
                    json!({
                        "id": uuid::Uuid::new_v4().to_string(),
                        "role": "assistant",
                        "content": question,
                        "created_at": chrono::Utc::now()
                    }),
                )
                .await;

                let (tx, mut rx) = mpsc::channel::<String>(1);
                let manager = app.try_state::<RunManager>().ok_or("Run manager not initialized")?;
                manager.register_ask_user_channel(request_id.clone(), tx).await;

                match rx.recv().await {
                    Some(answer) => {
                        if !answer.is_empty() {
                            let _ = store::append_event(
                                &app,
                                &run_id,
                                crate::agent::events::MESSAGE_APPENDED,
                                json!({
                                    "id": uuid::Uuid::new_v4().to_string(),
                                    "role": "user",
                                    "content": answer,
                                    "created_at": chrono::Utc::now()
                                }),
                            )
                            .await;
                        }
                        let _ = store::append_event(
                            &app,
                            &run_id,
                            crate::agent::events::RUN_STATUS_CHANGED,
                            json!({ "status": "running" }),
                        )
                        .await;
                    }
                    None => {
                        let _ = store::append_event(
                            &app,
                            &run_id,
                            crate::agent::events::RUN_STATUS_CHANGED,
                            json!({ "status": "running" }),
                        )
                        .await;
                    }
                }
            }
            _ => {
                return Err(format!("Unknown action_type: {}", decision.action_type()));
            }
        }

        // No artificial delay: each iteration does observe_state (DB) + decide_next (LLM) + optional execute.
        // That is enough backoff; a sleep here added ~100ms per step and slowed simple tasks (e.g. create file).
    }

    Ok(())
}

/// Helper function to check for missing required inputs
fn check_missing_inputs(inputs: &serde_json::Value, schema: &serde_json::Value) -> Vec<String> {
    let mut missing = Vec::new();
    
    // Get required fields from schema
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        let input_obj = inputs.as_object();
        
        for field in required {
            if let Some(field_name) = field.as_str() {
                let is_missing = if let Some(obj) = input_obj {
                    if let Some(value) = obj.get(field_name) {
                        // Check if value is empty
                        match value {
                            serde_json::Value::String(s) => s.trim().is_empty(),
                            serde_json::Value::Null => true,
                            serde_json::Value::Array(arr) => arr.is_empty(),
                            serde_json::Value::Object(obj) => obj.is_empty(),
                            _ => false, // Numbers, booleans are never "empty"
                        }
                    } else {
                        true // Field is missing
                    }
                } else {
                    true // Inputs is not an object
                };
                
                if is_missing {
                    missing.push(field_name.to_string());
                }
            }
        }
    }
    
    missing
}

/// Active run management
pub struct RunManager {
    active_runs: Arc<RwLock<HashMap<String, CancellationToken>>>,
    permission_channels: Arc<RwLock<HashMap<String, mpsc::Sender<bool>>>>, // permission_id -> sender
    input_channels: Arc<RwLock<HashMap<String, mpsc::Sender<serde_json::Value>>>>, // input_request_id -> sender
    ask_user_channels: Arc<RwLock<HashMap<String, mpsc::Sender<String>>>>, // request_id -> sender (user answer)
}

impl RunManager {
    pub fn new() -> Self {
        Self {
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            permission_channels: Arc::new(RwLock::new(HashMap::new())),
            input_channels: Arc::new(RwLock::new(HashMap::new())),
            ask_user_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_run(&self, app: AppHandle, run_id: String) -> Result<(), String> {
        eprintln!("[run_manager] start_run entered run_id={}", run_id);
        // Check if run is already active
        {
            let runs = self.active_runs.read().await;
            if runs.contains_key(&run_id) {
                return Err(format!("Run {} is already active", run_id));
            }
        }

        let cancel = CancellationToken::new();

        // Mark run as running when we start the loop
        eprintln!("[run_manager] start_run appending RUN_STATUS_CHANGED run_id={}", run_id);
        let _ = store::append_event(
            &app,
            &run_id,
            crate::agent::events::RUN_STATUS_CHANGED,
            json!({
                "status": "running"
            }),
        )
        .await;
        eprintln!("[run_manager] start_run append_event done run_id={}", run_id);

        // Load goal and route: direct path or planner (embedding-based when available).
        // Run router in spawn_blocking so HTTP/API embedders can use block_on without panicking.
        let run_state = store::load_run_state(&app, &run_id).await?;
        let goal = run_state.goal.clone();
        let working_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let env_ctx = EnvContext {
            working_directory: working_dir,
        };
        let goal_for_router = goal.clone();
        let router_result = tokio::task::spawn_blocking(move || route_goal_with_result(&goal_for_router, &env_ctx))
            .await
            .map_err(|_| "router spawn_blocking join error")?;
        let decision = router_result.decision.clone();
        let intent_hint = router_result.intent_hint.clone();

        // Emit router decision for debugging and tuning
        let _ = store::append_event(
            &app,
            &run_id,
            crate::agent::events::ROUTER_DECISION,
            json!({
                "prompt": goal,
                "predicted_intent": router_result.predicted_intent,
                "confidence": router_result.confidence,
                "final_route": router_result.final_route,
                "fallback_reason": router_result.fallback_reason,
            }),
        )
        .await;

        // Store cancellation token (used by run_loop; direct path does not support cancel yet)
        {
            let mut runs = self.active_runs.write().await;
            runs.insert(run_id.clone(), cancel.clone());
        }

        let app_clone = app.clone();
        let run_id_clone = run_id.clone();
        let manager_clone = self.active_runs.clone();

        match decision {
            RouteDecision::Direct(cmd) => {
                tokio::spawn(async move {
                    let result = run_direct_path(app_clone, run_id_clone.clone(), cmd).await;
                    let mut runs = manager_clone.write().await;
                    runs.remove(&run_id_clone);
                    if let Err(e) = result {
                        eprintln!("Direct path error for {}: {}", run_id_clone, e);
                    }
                });
            }
            RouteDecision::AskUser(_clarification) => {
                // v1: defer to planner with optional intent hint
                let hint = intent_hint.clone();
                tokio::spawn(async move {
                    let result = run_loop(app_clone.clone(), run_id_clone.clone(), cancel, hint).await;
                    let mut runs = manager_clone.write().await;
                    runs.remove(&run_id_clone);
                    if let Err(e) = result {
                        eprintln!("Run loop error for {}: {}", run_id_clone, e);
                    }
                });
            }
            RouteDecision::DeferToPlanner => {
                let hint = intent_hint.clone();
                tokio::spawn(async move {
                    let result = run_loop(app_clone.clone(), run_id_clone.clone(), cancel, hint).await;
                    let mut runs = manager_clone.write().await;
                    runs.remove(&run_id_clone);
                    if let Err(e) = result {
                        eprintln!("Run loop error for {}: {}", run_id_clone, e);
                    }
                });
            }
        }

        eprintln!("[run_manager] start_run returning Ok run_id={}", run_id);
        Ok(())
    }

    /// Resume a run after restart (with restart safety checks)
    pub async fn resume_run(&self, app: AppHandle, run_id: String) -> Result<(), String> {
        // Detect last completed side-effect
        let safe_event_id = store::detect_last_completed_side_effect(&app, &run_id).await?;
        
        if safe_event_id.is_none() {
            // Incomplete side-effect detected - mark as failed or ask user
            // For now, mark as failed
            store::append_event(
                &app,
                &run_id,
                crate::agent::events::RUN_STATUS_CHANGED,
                json!({
                    "status": "failed",
                    "reason": "Incomplete side-effect detected on restart"
                }),
            )
            .await?;
            return Err("Run has incomplete side-effect, cannot resume safely".to_string());
        }

        // Resume from safe point
        self.start_run(app, run_id).await
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<(), String> {
        let mut runs = self.active_runs.write().await;
        if let Some(cancel) = runs.remove(run_id) {
            cancel.cancel();
            Ok(())
        } else {
            Err(format!("Run {} not found", run_id))
        }
    }

    pub async fn cancel_all_runs(&self) -> Result<(), String> {
        let mut runs = self.active_runs.write().await;
        for (_, cancel) in runs.drain() {
            cancel.cancel();
        }
        Ok(())
    }

    /// Register a permission channel for waiting on user decision
    pub async fn register_permission_channel(&self, permission_id: String, sender: mpsc::Sender<bool>) {
        let mut channels = self.permission_channels.write().await;
        channels.insert(permission_id, sender);
    }

    /// Send permission decision to waiting loop
    pub async fn send_permission_decision(&self, permission_id: &str, granted: bool) -> Result<(), String> {
        let mut channels = self.permission_channels.write().await;
        if let Some(sender) = channels.remove(permission_id) {
            sender.send(granted).await
                .map_err(|_| "Failed to send permission decision".to_string())?;
            Ok(())
        } else {
            Err(format!("Permission channel not found for {}", permission_id))
        }
    }

    /// Register an input channel for waiting on user input
    pub async fn register_input_channel(&self, input_request_id: String, sender: mpsc::Sender<serde_json::Value>) {
        let mut channels = self.input_channels.write().await;
        channels.insert(input_request_id, sender);
    }

    /// Send input response to waiting loop
    pub async fn send_input_response(&self, input_request_id: &str, inputs: serde_json::Value) -> Result<(), String> {
        let mut channels = self.input_channels.write().await;
        if let Some(sender) = channels.remove(input_request_id) {
            sender.send(inputs).await
                .map_err(|_| "Failed to send input response".to_string())?;
            Ok(())
        } else {
            Err(format!("Input channel not found for {}", input_request_id))
        }
    }

    /// Cancel an input request by dropping its channel; the run loop will receive None and treat it as cancelled.
    pub async fn cancel_input_request(&self, input_request_id: &str) -> Result<(), String> {
        let mut channels = self.input_channels.write().await;
        if channels.remove(input_request_id).is_some() {
            Ok(())
        } else {
            Err(format!("Input channel not found for {}", input_request_id))
        }
    }

    /// Register an ask_user channel for waiting on user clarification reply
    pub async fn register_ask_user_channel(&self, request_id: String, sender: mpsc::Sender<String>) {
        let mut channels = self.ask_user_channels.write().await;
        channels.insert(request_id, sender);
    }

    /// Send ask_user reply to waiting loop
    pub async fn send_ask_user_reply(&self, request_id: &str, answer: String) -> Result<(), String> {
        let mut channels = self.ask_user_channels.write().await;
        if let Some(sender) = channels.remove(request_id) {
            sender.send(answer).await
                .map_err(|_| "Failed to send ask_user reply".to_string())?;
            Ok(())
        } else {
            Err(format!("Ask user channel not found for {}", request_id))
        }
    }
}

/// Resume all non-terminal runs on app start
pub async fn resume_all_runs(app: &AppHandle) -> Result<(), String> {
    let manager = app
        .try_state::<RunManager>()
        .ok_or("Run manager not initialized")?;
    
    let run_ids = store::get_non_terminal_runs(app).await?;
    
    for run_id in run_ids {
        // Try to resume each run
        if let Err(e) = manager.resume_run(app.clone(), run_id.clone()).await {
            eprintln!("Failed to resume run {}: {}", run_id, e);
            // Continue with other runs
        }
    }
    
    Ok(())
}
