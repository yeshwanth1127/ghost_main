// Agent loop orchestrator

use crate::agent::events::RunEvent;
use crate::agent::run::store;
use crate::agent::state::run_state::RunState;
use crate::api;
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
}

/// Environment snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub current_directory: String,
    pub working_directory: String,
}

/// Decision from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: String, // "fs_read" | "fs_write" | "ask_permission" | "ask_user" | "finish"
    pub args: serde_json::Value,
    pub confidence: f32,
    pub reason: String,
}

/// Observe current state for decision making
async fn observe_state(
    app: &AppHandle,
    run_id: &str,
    last_decision_event_id: Option<i64>,
) -> Result<Observation, String> {
    // 1. Load RunState via replay
    let run_state = store::load_run_state(app, run_id).await?;

    // 2. Load events since last decision (deterministic by event ID, NOT by count)
    let all_events = store::load_run_events(app, run_id).await?;
    let recent_events: Vec<RunEvent> = if let Some(last_id) = last_decision_event_id {
        all_events
            .into_iter()
            .filter(|e| e.id > last_id)
            .collect()
    } else {
        all_events
    };

    // 3. Collect environment snapshot
    let current_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    
    let environment = EnvironmentSnapshot {
        current_directory: current_dir.clone(),
        working_directory: current_dir,
    };

    // 4. Include relevant artifacts in observation
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
    })
}

/// Decide next action using LLM
/// Uses existing chat_stream infrastructure to route through the same chat API
async fn decide_next(
    app: &AppHandle,
    observation: &Observation,
) -> Result<Decision, String> {
    // Extract and format artifacts for LLM context
    let artifacts_context = if observation.run_state.artifacts.is_empty() {
        "No artifacts available yet.".to_string()
    } else {
        let mut artifact_descriptions = Vec::new();
        for artifact in &observation.run_state.artifacts {
            artifact_descriptions.push(format!(
                "- {}: {} ({})",
                artifact.kind,
                artifact.location,
                artifact.summary
            ));
        }
        format!(
            "Available artifacts ({}):\n{}",
            observation.run_state.artifacts.len(),
            artifact_descriptions.join("\n")
        )
    };

    // Format recent events for context
    let recent_events_context = if observation.recent_events.is_empty() {
        "No recent events.".to_string()
    } else {
        let mut event_summaries = Vec::new();
        for event in &observation.recent_events {
            event_summaries.push(format!(
                "- {}: {}",
                event.event_type,
                serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".to_string())
            ));
        }
        format!(
            "Recent events since last decision ({}):\n{}",
            observation.recent_events.len(),
            event_summaries.join("\n")
        )
    };

    // Create system prompt for decision making with artifact context
    let system_prompt = format!(
        r#"You are an autonomous agent making decisions about what action to take next.

GOAL: {}
STATUS: {:?}

AVAILABLE ARTIFACTS (use these to inform your decisions):
{}

RECENT EVENTS:
{}

AVAILABLE ACTIONS:
- fs_read: Read a file (args: {{"path": "string"}})
  Use this to read files that might contain information needed for the goal.
  Consider artifacts - if a file was already read, you may not need to read it again unless checking for updates.
  
- fs_write: Write to a file (args: {{"path": "string", "content": "string"}})
  Use this to create or modify files.
  Consider artifacts - check if files were already created/modified.
  
- ask_permission: Request permission for an operation (args: {{"scope": "string", "reason": "string"}})
  Use this when an operation requires user approval.
  
- ask_user: Ask the user a question (args: {{"question": "string"}})
  Use this when you need clarification or information from the user.
  
- finish: Complete the run (args: {{}})
  Use this when the goal has been achieved.

IMPORTANT:
- Review artifacts before taking actions - they show what has already been done
- Use artifacts to avoid redundant operations
- Consider the goal and current artifacts when deciding the next step

You must respond with valid JSON in this exact format:
{{
  "action": "action_name",
  "args": {{...}},
  "confidence": 0.0-1.0,
  "reason": "short explanation referencing artifacts if relevant"
}}"#,
        observation.run_state.goal,
        observation.run_state.status,
        artifacts_context,
        recent_events_context
    );

    // User message for the LLM
    let user_message = format!(
        "Based on the goal '{}', current status {:?}, available artifacts, and recent events, what should I do next? Respond with only valid JSON in the required format.",
        observation.run_state.goal,
        observation.run_state.status
    );

    // Call LLM via existing chat_stream infrastructure
    // This routes through the same chat API endpoint (/api/v1/chat) used by the chat mode
    // Uses the same authentication, model selection, and streaming infrastructure
    let response = api::chat_stream(
        app.clone(),
        user_message,
        Some(system_prompt),
        None, // No image for agent decisions
        None, // No history - each decision is independent
    )
    .await?;

    // Parse JSON response
    let decision: Decision = serde_json::from_str(&response)
        .map_err(|e| format!("Invalid JSON response from LLM: {}. Response: {}", e, response))?;

    // Validate action
    let valid_actions = ["fs_read", "fs_write", "ask_permission", "ask_user", "finish"];
    if !valid_actions.contains(&decision.action.as_str()) {
        return Err(format!("Invalid action: {}", decision.action));
    }

    Ok(decision)
}

