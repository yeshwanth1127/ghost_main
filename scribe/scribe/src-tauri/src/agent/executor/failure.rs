// Failure Taxonomy - Classification and Recovery
//
// Failures must be categorized to enable intelligent recovery.
// This module answers: "What kind of failure was this, and what should we do?"

use serde::{Deserialize, Serialize};

/// Types of failures the agent can encounter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FailureKind {
    /// Planner (LLM) is unavailable or unreachable
    PlannerUnavailable,
    
    /// Planner (LLM) request timed out
    PlannerTimeout,
    
    /// Execution error (network timeout, I/O error, etc.)
    ExecutionError,
    
    /// Permission was denied
    PermissionDenied,
    
    /// Invalid assumption (file doesn't exist, path wrong, etc.)
    InvalidAssumption,
    
    /// Plan error (plan was flawed from the start)
    PlanError,
    
    /// Unknown failure type
    Unknown,
}

/// Recovery action to take based on failure kind
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry,
    
    /// Wait and retry (for transient failures like planner unavailability)
    WaitAndRetry,
    
    /// Revise the plan
    RevisePlan,
    
    /// Ask the user for help
    AskUser,
    
    /// Abort the run
    Abort,
}

/// Classify a failure based on error message
pub fn classify_failure(error: &str) -> FailureKind {
    let error_lower = error.to_lowercase();
    
    // Check for planner failures first (most specific)
    if error_lower.contains("planner") 
        || error_lower.contains("ollama") 
        || error_lower.contains("llm")
        || error_lower.contains("unreachable")
        || error_lower.contains("connection refused")
        || error_lower.contains("failed to connect") {
        // Check if it's a timeout specifically
        if error_lower.contains("timeout") {
            FailureKind::PlannerTimeout
        } else {
            FailureKind::PlannerUnavailable
        }
    } else if error_lower.contains("permission") || error_lower.contains("denied") || error_lower.contains("unauthorized") {
        FailureKind::PermissionDenied
    } else if error_lower.contains("not found") 
        || error_lower.contains("does not exist")
        || error_lower.contains("invalid path")
        || error_lower.contains("no such file")
        || error_lower.contains("missing") {
        FailureKind::InvalidAssumption
    } else if error_lower.contains("plan") || error_lower.contains("strategy") {
        FailureKind::PlanError
    } else if error_lower.contains("timeout")
        || error_lower.contains("network")
        || error_lower.contains("connection")
        || error_lower.contains("io error") {
        FailureKind::ExecutionError
    } else {
        FailureKind::Unknown
    }
}

/// Determine recovery action based on failure kind
pub fn recovery_for(kind: &FailureKind) -> RecoveryAction {
    match kind {
        FailureKind::PlannerUnavailable | FailureKind::PlannerTimeout => RecoveryAction::WaitAndRetry,
        FailureKind::PermissionDenied => RecoveryAction::AskUser,
        FailureKind::InvalidAssumption => RecoveryAction::RevisePlan,
        FailureKind::ExecutionError => RecoveryAction::Retry,
        FailureKind::PlanError => RecoveryAction::RevisePlan,
        FailureKind::Unknown => RecoveryAction::Abort,
    }
}

/// Get human-readable description of failure kind
pub fn failure_description(kind: &FailureKind) -> &'static str {
    match kind {
        FailureKind::PlannerUnavailable => "Planner unavailable - LLM service unreachable",
        FailureKind::PlannerTimeout => "Planner timeout - LLM request timed out",
        FailureKind::ExecutionError => "Execution error - may be transient",
        FailureKind::PermissionDenied => "Permission denied - requires user approval",
        FailureKind::InvalidAssumption => "Invalid assumption - plan needs revision",
        FailureKind::PlanError => "Plan error - strategy was flawed",
        FailureKind::Unknown => "Unknown failure - requires investigation",
    }
}

/// Check if a failure kind is a planner failure (should not affect plan confidence)
pub fn is_planner_failure(kind: &FailureKind) -> bool {
    matches!(kind, FailureKind::PlannerUnavailable | FailureKind::PlannerTimeout)
}
