import { fetch as tauriFetch } from '@tauri-apps/plugin-http';

// ============================================
// TYPES
// ============================================

export interface UsageStats {
  user_id: string;
  plan: string;
  tokens_used: number;
  token_limit: number;
  percentage_used: number;
  total_cost_usd: number | string; // Backend returns decimals as strings
  total_cost_inr: number | string; // Backend returns decimals as strings
  total_requests: number;
  monthly_reset_at: string;
  model_breakdown: ModelUsageBreakdown[];
}

export interface ModelUsageBreakdown {
  model: string;
  provider: string;
  tokens: number;
  requests: number;
  cost_usd: number | string; // Backend returns decimals as strings
}

export interface UsageHistoryItem {
  id: string;
  model: string;
  provider: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cost_usd: number | string; // Backend returns decimals as strings
  cost_inr: number;
  status: string;
  created_at: string;
}

export interface TokenLimitCheck {
  allowed: boolean;
  tokens_available: number;
  tokens_used: number;
  token_limit: number;
  percentage_used: number;
  warning?: string;
}

export interface ModelPricing {
  id: string;
  model: string;
  provider: string;
  input_cost_per_1m: number;
  output_cost_per_1m: number;
  active: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================
// API CLIENT
// ============================================

import { APP_ENDPOINT } from "@/config/constants";
const API_BASE_URL = import.meta.env.VITE_API_URL || APP_ENDPOINT;

/**
 * Get current month's usage statistics for a user
 */
export async function getUserUsageStats(userId: string): Promise<UsageStats> {
  try {
    const response = await tauriFetch(`${API_BASE_URL}/api/v1/usage/${userId}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    if (!response.ok) {
      throw new Error(`Failed to fetch usage stats: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error('Error fetching usage stats:', error);
    throw error;
  }
}

/**
 * Get usage history (recent messages)
 */
export async function getUserUsageHistory(
  userId: string,
  limit: number = 50
): Promise<UsageHistoryItem[]> {
  try {
    const response = await tauriFetch(
      `${API_BASE_URL}/api/v1/usage/${userId}/history?limit=${limit}`,
      {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
    if (!response.ok) {
      throw new Error(`Failed to fetch usage history: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error('Error fetching usage history:', error);
    throw error;
  }
}

/**
 * Check token limit for a user
 */
export async function checkTokenLimit(userId: string): Promise<TokenLimitCheck> {
  try {
    const response = await tauriFetch(
      `${API_BASE_URL}/api/v1/usage/${userId}/limit-check`,
      {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
    if (!response.ok) {
      throw new Error(`Failed to check token limit: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error('Error checking token limit:', error);
    throw error;
  }
}

/**
 * Get all active model pricing
 */
export async function getModelPricing(): Promise<ModelPricing[]> {
  try {
    const response = await tauriFetch(`${API_BASE_URL}/api/v1/usage/pricing`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    if (!response.ok) {
      throw new Error(`Failed to fetch model pricing: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error('Error fetching model pricing:', error);
    throw error;
  }
}

// ============================================
// UTILITY FUNCTIONS
// ============================================

/**
 * Format tokens in human-readable format
 */
export function formatTokens(tokens: number): string {
  if (tokens < 1000) return `${tokens}`;
  if (tokens < 1000000) return `${(tokens / 1000).toFixed(1)}K`;
  return `${(tokens / 1000000).toFixed(2)}M`;
}

/**
 * Format currency
 */
export function formatCurrency(amount: number | string, currency: 'USD' | 'INR' = 'USD'): string {
  // Convert string to number if needed (backend sends decimals as strings)
  const numAmount = typeof amount === 'string' ? parseFloat(amount) : amount;
  
  if (isNaN(numAmount)) {
    return currency === 'USD' ? '$0.0000' : '₹0.00';
  }
  
  if (currency === 'USD') {
    return `$${numAmount.toFixed(4)}`;
  }
  return `₹${numAmount.toFixed(2)}`;
}

/**
 * Get plan display name
 */
export function getPlanDisplayName(plan: string): string {
  const plans: Record<string, string> = {
    free: 'Free',
    starter: 'Starter',
    pro: 'Pro',
    power: 'Power',
  };
  return plans[plan] || plan;
}

/**
 * Get plan color for UI
 */
export function getPlanColor(plan: string): string {
  const colors: Record<string, string> = {
    free: 'text-gray-500',
    starter: 'text-blue-500',
    pro: 'text-primary',
    power: 'text-orange-500',
  };
  return colors[plan] || 'text-gray-500';
}

/**
 * Format date relative to now
 */
export function formatRelativeDate(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return 'just now';
}
