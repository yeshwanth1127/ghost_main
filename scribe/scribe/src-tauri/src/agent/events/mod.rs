// Event types and constants
pub mod event;

// Re-export RunEvent for convenience
pub use event::RunEvent;

// Fact Events (Ontology - What Happened)
pub const RUN_CREATED: &str = "run.created";
pub const TOOL_EXECUTED: &str = "tool.executed";
pub const FILE_WRITTEN: &str = "file.written";
pub const FILE_READ: &str = "file.read";
pub const FILE_DELETED: &str = "file.deleted";
pub const PROCESS_COMPLETED: &str = "process.completed";
pub const PERMISSION_REQUESTED: &str = "permission.requested";
pub const PERMISSION_DECISION: &str = "permission.decision";
pub const MESSAGE_APPENDED: &str = "message.appended";
pub const DECISION_MADE: &str = "decision.made";
pub const DECISION_DIRECT_COMMAND_SELECTED: &str = "decision.direct_command_selected";

// Projection Events (Interpretations - What It Means)
// PROJECTION: These event types represent interpretations, not facts.
pub const RUN_STATUS_CHANGED: &str = "run.status_changed";
pub const STEP_STARTED: &str = "step.started";
pub const STEP_COMPLETED: &str = "step.completed";
pub const STEP_FAILED: &str = "step.failed";
pub const ARTIFACT_CREATED: &str = "artifact.created";

// Evaluation Events (Judgments - Did Reality Match Intention?)
// EVALUATION: These events judge whether outcomes matched expectations.
pub const STEP_EVALUATED: &str = "step.evaluated";
pub const RUN_EVALUATED: &str = "run.evaluated";

// Planning Events (Hypotheses - How Will We Achieve The Goal?)
// PLANNING: These events represent plans as first-class citizens.
pub const PLAN_CREATED: &str = "plan.created";
pub const PLAN_REVISED: &str = "plan.revised";
pub const PLAN_STEP_SELECTED: &str = "plan.step_selected";

// Reflection Events (Learning - What Should Change Next Time?)
// REFLECTION: These events capture lessons learned.
pub const RUN_REFLECTED: &str = "run.reflected";

// Failure Analysis Events (Classification - What Kind of Failure Was This?)
// FAILURE: These events classify failures for recovery.
pub const FAILURE_ANALYZED: &str = "failure.analyzed";
pub const PLANNER_FAILED: &str = "planner.failed";

// Planner visibility (exact prompt sent to LLM on every call)
pub const PLANNER_PROMPT_SENT: &str = "planner.prompt_sent";

// Input Request Events (User Interaction - Missing Required Information)
// INPUT: These events request missing inputs from the user.
pub const INPUT_REQUESTED: &str = "input.requested";
pub const INPUT_PROVIDED: &str = "input.provided";

// Ask User (Planner requested clarification - free-form question/answer)
pub const ASK_USER_REQUESTED: &str = "ask_user.requested";

// Router (embedding-based intent routing decision for debugging/tuning)
pub const ROUTER_DECISION: &str = "router.decision";
