// Deterministic parsers for simple goals (no LLM).

use super::types::{DirectCommand, WriteMode};
use std::result::Result as StdResult;

#[derive(Debug, Clone)]
pub enum ParseError {
    Unrecognized,
    Ambiguous,
    MissingInput,
}

/// Try to parse "write file X with content Y" or "write X with content Y".
fn parse_write_file(goal: &str) -> StdResult<DirectCommand, ParseError> {
    let lower = goal.trim().to_lowercase();
    let g = lower.as_str();

    // "write file <path> with content <content>" or "write <path> with content <content>"
    let with_content = "with content ";
    if let Some(idx) = g.find(with_content) {
        let before = g[..idx].trim();
        let after = g[idx + with_content.len()..].trim();
        let path = if before.starts_with("write file ") {
            before["write file ".len()..].trim()
        } else if before.starts_with("write ") {
            before["write ".len()..].trim()
        } else {
            return Err(ParseError::Unrecognized);
        };
        if path.is_empty() {
            return Err(ParseError::MissingInput);
        }
        if after.is_empty() {
            return Err(ParseError::MissingInput);
        }
        return Ok(DirectCommand::WriteFile {
            path: path.to_string(),
            content: after.to_string(),
            mode: WriteMode::Overwrite,
        });
    }

    Err(ParseError::Unrecognized)
}

/// Try to parse "create (empty )?file X" or "create file X".
fn parse_create_file(goal: &str) -> StdResult<DirectCommand, ParseError> {
    let lower = goal.trim().to_lowercase();
    let g = lower.as_str();

    let path = if g.starts_with("create empty file ") {
        g["create empty file ".len()..].trim()
    } else if g.starts_with("create file ") {
        g["create file ".len()..].trim()
    } else if g.starts_with("create ") && g.len() > 7 {
        let rest = g["create ".len()..].trim();
        if rest.is_empty() {
            return Err(ParseError::MissingInput);
        }
        rest
    } else {
        return Err(ParseError::Unrecognized);
    };

    if path.is_empty() {
        return Err(ParseError::MissingInput);
    }
    Ok(DirectCommand::CreateFile {
        path: path.to_string(),
    })
}

/// Try to parse "read file X" or "read X".
fn parse_read_file(goal: &str) -> StdResult<DirectCommand, ParseError> {
    let lower = goal.trim().to_lowercase();
    let g = lower.as_str();

    let path = if g.starts_with("read file ") {
        g["read file ".len()..].trim()
    } else if g.starts_with("read ") {
        g["read ".len()..].trim()
    } else {
        return Err(ParseError::Unrecognized);
    };

    if path.is_empty() {
        return Err(ParseError::MissingInput);
    }
    Ok(DirectCommand::ReadFile {
        path: path.to_string(),
    })
}

/// Try to parse "list files in X" or "list files X" or "list X".
fn parse_list_files(goal: &str) -> StdResult<DirectCommand, ParseError> {
    let lower = goal.trim().to_lowercase();
    let g = lower.as_str();

    let path = if g.starts_with("list files in ") {
        g["list files in ".len()..].trim()
    } else if g.starts_with("list files ") {
        g["list files ".len()..].trim()
    } else if g.starts_with("list ") {
        g["list ".len()..].trim()
    } else {
        return Err(ParseError::Unrecognized);
    };

    if path.is_empty() {
        return Err(ParseError::MissingInput);
    }
    Ok(DirectCommand::ListFiles {
        path: path.to_string(),
    })
}

/// Fast path: exact or prefix match for common CLI commands (no embedding needed).
pub fn parse_cli_shortcuts(goal: &str) -> Option<DirectCommand> {
    let g = goal.trim();
    if g.is_empty() {
        return None;
    }
    let lower = g.to_lowercase();
    let rest = lower.as_str();
    // npm install / npm run <script>
    if rest == "npm install" || rest.starts_with("npm install ") {
        return Some(DirectCommand::RunCommand {
            cmd: "npm".to_string(),
            args: if rest == "npm install" {
                vec!["install".to_string()]
            } else {
                ["install".to_string()]
                    .into_iter()
                    .chain(rest["npm install ".len()..].split_whitespace().map(String::from))
                    .collect()
            },
        });
    }
    if rest.starts_with("npm run ") {
        let script = rest["npm run ".len()..].trim();
        if !script.is_empty() {
            return Some(DirectCommand::RunCommand {
                cmd: "npm".to_string(),
                args: vec!["run".to_string(), script.to_string()],
            });
        }
    }
    // cargo build / run / test
    if rest == "cargo build" || rest.starts_with("cargo build ") {
        return Some(DirectCommand::RunCommand {
            cmd: "cargo".to_string(),
            args: vec!["build".to_string()],
        });
    }
    if rest == "cargo run" || rest.starts_with("cargo run ") {
        return Some(DirectCommand::RunCommand {
            cmd: "cargo".to_string(),
            args: vec!["run".to_string()],
        });
    }
    if rest == "cargo test" || rest.starts_with("cargo test ") {
        return Some(DirectCommand::RunCommand {
            cmd: "cargo".to_string(),
            args: vec!["test".to_string()],
        });
    }
    None
}

/// Try to parse "run CMD" or "run CMD arg1 arg2" or "execute CMD ...".
fn parse_run_command(goal: &str) -> StdResult<DirectCommand, ParseError> {
    let trimmed = goal.trim();
    let lower = trimmed.to_lowercase();
    let g = lower.as_str();

    let rest = if g.starts_with("run ") {
        trimmed["run ".len()..].trim()
    } else if g.starts_with("execute ") {
        trimmed["execute ".len()..].trim()
    } else {
        return Err(ParseError::Unrecognized);
    };

    if rest.is_empty() {
        return Err(ParseError::MissingInput);
    }

    let parts: Vec<&str> = rest.split_whitespace().collect();
    let cmd = parts.first().map(|s| (*s).to_string()).ok_or(ParseError::MissingInput)?;
    let args: Vec<String> = parts.iter().skip(1).map(|s| (*s).to_string()).collect();

    Ok(DirectCommand::RunCommand { cmd, args })
}

/// Parse goal into a DirectCommand if it matches a known simple pattern.
/// Order: write_file → create_file → read_file → list_files → run_command.
pub fn parse_goal(goal: &str) -> StdResult<DirectCommand, ParseError> {
    let goal = goal.trim();
    if goal.is_empty() {
        return Err(ParseError::MissingInput);
    }

    if let Ok(cmd) = parse_write_file(goal) {
        return Ok(cmd);
    }
    if let Ok(cmd) = parse_create_file(goal) {
        return Ok(cmd);
    }
    if let Ok(cmd) = parse_read_file(goal) {
        return Ok(cmd);
    }
    if let Ok(cmd) = parse_list_files(goal) {
        return Ok(cmd);
    }
    if let Ok(cmd) = parse_run_command(goal) {
        return Ok(cmd);
    }

    Err(ParseError::Unrecognized)
}
