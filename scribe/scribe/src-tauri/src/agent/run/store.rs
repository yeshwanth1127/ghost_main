// Run store - database operations for runs and events

use crate::agent::events::{RunEvent, RUN_CREATED};
use crate::agent::state::run_state::*;
use crate::agent::state::RunState;
use chrono::Utc;
use serde_json::json;
use tauri::{AppHandle, Manager, Emitter};
use tauri_plugin_sql::DbInstances;
use sqlx::Row;

/// Get the SQLite pool from DbInstances
fn get_sqlite_pool<'a>(
    instances_guard: &'a tokio::sync::RwLockReadGuard<'a, std::collections::HashMap<String, tauri_plugin_sql::DbPool>>,
) -> Result<&'a sqlx::SqlitePool, String> {
    let db_pool = instances_guard
        .get("sqlite:ghost.db")
        .ok_or_else(|| "Database instance not found".to_string())?;
    
    match db_pool {
        tauri_plugin_sql::DbPool::Sqlite(ref pool) => Ok(pool),
    }
}

/// Create a new run and emit run.created event
pub async fn create_run(
    app: &AppHandle,
    goal: String,
) -> Result<String, String> {
    let run_id = uuid::Uuid::new_v4().to_string();

    // Insert run into database
    let status = "pending".to_string();
    let instances = app.state::<DbInstances>();
    let instances_guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&instances_guard)?;
    
    sqlx::query::<sqlx::Sqlite>("INSERT INTO runs (id, goal, status) VALUES (?1, ?2, ?3)")
        .bind(&run_id)
        .bind(&goal)
        .bind(&status)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to create run: {}", e))?;

    // Emit run.created event
    let payload = json!({
        "run_id": run_id,
        "goal": goal,
        "created_at": Utc::now()
    });

    append_event(app, &run_id, RUN_CREATED, payload).await?;

    Ok(run_id)
}

/// Append an event to the event log (append-only)
pub async fn append_event(
    app: &AppHandle,
    run_id: &str,
    event_type: &str,
    payload: serde_json::Value,
) -> Result<RunEvent, String> {
    let now = Utc::now();

    // Insert event into database
    let payload_str = payload.to_string();
    let created_at_str = now.to_rfc3339();
    let instances = app.state::<DbInstances>();
    let instances_guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&instances_guard)?;
    
    sqlx::query::<sqlx::Sqlite>("INSERT INTO run_events (run_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4)")
        .bind(&run_id)
        .bind(&event_type)
        .bind(&payload_str)
        .bind(&created_at_str)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to append event: {}", e))?;

    // Get the inserted event ID (SQLite last_insert_rowid)
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query::<sqlx::Sqlite>(
        "SELECT id, run_id, event_type, payload, created_at FROM run_events WHERE run_id = ?1 ORDER BY id DESC LIMIT 1"
    )
        .bind(&run_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch event: {}", e))?;

    let events: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| {
            json!({
                "id": row.get::<i64, _>("id"),
                "run_id": row.get::<String, _>("run_id"),
                "event_type": row.get::<String, _>("event_type"),
                "payload": row.get::<String, _>("payload"),
                "created_at": row.get::<String, _>("created_at"),
            })
        })
        .collect();

    let event_data = events
        .first()
        .ok_or("Failed to retrieve inserted event")?;

    let event = RunEvent {
        id: event_data["id"]
            .as_i64()
            .ok_or("Invalid event ID")?,
        run_id: event_data["run_id"]
            .as_str()
            .ok_or("Invalid run_id")?
            .to_string(),
        event_type: event_data["event_type"]
            .as_str()
            .ok_or("Invalid event_type")?
            .to_string(),
        payload: event_data["payload"].clone(),
        created_at: event_data["created_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc))
            .ok_or("Invalid created_at")?,
    };

    // Emit Tauri event for UI
    app.emit("run_event", &event)
        .map_err(|e| format!("Failed to emit Tauri event: {}", e))?;

    Ok(event)
}

