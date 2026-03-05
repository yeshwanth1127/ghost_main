// Planner Prompt - System and User Prompts
//
// The system prompt is stable (rules of the game).
// The user prompt is dynamic (current cognitive state).

use super::context::PlannerContext;

/// System prompt (stable)
/// This defines the rules for the LLM
pub const SYSTEM_PROMPT: &str = r#"
You are a planner.

Your job:
- Look at the GOAL and RECENT MESSAGES. If the user already replied with path, content, filename, etc., use those values in invoke_capability — do not ask again.
- When path is missing for a file operation (filesystem.write, filesystem.read): use invoke_capability with path empty (e.g. "path": ""). The system will request input; on desktop a file/folder picker will open — do not use ask_user for path.
- For other missing inputs (e.g. content), use ask_user only if the user has not provided them. When you have the required inputs, use invoke_capability. The system will show Allow/Deny; user clicks Allow.

Rules:
- Do not execute actions. Do not explain. Output VALID JSON only.
- Do not ask for the same information twice. If RECENT MESSAGES contains a user message, use it to fill inputs.
- For missing path: invoke_capability with path "" so the system opens the file picker on desktop.

Allowed actions:
1) invoke_capability
2) ask_user - only when a required input is missing (e.g. path, content). Ask for the value. Include "question" and "reason".
3) revise_plan
4) finish

When invoking a capability:
- Use ONLY keys from its input schema. Provide ALL required fields.
- If the user replied with path, content, filename, etc., extract those from the user's message and put them in "inputs" (e.g. path, content for filesystem.write). Do not ask again.

Return ONE JSON object.
"#;

/// Build user prompt from planner context
/// This is the dynamic part - current cognitive state
pub fn build_user_prompt(ctx: &PlannerContext) -> String {
    let plan_str = if let Some(ref plan) = ctx.current_plan {
        format!(
            "Summary: {}\nSteps: {}\nConfidence: {:.2}",
            plan.summary,
            plan.steps.join(", "),
            plan.confidence
        )
    } else {
        "No plan yet".to_string()
    };

    let beliefs_str = format!(
        "Plan Confidence: {:.2}\nKnown Failures: {}\nKnown Constraints: {}",
        ctx.belief_state.plan_confidence,
        if ctx.belief_state.known_failures.is_empty() {
            "None".to_string()
        } else {
            ctx.belief_state.known_failures.join(", ")
        },
        if ctx.belief_state.known_constraints.is_empty() {
            "None".to_string()
        } else {
            ctx.belief_state.known_constraints.join(", ")
        }
    );

    let last_step_str = if let Some(ref step) = ctx.last_step {
        format!(
            "Capability: {}\nSuccess: {}\nReason: {}",
            step.capability, step.success, step.reason
        )
    } else {
        "No steps executed yet".to_string()
    };

    let lessons_str = if ctx.lessons.is_empty() {
        "None".to_string()
    } else {
        ctx.lessons.join("\n")
    };

    let messages_str = if ctx.recent_messages.is_empty() {
        "None".to_string()
    } else {
        ctx.recent_messages
            .iter()
            .map(|(role, content)| format!("{}: {}", role, content))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let caps_str = ctx
        .capabilities
        .iter()
        .map(|c| {
            let schema_str = if let Some(ref schema) = c.input_schema {
                // Format schema for readability
                if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                    let required = schema.get("required")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                        .unwrap_or_default();
                    
                    let props: Vec<String> = properties
                        .iter()
                        .map(|(key, val)| {
                            let prop_type = val.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let is_required = required.iter().any(|r| r == key);
                            let req_marker = if is_required { " (REQUIRED)" } else { " (optional)" };
                            format!("  - {}: {} - {}", key, prop_type, req_marker)
                        })
                        .collect();
                    
                    format!(
                        "- {} ({}) - {}\n  Input Schema:\n{}\n  REQUIRED fields: {}",
                        c.name,
                        c.risk,
                        c.description,
                        props.join("\n"),
                        if required.is_empty() {
                            "None".to_string()
                        } else {
                            required.join(", ")
                        }
                    )
                } else {
                    format!("- {} ({}) - {}\n  Input Schema: {}", c.name, c.risk, c.description, schema)
                }
            } else {
                format!("- {} ({}) - {}", c.name, c.risk, c.description)
            };
            schema_str
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let intent_hint_line = ctx
        .routed_intent_hint
        .as_ref()
        .map(|h| format!("\nROUTED INTENT HINT: {} (use as a hint; user goal is authoritative)\n", h))
        .unwrap_or_default();

    format!(
        r#"
GOAL:
{goal}
{intent_hint}
RUN STATUS:
{status}

CURRENT PLAN:
{plan}

BELIEFS:
{beliefs}

LAST STEP:
{last_step}

RECENT MESSAGES (assistant asked, user replied — use the user's reply as the provided details; do not ask again):
{messages}

LESSONS LEARNED:
{lessons}

AVAILABLE CAPABILITIES:
{caps}

Decide the NEXT action. Output ONLY valid JSON.
"#,
        goal = ctx.goal,
        intent_hint = intent_hint_line,
        status = ctx.run_status,
        plan = plan_str,
        beliefs = beliefs_str,
        last_step = last_step_str,
        messages = messages_str,
        lessons = lessons_str,
        caps = caps_str,
    )
}
