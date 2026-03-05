#!/usr/bin/env node
/**
 * Ghost gateway CLI (agent mode). Our own implementation (no Moltbot wrapper).
 */
import { setGhostEnv, ensureGhostConfigDir } from "./env.js";
import { startGatewayServer } from "./gateway/server.js";
import { getGhostConfigPath } from "./config/paths.js";
import { DEFAULT_GATEWAY_PORT } from "./config/paths.js";

setGhostEnv();
ensureGhostConfigDir();

const portArg = process.argv.indexOf("--port");
const port = portArg !== -1 && process.argv[portArg + 1]
  ? Number.parseInt(process.argv[portArg + 1], 10) || DEFAULT_GATEWAY_PORT
  : DEFAULT_GATEWAY_PORT;

async function main(): Promise<void> {
  const server = await startGatewayServer(port, { bind: "loopback" });
  console.log(`Ghost gateway (agent mode) listening on ws://127.0.0.1:${port}`);
  console.log(`Config: ${getGhostConfigPath()}`);
  process.on("SIGINT", () => {
    server.close({ reason: "SIGINT" }).then(() => process.exit(0));
  });
  process.on("SIGTERM", () => {
    server.close({ reason: "SIGTERM" }).then(() => process.exit(0));
  });
}

main().catch((err) => {
  console.error("Ghost gateway failed:", err);
  process.exit(1);
});
