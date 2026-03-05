// Capability system - domain-agnostic action execution
// 
// This module provides a unified interface for all agent actions.
// The agent never knows about "files", "processes", or "networks" directly.
// It only knows about capabilities with contracts.

pub mod filesystem;
pub mod process;
pub mod code;
pub mod project;
pub mod http;
pub mod docker;
pub mod repo;
pub mod env;
pub mod registry;

use crate::agent::state::RunState;
use async_trait::async_trait;
use serde_json::Value;
use tauri::AppHandle;

/// Result of preflight check: validate inputs and declare permission/input needs (no side effects).
#[derive(Debug, Clone)]
pub enum PreflightResult {
    Ok,
    NeedsPermission(PermissionRequest),
    NeedsInput(InputRequest),
    Reject(String),
}

/// Request for user permission (mirrors permission flow in run_loop).
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub reason: String,
}

/// Request for missing user input (mirrors input flow in run_loop).
#[derive(Debug, Clone)]
pub struct InputRequest {
    pub missing_fields: Vec<String>,
    pub schema: Value,
    pub current_inputs: Value,
}

/// Context passed to capability execution
#[derive(Debug, Clone)]
pub struct CapabilityContext {
    pub app: AppHandle,
    pub run_id: String,
    pub state: RunState,
}

/// Result of capability execution
#[derive(Debug, Clone)]
pub struct CapabilityResult {
    pub outcome: CapabilityOutcome,
    pub artifacts: Vec<Value>,
    pub side_effects: Vec<String>,
}

/// Outcome of capability execution
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityOutcome {
    Success,
    Partial,
    Failure(String),
}

/// Descriptor for a capability (used for planning, permissions, UI)
#[derive(Debug, Clone)]
pub struct CapabilityDescriptor {
    pub name: String,
    pub description: String,
    pub side_effects: Vec<String>,
    pub risk_level: RiskLevel,
    pub requires_permission: bool,
    pub artifacts_produced: Vec<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Universal capability trait
/// All agent actions implement this interface
#[async_trait]
pub trait Capability: Send + Sync {
    /// Unique identifier (e.g., "filesystem.read", "process.spawn")
    fn name(&self) -> &'static str;

    /// Human-readable description
    fn description(&self) -> &'static str;

    /// Side effects this capability produces (for permission gating + safety)
    fn side_effects(&self) -> &'static [&'static str];

    /// Risk level for this capability
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// Whether this capability requires explicit permission
    fn requires_permission(&self) -> bool {
        true
    }

    /// Types of artifacts this capability produces
    fn artifacts_produced(&self) -> &'static [&'static str] {
        &[]
    }

    /// JSON Schema for inputs (used by LLM for planning + validation)
    fn input_schema(&self) -> Value;

    /// Preflight check: validate inputs and declare permission/input needs. No I/O or side effects.
    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let _ = inputs;
        PreflightResult::Ok
    }

    /// Get full descriptor for this capability
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: self.name().to_string(),
            description: self.description().to_string(),
            side_effects: self.side_effects().iter().map(|s| s.to_string()).collect(),
            risk_level: self.risk_level(),
            requires_permission: self.requires_permission(),
            artifacts_produced: self.artifacts_produced().iter().map(|s| s.to_string()).collect(),
            input_schema: self.input_schema(),
        }
    }

    /// Execute the capability
    /// 
    /// This is where the actual work happens.
    /// The capability should:
    /// 1. Emit TOOL_EXECUTED event (before side effect)
    /// 2. Perform the side effect
    /// 3. Emit domain-specific events (e.g., FILE_READ, FILE_WRITTEN)
    /// 4. Emit STEP_COMPLETED or STEP_FAILED
    /// 5. Emit ARTIFACT_CREATED if applicable
    /// 6. Return CapabilityResult
    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String>;
}
