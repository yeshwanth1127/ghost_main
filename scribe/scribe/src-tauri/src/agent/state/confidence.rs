// Confidence Calibration - Normalization Rules
//
// Confidence must be consistent across steps to enable reliable control.
// Without calibration, confidence can drift unpredictably over long runs.
//
// This module provides normalization rules to keep confidence meaningful.

/// Normalize confidence after a step evaluation
/// 
/// Rules:
/// - Success: slight boost (but bounded)
/// - Failure: decay (but not to zero immediately)
/// - Retry: additional decay per retry
/// - Independent success: larger boost
pub fn normalize_confidence(
    current: f32,
    step_success: bool,
    step_confidence: f32,
    retry_count: u32,
) -> f32 {
    let mut normalized = current;
    
    if step_success {
        // Success: weighted average with step confidence
        // This prevents unbounded growth
        normalized = (normalized * 0.7 + step_confidence * 0.3).min(1.0);
        
        // Independent success (first attempt) gets larger boost
        if retry_count == 0 {
            normalized = (normalized + 0.1).min(1.0);
        }
    } else {
        // Failure: exponential decay based on retry count
        let decay_factor = 0.8_f32.powi(retry_count as i32);
        normalized *= decay_factor;
        
        // Minimum floor (don't go below 0.1)
        normalized = normalized.max(0.1);
    }
    
    normalized
}

/// Calibrate confidence based on recent history
/// 
/// This prevents long-term drift by considering recent performance
pub fn calibrate_from_history(
    current: f32,
    recent_success_rate: f32, // 0.0 to 1.0
    total_steps: usize,
) -> f32 {
    // If we have enough history, blend with actual success rate
    if total_steps >= 5 {
        let empirical = recent_success_rate;
        // Blend: 60% current, 40% empirical
        (current * 0.6 + empirical * 0.4).max(0.1).min(1.0)
    } else {
        // Not enough history yet, use current
        current
    }
}

/// Confidence thresholds for control decisions
pub const CONFIDENCE_LOW: f32 = 0.4;
pub const CONFIDENCE_HIGH: f32 = 0.8;
pub const CONFIDENCE_CRITICAL: f32 = 0.2; // Below this, ask user
