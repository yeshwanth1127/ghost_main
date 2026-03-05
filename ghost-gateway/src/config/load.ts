/**
 * Load Ghost config. Our own implementation (minimal, ported from Moltbot config patterns).
 */
import fs from "node:fs";
import { getGhostConfigPath } from "./paths.js";

export type GhostConfig = {
  gateway?: { port?: number; bind?: string; auth?: { mode?: string; token?: string; password?: string } };
  agent?: Record<string, unknown>;
  channels?: Record<string, unknown>;
};

export function loadConfig(): GhostConfig {
  const configPath = getGhostConfigPath();
  if (!fs.existsSync(configPath)) {
    return {};
  }
  try {
    const raw = fs.readFileSync(configPath, "utf-8");
    const parsed = JSON.parse(raw) as unknown;
    return typeof parsed === "object" && parsed !== null ? (parsed as GhostConfig) : {};
  } catch {
    return {};
  }
}
