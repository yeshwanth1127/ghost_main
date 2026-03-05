// Direct execution path: run a single DirectCommand without the planner.
//
// This path does not call the planner, does not embed, and does not go through run_loop.
// Permission, input request, and execution-ticket flows are kept for safety and auditability;
// is_auto_approved already short-circuits permission for low-risk capabilities when appropriate.
// Optional future: a "trusted intent" fast path could skip some event logging for parse-only goals.

use super::route::direct_command_to_execution;
use super::types::DirectCommand;
use crate::agent::capabilities::PreflightResult;
use crate::agent::events::{
    DECISION_DIRECT_COMMAND_SELECTED, INPUT_REQUESTED, PERMISSION_REQUESTED,
    RUN_STATUS_CHANGED, STEP_STARTED,
};
use crate::agent::execution_ticket::{
    create_ticket, mark_execution_completed, mark_execution_failed, mark_execution_started,
    mark_permission_denied, mark_permission_granted,
    PermissionState, ExecutionState as TicketExecutionState,
};
use crate::agent::executor::{Executor, ExecutionRequest};
use crate::agent::intent::canonicalize_tool_intent;
use crate::agent::permissions::is_auto_approved;
use crate::agent::run::store;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tauri::{AppHandle, Manager};

/// Merge provided_inputs into base (override or add keys).
fn merge_inputs(base: &Value, provided: &Value) -> Value {
    let mut obj = base.as_object().cloned().unwrap_or_default();
    if let Some(prov) = provided.as_object() {
        for (k, v) in prov {
            obj.insert(k.clone(), v.clone());
        }
    }
    Value::Object(obj)
}

