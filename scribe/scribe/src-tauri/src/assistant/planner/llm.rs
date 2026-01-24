use crate::assistant::types::*;

/// LLM planner placeholder
/// The actual LLM planning happens in frontend using existing AI provider system
/// This module would validate and parse LLM responses if needed

pub fn parse_llm_response(_response: &str) -> Result<ActionPlan, String> {
    // Placeholder: LLM planning is done in frontend
    Err("LLM planning should be done in frontend".to_string())
}

