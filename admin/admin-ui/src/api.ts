const API_BASE = "/api";

function getToken(): string | null {
  return localStorage.getItem("admin_token");
}

export async function login(username: string, password: string): Promise<{ token: string }> {
  const res = await fetch(`${API_BASE}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || "Login failed");
  }
  return res.json();
}

async function fetchWithAuth(url: string) {
  const token = getToken();
  const res = await fetch(`${API_BASE}${url}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (res.status === 401) {
    localStorage.removeItem("admin_token");
    window.location.href = "/login";
    throw new Error("Unauthorized");
  }
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export interface GlobalStats {
  total_users: number;
  total_tokens: number;
  total_cost_usd: string;
  total_revenue: string;
}

export function getGlobalStats(): Promise<GlobalStats> {
  return fetchWithAuth("/stats/global");
}

export interface ModelBreakdownRow {
  model: string;
  provider: string;
  tokens: number;
  cost_usd: string;
  requests: number;
}

export function getModelBreakdown(): Promise<ModelBreakdownRow[]> {
  return fetchWithAuth("/stats/model-breakdown");
}

export interface TopUserRow {
  email: string | null;
  tokens: number;
  cost_usd: string;
}

export function getTopUsers(): Promise<TopUserRow[]> {
  return fetchWithAuth("/stats/top-users");
}

export interface RecentMessageRow {
  user_id: string;
  email: string | null;
  model: string;
  provider: string;
  total_tokens: number;
  cost_usd: string;
  created_at: string;
}

export function getRecentMessages(): Promise<RecentMessageRow[]> {
  return fetchWithAuth("/stats/recent-messages");
}
