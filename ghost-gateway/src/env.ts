/**
 * Ghost config and state paths. Our own implementation.
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { getGhostConfigPath, getGhostStateDir } from "./config/paths.js";

export function setGhostEnv(): void {
  const ghostDir = getGhostStateDir();
  const configPath = getGhostConfigPath();
  process.env.GHOST_CONFIG_PATH = configPath;
  process.env.GHOST_STATE_DIR = ghostDir;
}

export function ensureGhostConfigDir(): void {
  const ghostDir = getGhostStateDir();
  const configPath = getGhostConfigPath();
  if (!fs.existsSync(ghostDir)) {
    fs.mkdirSync(ghostDir, { recursive: true });
  }
  if (!fs.existsSync(configPath)) {
    const defaultConfig = {
      gateway: { port: 18789, bind: "loopback" },
      agent: {},
      channels: {},
    };
    fs.writeFileSync(configPath, JSON.stringify(defaultConfig, null, 2), "utf-8");
  }
}

export { getGhostConfigPath, getGhostStateDir } from "./config/paths.js";
