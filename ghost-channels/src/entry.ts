/**
 * Ghost channels entry: load config, connect to scribe-api gateway, start Telegram and WhatsApp adapters.
 */

import { dirname, join } from "path";
import { fileURLToPath } from "url";
import { config } from "dotenv";

// Load .env from the package root (ghost-channels/), not cwd
const __dirname = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(__dirname, "..");
config({ path: join(packageRoot, ".env") });

import { loadConfig } from "./config.js";
import { GatewayClient } from "./gateway-client.js";
import { startTelegramBot } from "./telegram/index.js";
import { startWhatsAppClient } from "./whatsapp/index.js";

async function main(): Promise<void> {
  const configPath = process.env.GHOST_CONFIG_PATH ?? undefined;
  const config = loadConfig(configPath);

  console.log("[ghost-channels] Gateway URL:", config.gateway.url);

  const gatewayClient = new GatewayClient(config.gateway.url);

  try {
    await gatewayClient.connect();
    console.log("[ghost-channels] Connected to gateway");
  } catch (err) {
    console.error("[ghost-channels] Failed to connect to gateway:", err);
    process.exit(1);
  }

  if (config.channels.telegram) {
    const bot = startTelegramBot({
      botToken: config.channels.telegram.botToken,
      gatewayClient,
    });
    void bot.start(); // long-running; do not await
    console.log("[ghost-channels] Telegram bot started");
  }

  if (config.channels.whatsapp) {
    await startWhatsAppClient({
      authDir: config.channels.whatsapp.authDir,
      gatewayClient,
    });
    console.log("[ghost-channels] WhatsApp client started (scan QR if first run)");
  }

  if (!config.channels.telegram && !config.channels.whatsapp) {
    console.warn(
      "[ghost-channels] No channels enabled. Set GHOST_TELEGRAM_BOT_TOKEN or GHOST_WHATSAPP_AUTH_DIR (or config file)."
    );
  }

  process.on("SIGINT", () => {
    gatewayClient.close();
    process.exit(0);
  });
  process.on("SIGTERM", () => {
    gatewayClient.close();
    process.exit(0);
  });
}

main().catch((err) => {
  console.error("[ghost-channels] Fatal:", err);
  process.exit(1);
});