/// Main agent loop
pub async fn run_loop(
    app: AppHandle,
    run_id: String,
    cancel: CancellationToken,
) -> Result<(), String> {
    // On start/resume, detect last decision point for restart safety
    let events = store::load_run_events(&app, &run_id).await?;
    let mut last_decision_event_id: Option<i64> = events
        .iter()
        .rev()
        .find(|e| e.event_type == crate::agent::events::DECISION_MADE)
        .map(|e| e.id);

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

        // Decide next action with retry
        let mut decision_attempt = 0;
        let max_decision_attempts = 3;
        let decision = loop {
            match decide_next(&app, &observation).await {
                Ok(d) => break d,
                Err(e) => {
                    decision_attempt += 1;
                    if decision_attempt >= max_decision_attempts {
                        // Emit step.failed event after max retries
                        let _ = store::append_event(
                            &app,
                            &run_id,
                            crate::agent::events::STEP_FAILED,
                            json!({
                                "error": format!("Decision failed after {} attempts: {}", max_decision_attempts, e),
                                "step_id": uuid::Uuid::new_v4().to_string()
                            }),
                        )
                        .await;
                        // Ask user or fail gracefully
                        // For now, continue to next iteration (will ask user in future)
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue 'main_loop;
                    }
                    // Retry with backoff
                    let delay_ms = 200 * decision_attempt;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        };

        // Emit DECISION_MADE event (for restart safety)
        let decision_event = store::append_event(
            &app,
            &run_id,
            crate::agent::events::DECISION_MADE,
            json!({
                "action": decision.action,
                "args": decision.args,
                "reason": decision.reason
            }),
        )
        .await?;
        last_decision_event_id = Some(decision_event.id);

        // Handle decision
        match decision.action.as_str() {
            "ask_permission" => {
                // Emit fact event: PERMISSION_REQUESTED
                let permission_id = uuid::Uuid::new_v4().to_string();
                let scope_type = decision.args.get("scope_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("once")
                    .to_string();
                
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::PERMISSION_REQUESTED,
                    json!({
                        "permission_id": permission_id,
                        "scope": decision.args.get("scope"),
                        "reason": decision.args.get("reason"),
                        "scope_type": scope_type,
                        "risk_score": decision.args.get("risk_score").and_then(|v| v.as_f64()).unwrap_or(0.5),
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
            "fs_read" => {
                // Emit step.started projection event
                let step_id = uuid::Uuid::new_v4().to_string();
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::STEP_STARTED,
                    json!({
                        "step_id": step_id,
                        "tool_name": "fs_read",
                        "started_at": chrono::Utc::now()
                    }),
                )
                .await;

                // Execute tool with retry logic
                let path = decision
                    .args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let scope = crate::agent::tools::fs_read::CapabilityScope {
                    allowed_paths: vec![path.clone()], // Simplified for now
                };

                let mut attempt = 0;
                let max_attempts = 3;
                let mut _last_error = None;

                loop {
                    match crate::agent::tools::fs_read::fs_read(&app, &run_id, path.clone(), scope.clone()).await {
                        Ok(_artifact) => {
                            // Tool execution emits its own events (TOOL_EXECUTED, FILE_READ, STEP_COMPLETED, ARTIFACT_CREATED)
                            break; // Success
                        }
                        Err(e) => {
                            _last_error = Some(e.clone());
                            attempt += 1;
                            
                            if attempt >= max_attempts {
                                // Emit step.failed after max retries
                                let _ = store::append_event(
                                    &app,
                                    &run_id,
                                    crate::agent::events::STEP_FAILED,
                                    json!({
                                        "step_id": step_id,
                                        "error": format!("Failed after {} attempts: {}", max_attempts, e)
                                    }),
                                )
                                .await;
                                // Step failure ≠ run failure - continue loop
                                break;
                            }
                            
                            // Retry with exponential backoff
                            let delay_ms = 100 * (1 << (attempt - 1)); // 100ms, 200ms, 400ms
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                }
            }
            "fs_write" => {
                // Emit step.started projection event
                let step_id = uuid::Uuid::new_v4().to_string();
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::STEP_STARTED,
                    json!({
                        "step_id": step_id,
                        "tool_name": "fs_write",
                        "started_at": chrono::Utc::now()
                    }),
                )
                .await;

                // Execute tool with retry logic
                let path = decision.args.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing path argument")?
                    .to_string();
                
                let content = decision.args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing content argument")?
                    .to_string();
                
                let scope = crate::agent::tools::fs_write::CapabilityScope {
                    allowed_paths: vec![path.clone()], // Simplified for now
                };

                let mut attempt = 0;
                let max_attempts = 3;
                let mut _last_error = None;

                loop {
                    match crate::agent::tools::fs_write::fs_write(&app, &run_id, path.clone(), content.clone(), scope.clone()).await {
                        Ok(_artifact) => {
                            // Tool execution emits its own events (TOOL_EXECUTED, FILE_WRITTEN, STEP_COMPLETED, ARTIFACT_CREATED)
                            break; // Success
                        }
                        Err(e) => {
                            if e.contains("Location picker was cancelled") {
                                let _ = store::append_event(
                                    &app,
                                    &run_id,
                                    crate::agent::events::STEP_FAILED,
                                    json!({
                                        "step_id": step_id,
                                        "error": e
                                    }),
                                )
                                .await;
                                break;
                            }
                            _last_error = Some(e.clone());
                            attempt += 1;
                            
                            if attempt >= max_attempts {
                                // Emit step.failed after max retries
                                let _ = store::append_event(
                                    &app,
                                    &run_id,
                                    crate::agent::events::STEP_FAILED,
                                    json!({
                                        "step_id": step_id,
                                        "error": format!("Failed after {} attempts: {}", max_attempts, e)
                                    }),
                                )
                                .await;
                                // Step failure ≠ run failure - continue loop
                                break;
                            }
                            
                            // Retry with exponential backoff
                            let delay_ms = 100 * (1 << (attempt - 1)); // 100ms, 200ms, 400ms
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                }
            }
            "ask_user" => {
                // Ask user (simplified for now)
                // Emit message event
                let _ = store::append_event(
                    &app,
                    &run_id,
                    crate::agent::events::MESSAGE_APPENDED,
                    json!({
                        "id": uuid::Uuid::new_v4().to_string(),
                        "role": "assistant",
                        "content": decision.args.get("question").and_then(|v| v.as_str()).unwrap_or("Question"),
                        "created_at": chrono::Utc::now()
                    }),
                )
                .await;
            }
            _ => {
                return Err(format!("Unknown action: {}", decision.action));
            }
        }

        // Small delay to prevent tight loop
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    Ok(())
}

/// Active run management
pub struct RunManager {
    active_runs: Arc<RwLock<HashMap<String, CancellationToken>>>,
    permission_channels: Arc<RwLock<HashMap<String, mpsc::Sender<bool>>>>, // permission_id -> sender
}

impl RunManager {
    pub fn new() -> Self {
        Self {
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            permission_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_run(&self, app: AppHandle, run_id: String) -> Result<(), String> {
        // Check if run is already active
        {
            let runs = self.active_runs.read().await;
            if runs.contains_key(&run_id) {
                return Err(format!("Run {} is already active", run_id));
            }
        }

        let cancel = CancellationToken::new();

        // Mark run as running when we start the loop
        let _ = store::append_event(
            &app,
            &run_id,
            crate::agent::events::RUN_STATUS_CHANGED,
            json!({
                "status": "running"
            }),
        )
        .await;
        
        // Store cancellation token
        {
            let mut runs = self.active_runs.write().await;
            runs.insert(run_id.clone(), cancel.clone());
        }

        // Spawn loop in background task
        let app_clone = app.clone();
        let run_id_clone = run_id.clone();
        let manager_clone = self.active_runs.clone();
        tokio::spawn(async move {
            let result = run_loop(app_clone.clone(), run_id_clone.clone(), cancel).await;
            
            // Remove from active runs when done
            let mut runs = manager_clone.write().await;
            runs.remove(&run_id_clone);
            
            if let Err(e) = result {
                eprintln!("Run loop error for {}: {}", run_id_clone, e);
            }
        });

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
