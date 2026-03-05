/**
 * Gateway wire protocol. Our own implementation (ported from Moltbot protocol).
 * First frame must be connect; then request/response and server-push events.
 */

export const PROTOCOL_VERSION = 1;

export type ConnectParams = {
  minProtocol: number;
  maxProtocol: number;
  client: {
    id: string;
    displayName?: string;
    version: string;
    platform: string;
    mode?: string;
    instanceId?: string;
  };
  caps?: string[];
  auth?: { token?: string; password?: string };
};

export type RequestFrame = {
  type: "req";
  id: string;
  method: string;
  params?: Record<string, unknown>;
};

export type ResponseFrame = {
  type: "res";
  id: string;
  ok: boolean;
  payload?: unknown;
  error?: string;
  cached?: boolean;
};

export type EventFrame = {
  type: "event";
  event: string;
  payload?: unknown;
  seq?: number;
  stateVersion?: { presence?: number; health?: number };
};

export type HelloOkPayload = {
  type: "hello-ok";
  protocol: number;
  server: { version: string; connId: string; host?: string };
  snapshot?: { presence?: unknown; health?: unknown };
};

export const ErrorCodes = {
  INVALID_REQUEST: "invalid_request",
  UNAUTHORIZED: "unauthorized",
  NOT_FOUND: "not_found",
  INTERNAL: "internal",
} as const;

export function errorShape(code: string, message: string): { code: string; message: string } {
  return { code, message };
}

export function isConnectParams(v: unknown): v is ConnectParams {
  if (!v || typeof v !== "object") return false;
  const o = v as Record<string, unknown>;
  if (typeof o.minProtocol !== "number" || typeof o.maxProtocol !== "number") return false;
  if (!o.client || typeof o.client !== "object") return false;
  const c = o.client as Record<string, unknown>;
  return (
    typeof c.id === "string" &&
    typeof c.version === "string" &&
    typeof c.platform === "string"
  );
}

export function isRequestFrame(v: unknown): v is RequestFrame {
  if (!v || typeof v !== "object") return false;
  const o = v as Record<string, unknown>;
  return o.type === "req" && typeof o.id === "string" && typeof o.method === "string";
}
