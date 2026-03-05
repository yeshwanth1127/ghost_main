//! Tool intent canonicalization - converts raw LLM intent to structured ToolIntent (Moltbot-style).
//! Used for permissions, UI, and auditing.

use crate::agent::capabilities::RiskLevel;
use crate::agent::execution_ticket::ToolIntent;
use serde_json::Value;

/// Build canonical ToolIntent from capability, inputs, user goal, and optional raw intent.
pub fn canonicalize_tool_intent(
    capability: &str,
    inputs: &Value,
    user_goal: Option<&str>,
    raw_intent: Option<&str>,
    _risk_level: RiskLevel,
) -> ToolIntent {
    let risk_factors = detect_risk_factors(capability, inputs);
    let irreversible = detect_irreversible(capability, inputs);
    let goal_alignment = extract_goal_alignment(capability, inputs, user_goal, raw_intent);
    let human_readable = generate_human_readable(capability, inputs, &goal_alignment);

    let context = user_goal.map(|g| crate::agent::execution_ticket::ToolIntentContext {
        user_goal: Some(g.to_string()),
        previous_actions: None,
    });

    ToolIntent {
        human_readable,
        goal_alignment,
        irreversible,
        risk_factors,
        context,
    }
}

fn detect_risk_factors(capability: &str, inputs: &Value) -> Vec<String> {
    let mut factors = Vec::new();

    // Filesystem write/edit: system paths, home directory
    if capability.contains("write") || capability.contains("edit") {
        if let Some(path) = inputs.get("path").and_then(|v| v.as_str()) {
            if path.starts_with("/etc/")
                || path.starts_with("/usr/")
                || path.contains("C:\\Windows\\")
                || path.contains("C:\\Program Files\\")
            {
                factors.push("system_path".to_string());
            }
            if path.contains('~') || path.contains("$HOME") {
                factors.push("home_directory".to_string());
            }
        }
    }

    // Process execution
    if capability.contains("process") || capability.contains("spawn") {
        factors.push("process_execution".to_string());
        let command = inputs
            .get("command")
            .or_else(|| inputs.get("program"))
            .and_then(|v| v.as_str());
        if let Some(cmd) = command {
            let lower = cmd.to_lowercase();
            if lower.contains("rm ")
                || lower.contains("del ")
                || lower.contains("format ")
                || lower.contains("delete")
            {
                factors.push("destructive_command".to_string());
            }
        }
    }

    factors
}

fn detect_irreversible(capability: &str, inputs: &Value) -> bool {
    if capability.contains("write") || capability.contains("edit") {
        return true;
    }
    if capability.contains("process") || capability.contains("spawn") {
        let command = inputs
            .get("command")
            .or_else(|| inputs.get("program"))
            .and_then(|v| v.as_str());
        if let Some(cmd) = command {
            let lower = cmd.to_lowercase();
            if lower.contains("rm ")
                || lower.contains("del ")
                || lower.contains("format ")
                || lower.contains("delete")
                || lower.contains("uninstall")
            {
                return true;
            }
        }
    }
    false
}

fn extract_goal_alignment(
    _capability: &str,
    _inputs: &Value,
    user_goal: Option<&str>,
    raw_intent: Option<&str>,
) -> String {
    if let Some(intent) = raw_intent {
        let lower = intent.to_lowercase();
        if lower.contains("save") || lower.contains("store") {
            return "save data".to_string();
        }
        if lower.contains("create") || lower.contains("make") {
            return "create file".to_string();
        }
        if lower.contains("modify") || lower.contains("update") || lower.contains("change") {
            return "modify file".to_string();
        }
        if lower.contains("read") || lower.contains("check") || lower.contains("view") {
            return "read information".to_string();
        }
        if lower.contains("run") || lower.contains("execute") || lower.contains("command") {
            return "execute command".to_string();
        }
    }
    if let Some(goal) = user_goal {
        return format!("achieve: {}", goal);
    }
    "complete requested task".to_string()
}

fn generate_human_readable(capability: &str, inputs: &Value, goal_alignment: &str) -> String {
    let path = inputs
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("specified path");

    if capability.contains("write") {
        return format!("Create file at {} to {}", path, goal_alignment);
    }
    if capability.contains("edit") {
        return format!("Edit file at {} to {}", path, goal_alignment);
    }
    if capability.contains("read") {
        return format!("Read file at {} to {}", path, goal_alignment);
    }
    if capability.contains("process") || capability.contains("spawn") {
        return format!("Run command to {}", goal_alignment);
    }
    format!("Execute {} to {}", capability, goal_alignment)
}
