-- Create action_snapshots table for undo functionality
CREATE TABLE IF NOT EXISTS action_snapshots (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL,
    original_path TEXT NOT NULL,
    snapshot_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    retention_until INTEGER NOT NULL
);

-- Create audit_logs table for tamper-evident logging
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    entry_json TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    prev_hash TEXT NOT NULL,
    signature TEXT NOT NULL,
    action_id TEXT
);

-- Create capability_tokens table for scoped permissions
CREATE TABLE IF NOT EXISTS capability_tokens (
    nonce TEXT PRIMARY KEY,
    scopes_json TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- Indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_action_snapshots_action_id ON action_snapshots(action_id);
CREATE INDEX IF NOT EXISTS idx_action_snapshots_retention ON action_snapshots(retention_until);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action_id ON audit_logs(action_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_prev_hash ON audit_logs(prev_hash);
CREATE INDEX IF NOT EXISTS idx_capability_tokens_expires ON capability_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_capability_tokens_revoked ON capability_tokens(revoked);

-- Trigger to clean up expired snapshots (optional, can be done via periodic cleanup job)
-- Note: SQLite doesn't support scheduled jobs, so cleanup should be done in application code

