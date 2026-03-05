const API_BASE = "/api";

function getAdminToken(): string | null {
  return localStorage.getItem("admin_token");
}

function getCustomerToken(): string | null {
  return localStorage.getItem("customer_token");
}

function getToken(): string | null {
  return getAdminToken() ?? getCustomerToken();
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

export interface CustomerLoginResponse {
  token: string;
  user_id: string;
  email: string;
  license_key: string;
  plan: string;
}

export async function customerLogin(email: string, password: string): Promise<CustomerLoginResponse> {
  const res = await fetch(`${API_BASE}/auth/customer-login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || "Login failed");
  }
  return res.json();
}

export interface RegisterResponse {
  user_id: string;
  email: string;
  license_key: string;
  plan: string;
  trial_ends_at: string;
  message: string;
}

export async function register(email: string, password: string): Promise<RegisterResponse> {
  const res = await fetch(`${API_BASE}/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || "Registration failed");
  }
  return res.json();
}

export interface CreateSubscriptionResponse {
  subscription_id: string;
  key_id: string;
}

export async function createSubscription(
  plan: string,
  email: string,
  user_id?: string,
  license_key?: string
): Promise<CreateSubscriptionResponse> {
  const res = await fetch(`${API_BASE}/payments/create-subscription`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ plan, email, user_id, license_key }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || "Failed to create subscription");
  }
  return res.json();
}

export interface VerifyPaymentResponse {
  success: boolean;
  license_key?: string;
  message: string;
}

export async function verifyPayment(
  razorpay_payment_id: string,
  razorpay_subscription_id: string,
  razorpay_signature: string
): Promise<VerifyPaymentResponse> {
  const res = await fetch(`${API_BASE}/payments/verify`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      razorpay_payment_id,
      razorpay_subscription_id,
      razorpay_signature,
    }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || "Verification failed");
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
    localStorage.removeItem("customer_token");
    localStorage.removeItem("customer_email");
    localStorage.removeItem("customer_license");
    localStorage.removeItem("customer_user_id");
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