/// Load all events for a run, ordered by id ASC
pub async fn load_run_events(
    app: &AppHandle,
    run_id: &str,
) -> Result<Vec<RunEvent>, String> {
    let instances = app.state::<DbInstances>();
    let instances_guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&instances_guard)?;
    
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query::<sqlx::Sqlite>(
        "SELECT id, run_id, event_type, payload, created_at FROM run_events WHERE run_id = ?1 ORDER BY id ASC"
    )
        .bind(&run_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load events: {}", e))?;

    let events: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| {
            json!({
                "id": row.get::<i64, _>("id"),
                "run_id": row.get::<String, _>("run_id"),
                "event_type": row.get::<String, _>("event_type"),
                "payload": row.get::<String, _>("payload"),
                "created_at": row.get::<String, _>("created_at"),
            })
        })
        .collect();

    let mut run_events = Vec::new();

    for event_data in events {
        let payload_str = event_data["payload"]
            .as_str()
            .ok_or("Invalid payload")?;
        let payload: serde_json::Value = serde_json::from_str(payload_str)
            .map_err(|e| format!("Failed to parse payload: {}", e))?;

        let created_at = event_data["created_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc))
            .ok_or("Invalid created_at")?;

        run_events.push(RunEvent {
            id: event_data["id"]
                .as_i64()
                .ok_or("Invalid event ID")?,
            run_id: event_data["run_id"]
                .as_str()
                .ok_or("Invalid run_id")?
                .to_string(),
            event_type: event_data["event_type"]
                .as_str()
                .ok_or("Invalid event_type")?
                .to_string(),
            payload,
            created_at,
        });
    }

    Ok(run_events)
}

/// Load run state by replaying all events
pub async fn load_run_state(
    app: &AppHandle,
    run_id: &str,
) -> Result<RunState, String> {
    // First, get the run record
    let instances = app.state::<DbInstances>();
    let instances_guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&instances_guard)?;
    
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query::<sqlx::Sqlite>("SELECT id, goal, status FROM runs WHERE id = ?1")
        .bind(&run_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load run: {}", e))?;

    let runs: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| {
            json!({
                "id": row.get::<String, _>("id"),
                "goal": row.get::<String, _>("goal"),
                "status": row.get::<String, _>("status"),
            })
        })
        .collect();

    let run_data = runs
        .first()
        .ok_or(format!("Run {} not found", run_id))?;

    let goal = run_data["goal"]
        .as_str()
        .ok_or("Invalid goal")?
        .to_string();

    // Create initial state
    let mut state = RunState::new(run_id.to_string(), goal);

    // Load all events and replay them
    let events = load_run_events(app, run_id).await?;

    for event in events {
        apply_event(&mut state, &event);
    }

    Ok(state)
}

/// Get all non-terminal runs
pub async fn get_non_terminal_runs(
    app: &AppHandle,
) -> Result<Vec<String>, String> {
    let instances = app.state::<DbInstances>();
    let instances_guard = instances.inner().0.read().await;
    let pool = get_sqlite_pool(&instances_guard)?;
    
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query::<sqlx::Sqlite>("SELECT id FROM runs WHERE status NOT IN ('completed', 'failed', 'cancelled')")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load runs: {}", e))?;

    let runs: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| {
            json!({
                "id": row.get::<String, _>("id"),
            })
        })
        .collect();

    Ok(runs
        .into_iter()
        .filter_map(|r| r["id"].as_str().map(|s| s.to_string()))
        .collect())
}

/// Detect last completed side-effect for restart safety
pub async fn detect_last_completed_side_effect(
    app: &AppHandle,
    run_id: &str,
) -> Result<Option<i64>, String> {
    let events = load_run_events(app, run_id).await?;
    
    // Find last TOOL_EXECUTED event
    let mut last_tool_event_id: Option<i64> = None;
    let mut last_file_event_id: Option<i64> = None;
    
    for event in events.iter().rev() {
        match event.event_type.as_str() {
            crate::agent::events::FILE_WRITTEN | crate::agent::events::FILE_READ => {
                if last_file_event_id.is_none() {
                    last_file_event_id = Some(event.id);
                }
            }
            crate::agent::events::TOOL_EXECUTED => {
                last_tool_event_id = Some(event.id);
                break; // Found the tool execution, check if file event exists
            }
            _ => {}
        }
    }
    
    // If we have both TOOL_EXECUTED and FILE_WRITTEN/FILE_READ, side effect is complete
    if let Some(_tool_id) = last_tool_event_id {
        if last_file_event_id.is_some() {
            // Side effect is complete - return the file event ID as the safe point
            return Ok(last_file_event_id);
        }
        // Tool executed but no file event - incomplete, return None to indicate unsafe
        return Ok(None);
    }
    
    // No tool execution found - safe to start from beginning
    Ok(Some(0))
}
