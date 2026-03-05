// Agent module - isolated from assistant logic
pub mod events;
pub mod execution_ticket;
pub mod intent;
pub mod router;
pub mod run;
pub mod state;
pub mod r#loop;
pub mod tools;
pub mod memory;
pub mod permissions;
pub mod capabilities;
pub mod executor;
pub mod llm;
pub mod planner;

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

/// Get belief state for a run (cognitive projection)
#[tauri::command]
pub async fn get_belief_state(
    app: AppHandle,
    run_id: String,
) -> Result<crate::agent::state::BeliefState, String> {
    let events = store::load_run_events(&app, &run_id).await?;
    Ok(crate::agent::state::load_belief_state(&events))
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

/// Clear all runs, run_events, and execution_tickets from the database. Cannot be undone.
#[tauri::command]
pub async fn clear_all_runs(app: AppHandle) -> Result<(), String> {
    store::clear_all_runs(&app).await
}

/// Check Ollama: list available models and whether configured model exists. Use for UI hints when planner gets 404.
#[tauri::command]
pub async fn check_ollama() -> crate::agent::llm::OllamaCheck {
    crate::agent::llm::check_ollama().await
}

/// Reply to an ask_user (planner clarification) request
#[tauri::command]
pub async fn reply_ask_user(
    app: AppHandle,
    request_id: String,
    answer: String,
) -> Result<(), String> {
    let manager = app
        .try_state::<crate::agent::r#loop::run_loop::RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.send_ask_user_reply(&request_id, answer).await
}

/// Cancel an input request (closes the channel so the run loop receives None and cancels).
#[tauri::command]
pub async fn cancel_input_request(
    app: AppHandle,
    input_request_id: String,
) -> Result<(), String> {
    let manager = app
        .try_state::<crate::agent::r#loop::run_loop::RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.cancel_input_request(&input_request_id).await
}

/// Reply to an input request
#[tauri::command]
pub async fn reply_input(
    app: AppHandle,
    run_id: String,
    input_request_id: String,
    inputs: serde_json::Value,
) -> Result<(), String> {
    // Emit fact event: INPUT_PROVIDED
    store::append_event(
        &app,
        &run_id,
        crate::agent::events::INPUT_PROVIDED,
        serde_json::json!({
            "input_request_id": input_request_id.clone(),
            "provided_inputs": inputs,
            "provided_at": chrono::Utc::now()
        }),
    )
    .await?;

    // Send inputs to waiting loop via channel
    let manager = app
        .try_state::<crate::agent::r#loop::run_loop::RunManager>()
        .ok_or("Run manager not initialized")?;
    manager.send_input_response(&input_request_id, inputs).await?;

    // Update run status back to running
    store::append_event(
        &app,
        &run_id,
        crate::agent::events::RUN_STATUS_CHANGED,
        serde_json::json!({
            "status": "running"
        }),
    )
    .await?;

    Ok(())
}
