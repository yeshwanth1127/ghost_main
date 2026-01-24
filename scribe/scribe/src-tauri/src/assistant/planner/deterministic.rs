use crate::assistant::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Parse user intent into an action plan using rule-based matching
/// Returns ActionPlan with placeholder paths if path is missing (frontend will prompt)
pub fn parse_intent(
    user_input: &str,
    user_home: &Path,
) -> Result<ActionPlan, String> {
    let input_lower = user_input.to_lowercase().trim().to_string();
    
    // Generate unique IDs
    let plan_id = Uuid::new_v4().to_string();
    let request_id = Uuid::new_v4().to_string();

    // Try to match common patterns
    if let Some(action) = parse_create_file(&input_lower, user_home) {
        return Ok(ActionPlan {
            schema: ActionSchema {
                id: plan_id,
                origin: ActionOrigin {
                    user_input: user_input.to_string(),
                    source: ActionSource::Ui,
                    request_id,
                },
                actions: vec![action],
                summary: format!("Create file: {}", extract_file_path(&input_lower)),
                risk_score: RiskScore::Low.value(),
                dry_run: true,
            },
        });
    }

    if let Some(action) = parse_read_file(&input_lower, user_home) {
        return Ok(ActionPlan {
            schema: ActionSchema {
                id: plan_id,
                origin: ActionOrigin {
                    user_input: user_input.to_string(),
                    source: ActionSource::Ui,
                    request_id,
                },
                actions: vec![action],
                summary: format!("Read file: {}", extract_file_path(&input_lower)),
                risk_score: RiskScore::Low.value(),
                dry_run: true,
            },
        });
    }

    if let Some((src_action, dst_action)) = parse_copy_file(&input_lower, user_home) {
        return Ok(ActionPlan {
            schema: ActionSchema {
                id: plan_id,
                origin: ActionOrigin {
                    user_input: user_input.to_string(),
                    source: ActionSource::Ui,
                    request_id,
                },
                actions: vec![src_action, dst_action],
                summary: format!("Copy file"),
                risk_score: RiskScore::Medium.value(),
                dry_run: true,
            },
        });
    }

    if let Some(action) = parse_move_file(&input_lower, user_home) {
        return Ok(ActionPlan {
            schema: ActionSchema {
                id: plan_id,
                origin: ActionOrigin {
                    user_input: user_input.to_string(),
                    source: ActionSource::Ui,
                    request_id,
                },
                actions: vec![action],
                summary: format!("Move file"),
                risk_score: RiskScore::Medium.value(),
                dry_run: true,
            },
        });
    }

    if let Some(action) = parse_delete_file(&input_lower, user_home) {
        return Ok(ActionPlan {
            schema: ActionSchema {
                id: plan_id,
                origin: ActionOrigin {
                    user_input: user_input.to_string(),
                    source: ActionSource::Ui,
                    request_id,
                },
                actions: vec![action],
                summary: format!("Delete file: {}", extract_file_path(&input_lower)),
                risk_score: RiskScore::High.value(),
                dry_run: true,
            },
        });
    }

    if let Some(action) = parse_create_directory(&input_lower, user_home) {
        return Ok(ActionPlan {
            schema: ActionSchema {
                id: plan_id,
                origin: ActionOrigin {
                    user_input: user_input.to_string(),
                    source: ActionSource::Ui,
                    request_id,
                },
                actions: vec![action],
                summary: format!("Create directory: {}", extract_file_path(&input_lower)),
                risk_score: RiskScore::Low.value(),
                dry_run: true,
            },
        });
    }

    Err("Could not parse intent. Please use LLM planner for complex requests.".to_string())
}

