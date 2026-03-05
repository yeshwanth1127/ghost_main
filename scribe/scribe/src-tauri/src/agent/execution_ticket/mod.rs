//! Execution tickets - one state machine per tool call (Moltbot-style).
//! Restart-safe, auditable; permission flow: create ticket -> permission (or auto-approve) -> execute -> completed/failed.

mod schema;
mod store;

pub use schema::{ExecutionState, ExecutionTicket, PermissionState, ToolIntent, ToolIntentContext};
#[allow(unused_imports)]
pub use store::{
    create_ticket, get_ticket, mark_execution_completed, mark_execution_failed,
    mark_execution_started, mark_permission_denied, mark_permission_granted,
};
