// Event types and constants
pub mod event;

// Re-export RunEvent for convenience
pub use event::RunEvent;

// Fact Events (Ontology - What Happened)
pub const RUN_CREATED: &str = "run.created";
pub const TOOL_EXECUTED: &str = "tool.executed";
pub const FILE_WRITTEN: &str = "file.written";
pub const FILE_READ: &str = "file.read";
pub const PERMISSION_REQUESTED: &str = "permission.requested";
pub const PERMISSION_DECISION: &str = "permission.decision";
pub const MESSAGE_APPENDED: &str = "message.appended";
pub const DECISION_MADE: &str = "decision.made";

// Projection Events (Interpretations - What It Means)
// PROJECTION: These event types represent interpretations, not facts.
pub const RUN_STATUS_CHANGED: &str = "run.status_changed";
pub const STEP_STARTED: &str = "step.started";
pub const STEP_COMPLETED: &str = "step.completed";
pub const STEP_FAILED: &str = "step.failed";
pub const ARTIFACT_CREATED: &str = "artifact.created";
