import { getDatabase } from "./config";
import type { AuditEntry } from "@/types/assistant";

interface ActionSnapshot {
  id: string;
  action_id: string;
  original_path: string;
  snapshot_path: string;
  created_at: number;
  retention_until: number;
}

/**
 * Save an action snapshot
 */
export async function saveActionSnapshot(
  snapshot: ActionSnapshot
): Promise<void> {
  const db = await getDatabase();

  await db.execute(
    "INSERT INTO action_snapshots (id, action_id, original_path, snapshot_path, created_at, retention_until) VALUES (?, ?, ?, ?, ?, ?)",
    [
      snapshot.id,
      snapshot.action_id,
      snapshot.original_path,
      snapshot.snapshot_path,
      snapshot.created_at,
      snapshot.retention_until,
    ]
  );
}

/**
 * Get snapshots for an action
 */
export async function getActionSnapshots(
  actionId: string
): Promise<ActionSnapshot[]> {
  const db = await getDatabase();

  const rows = await db.select<Array<{
    id: string;
    action_id: string;
    original_path: string;
    snapshot_path: string;
    created_at: number;
    retention_until: number;
  }>>(
    "SELECT * FROM action_snapshots WHERE action_id = ? ORDER BY created_at DESC",
    [actionId]
  );

  return rows.map((row) => ({
    id: row.id,
    action_id: row.action_id,
    original_path: row.original_path,
    snapshot_path: row.snapshot_path,
    created_at: row.created_at,
    retention_until: row.retention_until,
  }));
}

/**
 * Get all snapshots (for cleanup)
 */
export async function getAllSnapshots(): Promise<ActionSnapshot[]> {
  const db = await getDatabase();

  const rows = await db.select<Array<{
    id: string;
    action_id: string;
    original_path: string;
    snapshot_path: string;
    created_at: number;
    retention_until: number;
  }>>(
    "SELECT * FROM action_snapshots ORDER BY created_at DESC"
  );

  return rows.map((row) => ({
    id: row.id,
    action_id: row.action_id,
    original_path: row.original_path,
    snapshot_path: row.snapshot_path,
    created_at: row.created_at,
    retention_until: row.retention_until,
  }));
}

/**
 * Delete a snapshot
 */
export async function deleteSnapshot(snapshotId: string): Promise<void> {
  const db = await getDatabase();

  await db.execute("DELETE FROM action_snapshots WHERE id = ?", [snapshotId]);
}

/**
 * Save audit log entry with hash chain
 */
export async function saveAuditLogCache(
  entry: AuditEntry
): Promise<void> {
  const db = await getDatabase();

  // Calculate prev_hash from last entry if not provided
  let prevHash = entry.prev_hash;
  if (!prevHash) {
    const lastEntry = await db.select<Array<{
      signature: string;
    }>>(
      "SELECT signature FROM audit_logs ORDER BY timestamp DESC LIMIT 1"
    );
    prevHash = lastEntry.length > 0 ? lastEntry[0].signature : "";
  }

  // Calculate signature if not provided
  let signature = entry.signature;
  if (!signature) {
    // Simple hash using Web Crypto API
    const encoder = new TextEncoder();
    const data = encoder.encode(entry.entry_json + prevHash);
    const hashBuffer = await crypto.subtle.digest("SHA-256", data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    signature = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  }

  await db.execute(
    "INSERT INTO audit_logs (id, entry_json, timestamp, prev_hash, signature, action_id) VALUES (?, ?, ?, ?, ?, ?)",
    [
      entry.id,
      entry.entry_json,
      entry.timestamp,
      prevHash,
      signature,
      entry.action_id || null,
    ]
  );
}

/**
 * Get audit logs from database (fallback if IPC fails)
 */
export async function getAuditLogsFromDB(
  limit: number = 50
): Promise<AuditEntry[]> {
  const db = await getDatabase();

  const rows = await db.select<Array<{
    id: string;
    entry_json: string;
    timestamp: number;
    prev_hash: string;
    signature: string;
    action_id: string | null;
  }>>(
    "SELECT * FROM audit_logs ORDER BY timestamp DESC LIMIT ?",
    [limit]
  );

  return rows.map((row) => ({
    id: row.id,
    entry_json: row.entry_json,
    timestamp: row.timestamp,
    prev_hash: row.prev_hash,
    signature: row.signature,
    action_id: row.action_id || undefined,
  }));
}