/// Run the direct path: resolve capability + inputs, preflight, permission if needed, execute, then complete.
pub async fn run_direct_path(
    app: AppHandle,
    run_id: String,
    cmd: DirectCommand,
) -> Result<(), String> {
    let (capability_name, inputs) = direct_command_to_execution(&cmd)?;
    let mut current_inputs = inputs.clone();
    let mut did_input_merge = false;

    let step_id = uuid::Uuid::new_v4().to_string();
    let direct_command_variant = match &cmd {
        DirectCommand::WriteFile { .. } => "write_file",
        DirectCommand::ReadFile { .. } => "read_file",
        DirectCommand::ListFiles { .. } => "list_files",
        DirectCommand::CreateFile { .. } => "create_file",
        DirectCommand::RunCommand { .. } => "run_command",
    };

    store::append_event(
        &app,
        &run_id,
        DECISION_DIRECT_COMMAND_SELECTED,
        json!({
            "direct_command": direct_command_variant,
            "capability": capability_name,
            "inputs": inputs,
            "step_id": step_id,
        }),
    )
    .await?;

    let registry = app
        .try_state::<crate::agent::capabilities::registry::CapabilityRegistry>()
        .ok_or("Capability registry not initialized")?;
    let capability = registry
        .get(&capability_name)
        .await
        .ok_or_else(|| format!("Capability not found: {}", capability_name))?;

    let preflight = capability.preflight(&inputs);

    match preflight {
        PreflightResult::NeedsPermission(perm) => {
            let run_state = store::load_run_state(&app, &run_id).await?;
            let canonical_intent = canonicalize_tool_intent(
                &capability_name,
                &inputs,
                Some(&run_state.goal),
                Some(&perm.reason),
                capability.risk_level(),
            );
            let auto_approved = is_auto_approved(
                &capability_name,
                capability.risk_level(),
                &canonical_intent,
            );
            if auto_approved {
                if let Err(e) = create_ticket(
                    &app,
                    &step_id,
                    &run_id,
                    &step_id,
                    &capability_name,
                    &inputs,
                    &canonical_intent,
                    "Operation completed",
                    PermissionState::AutoApproved,
                    TicketExecutionState::Pending,
                )
                .await
                {
                    tracing::warn!("[run_direct_path] create_ticket failed: {}", e);
                    return Err(e);
                }
                if let Err(e) = mark_execution_started(&app, &step_id).await {
                    tracing::warn!("[run_direct_path] mark_execution_started failed: {}", e);
                }
            } else {
                let ticket_id = step_id.clone();
                if let Err(e) = create_ticket(
                    &app,
                    &ticket_id,
                    &run_id,
                    &step_id,
                    &capability_name,
                    &inputs,
                    &canonical_intent,
                    "Operation completed",
                    PermissionState::Requested,
                    TicketExecutionState::Pending,
                )
                .await
                {
                    tracing::warn!("[run_direct_path] create_ticket failed: {}", e);
                    return Err(e);
                }
                store::append_event(
                    &app,
                    &run_id,
                    PERMISSION_REQUESTED,
                    json!({
                        "permission_id": ticket_id,
                        "ticket_id": ticket_id,
                        "capability": capability_name,
                        "scope": capability_name,
                        "reason": perm.reason,
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
                .await?;
                store::append_event(
                    &app,
                    &run_id,
                    RUN_STATUS_CHANGED,
                    json!({ "status": "waiting_permission" }),
                )
                .await?;

                let (tx, mut rx) = mpsc::channel::<bool>(1);
                let manager = app
                    .try_state::<crate::agent::r#loop::run_loop::RunManager>()
                    .ok_or("Run manager not initialized")?;
                manager.register_permission_channel(ticket_id.clone(), tx).await;

                match rx.recv().await {
                    Some(true) => {
                        if let Err(e) = mark_permission_granted(&app, &ticket_id, &ticket_id).await
                        {
                            tracing::warn!("[run_direct_path] mark_permission_granted failed: {}", e);
                            return Err(e);
                        }
                        if let Err(e) = mark_execution_started(&app, &ticket_id).await {
                            tracing::warn!("[run_direct_path] mark_execution_started failed: {}", e);
                        }
                    }
                    _ => {
                        let _ = mark_permission_denied(&app, &ticket_id).await;
                        store::append_event(
                            &app,
                            &run_id,
                            RUN_STATUS_CHANGED,
                            json!({ "status": "failed", "reason": "Permission denied" }),
                        )
                        .await?;
                        return Err("Permission denied".to_string());
                    }
                }
            }
        }
        PreflightResult::NeedsInput(input_req) => {
            let input_request_id = uuid::Uuid::new_v4().to_string();
            store::append_event(
                &app,
                &run_id,
                INPUT_REQUESTED,
                json!({
                    "input_request_id": input_request_id,
                    "capability": capability_name,
                    "missing_fields": input_req.missing_fields,
                    "schema": input_req.schema,
                    "current_inputs": input_req.current_inputs,
                    "requested_at": chrono::Utc::now()
                }),
            )
            .await?;
            store::append_event(
                &app,
                &run_id,
                RUN_STATUS_CHANGED,
                json!({ "status": "waiting_input" }),
            )
            .await?;

            let (tx, mut rx) = mpsc::channel::<Value>(1);
            let manager = app
                .try_state::<crate::agent::r#loop::run_loop::RunManager>()
                .ok_or("Run manager not initialized")?;
            manager.register_input_channel(input_request_id.clone(), tx).await;

            let provided = rx.recv().await.ok_or("Input channel closed before response".to_string())?;
            current_inputs = merge_inputs(&current_inputs, &provided);
            did_input_merge = true;
        }
        PreflightResult::Reject(reason) => {
            store::append_event(
                &app,
                &run_id,
                RUN_STATUS_CHANGED,
                json!({ "status": "failed", "reason": reason }),
            )
            .await?;
            return Err(reason);
        }
        PreflightResult::Ok => {
            let run_state = store::load_run_state(&app, &run_id).await.ok();
            let goal = run_state.as_ref().map(|s| s.goal.as_str());
            let canonical_intent = canonicalize_tool_intent(
                &capability_name,
                &inputs,
                goal,
                None,
                capability.risk_level(),
            );
            if let Err(e) = create_ticket(
                &app,
                &step_id,
                &run_id,
                &step_id,
                &capability_name,
                &inputs,
                &canonical_intent,
                "Operation completed",
                PermissionState::AutoApproved,
                TicketExecutionState::Pending,
            )
            .await
            {
                tracing::warn!("[run_direct_path] create_ticket failed: {}", e);
            }
            if let Err(e) = mark_execution_started(&app, &step_id).await {
                tracing::warn!("[run_direct_path] mark_execution_started failed: {}", e);
            }
        }
    }

    // After merging input (path from Telegram), re-run preflight so we ask for permission before execute
    if did_input_merge {
        let preflight2 = capability.preflight(&current_inputs);
        if let PreflightResult::NeedsPermission(perm) = preflight2 {
            let run_state = store::load_run_state(&app, &run_id).await?;
            let canonical_intent = canonicalize_tool_intent(
                &capability_name,
                &current_inputs,
                Some(&run_state.goal),
                Some(&perm.reason),
                capability.risk_level(),
            );
            let auto_approved = is_auto_approved(
                &capability_name,
                capability.risk_level(),
                &canonical_intent,
            );
            if auto_approved {
                if let Err(e) = create_ticket(
                    &app,
                    &step_id,
                    &run_id,
                    &step_id,
                    &capability_name,
                    &current_inputs,
                    &canonical_intent,
                    "Operation completed",
                    PermissionState::AutoApproved,
                    TicketExecutionState::Pending,
                )
                .await
                {
                    tracing::warn!("[run_direct_path] create_ticket (post-input) failed: {}", e);
                    return Err(e);
                }
                if let Err(e) = mark_execution_started(&app, &step_id).await {
                    tracing::warn!("[run_direct_path] mark_execution_started failed: {}", e);
                }
            } else {
                let ticket_id = step_id.clone();
                if let Err(e) = create_ticket(
                    &app,
                    &ticket_id,
                    &run_id,
                    &step_id,
                    &capability_name,
                    &current_inputs,
                    &canonical_intent,
                    "Operation completed",
                    PermissionState::Requested,
                    TicketExecutionState::Pending,
                )
                .await
                {
                    tracing::warn!("[run_direct_path] create_ticket (post-input) failed: {}", e);
                    return Err(e);
                }
                store::append_event(
                    &app,
                    &run_id,
                    PERMISSION_REQUESTED,
                    json!({
                        "permission_id": ticket_id,
                        "ticket_id": ticket_id,
                        "capability": capability_name,
                        "scope": capability_name,
                        "reason": perm.reason,
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
                .await?;
                store::append_event(
                    &app,
                    &run_id,
                    RUN_STATUS_CHANGED,
                    json!({ "status": "waiting_permission" }),
                )
                .await?;

                let (tx, mut rx) = mpsc::channel::<bool>(1);
                let manager = app
                    .try_state::<crate::agent::r#loop::run_loop::RunManager>()
                    .ok_or("Run manager not initialized")?;
                manager.register_permission_channel(ticket_id.clone(), tx).await;

                match rx.recv().await {
                    Some(true) => {
                        if let Err(e) = mark_permission_granted(&app, &ticket_id, &ticket_id).await
                        {
                            tracing::warn!("[run_direct_path] mark_permission_granted failed: {}", e);
                            return Err(e);
                        }
                        if let Err(e) = mark_execution_started(&app, &ticket_id).await {
                            tracing::warn!("[run_direct_path] mark_execution_started failed: {}", e);
                        }
                    }
                    _ => {
                        let _ = mark_permission_denied(&app, &ticket_id).await;
                        store::append_event(
                            &app,
                            &run_id,
                            RUN_STATUS_CHANGED,
                            json!({ "status": "failed", "reason": "Permission denied" }),
                        )
                        .await?;
                        return Err("Permission denied".to_string());
                    }
                }
            }
        } else {
            match preflight2 {
                PreflightResult::Reject(r) => {
                    store::append_event(
                        &app,
                        &run_id,
                        RUN_STATUS_CHANGED,
                        json!({ "status": "failed", "reason": r }),
                    )
                    .await?;
                    return Err(r);
                }
                PreflightResult::NeedsInput(_) => {
                    return Err("Still missing input after merge".to_string());
                }
                _ => {}
            }
        }
    }

    store::append_event(
        &app,
        &run_id,
        STEP_STARTED,
        json!({
            "step_id": step_id,
            "capability": capability_name,
            "intent": format!("Direct: {}", direct_command_variant),
            "expected_outcome": "Operation completed",
            "started_at": chrono::Utc::now()
        }),
    )
    .await?;

    let intent = format!("Direct command: {}", direct_command_variant);
    let expected_outcome = "Operation completed".to_string();
    let exec_result = Executor::execute(
        &app,
        ExecutionRequest {
            run_id: run_id.clone(),
            step_id: step_id.clone(),
            capability: capability_name.clone(),
            intent,
            expected_outcome: expected_outcome.clone(),
            inputs: current_inputs.clone(),
            retry_count: 0,
        },
    )
    .await;

    match &exec_result {
        Ok(result) => {
            let result_json = json!({
                "success": result.success,
                "confidence": result.confidence,
                "reason": result.reason,
            })
            .to_string();
            if result.success {
                let _ = mark_execution_completed(&app, &step_id, &result_json).await;
                store::append_event(
                    &app,
                    &run_id,
                    RUN_STATUS_CHANGED,
                    json!({ "status": "completed" }),
                )
                .await?;
            } else {
                let _ = mark_execution_failed(&app, &step_id, &result.reason).await;
                store::append_event(
                    &app,
                    &run_id,
                    RUN_STATUS_CHANGED,
                    json!({ "status": "failed", "reason": result.reason }),
                )
                .await?;
                return Err(result.reason.clone());
            }
        }
        Err(e) => {
            let _ = mark_execution_failed(&app, &step_id, e).await;
            store::append_event(
                &app,
                &run_id,
                RUN_STATUS_CHANGED,
                json!({ "status": "failed", "reason": e }),
            )
            .await?;
            return Err(e.clone());
        }
    }

    Ok(())
}
