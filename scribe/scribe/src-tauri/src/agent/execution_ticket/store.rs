//! Execution ticket store - CRUD for execution_tickets table.

use super::schema::{ExecutionState, ExecutionTicket, PermissionState, ToolIntent};
use sqlx::Row;
use tauri::{AppHandle, Manager};
use tauri_plugin_sql::DbInstances;

fn get_sqlite_pool<'a>(
    instances_guard: &'a tokio::sync::RwLockReadGuard<
        'a,
        std::collections::HashMap<String, tauri_plugin_sql::DbPool>,
    >,
) -> Result<&'a sqlx::SqlitePool, String> {
    let db_pool = instances_guard
        .get("sqlite:ghost.db")
        .ok_or_else(|| "Database instance not found".to_string())?;
    match db_pool {
        tauri_plugin_sql::DbPool::Sqlite(ref pool) => Ok(pool),
    }
}

/// Create a new execution ticket (permission_state = requested or auto_approved).
pub async fn create_ticket(
    app: &AppHandle,
    ticket_id: &str,
    run_id: &str,
    step_id: &str,
    capability: &str,
    inputs: &serde_json::Value,
    canonical_intent: &ToolIntent,
    expected_outcome: &str,
    permission_state: PermissionState,
    execution_state: ExecutionState,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let inputs_str = inputs.to_string();
    let intent_str = serde_json::to_string(canonical_intent).map_err(|e| e.to_string())?;

    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    sqlx::query::<sqlx::Sqlite>(
        r#"
        INSERT INTO execution_tickets (
            ticket_id, run_id, step_id, capability, inputs, canonical_intent,
            expected_outcome, permission_state, execution_state,
            created_at, retry_count, max_retries
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 3)
        "#,
    )
    .bind(ticket_id)
    .bind(run_id)
    .bind(step_id)
    .bind(capability)
    .bind(&inputs_str)
    .bind(&intent_str)
    .bind(expected_outcome)
    .bind(permission_state.as_str())
    .bind(execution_state.as_str())
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create ticket: {}", e))?;

    Ok(())
}

/// Get ticket by id.
pub async fn get_ticket(app: &AppHandle, ticket_id: &str) -> Result<Option<ExecutionTicket>, String> {
    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    let row = sqlx::query::<sqlx::Sqlite>(
        "SELECT ticket_id, run_id, step_id, capability, inputs, canonical_intent, expected_outcome, permission_state, execution_state, permission_id, execution_result, failure_reason, created_at, permission_granted_at, execution_started_at, execution_completed_at, retry_count, max_retries FROM execution_tickets WHERE ticket_id = ?1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get ticket: {}", e))?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let canonical_intent: ToolIntent =
        serde_json::from_str(&row.get::<String, _>("canonical_intent")).map_err(|e| e.to_string())?;
    let inputs: serde_json::Value =
        serde_json::from_str(&row.get::<String, _>("inputs")).map_err(|e| e.to_string())?;

    let permission_state = match row.get::<String, _>("permission_state").as_str() {
        "granted" => PermissionState::Granted,
        "denied" => PermissionState::Denied,
        "auto_approved" => PermissionState::AutoApproved,
        _ => PermissionState::Requested,
    };
    let execution_state = match row.get::<String, _>("execution_state").as_str() {
        "running" => ExecutionState::Running,
        "completed" => ExecutionState::Completed,
        "failed" => ExecutionState::Failed,
        "cancelled" => ExecutionState::Cancelled,
        _ => ExecutionState::Pending,
    };

    Ok(Some(ExecutionTicket {
        ticket_id: row.get::<String, _>("ticket_id"),
        run_id: row.get::<String, _>("run_id"),
        step_id: row.get::<String, _>("step_id"),
        capability: row.get::<String, _>("capability"),
        inputs,
        canonical_intent,
        expected_outcome: row.get::<String, _>("expected_outcome"),
        permission_state,
        execution_state,
        permission_id: row.get::<Option<String>, _>("permission_id"),
        execution_result: row.get::<Option<String>, _>("execution_result"),
        failure_reason: row.get::<Option<String>, _>("failure_reason"),
        created_at: row.get::<i64, _>("created_at"),
        permission_granted_at: row.get::<Option<i64>, _>("permission_granted_at"),
        execution_started_at: row.get::<Option<i64>, _>("execution_started_at"),
        execution_completed_at: row.get::<Option<i64>, _>("execution_completed_at"),
        retry_count: row.get::<i32, _>("retry_count"),
        max_retries: row.get::<i32, _>("max_retries"),
    }))
}

/// Mark permission granted and set execution_state to running.
pub async fn mark_permission_granted(
    app: &AppHandle,
    ticket_id: &str,
    permission_id: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    sqlx::query::<sqlx::Sqlite>(
        "UPDATE execution_tickets SET permission_state = 'granted', permission_id = ?1, permission_granted_at = ?2, execution_state = 'running', execution_started_at = ?2 WHERE ticket_id = ?3",
    )
    .bind(permission_id)
    .bind(now)
    .bind(ticket_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark permission granted: {}", e))?;

    Ok(())
}

/// Mark permission denied.
pub async fn mark_permission_denied(app: &AppHandle, ticket_id: &str) -> Result<(), String> {
    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    sqlx::query::<sqlx::Sqlite>(
        "UPDATE execution_tickets SET permission_state = 'denied', execution_state = 'cancelled' WHERE ticket_id = ?1",
    )
    .bind(ticket_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark permission denied: {}", e))?;

    Ok(())
}

/// Mark execution started (for auto_approved path).
pub async fn mark_execution_started(app: &AppHandle, ticket_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    sqlx::query::<sqlx::Sqlite>(
        "UPDATE execution_tickets SET execution_state = 'running', execution_started_at = ?1 WHERE ticket_id = ?2",
    )
    .bind(now)
    .bind(ticket_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark execution started: {}", e))?;

    Ok(())
}

/// Mark execution completed.
pub async fn mark_execution_completed(
    app: &AppHandle,
    ticket_id: &str,
    execution_result: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    sqlx::query::<sqlx::Sqlite>(
        "UPDATE execution_tickets SET execution_state = 'completed', execution_result = ?1, execution_completed_at = ?2 WHERE ticket_id = ?3",
    )
    .bind(execution_result)
    .bind(now)
    .bind(ticket_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark execution completed: {}", e))?;

    Ok(())
}

/// Mark execution failed.
pub async fn mark_execution_failed(
    app: &AppHandle,
    ticket_id: &str,
    failure_reason: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let instances = app.state::<DbInstances>();
    let guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&guard)?;

    sqlx::query::<sqlx::Sqlite>(
        "UPDATE execution_tickets SET execution_state = 'failed', failure_reason = ?1, execution_completed_at = ?2 WHERE ticket_id = ?3",
    )
    .bind(failure_reason)
    .bind(now)
    .bind(ticket_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark execution failed: {}", e))?;

    Ok(())
}
