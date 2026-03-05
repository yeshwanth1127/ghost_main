//! Auto-approval policy - low-risk capabilities can be auto-approved (Moltbot-style).

use crate::agent::capabilities::RiskLevel;
use crate::agent::execution_ticket::ToolIntent;

/// Returns true if this capability + intent can be auto-approved without user confirmation.
/// Policy: Low risk only; no irreversible; no destructive/system-path risk factors.
pub fn is_auto_approved(capability: &str, risk_level: RiskLevel, intent: &ToolIntent) -> bool {
    // Only low-risk capabilities are candidates
    if risk_level != RiskLevel::Low {
        return false;
    }
    // Never auto-approve irreversible actions
    if intent.irreversible {
        return false;
    }
    // Never auto-approve if risk factors include system_path or destructive_command
    for factor in &intent.risk_factors {
        if factor == "system_path" || factor == "destructive_command" {
            return false;
        }
    }
    // Explicit allowlist: filesystem.read is safe to auto-approve
    if capability == "filesystem.read" {
        return true;
    }
    // Future: process.spawn with allowlisted commands, etc.
    false
}
