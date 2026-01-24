use crate::assistant::types::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uuid::Uuid;

/// Snapshot manager for undo functionality
pub struct SnapshotManager {
    temp_dir: TempDir,
    user_home: PathBuf,
}

impl SnapshotManager {
    pub fn new(user_home: PathBuf) -> Result<Self, String> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        Ok(Self {
            temp_dir,
            user_home,
        })
    }

    /// Create a snapshot of a file before modification
    pub fn create_snapshot(
        &self,
        action_id: &str,
        original_path: &Path,
    ) -> Result<ActionSnapshot, String> {
        if !original_path.exists() {
            return Err("File does not exist, cannot create snapshot".to_string());
        }

        if !original_path.is_file() {
            return Err("Path is not a file, cannot snapshot".to_string());
        }

        // Generate snapshot ID
        let snapshot_id = Uuid::new_v4().to_string();
        
        // Create snapshot path
        let snapshot_path = self.temp_dir.path().join(&snapshot_id);

        // Copy file to snapshot location
        fs::copy(original_path, &snapshot_path)
            .map_err(|e| format!("Failed to copy file to snapshot: {}", e))?;

        // Calculate retention (7 days from now)
        let created_at = chrono::Utc::now().timestamp();
        let retention_until = created_at + (7 * 24 * 60 * 60); // 7 days in seconds

        Ok(ActionSnapshot {
            id: snapshot_id,
            action_id: action_id.to_string(),
            original_path: original_path.to_string_lossy().to_string(),
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
            created_at,
            retention_until,
        })
    }

    /// Restore a file from snapshot
    pub fn restore_from_snapshot(&self, snapshot: &ActionSnapshot) -> Result<(), String> {
        let snapshot_path = Path::new(&snapshot.snapshot_path);
        let original_path = Path::new(&snapshot.original_path);

        if !snapshot_path.exists() {
            return Err("Snapshot file does not exist".to_string());
        }

        // Create parent directory if needed
        if let Some(parent) = original_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }

        // Copy snapshot back to original location
        fs::copy(snapshot_path, original_path)
            .map_err(|e| format!("Failed to restore from snapshot: {}", e))?;

        Ok(())
    }

    /// Delete a snapshot (cleanup)
    pub fn delete_snapshot(&self, snapshot: &ActionSnapshot) -> Result<(), String> {
        let snapshot_path = Path::new(&snapshot.snapshot_path);
        
        if snapshot_path.exists() {
            fs::remove_file(snapshot_path)
                .map_err(|e| format!("Failed to delete snapshot: {}", e))?;
        }

        Ok(())
    }

    /// Clean up expired snapshots
    pub fn cleanup_expired(&self, snapshots: Vec<ActionSnapshot>) -> Result<usize, String> {
        let now = chrono::Utc::now().timestamp();
        let mut cleaned = 0;

        for snapshot in snapshots {
            if snapshot.retention_until < now {
                if let Err(e) = self.delete_snapshot(&snapshot) {
                    eprintln!("Failed to delete expired snapshot {}: {}", snapshot.id, e);
                } else {
                    cleaned += 1;
                }
            }
        }

        Ok(cleaned)
    }
}
