// Validation Layer - Never Trust LLM Output
//
// All LLM output must be validated before use.
// Reject invalid decisions and re-ask (max retries = 2).

use super::decision::PlannerDecision;
use crate::agent::capabilities::registry::CapabilityRegistry;
use serde_json::Value;

/// Validate a planner decision
/// Returns error if decision is invalid
pub async fn validate_decision(
    decision: &PlannerDecision,
    available_capabilities: &[String],
    registry: &CapabilityRegistry,
) -> Result<(), String> {
    match decision {
        PlannerDecision::InvokeCapability {
            capability,
            confidence,
            inputs,
            ..
        } => {
            // Validate capability exists
            if !available_capabilities.contains(capability) {
                return Err(format!(
                    "Unknown capability: {}. Available: {:?}",
                    capability, available_capabilities
                ));
            }

            // Validate confidence range
            if !(0.0..=1.0).contains(confidence) {
                return Err(format!("Invalid confidence: {}. Must be 0.0-1.0", confidence));
            }

            // intent/expected_outcome are optional; defaults applied when converting to Decision

            // Validate inputs against capability schema
            let empty = Value::Object(serde_json::Map::new());
            let inputs_val = inputs.as_ref().unwrap_or(&empty);
            if let Some(descriptor) = registry.get_descriptor(capability).await {
                if let Err(schema_err) = validate_inputs_against_schema(inputs_val, &descriptor.input_schema) {
                    return Err(format!(
                        "Invalid capability inputs for {}: {}\nExpected schema: {}",
                        capability,
                        schema_err,
                        serde_json::to_string_pretty(&descriptor.input_schema).unwrap_or_default()
                    ));
                }
            }
        }
        PlannerDecision::RevisePlan { summary, steps, .. } => {
            if summary.is_empty() {
                return Err("Plan summary cannot be empty".to_string());
            }
            if steps.is_empty() {
                return Err("Plan steps cannot be empty".to_string());
            }
        }
        PlannerDecision::AskUser { .. } => {
            // question/reason are optional; defaults applied when converting to Decision
        }
        PlannerDecision::Finish { .. } => {
            // Finish is always valid
        }
    }

    Ok(())
}

/// Validate inputs against capability schema
/// Returns error message if validation fails
fn validate_inputs_against_schema(inputs: &Value, schema: &Value) -> Result<(), String> {
    // Get required fields from schema
    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Check all required fields are present and non-empty
    let input_obj = inputs.as_object().ok_or("Inputs must be an object")?;
    
    for field in &required {
        if let Some(value) = input_obj.get(field) {
            // Check if value is empty (empty string, null, empty array, empty object)
            let is_empty = match value {
                Value::String(s) => s.trim().is_empty(),
                Value::Null => true,
                Value::Array(arr) => arr.is_empty(),
                Value::Object(obj) => obj.is_empty(),
                _ => false, // Numbers, booleans are never "empty"
            };
            
            if is_empty {
                return Err(format!("Required field '{}' is empty", field));
            }
        } else {
            return Err(format!("Required field '{}' is missing", field));
        }
    }

    // Check for unknown fields (warn but don't fail - might be extra metadata)
    if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
        for (key, _) in input_obj {
            if !properties.contains_key(key) && !required.contains(key) {
                eprintln!("[VALIDATION] Warning: Unknown input field '{}' (not in schema)", key);
            }
        }
    }

    Ok(())
}
