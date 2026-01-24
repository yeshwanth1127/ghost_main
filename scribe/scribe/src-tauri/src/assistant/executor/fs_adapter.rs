use crate::assistant::types::*;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Execute file system actions safely
pub fn execute_fs_action(
    action: &Action,
    ctx: &ExecutionContext,
) -> Result<ActionExecutionResult, String> {
    match action.action_type {
        ActionType::FsCreateFile => create_file(action, ctx),
        ActionType::FsReadFile => read_file(action, ctx),
        ActionType::FsCopyFile => copy_file(action, ctx),
        ActionType::FsMoveFile => move_file(action, ctx),
        ActionType::FsDeleteFile => delete_file(action, ctx),
        ActionType::FsCreateDirectory => create_directory(action, ctx),
    }
}

/// Create a new file
fn create_file(action: &Action, _ctx: &ExecutionContext) -> Result<ActionExecutionResult, String> {
    let path_str = action.args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let path = PathBuf::from(path_str);

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    // Get content (default to empty string)
    let content = action.args
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let encoding = action.args
        .get("encoding")
        .and_then(|v| v.as_str())
        .unwrap_or("utf-8");

    // Create and write file
    let mut file = fs::File::create(&path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    match encoding {
        "utf-8" | "utf8" => {
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
        _ => {
            return Err(format!("Unsupported encoding: {}", encoding));
        }
    }

    Ok(ActionExecutionResult {
        action_id: action.id.clone(),
        success: true,
        output: Some(serde_json::json!({
            "path": path_str,
            "size": content.len(),
        })),
        error: None,
        snapshot_id: None, // Creation doesn't need snapshot
    })
}

/// Read a file
fn read_file(action: &Action, _ctx: &ExecutionContext) -> Result<ActionExecutionResult, String> {
    let path_str = action.args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let path = PathBuf::from(path_str);

    // Check if file exists
    if !path.exists() {
        return Err(format!("File does not exist: {}", path_str));
    }

    // Check if it's a file (not directory)
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path_str));
    }

    // Read file content
    let mut file = fs::File::open(&path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(ActionExecutionResult {
        action_id: action.id.clone(),
        success: true,
        output: Some(serde_json::json!({
            "path": path_str,
            "content": contents,
            "size": contents.len(),
        })),
        error: None,
        snapshot_id: None,
    })
}

/// Copy a file
fn copy_file(action: &Action, _ctx: &ExecutionContext) -> Result<ActionExecutionResult, String> {
    let source_str = action.args
        .get("source_path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'source_path' argument")?;

    let destination_str = action.args
        .get("destination_path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'destination_path' argument")?;

    let source = PathBuf::from(source_str);
    let destination = PathBuf::from(destination_str);

    // Check if source exists
    if !source.exists() {
        return Err(format!("Source file does not exist: {}", source_str));
    }

    if !source.is_file() {
        return Err(format!("Source is not a file: {}", source_str));
    }

    // Create parent directory if needed
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    // Copy file
    fs::copy(&source, &destination)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    Ok(ActionExecutionResult {
        action_id: action.id.clone(),
        success: true,
        output: Some(serde_json::json!({
            "source": source_str,
            "destination": destination_str,
        })),
        error: None,
        snapshot_id: None, // Copy doesn't modify source, so no snapshot needed
    })
}

/// Move a file
fn move_file(action: &Action, _ctx: &ExecutionContext) -> Result<ActionExecutionResult, String> {
    let source_str = action.args
        .get("source_path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'source_path' argument")?;

    let destination_str = action.args
        .get("destination_path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'destination_path' argument")?;

    let source = PathBuf::from(source_str);
    let destination = PathBuf::from(destination_str);

    // Check if source exists
    if !source.exists() {
        return Err(format!("Source file does not exist: {}", source_str));
    }

    // Create parent directory if needed
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    // Move file
    fs::rename(&source, &destination)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    Ok(ActionExecutionResult {
        action_id: action.id.clone(),
        success: true,
        output: Some(serde_json::json!({
            "source": source_str,
            "destination": destination_str,
        })),
        error: None,
        snapshot_id: None, // Snapshot should be created before move
    })
}

/// Delete a file (move to trash)
fn delete_file(action: &Action, _ctx: &ExecutionContext) -> Result<ActionExecutionResult, String> {
    let path_str = action.args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let path = PathBuf::from(path_str);

    // Check if file exists
    if !path.exists() {
        return Err(format!("File does not exist: {}", path_str));
    }

    // Try to move to trash (platform-specific)
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("osascript")
            .arg("-e")
            .arg(format!("tell application \"Finder\" to move POSIX file \"{}\" to trash", path_str))
            .output()
            .map_err(|e| format!("Failed to move to trash: {}", e))?;
        
        if !output.status.success() {
            // Fallback to regular delete if trash fails
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(format!("Remove-Item -Path '{}' -Force -ErrorAction Stop", path_str))
            .output();
        
        if output.is_err() || !output.as_ref().unwrap().status.success() {
            // Fallback to regular delete
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, use gvfs-trash if available, otherwise regular delete
        use std::process::Command;
        let output = Command::new("gvfs-trash")
            .arg(&path_str)
            .output();
        
        if output.is_err() || !output.as_ref().unwrap().status.success() {
            // Fallback to regular delete
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        // Fallback: regular delete
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    Ok(ActionExecutionResult {
        action_id: action.id.clone(),
        success: true,
        output: Some(serde_json::json!({
            "path": path_str,
            "deleted": true,
        })),
        error: None,
        snapshot_id: None, // Snapshot should be created before delete
    })
}

/// Create a directory
fn create_directory(action: &Action, _ctx: &ExecutionContext) -> Result<ActionExecutionResult, String> {
    let path_str = action.args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let path = PathBuf::from(path_str);

    // Create directory (and parents)
    fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    Ok(ActionExecutionResult {
        action_id: action.id.clone(),
        success: true,
        output: Some(serde_json::json!({
            "path": path_str,
        })),
        error: None,
        snapshot_id: None,
    })
}

