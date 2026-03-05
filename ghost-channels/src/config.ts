/**
 * Load Ghost channels config from ~/.ghost/ghost.json or env.
 */

import { readFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

export interface GhostChannelsConfig {
  gateway: {
    url: string;
  };
  channels: {
    telegram?: {
      botToken: string;
    };
    whatsapp?: {
      authDir: string;
    };
  };
}

const DEFAULT_GATEWAY_URL = "ws://127.0.0.1:8083/gateway";
const DEFAULT_CONFIG_PATH = join(homedir(), ".ghost", "ghost.json");

export function loadConfig(configPath?: string): GhostChannelsConfig {
  const path = configPath ?? DEFAULT_CONFIG_PATH;
  const gatewayUrl =
    process.env.GHOST_GATEWAY_URL ??
    process.env.VITE_GHOST_GATEWAY_WS_URL ??
    DEFAULT_GATEWAY_URL;
  const telegramToken = process.env.GHOST_TELEGRAM_BOT_TOKEN;
  const whatsappAuthDir = process.env.GHOST_WHATSAPP_AUTH_DIR;

  let fileConfig: Partial<GhostChannelsConfig> = {};
  try {
    const raw = readFileSync(path, "utf-8");
    fileConfig = JSON.parse(raw) as Partial<GhostChannelsConfig>;
  } catch {
    // No file or invalid JSON: use env/defaults only
  }

  const telegramBotToken =
    fileConfig.channels?.telegram?.botToken ?? telegramToken ?? "";
  const whatsappAuth =
    fileConfig.channels?.whatsapp?.authDir ?? whatsappAuthDir ?? "";

  return {
    gateway: {
      url: fileConfig.gateway?.url ?? gatewayUrl,
    },
    channels: {
      telegram:
        telegramBotToken.length > 0 ? { botToken: telegramBotToken } : undefined,
      whatsapp:
        whatsappAuth.length > 0 ? { authDir: whatsappAuth } : undefined,
    },
  };
}