/// Parse "create file X" or "create file X with content Y"
/// Returns action with "__PROMPT_PATH__" placeholder if path is missing
fn parse_create_file(input: &str, user_home: &Path) -> Option<Action> {
    // Check for missing path patterns
    if regex::Regex::new(r"create\s+(?:a\s+)?file\s*$|new\s+file\s*$")
        .ok()
        .and_then(|re| if re.is_match(input) { Some(()) } else { None })
        .is_some()
    {
        // Path missing - return action with placeholder
        let mut args = HashMap::new();
        args.insert("path".to_string(), serde_json::Value::String("__PROMPT_PATH__".to_string()));
        args.insert("content".to_string(), serde_json::Value::String("".to_string()));
        args.insert("encoding".to_string(), serde_json::Value::String("utf-8".to_string()));
        
        return Some(Action {
            id: Uuid::new_v4().to_string(),
            action_type: ActionType::FsCreateFile,
            args,
            preconditions: Some(Precondition {
                writable: Some(true),
                exists: Some(false),
                readable: None,
                directory: None,
            }),
            metadata: Some(ActionMetadata {
                confidence: Some(0.7), // Lower confidence when path missing
            }),
        });
    }

    let patterns = vec![
        r"create\s+file\s+(.+?)(?:\s+with\s+content\s+(.+))?$",
        r"create\s+a\s+file\s+(.+?)(?:\s+with\s+content\s+(.+))?$",
        r"new\s+file\s+(.+?)(?:\s+with\s+content\s+(.+))?$",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(input))
        {
            let file_path = captures.get(1)?.as_str().trim();
            if file_path.is_empty() {
                continue; // Skip empty captures
            }
            let content = captures.get(2).map(|m| m.as_str().trim().to_string());
            
            let full_path = match resolve_path(file_path, user_home) {
                Some(p) => p,
                None => continue, // Try next pattern
            };
            
            let mut args = HashMap::new();
            args.insert("path".to_string(), serde_json::Value::String(full_path.to_string_lossy().to_string()));
            if let Some(content) = content {
                args.insert("content".to_string(), serde_json::Value::String(content));
            } else {
                args.insert("content".to_string(), serde_json::Value::String("".to_string()));
            }
            args.insert("encoding".to_string(), serde_json::Value::String("utf-8".to_string()));

            return Some(Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsCreateFile,
                args,
                preconditions: Some(Precondition {
                    writable: Some(true),
                    exists: Some(false),
                    readable: None,
                    directory: None,
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.9),
                }),
            });
        }
    }

    None
}

/// Parse "read file X" or "open file X" or "show file X"
/// Returns action with "__PROMPT_PATH__" placeholder if path is missing
fn parse_read_file(input: &str, user_home: &Path) -> Option<Action> {
    // Check for missing path patterns
    if regex::Regex::new(r"(?:read|open|show|view|display)\s+file\s*$")
        .ok()
        .and_then(|re| if re.is_match(input) { Some(()) } else { None })
        .is_some()
    {
        let mut args = HashMap::new();
        args.insert("path".to_string(), serde_json::Value::String("__PROMPT_PATH__".to_string()));
        
        return Some(Action {
            id: Uuid::new_v4().to_string(),
            action_type: ActionType::FsReadFile,
            args,
            preconditions: Some(Precondition {
                readable: Some(true),
                exists: Some(true),
                writable: None,
                directory: None,
            }),
            metadata: Some(ActionMetadata {
                confidence: Some(0.7),
            }),
        });
    }

    let patterns = vec![
        r"(?:read|open|show|view|display)\s+file\s+(.+?)$",
        r"(?:read|open|show|view|display)\s+(.+?)$",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(input))
        {
            let file_path = captures.get(1)?.as_str().trim();
            if file_path.is_empty() {
                continue;
            }
            let full_path = match resolve_path(file_path, user_home) {
                Some(p) => p,
                None => continue,
            };

            let mut args = HashMap::new();
            args.insert("path".to_string(), serde_json::Value::String(full_path.to_string_lossy().to_string()));

            return Some(Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsReadFile,
                args,
                preconditions: Some(Precondition {
                    readable: Some(true),
                    exists: Some(true),
                    writable: None,
                    directory: None,
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.9),
                }),
            });
        }
    }

    None
}

/// Parse "copy X to Y" or "copy file X to Y"
fn parse_copy_file(input: &str, user_home: &Path) -> Option<(Action, Action)> {
    let patterns = vec![
        r"copy\s+(?:file\s+)?(.+?)\s+to\s+(.+?)$",
        r"copy\s+(.+?)\s+(?:to\s+)?(.+?)$",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(input))
        {
            let src_path = captures.get(1)?.as_str().trim();
            let dst_path = captures.get(2)?.as_str().trim();
            
            let full_src_path = resolve_path(src_path, user_home)?;
            let full_dst_path = resolve_path(dst_path, user_home)?;

            let mut src_args = HashMap::new();
            src_args.insert("path".to_string(), serde_json::Value::String(full_src_path.to_string_lossy().to_string()));

            let mut dst_args = HashMap::new();
            dst_args.insert("source_path".to_string(), serde_json::Value::String(full_src_path.to_string_lossy().to_string()));
            dst_args.insert("destination_path".to_string(), serde_json::Value::String(full_dst_path.to_string_lossy().to_string()));

            let read_action = Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsReadFile,
                args: src_args,
                preconditions: Some(Precondition {
                    readable: Some(true),
                    exists: Some(true),
                    writable: None,
                    directory: None,
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.85),
                }),
            };

            let copy_action = Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsCopyFile,
                args: dst_args,
                preconditions: Some(Precondition {
                    writable: Some(true),
                    exists: Some(false), // Destination shouldn't exist
                    readable: None,
                    directory: None,
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.85),
                }),
            };

            return Some((read_action, copy_action));
        }
    }

    None
}

