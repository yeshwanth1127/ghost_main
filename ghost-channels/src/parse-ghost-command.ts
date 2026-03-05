/**
 * Detect /ghost or @ghost at the start of a message. If present, treat as Pi agent command.
 * Use either: "/ghost create a file" or "@ghost create a file" (both work the same).
 */

export interface GhostCommandParse {
  isAgent: boolean;
  command?: string;
}

const GHOST_PREFIXES = ["/ghost", "@ghost"];

export function parseGhostCommand(body: string): GhostCommandParse {
  const trimmed = (body ?? "").trim();
  if (!trimmed) return { isAgent: false };

  const lower = trimmed.toLowerCase();
  for (const prefix of GHOST_PREFIXES) {
    if (lower === prefix) {
      return { isAgent: true, command: "" };
    }
    if (lower.startsWith(prefix)) {
      const rest = trimmed.slice(prefix.length).trim();
      return { isAgent: true, command: rest };
    }
  }
  return { isAgent: false };
}
