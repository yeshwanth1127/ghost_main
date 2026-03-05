// Plan Revision Dampening - Prevent Oscillation
//
// Plan revisions can oscillate if not controlled:
// - Failure → revise → confidence drop → revise again → loop
//
// This module provides dampening mechanisms to prevent thrashing.

use chrono::{DateTime, Utc};

/// Revision state tracker
#[derive(Debug, Clone)]
pub struct RevisionState {
    pub last_revision_step_id: Option<String>,
    pub last_revision_time: Option<DateTime<Utc>>,
    pub revision_count: u32,
    pub consecutive_failures: u32,
}

impl RevisionState {
    pub fn new() -> Self {
        Self {
            last_revision_step_id: None,
            last_revision_time: None,
            revision_count: 0,
            consecutive_failures: 0,
        }
    }
}

/// Check if plan revision should be allowed
/// 
/// Rules:
/// - Minimum steps between revisions (default: 2)
/// - Cooldown period (default: 5 seconds)
/// - Require multiple signals (failure + low confidence)
pub fn should_allow_revision(
    state: &RevisionState,
    _current_step_id: &str,
    current_confidence: f32,
    has_failure: bool,
    _min_steps_between: u32,
    cooldown_seconds: u32,
) -> bool {
    // Check cooldown
    if let Some(last_time) = state.last_revision_time {
        let elapsed = Utc::now().signed_duration_since(last_time);
        if elapsed.num_seconds() < cooldown_seconds as i64 {
            return false; // Still in cooldown
        }
    }
    
    // Check minimum steps (if we have a last revision step)
    // This is approximate - we'd need step counting, but for now we use time-based
    
    // Require multiple signals (failure + low confidence)
    // This prevents single-failure revisions
    let has_low_confidence = current_confidence < 0.3;
    let has_multiple_signals = has_failure && has_low_confidence;
    
    // Also allow if we have too many consecutive failures
    let too_many_failures = state.consecutive_failures >= 3;
    
    has_multiple_signals || too_many_failures
}

/// Update revision state after a revision
pub fn record_revision(
    state: &mut RevisionState,
    step_id: String,
) {
    state.last_revision_step_id = Some(step_id);
    state.last_revision_time = Some(Utc::now());
    state.revision_count += 1;
    state.consecutive_failures = 0; // Reset on revision
}

/// Update revision state after a failure
/// 
/// Note: Planner failures should NOT be counted (they're infrastructure, not plan failures)
pub fn record_failure(state: &mut RevisionState) {
    state.consecutive_failures += 1;
}

/// Check if a failure should be counted toward plan revision
/// Planner failures should NOT count
pub fn should_count_failure(failure_kind: Option<&crate::agent::executor::failure::FailureKind>) -> bool {
    if let Some(kind) = failure_kind {
        !crate::agent::executor::failure::is_planner_failure(kind)
    } else {
        true // Unknown failures count (conservative)
    }
}

/// Update revision state after a success
pub fn record_success(state: &mut RevisionState) {
    state.consecutive_failures = 0;
}
