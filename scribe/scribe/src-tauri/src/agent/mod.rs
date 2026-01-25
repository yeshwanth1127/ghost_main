// Agent module - isolated from assistant logic
pub mod events;
pub mod run;
pub mod state;
pub mod r#loop;
pub mod tools;
pub mod memory;
pub mod permissions;

use crate::agent::run::store;
use crate::agent::state::RunState;
use crate::agent::events::RunEvent;
use crate::agent::r#loop::run_loop::RunManager;
use tauri::{AppHandle, Manager};

// Re-export for use in lib.rs
#[allow(unused_imports)] // Used in lib.rs setup
pub use crate::agent::r#loop::run_loop::resume_all_runs;

/// Create a new run
#[tauri::command]
pub async fn create_run(
    app: AppHandle,
    goal: String,
) -> Result<String, String> {
    store::create_run(&app, goal).await
}

/// Get run state by replaying events
#[tauri::command]
pub async fn get_run_state(
    app: AppHandle,
    run_id: String,
) -> Result<RunState, String> {
    store::load_run_state(&app, &run_id).await
}

/// Get all events for a run
#[tauri::command]
pub async fn get_run_events(
    app: AppHandle,
    run_id: String,
) -> Result<Vec<RunEvent>, String> {
    store::load_run_events(&app, &run_id).await
}

/// Start a run (starts the agent loop)
#[tauri::command]
pub async fn start_run(
    app: AppHandle,
    run_id: String,
) -> Result<(), String> {
    // Get or create run manager from app state
    let manager = app
        .try_state::<RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.start_run(app.clone(), run_id).await
}


/// Cancel a run
#[tauri::command]
pub async fn cancel_run(
    app: AppHandle,
    run_id: String,
) -> Result<(), String> {
    let manager = app
        .try_state::<RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.cancel_run(&run_id).await
}

/// Cancel all active runs
#[tauri::command]
pub async fn cancel_all_runs(app: AppHandle) -> Result<(), String> {
    let manager = app
        .try_state::<RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.cancel_all_runs().await
}

/// Reply to a permission request
#[tauri::command]
pub async fn reply_permission(
    app: AppHandle,
    run_id: String,
    permission_id: String,
    granted: bool,
) -> Result<(), String> {
    // Emit fact event: PERMISSION_DECISION
    store::append_event(
        &app,
        &run_id,
        crate::agent::events::PERMISSION_DECISION,
        serde_json::json!({
            "permission_id": permission_id.clone(),
            "granted": granted,
            "decided_at": chrono::Utc::now()
        }),
    )
    .await?;

    // Send decision to waiting loop via channel
    let manager = app
        .try_state::<crate::agent::r#loop::run_loop::RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.send_permission_decision(&permission_id, granted).await?;

    // Update run status back to running if granted
    if granted {
        store::append_event(
            &app,
            &run_id,
            crate::agent::events::RUN_STATUS_CHANGED,
            serde_json::json!({
                "status": "running"
            }),
        )
        .await?;
    } else {
        // If denied, status remains waiting or can be set to failed
        // The loop will replan
    }

    Ok(())
}
