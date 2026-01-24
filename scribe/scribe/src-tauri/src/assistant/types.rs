use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Action Schema v2 - Canonical action representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSchema {
    pub id: String,
    pub origin: ActionOrigin,
    pub actions: Vec<Action>,
    pub summary: String,
    pub risk_score: f64,
    pub dry_run: bool,
}

/// Origin/provenance of an action request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOrigin {
    pub user_input: String,
    pub source: ActionSource,
    pub request_id: String,
}

/// Source of the action request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionSource {
    #[serde(rename = "ui")]
    Ui,
    #[serde(rename = "voice")]
    Voice,
    #[serde(rename = "automation")]
    Automation,
    #[serde(rename = "plugin")]
    Plugin,
}

/// Single action in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: ActionType,
    pub args: HashMap<String, serde_json::Value>,
    pub preconditions: Option<Precondition>,
    pub metadata: Option<ActionMetadata>,
}

/// Types of actions supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    FsCreateFile,
    FsReadFile,
    FsCopyFile,
    FsMoveFile,
    FsDeleteFile,
    FsCreateDirectory,
}

/// Preconditions for action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precondition {
    pub writable: Option<bool>,
    pub readable: Option<bool>,
    pub exists: Option<bool>,
    pub directory: Option<bool>,
}

/// Action metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    pub confidence: Option<f64>,
}

/// Complete action plan (before verification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    #[serde(flatten)]
    pub schema: ActionSchema,
}

/// Verified action plan (after verification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedPlan {
    pub plan: ActionPlan,
    pub verified_at: i64,
    pub verification_notes: Vec<String>,
}

/// Result of executing an action plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub success: bool,
    pub executed_at: i64,
    pub results: Vec<ActionExecutionResult>,
    pub error: Option<String>,
    pub undo_available: bool,
    pub undo_ttl: Option<i64>,
}

/// Result of executing a single action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecutionResult {
    pub action_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub snapshot_id: Option<String>,
}

/// Preview result shown to user before execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResult {
    pub plan: ActionPlan,
    pub risk_score: f64,
    pub affected_items: Vec<AffectedItem>,
    pub warnings: Vec<String>,
    pub requires_explicit_confirmation: bool,
    pub missing_paths: Vec<String>, // Action IDs that need path input
}

/// Item affected by an action (for preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedItem {
    pub path: String,
    pub operation: String,
    pub preview: Option<String>, // Visual diff or preview content
}

/// Risk score calculation
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum RiskScore {
    Low = 0,    // 0.0-0.3: Safe operations (read, create in user dir)
    Medium = 1, // 0.3-0.7: Moderate risk (modify, copy)
    High = 2,   // 0.7-0.9: High risk (delete, move outside user dir)
    Critical = 3, // 0.9-1.0: Critical risk (system files, execute)
}

impl RiskScore {
    pub fn value(&self) -> f64 {
        match self {
            RiskScore::Low => 0.15,
            RiskScore::Medium => 0.5,
            RiskScore::High => 0.8,
            RiskScore::Critical => 0.95,
        }
    }

    pub fn from_value(value: f64) -> Self {
        if value < 0.3 {
            RiskScore::Low
        } else if value < 0.7 {
            RiskScore::Medium
        } else if value < 0.9 {
            RiskScore::High
        } else {
            RiskScore::Critical
        }
    }
}

/// Capability token for scoped permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub nonce: String,
    pub scopes: Vec<String>,
    pub ttl_seconds: i64,
    pub session_id: String,
    pub expires_at: i64,
    pub issued_at: i64,
}

/// Scope for capability token (e.g., "fs:create:/home/user/Documents/*")
#[derive(Debug, Clone)]
pub struct Scope {
    pub action_type: String,
    pub operation: String,
    pub resource_pattern: String,
}

impl Scope {
    pub fn parse(scope_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = scope_str.split(':').collect();
        if parts.len() < 3 {
            return Err("Invalid scope format. Expected: action_type:operation:resource_pattern".to_string());
        }
        Ok(Scope {
            action_type: parts[0].to_string(),
            operation: parts[1].to_string(),
            resource_pattern: parts[2..].join(":"),
        })
    }

    pub fn matches(&self, action_type: &str, resource: &str) -> bool {
        if self.action_type != action_type {
            return false;
        }
        // Simple glob matching (basic implementation)
        self.resource_pattern
            .replace("*", ".*")
            .as_str()
            .lines()
            .next()
            .map(|pattern| {
                regex::Regex::new(&format!("^{}$", pattern))
                    .map(|re| re.is_match(resource))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub entry_json: String,
    pub timestamp: i64,
    pub prev_hash: String,
    pub signature: String,
    pub action_id: Option<String>,
}

/// Action snapshot for undo functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSnapshot {
    pub id: String,
    pub action_id: String,
    pub original_path: String,
    pub snapshot_path: String,
    pub created_at: i64,
    pub retention_until: i64,
}

/// Execution context for actions
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub capability_token: Option<CapabilityToken>,
    pub user_home: std::path::PathBuf,
    pub allowed_paths: Vec<std::path::PathBuf>,
}

impl ExecutionContext {
    pub fn new(user_home: std::path::PathBuf) -> Self {
        Self {
            capability_token: None,
            user_home: user_home.clone(),
            allowed_paths: vec![user_home],
        }
    }

    pub fn with_token(mut self, token: CapabilityToken) -> Self {
        self.capability_token = Some(token);
        self
    }
}

