/**
 * Ghost config and state paths. Our own implementation (ported from Moltbot patterns).
 */
import os from "node:os";
import path from "node:path";

const GHOST_DIR = ".ghost";
const GHOST_CONFIG_FILE = "ghost.json";

export function getGhostStateDir(homedir: () => string = os.homedir): string {
  const override = process.env.GHOST_STATE_DIR?.trim();
  if (override) return path.resolve(override.replace(/^~/, os.homedir()));
  return path.join(homedir(), GHOST_DIR);
}

export function getGhostConfigPath(homedir: () => string = os.homedir): string {
  const override = process.env.GHOST_CONFIG_PATH?.trim();
  if (override) return path.resolve(override.replace(/^~/, os.homedir()));
  return path.join(getGhostStateDir(homedir), GHOST_CONFIG_FILE);
}

export const DEFAULT_GATEWAY_PORT = 18789;
