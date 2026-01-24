use crate::assistant::*;
use crate::assistant::planner::{deterministic, verifier};
use crate::assistant::executor::worker;
use crate::assistant::policy;
use crate::assistant::audit::AuditLog;
use tauri::AppHandle;

/// Parse user intent using deterministic parser
#[tauri::command]
pub async fn parse_intent(
    _app: AppHandle,
    user_input: String,
) -> Result<ActionPlan, String> {
    let user_home = dirs::home_dir()
        .ok_or("Failed to get user home directory".to_string())?;

    deterministic::parse_intent(&user_input, &user_home)
}

/// Validate an LLM-generated action plan (LLM planning happens in frontend)
#[tauri::command]
pub async fn plan_with_llm(
    _app: AppHandle,
    plan: ActionPlan, // LLM-generated plan from frontend
) -> Result<VerifiedPlan, String> {
    // Validate the LLM-generated plan
    let user_home = dirs::home_dir()
        .ok_or("Failed to get user home directory".to_string())?;

    verifier::verify_action_plan(&plan, &user_home)
}

/// Verify an action plan
#[tauri::command]
pub async fn verify_action_plan(
    _app: AppHandle,
    plan: ActionPlan,
) -> Result<VerifiedPlan, String> {
    let user_home = dirs::home_dir()
        .ok_or("Failed to get user home directory".to_string())?;

    verifier::verify_action_plan(&plan, &user_home)
}

/// Preview an action plan (returns preview with risk score)
#[tauri::command]
pub async fn preview_action_plan(
    _app: AppHandle,
    plan: ActionPlan,
) -> Result<PreviewResult, String> {
    let user_home = dirs::home_dir()
        .ok_or("Failed to get user home directory".to_string())?;

    // Verify the plan
    let verified = verifier::verify_action_plan(&plan, &user_home)?;

    // Build preview result
    let mut affected_items = Vec::new();
    let mut missing_paths = Vec::new();
    let warnings: Vec<String> = verified.verification_notes.clone();

    for action in &plan.schema.actions {
        let mut needs_path = false;
        
        if let Some(path_value) = action.args.get("path") {
            if let Some(path_str) = path_value.as_str() {
                if path_str == "__PROMPT_PATH__" {
                    needs_path = true;
                    missing_paths.push(action.id.clone());
                    affected_items.push(AffectedItem {
                        path: "[Path needed]".to_string(),
                        operation: format!("{:?}", action.action_type),
                        preview: None,
                    });
                } else {
                    affected_items.push(AffectedItem {
                        path: path_str.to_string(),
                        operation: format!("{:?}", action.action_type),
                        preview: None,
                    });
                }
            }
        }

        if let Some(dst_path_value) = action.args.get("destination_path") {
            if let Some(dst_path_str) = dst_path_value.as_str() {
                if dst_path_str == "__PROMPT_PATH__" {
                    needs_path = true;
                    missing_paths.push(action.id.clone());
                    affected_items.push(AffectedItem {
                        path: "[Destination path needed]".to_string(),
                        operation: format!("{:?}", action.action_type),
                        preview: None,
                    });
                } else {
                    affected_items.push(AffectedItem {
                        path: dst_path_str.to_string(),
                        operation: format!("{:?}", action.action_type),
                        preview: None,
                    });
                }
            }
        }

        if let Some(src_path_value) = action.args.get("source_path") {
            if let Some(src_path_str) = src_path_value.as_str() {
                if src_path_str == "__PROMPT_PATH__" {
                    needs_path = true;
                    missing_paths.push(action.id.clone());
                }
            }
        }
    }

    // Check if requires explicit confirmation (high risk or delete operation)
    let requires_explicit_confirmation = plan.schema.risk_score > 0.7
        || plan.schema.actions.iter().any(|a| matches!(a.action_type, ActionType::FsDeleteFile))
        || !missing_paths.is_empty(); // Also require confirmation if paths missing

    Ok(PreviewResult {
        plan,
        risk_score: verified.plan.schema.risk_score,
        affected_items,
        warnings: verified.verification_notes,
        requires_explicit_confirmation,
        missing_paths,
    })
}

/// Execute an action plan
#[tauri::command]
pub async fn execute_action_plan(
    app: AppHandle,
    plan: ActionPlan,
    confirm_token: Option<String>,
) -> Result<ActionResult, String> {
    let user_home = dirs::home_dir()
        .ok_or("Failed to get user home directory".to_string())?;

    // Verify plan before execution
    verifier::verify_action_plan(&plan, &user_home)?;

    // Execute the plan
    let result = worker::execute_action_plan(&plan, confirm_token, user_home.clone()).await?;

    // Prepare audit log entry (frontend will save to database)
    // The audit log entry is returned in the result for frontend to persist
    let _audit_log = AuditLog::new(app.clone());
    // Frontend will handle saving to database via database actions

    Ok(result)
}

/// Undo an action
#[tauri::command]
pub async fn undo_action(
    _app: AppHandle,
    action_id: String,
) -> Result<(), String> {
    let user_home = dirs::home_dir()
        .ok_or("Failed to get user home directory".to_string())?;

    worker::undo_action(&action_id, user_home).await
}

/// Get audit history
#[tauri::command]
pub async fn get_audit_history(
    app: AppHandle,
    limit: i32,
) -> Result<Vec<AuditEntry>, String> {
    // Audit history is fetched via frontend database actions
    // This returns empty - frontend should use getAuditLogsFromDB() instead
    Ok(vec![])
}

/// Mint a capability token
#[tauri::command]
pub async fn mint_capability_token(
    _app: AppHandle,
    scopes: Vec<String>,
    ttl_seconds: i32,
    session_id: String,
) -> Result<String, String> {
    let token = policy::mint_capability_token(
        scopes,
        ttl_seconds as i64,
        session_id,
    )?;

    // Return token as JSON string (will be encoded in frontend)
    serde_json::to_string(&token)
        .map_err(|e| format!("Failed to serialize token: {}", e))
}

