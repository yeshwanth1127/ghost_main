import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TrendingUp, AlertCircle, DollarSign, Zap, Clock, Copy, Check } from 'lucide-react';
import { Header, Button } from '@/components';
import {
  getUserUsageStats,
  getUserUsageHistory,
  formatTokens,
  formatCurrency,
  getPlanDisplayName,
  getPlanColor,
  formatRelativeDate,
  UsageStats,
  UsageHistoryItem,
} from '@/lib/usage-api';

interface UsageDashboardProps {
  userId?: string;
}

interface RegistrationResponse {
  user_id: string;
  email: string;
  license_key: string;
  plan: string;
  trial_ends_at: string;
  message: string;
}

export const UsageDashboard = ({ userId }: UsageDashboardProps) => {
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [history, setHistory] = useState<UsageHistoryItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // ============================================
  // FETCH DATA
  // ============================================

  const fetchUsageData = async () => {
    if (!userId) {
      setError('No user ID available');
      setIsLoading(false);
      return;
    }

    try {
      setIsLoading(true);
      setError(null);
      const [statsData, historyData] = await Promise.all([
        getUserUsageStats(userId),
        getUserUsageHistory(userId, 10),
      ]);
      setStats(statsData);
      setHistory(historyData);
    } catch (err) {
      console.error('Error fetching usage data:', err);
      setError('Failed to load usage data');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchUsageData();
    // Refresh every 30 seconds
    const interval = setInterval(fetchUsageData, 30000);
    return () => clearInterval(interval);
  }, [userId]);

  // ============================================
  // LOADING STATE
  // ============================================

  if (isLoading) {
    return (
      <div id="usage-dashboard" className="space-y-3">
        <Header
          title="Usage & Billing"
          description="Track your AI usage, token consumption, and costs."
          isMainTitle
        />
        <div className="flex items-center justify-center p-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
        </div>
      </div>
    );
  }

  // ============================================
  // ERROR STATE - SHOW ERROR
  // ============================================

  if (error || !stats) {
    // Only show registration form if explicitly no user ID
    if (!userId) {
      return (
        <div id="usage-dashboard" className="space-y-3">
          <Header
            title="Usage & Billing"
            description="Track your AI usage, token consumption, and costs."
            isMainTitle
          />
          <RegistrationForm onSuccess={() => window.location.reload()} />
        </div>
      );
    }

    // Show error for logged-in users
    return (
      <div id="usage-dashboard" className="space-y-3">
        <Header
          title="Usage & Billing"
          description="Track your AI usage, token consumption, and costs."
          isMainTitle
        />
        <div className="p-4 bg-destructive/10 border border-destructive/30 rounded-md">
          <p className="text-sm text-destructive">
            {error || 'No usage data available'}
          </p>
          <Button
            onClick={fetchUsageData}
            variant="outline"
            size="sm"
            className="mt-2"
          >
            Retry
          </Button>
        </div>
      </div>
    );
  }

  // ============================================
  // USAGE CALCULATIONS
  // ============================================

  const usagePercentage = Math.min(stats.percentage_used, 100);
  const isNearLimit = usagePercentage >= 90;
  const isOverLimit = usagePercentage >= 100;

  const progressBarColor = isOverLimit
    ? 'bg-red-500'
    : isNearLimit
    ? 'bg-yellow-500'
    : 'bg-green-500';

  const resetDate = new Date(stats.monthly_reset_at);
  const daysUntilReset = Math.ceil(
    (resetDate.getTime() - Date.now()) / (1000 * 60 * 60 * 24)
  );

  // ============================================
  // RENDER
  // ============================================

  return (
    <div id="usage-dashboard" className="space-y-4">
      {/* Header */}
      <Header
        title="Usage & Billing"
        description="Track your AI usage, token consumption, and costs."
        isMainTitle
      />

      {/* Plan Badge */}
      <div className="flex items-center justify-between p-3 bg-secondary/20 border rounded-md">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Current Plan:</span>
          <span className={`text-sm font-bold ${getPlanColor(stats.plan)}`}>
            {getPlanDisplayName(stats.plan)}
          </span>
        </div>
        <Button variant="outline" size="sm" title="Upgrade your plan">
          Upgrade
        </Button>
      </div>

      {/* Warning Banner (if near/over limit) */}
      {(isNearLimit || isOverLimit) && (
        <div
          className={`p-3 border rounded-md ${
            isOverLimit
              ? 'bg-red-50 border-red-200 dark:bg-red-950/20 dark:border-red-900'
              : 'bg-yellow-50 border-yellow-200 dark:bg-yellow-950/20 dark:border-yellow-900'
          }`}
        >
          <div className="flex items-start gap-2">
            <AlertCircle
              className={`h-5 w-5 mt-0.5 ${
                isOverLimit ? 'text-red-600' : 'text-yellow-600'
              }`}
            />
            <div>
              <p
                className={`text-sm font-medium ${
                  isOverLimit ? 'text-red-700' : 'text-yellow-700'
                } dark:text-current`}
              >
                {isOverLimit
                  ? 'Token Limit Exceeded'
                  : 'Approaching Token Limit'}
              </p>
              <p
                className={`text-xs ${
                  isOverLimit ? 'text-red-600' : 'text-yellow-600'
                } dark:text-current`}
              >
                {isOverLimit
                  ? 'Please upgrade your plan to continue using AI features.'
                  : `You've used ${usagePercentage.toFixed(1)}% of your monthly limit.`}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Token Usage */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Zap className="h-4 w-4 text-primary" />
            <span className="text-sm font-medium">Token Usage</span>
          </div>
          <span className="text-xs text-muted-foreground">
            {formatTokens(stats.tokens_used)} / {formatTokens(stats.token_limit)}
          </span>
        </div>

        {/* Progress Bar */}
        <div className="h-3 bg-secondary/30 rounded-full overflow-hidden">
          <div
            className={`h-full ${progressBarColor} transition-all duration-300`}
            style={{ width: `${Math.min(usagePercentage, 100)}%` }}
          ></div>
        </div>

        <p className="text-xs text-muted-foreground">
          {usagePercentage.toFixed(1)}% used · Resets in {daysUntilReset} days
        </p>
      </div>

      {/* Cost Summary */}
      <div className="grid grid-cols-2 gap-3">
        <div className="p-3 bg-secondary/20 border rounded-md">
          <div className="flex items-center gap-2 mb-1">
            <DollarSign className="h-4 w-4 text-green-600" />
            <span className="text-xs text-muted-foreground">Cost (USD)</span>
          </div>
          <p className="text-lg font-bold">
            {formatCurrency(stats.total_cost_usd, 'USD')}
          </p>
        </div>

        <div className="p-3 bg-secondary/20 border rounded-md">
          <div className="flex items-center gap-2 mb-1">
            <TrendingUp className="h-4 w-4 text-blue-600" />
            <span className="text-xs text-muted-foreground">Requests</span>
          </div>
          <p className="text-lg font-bold">{stats.total_requests}</p>
        </div>
      </div>

      {/* Model Breakdown */}
      {stats.model_breakdown.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-sm font-medium">Model Usage Breakdown</h4>
          <div className="space-y-2">
            {stats.model_breakdown.map((model, index) => (
              <div
                key={index}
                className="flex items-center justify-between p-2 bg-secondary/10 border rounded-md text-xs"
              >
                <div>
                  <p className="font-medium">
                    {model.provider} / {model.model}
                  </p>
                  <p className="text-muted-foreground">
                    {formatTokens(model.tokens)} tokens · {model.requests}{' '}
                    requests
                  </p>
                </div>
                <span className="font-mono">
                  {formatCurrency(model.cost_usd, 'USD')}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Recent Activity */}
      {history.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-sm font-medium">Recent Activity</h4>
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {history.map((item) => (
              <div
                key={item.id}
                className="flex items-center justify-between p-2 bg-secondary/10 border rounded-md text-xs hover:bg-secondary/20 transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <p className="font-medium truncate">
                    {item.provider} / {item.model}
                  </p>
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3 w-3" />
                    <span>{formatRelativeDate(item.created_at)}</span>
                  </div>
                </div>
                <div className="text-right ml-2">
                  <p className="font-mono">{formatTokens(item.total_tokens)}</p>
                  <p className="text-muted-foreground">
                    {formatCurrency(item.cost_usd, 'USD')}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Refresh Button */}
      <Button
        onClick={fetchUsageData}
        variant="outline"
        size="sm"
        className="w-full"
        title="Refresh usage data"
      >
        Refresh Usage Data
      </Button>
    </div>
  );
};

// ============================================
// REGISTRATION FORM COMPONENT
// ============================================

interface RegistrationFormProps {
  onSuccess?: () => void;
}

const RegistrationForm = ({ onSuccess }: RegistrationFormProps) => {
  const [email, setEmail] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [registered, setRegistered] = useState(false);
  const [licenseKey, setLicenseKey] = useState('');
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!email.trim()) {
      setError('Please enter a valid email');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch('http://localhost:8083/api/v1/auth/register', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email }),
      });

      // Read response body once
      const responseText = await response.text();
      
      if (!response.ok) {
        try {
          const errorData = JSON.parse(responseText);
          throw new Error(errorData.error || `Registration failed: ${response.status}`);
        } catch {
          throw new Error(`Registration failed: ${response.status}`);
        }
      }

      // Parse the successful response
      try {
        const data: RegistrationResponse = JSON.parse(responseText);
        const { license_key } = data;
        
        // Generate instance_id (required by app initialization)
        const instanceId = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
        
        // Store license_key and instance_id for app to use
        try {
          await invoke("secure_storage_save", {
            items: [
              {
                key: "Scribe_license_key",
                value: license_key,
              },
              {
                key: "Scribe_instance_id",
                value: instanceId,
              },
            ],
          });
        } catch (storageErr) {
          console.warn('Failed to store credentials:', storageErr);
        }
        
        setLicenseKey(license_key);
        setRegistered(true);
        
        // Auto-dismiss after 2 seconds
        setTimeout(() => {
          if (onSuccess) onSuccess();
        }, 2000);
      } catch (parseErr) {
        console.error('Failed to parse registration response:', parseErr);
        console.error('Response text:', responseText);
        throw new Error('Invalid response format from server');
      }
    } catch (err) {
      console.error('Registration error:', err);
      setError(err instanceof Error ? err.message : 'Registration failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleCopyLicense = () => {
    navigator.clipboard.writeText(licenseKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (registered) {
    return (
      <div className="space-y-3 p-4 bg-green-50 border border-green-200 rounded-md dark:bg-green-950/20 dark:border-green-900">
        <div className="flex items-center gap-2 mb-2">
          <Check className="h-5 w-5 text-green-600" />
          <p className="font-semibold text-green-700 dark:text-green-400">
            ✅ Registration Successful!
          </p>
        </div>
        
        <div className="space-y-2">
          <p className="text-sm text-green-600 dark:text-green-400">
            Your 14-day free trial is now active.
          </p>
          
          <div className="bg-white dark:bg-slate-800 p-3 rounded-md border border-green-200 dark:border-green-900">
            <p className="text-xs text-muted-foreground mb-1">Your License Key:</p>
            <div className="flex items-center gap-2">
              <code className="text-xs font-mono bg-slate-100 dark:bg-slate-900 px-2 py-1 rounded flex-1 truncate">
                {licenseKey}
              </code>
              <button
                onClick={handleCopyLicense}
                className="p-1.5 hover:bg-slate-200 dark:hover:bg-slate-700 rounded transition-colors"
                title={copied ? 'Copied!' : 'Copy license key'}
              >
                {copied ? (
                  <Check className="h-4 w-4 text-green-600" />
                ) : (
                  <Copy className="h-4 w-4 text-slate-600 dark:text-slate-400" />
                )}
              </button>
            </div>
          </div>

          <p className="text-xs text-green-600 dark:text-green-400 mt-3">
            App will reload in a moment with your new account...
          </p>
        </div>
      </div>
    );
  }

  return (
    <form onSubmit={handleRegister} className="space-y-3">
      <div className="p-4 bg-blue-50 border border-blue-200 rounded-md dark:bg-blue-950/20 dark:border-blue-900">
        <p className="text-sm text-blue-700 dark:text-blue-400 mb-3">
          Create a new account to get started with your 14-day free trial.
        </p>

        <div className="space-y-2">
          <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
            Email Address
          </label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="you@example.com"
            className="w-full px-3 py-2 text-sm border rounded-md bg-white dark:bg-slate-800 border-input focus:outline-none focus:ring-2 focus:ring-blue-500"
            disabled={isLoading}
          />
        </div>

        {error && (
          <div className="mt-2 p-2 bg-red-50 border border-red-200 rounded-md dark:bg-red-950/20 dark:border-red-900">
            <p className="text-xs text-red-700 dark:text-red-400">{error}</p>
          </div>
        )}

        <Button
          type="submit"
          disabled={isLoading || !email.trim()}
          className="w-full mt-3"
        >
          {isLoading ? (
            <>
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
              Creating Account...
            </>
          ) : (
            'Get Free Trial (14 Days)'
          )}
        </Button>

        <p className="text-xs text-slate-500 dark:text-slate-400 mt-2">
          Free plan includes 5,000 tokens/month. Upgrade anytime.
        </p>
      </div>
    </form>
  );
};

