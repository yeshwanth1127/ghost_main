/**
 * Ghost gateway. Our own implementation (code ported from Moltbot, no wrapper).
 */
import { setGhostEnv, ensureGhostConfigDir } from "./env.js";
import { startGatewayServer as startServer } from "./gateway/server.js";
import { getGhostConfigPath } from "./config/paths.js";
import { DEFAULT_GATEWAY_PORT } from "./config/paths.js";

export type GhostGatewayServer = Awaited<ReturnType<typeof startGatewayServer>>;

export type GhostGatewayServerOptions = {
  port?: number;
  bind?: "loopback" | "lan";
};

/**
 * Start the Ghost gateway server (agent mode).
 * Our own implementation; config at ~/.ghost/ghost.json, state at ~/.ghost.
 */
export async function startGhostGateway(
  port: number = DEFAULT_GATEWAY_PORT,
  opts?: GhostGatewayServerOptions,
): Promise<GhostGatewayServer> {
  setGhostEnv();
  ensureGhostConfigDir();
  const actualPort = opts?.port ?? port;
  const bind = opts?.bind === "lan" ? "lan" : undefined;
  return startServer(actualPort, { bind });
}

export { setGhostEnv, ensureGhostConfigDir, getGhostConfigPath, getGhostStateDir } from "./env.js";
