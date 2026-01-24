use crate::assistant::types::*;
use crate::assistant::validator::validate_path;
use std::path::PathBuf;

/// Verify and validate an action plan
pub fn verify_action_plan(plan: &ActionPlan, user_home: &PathBuf) -> Result<VerifiedPlan, String> {
    let mut notes = Vec::new();
    let mut risk_score = plan.schema.risk_score;

    // Validate each action
    for action in &plan.schema.actions {
        // Type check
        validate_action_type(&action.action_type)?;

        // Path validation (skip if path is placeholder)
        if let Some(path_value) = action.args.get("path") {
            if let Some(path_str) = path_value.as_str() {
                // Skip validation for placeholder paths
                if path_str != "__PROMPT_PATH__" {
                    validate_path(path_str, user_home)?;
                }
            }
        }

        if let Some(src_path_value) = action.args.get("source_path") {
            if let Some(src_path_str) = src_path_value.as_str() {
                // Skip validation for placeholder paths
                if src_path_str != "__PROMPT_PATH__" {
                    validate_path(src_path_str, user_home)?;
                }
            }
        }

        if let Some(dst_path_value) = action.args.get("destination_path") {
            if let Some(dst_path_str) = dst_path_value.as_str() {
                // Skip validation for placeholder paths
                if dst_path_str != "__PROMPT_PATH__" {
                    validate_path(dst_path_str, user_home)?;
                }
            }
        }

        // Check preconditions
        if let Some(preconditions) = &action.preconditions {
            validate_preconditions(action, preconditions)?;
        }

        // Recalculate risk based on action type
        let action_risk = calculate_action_risk(&action.action_type);
        risk_score = (risk_score * 0.7 + action_risk * 0.3).min(1.0);
    }

    Ok(VerifiedPlan {
        plan: plan.clone(),
        verified_at: chrono::Utc::now().timestamp(),
        verification_notes: notes,
    })
}

/// Validate action type
fn validate_action_type(action_type: &ActionType) -> Result<(), String> {
    match action_type {
        ActionType::FsCreateFile
        | ActionType::FsReadFile
        | ActionType::FsCopyFile
        | ActionType::FsMoveFile
        | ActionType::FsDeleteFile
        | ActionType::FsCreateDirectory => Ok(()),
    }
}

/// Validate preconditions
fn validate_preconditions(action: &Action, preconditions: &Precondition) -> Result<(), String> {
    // Check if file exists (skip if path is placeholder)
    if let Some(should_exist) = preconditions.exists {
        if let Some(path_value) = action.args.get("path") {
            if let Some(path_str) = path_value.as_str() {
                // Skip precondition check for placeholder paths
                if path_str == "__PROMPT_PATH__" {
                    return Ok(()); // Will be validated after user provides path
                }
                let path = std::path::Path::new(path_str);
                let exists = path.exists();
                if should_exist && !exists {
                    return Err(format!("Precondition failed: file should exist but doesn't: {}", path_str));
                }
                if !should_exist && exists {
                    return Err(format!("Precondition failed: file should not exist but does: {}", path_str));
                }
            }
        }
    }

    // Check if writable (skip if path is placeholder)
    if let Some(should_be_writable) = preconditions.writable {
        if let Some(path_value) = action.args.get("path") {
            if let Some(path_str) = path_value.as_str() {
                // Skip precondition check for placeholder paths
                if path_str == "__PROMPT_PATH__" {
                    return Ok(()); // Will be validated after user provides path
                }
                let path = std::path::Path::new(path_str);
                if let Some(parent) = path.parent() {
                    let metadata_result = std::fs::metadata(parent);
                    if let Ok(_metadata) = metadata_result {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let permissions = _metadata.permissions();
                            let writable = permissions.mode() & 0o200 != 0;
                            if should_be_writable && !writable {
                                return Err(format!("Precondition failed: directory not writable: {}", path_str));
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            // On Windows, assume writable if we can write
                            if should_be_writable {
                                // Try to create a test file
                                if let Err(_) = std::fs::File::create(parent.join(".test_write")) {
                                    return Err(format!("Precondition failed: directory not writable: {}", path_str));
                                }
                                let _ = std::fs::remove_file(parent.join(".test_write"));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Calculate risk score for action type
fn calculate_action_risk(action_type: &ActionType) -> f64 {
    match action_type {
        ActionType::FsReadFile | ActionType::FsCreateDirectory | ActionType::FsCreateFile => {
            RiskScore::Low.value()
        }
        ActionType::FsCopyFile => RiskScore::Medium.value(),
        ActionType::FsMoveFile => RiskScore::High.value(),
        ActionType::FsDeleteFile => RiskScore::High.value(),
    }
}

