// Permission model

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub scope: PermissionScope,
    pub reason: String,
    pub risk_score: f32,
    pub scope_type: PermissionScopeType, // CRITICAL: Explicit scope type
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionScopeType {
    Once,      // Allow for this single operation only
    Run,       // Allow for the entire run
    // Future: Global, TimeLimited, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionScope {
    FileRead { path: String },
    FileWrite { path: String },
    DirectoryRead { path: String },
    DirectoryWrite { path: String },
}

impl PermissionScope {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            PermissionScope::FileRead { path } => {
                json!({
                    "type": "FileRead",
                    "path": path
                })
            }
            PermissionScope::FileWrite { path } => {
                json!({
                    "type": "FileWrite",
                    "path": path
                })
            }
            PermissionScope::DirectoryRead { path } => {
                json!({
                    "type": "DirectoryRead",
                    "path": path
                })
            }
            PermissionScope::DirectoryWrite { path } => {
                json!({
                    "type": "DirectoryWrite",
                    "path": path
                })
            }
        }
    }
}

use serde_json::json;
