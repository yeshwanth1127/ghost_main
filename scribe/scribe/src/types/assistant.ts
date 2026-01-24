// TypeScript types matching Rust Action Schema v2

export interface ActionOrigin {
  user_input: string;
  source: "ui" | "voice" | "automation" | "plugin";
  request_id: string;
}

export interface Precondition {
  writable?: boolean;
  readable?: boolean;
  exists?: boolean;
  directory?: boolean;
}

export interface ActionMetadata {
  confidence?: number;
}

export type ActionType =
  | "fs_create_file"
  | "fs_read_file"
  | "fs_copy_file"
  | "fs_move_file"
  | "fs_delete_file"
  | "fs_create_directory";

export interface Action {
  id: string;
  type: ActionType;
  args: Record<string, any>;
  preconditions?: Precondition;
  metadata?: ActionMetadata;
}

export interface ActionSchema {
  id: string;
  origin: ActionOrigin;
  actions: Action[];
  summary: string;
  risk_score: number;
  dry_run: boolean;
}

export interface ActionPlan {
  id: string;
  origin: ActionOrigin;
  actions: Action[];
  summary: string;
  risk_score: number;
  dry_run: boolean;
}

export interface VerifiedPlan {
  plan: ActionPlan;
  verified_at: number;
  verification_notes: string[];
}

export interface ActionExecutionResult {
  action_id: string;
  success: boolean;
  output?: any;
  error?: string;
  snapshot_id?: string;
}

export interface ActionResult {
  action_id: string;
  success: boolean;
  executed_at: number;
  results: ActionExecutionResult[];
  error?: string;
  undo_available: boolean;
  undo_ttl?: number;
}

export interface AffectedItem {
  path: string;
  operation: string;
  preview?: string;
}

export interface PreviewResult {
  plan: ActionPlan;
  risk_score: number;
  affected_items: AffectedItem[];
  warnings: string[];
  requires_explicit_confirmation: boolean;
  missing_paths: string[]; // Action IDs that need path input
}

export interface AuditEntry {
  id: string;
  entry_json: string;
  timestamp: number;
  prev_hash: string;
  signature: string;
  action_id?: string;
}

export interface CapabilityToken {
  nonce: string;
  scopes: string[];
  ttl_seconds: number;
  session_id: string;
  expires_at: number;
  issued_at: number;
}

