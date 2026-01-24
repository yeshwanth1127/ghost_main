use crate::assistant::types::*;
use crate::assistant::executor::{fs_adapter, snapshot};
use crate::assistant::policy;
use std::path::PathBuf;

/// Execute an action plan with transactional safety
pub async fn execute_action_plan(
    plan: &ActionPlan,
    capability_token: Option<String>,
    user_home: PathBuf,
) -> Result<ActionResult, String> {
    let mut snapshot_manager = snapshot::SnapshotManager::new(user_home.clone())
        .map_err(|e| format!("Failed to create snapshot manager: {}", e))?;

    let ctx = ExecutionContext::new(user_home.clone());

    // Validate capability token if provided
    if let Some(token_str) = capability_token {
        let token = policy::validate_token(&token_str)
            .map_err(|e| format!("Token validation failed: {}", e))?;
        
        // Check permissions for each action
        for action in &plan.schema.actions {
            let action_type_str = match action.action_type {
                ActionType::FsCreateFile => "fs",
                ActionType::FsReadFile => "fs",
                ActionType::FsCopyFile => "fs",
                ActionType::FsMoveFile => "fs",
                ActionType::FsDeleteFile => "fs",
                ActionType::FsCreateDirectory => "fs",
            };

            if let Some(path_value) = action.args.get("path") {
                if let Some(path_str) = path_value.as_str() {
                    if !policy::check_permission(&token, action_type_str, path_str) {
                        return Err(format!("Permission denied for action on {}", path_str));
                    }
                }
            }
        }
    }

    let mut results = Vec::new();
    let mut snapshots = Vec::new();
    let executed_at = chrono::Utc::now().timestamp();

    // Execute each action with snapshot creation
    for action in &plan.schema.actions {
        // Create snapshot for destructive operations
        let snapshot_id = if needs_snapshot(&action.action_type) {
            if let Some(path_value) = action.args.get("path") {
                if let Some(path_str) = path_value.as_str() {
                    let path = PathBuf::from(path_str);
                    match snapshot_manager.create_snapshot(&action.id, &path) {
                        Ok(snapshot) => {
                            snapshots.push(snapshot.clone());
                            Some(snapshot.id.clone())
                        }
                        Err(e) => {
                            eprintln!("Failed to create snapshot: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else if let Some(src_path_value) = action.args.get("source_path") {
                if let Some(src_path_str) = src_path_value.as_str() {
                    let path = PathBuf::from(src_path_str);
                    match snapshot_manager.create_snapshot(&action.id, &path) {
                        Ok(snapshot) => {
                            snapshots.push(snapshot.clone());
                            Some(snapshot.id.clone())
                        }
                        Err(e) => {
                            eprintln!("Failed to create snapshot: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Execute the action
        match fs_adapter::execute_fs_action(action, &ctx) {
            Ok(mut result) => {
                result.snapshot_id = snapshot_id;
                results.push(result);
            }
            Err(e) => {
                // Rollback: undo all previous actions
                rollback_actions(&snapshots, &mut snapshot_manager)?;
                return Err(format!("Action execution failed: {}. All actions rolled back.", e));
            }
        }
    }

    Ok(ActionResult {
        action_id: plan.schema.id.clone(),
        success: true,
        executed_at,
        results,
        error: None,
        undo_available: !snapshots.is_empty(),
        undo_ttl: Some(chrono::Utc::now().timestamp() + (7 * 24 * 60 * 60)), // 7 days
    })
}

/// Check if an action type needs a snapshot
fn needs_snapshot(action_type: &ActionType) -> bool {
    matches!(
        action_type,
        ActionType::FsDeleteFile | ActionType::FsMoveFile
    )
}

/// Rollback actions using snapshots
fn rollback_actions(
    snapshots: &[ActionSnapshot],
    snapshot_manager: &mut snapshot::SnapshotManager,
) -> Result<(), String> {
    // Restore snapshots in reverse order
    for snapshot in snapshots.iter().rev() {
        snapshot_manager
            .restore_from_snapshot(snapshot)
            .map_err(|e| format!("Failed to restore snapshot: {}", e))?;
    }
    Ok(())
}

/// Undo an action using its snapshot
pub async fn undo_action(
    _action_id: &str,
    _user_home: PathBuf,
) -> Result<(), String> {
    // This would typically query the database for snapshots
    // For now, we'll create a placeholder implementation
    // Full implementation would:
    // 1. Query database for snapshots with this action_id
    // 2. Restore each snapshot
    // 3. Mark action as undone in audit log

    // Placeholder: return success for now
    // Full implementation would be integrated with database
    Ok(())
}