/// Parse "move X to Y" or "rename X to Y"
fn parse_move_file(input: &str, user_home: &Path) -> Option<Action> {
    let patterns = vec![
        r"(?:move|rename)\s+(?:file\s+)?(.+?)\s+to\s+(.+?)$",
        r"rename\s+(.+?)\s+(?:to\s+)?(.+?)$",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(input))
        {
            let src_path = captures.get(1)?.as_str().trim();
            let dst_path = captures.get(2)?.as_str().trim();
            
            let full_src_path = resolve_path(src_path, user_home)?;
            let full_dst_path = resolve_path(dst_path, user_home)?;

            let mut args = HashMap::new();
            args.insert("source_path".to_string(), serde_json::Value::String(full_src_path.to_string_lossy().to_string()));
            args.insert("destination_path".to_string(), serde_json::Value::String(full_dst_path.to_string_lossy().to_string()));

            return Some(Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsMoveFile,
                args,
                preconditions: Some(Precondition {
                    writable: Some(true),
                    exists: Some(true), // Source must exist
                    readable: None,
                    directory: None,
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.85),
                }),
            });
        }
    }

    None
}

/// Parse "delete X" or "remove X"
/// Returns action with "__PROMPT_PATH__" placeholder if path is missing
fn parse_delete_file(input: &str, user_home: &Path) -> Option<Action> {
    // Check for missing path
    if regex::Regex::new(r"(?:delete|remove|rm)\s+(?:file\s+)?$")
        .ok()
        .and_then(|re| if re.is_match(input) { Some(()) } else { None })
        .is_some()
    {
        let mut args = HashMap::new();
        args.insert("path".to_string(), serde_json::Value::String("__PROMPT_PATH__".to_string()));
        
        return Some(Action {
            id: Uuid::new_v4().to_string(),
            action_type: ActionType::FsDeleteFile,
            args,
            preconditions: Some(Precondition {
                exists: Some(true),
                writable: None,
                readable: None,
                directory: None,
            }),
            metadata: Some(ActionMetadata {
                confidence: Some(0.7),
            }),
        });
    }

    let patterns = vec![
        r"(?:delete|remove|rm)\s+(?:file\s+)?(.+?)$",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(input))
        {
            let file_path = captures.get(1)?.as_str().trim();
            if file_path.is_empty() {
                continue;
            }
            let full_path = match resolve_path(file_path, user_home) {
                Some(p) => p,
                None => continue,
            };

            let mut args = HashMap::new();
            args.insert("path".to_string(), serde_json::Value::String(full_path.to_string_lossy().to_string()));

            return Some(Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsDeleteFile,
                args,
                preconditions: Some(Precondition {
                    exists: Some(true),
                    writable: None,
                    readable: None,
                    directory: None,
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.9),
                }),
            });
        }
    }

    None
}

/// Parse "create directory X" or "mkdir X"
fn parse_create_directory(input: &str, user_home: &Path) -> Option<Action> {
    let patterns = vec![
        r"create\s+(?:directory|dir|folder)\s+(.+?)$",
        r"mkdir\s+(.+?)$",
        r"new\s+(?:directory|dir|folder)\s+(.+?)$",
    ];

    for pattern in patterns {
        if let Some(captures) = regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(input))
        {
            let dir_path = captures.get(1)?.as_str().trim();
            let full_path = resolve_path(dir_path, user_home)?;

            let mut args = HashMap::new();
            args.insert("path".to_string(), serde_json::Value::String(full_path.to_string_lossy().to_string()));

            return Some(Action {
                id: Uuid::new_v4().to_string(),
                action_type: ActionType::FsCreateDirectory,
                args,
                preconditions: Some(Precondition {
                    writable: Some(true),
                    exists: Some(false),
                    readable: None,
                    directory: Some(false), // Parent should be a directory
                }),
                metadata: Some(ActionMetadata {
                    confidence: Some(0.9),
                }),
            });
        }
    }

    None
}

/// Extract file path from user input (heuristic)
fn extract_file_path(input: &str) -> String {
    // Try to find quoted strings first
    if let Some(captures) = regex::Regex::new(r#""([^"]+)""#)
        .ok()
        .and_then(|re| re.captures(input))
    {
        return captures.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
    }
    
    // Otherwise, try to extract last path-like segment
    if let Some(captures) = regex::Regex::new(r"(\S+(?:/[^/\s]+)+)")
        .ok()
        .and_then(|re| re.captures(input))
    {
        return captures.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
    }

    input.to_string()
}

/// Resolve relative path to absolute path (anywhere on system)
fn resolve_path(path: &str, _user_home: &Path) -> Option<PathBuf> {
    let path = Path::new(path);
    
    if path.is_absolute() {
        // Absolute path - use as is
        Some(path.to_path_buf())
    } else {
        // Relative path - resolve from current working directory
        let cwd = std::env::current_dir().ok()?;
        Some(cwd.join(path))
    }
}

