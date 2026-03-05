/**
 * Minimal inbound context (Moltbot-style). Ghost's own types.
 */

export interface InboundContext {
  Body: string;
  From: string;
  To: string;
  SessionKey: string;
}

export type ChannelOutbound = (text: string) => Promise<void>;
