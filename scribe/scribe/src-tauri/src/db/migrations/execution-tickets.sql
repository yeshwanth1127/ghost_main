-- Execution tickets: one state machine per tool call (Moltbot-style)
-- Restart-safe, auditable; permission flow: pending -> granted/denied -> running -> completed/failed

CREATE TABLE IF NOT EXISTS execution_tickets (
  ticket_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  step_id TEXT NOT NULL,
  capability TEXT NOT NULL,
  inputs TEXT NOT NULL,
  canonical_intent TEXT NOT NULL,
  expected_outcome TEXT,
  permission_state TEXT NOT NULL,
  execution_state TEXT NOT NULL,
  permission_id TEXT,
  execution_result TEXT,
  failure_reason TEXT,
  created_at INTEGER NOT NULL,
  permission_granted_at INTEGER,
  execution_started_at INTEGER,
  execution_completed_at INTEGER,
  retry_count INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL DEFAULT 3,
  FOREIGN KEY (run_id) REFERENCES runs(id)
);

CREATE INDEX IF NOT EXISTS idx_execution_tickets_run_id ON execution_tickets(run_id);
CREATE INDEX IF NOT EXISTS idx_execution_tickets_permission_state ON execution_tickets(permission_state);
CREATE INDEX IF NOT EXISTS idx_execution_tickets_execution_state ON execution_tickets(execution_state);
