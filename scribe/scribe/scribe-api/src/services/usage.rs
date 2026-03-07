use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::config::Config;
use crate::models::{
    CostCalculation, Message, ModelPricing, MonthlyUsage, TokenLimitCheck, UsageError,
    UsageHistoryItem, UsageRecord, UsageStats, ModelUsageBreakdown, User,
};

const USD_TO_INR_RATE: f64 = 84.0; // Update periodically

#[derive(Clone)]
pub struct UsageService {
    pool: PgPool,
    config: Config,
}

impl UsageService {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }

    // ============================================
    // MODEL PRICING
    // ============================================

    /// Get pricing for a specific model from OpenRouter with database fallback
    pub async fn get_model_pricing(&self, model: &str) -> Result<ModelPricing, UsageError> {
        // Try to fetch from OpenRouter first
        if let Ok(pricing) = self.fetch_pricing_from_openrouter(model).await {
            return Ok(pricing);
        }

        // Fall back to database pricing
        let pricing = sqlx::query_as::<_, ModelPricing>(
            "SELECT * FROM model_pricing WHERE model = $1 AND active = TRUE"
        )
        .bind(model)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| UsageError::ModelPricingNotFound(model.to_string()))?;

        Ok(pricing)
    }

    /// Fetch pricing from OpenRouter API
    async fn fetch_pricing_from_openrouter(&self, model: &str) -> Result<ModelPricing, UsageError> {
        let client = reqwest::Client::new();
        let url = format!("{}/models", self.config.openrouter_base_url);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.openrouter_api_key))
            .header("HTTP-Referer", "https://exora.solutions")
            .header("X-Title", "Ghost API")
            .send()
            .await
            .map_err(|e| UsageError::ModelPricingNotFound(format!("Failed to fetch from OpenRouter: {}", e)))?;

        if !response.status().is_success() {
            return Err(UsageError::ModelPricingNotFound("OpenRouter API error".to_string()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| UsageError::ModelPricingNotFound(format!("Failed to parse OpenRouter response: {}", e)))?;

        let models = json
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| UsageError::ModelPricingNotFound("Invalid OpenRouter response format".to_string()))?;

        // Find matching model
        for m in models {
            if let Some(model_id) = m.get("id").and_then(|id| id.as_str()) {
                if model_id == model {
                    let pricing = m
                        .get("pricing")
                        .and_then(|p| p.as_object())
                        .ok_or_else(|| UsageError::ModelPricingNotFound("Missing pricing in model".to_string()))?;

                    let input_cost_str = pricing
                        .get("prompt")
                        .and_then(|p| p.as_str())
                        .unwrap_or("0");
                    let output_cost_str = pricing
                        .get("completion")
                        .and_then(|c| c.as_str())
                        .unwrap_or("0");

                    let input_cost: Decimal = input_cost_str
                        .parse()
                        .unwrap_or_else(|_| Decimal::ZERO);
                    let output_cost: Decimal = output_cost_str
                        .parse()
                        .unwrap_or_else(|_| Decimal::ZERO);

                    return Ok(ModelPricing {
                        id: uuid::Uuid::new_v4(),
                        model: model.to_string(),
                        provider: model.split('/').next().unwrap_or("unknown").to_string(),
                        input_cost_per_1m: input_cost,
                        output_cost_per_1m: output_cost,
                        active: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
        }

        Err(UsageError::ModelPricingNotFound(model.to_string()))
    }

    /// Get all active model pricing
    pub async fn get_all_model_pricing(&self) -> Result<Vec<ModelPricing>, UsageError> {
        let pricing = sqlx::query_as::<_, ModelPricing>(
            "SELECT * FROM model_pricing WHERE active = TRUE ORDER BY model"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(pricing)
    }

    // ============================================
    // COST CALCULATION
    // ============================================

    /// Calculate cost for a given usage
    pub async fn calculate_cost(
        &self,
        model: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> Result<CostCalculation, UsageError> {
        let pricing = self.get_model_pricing(model).await?;

        // Calculate cost: (tokens / 1,000,000) * rate_per_1m
        let input_cost = (Decimal::from(prompt_tokens) / Decimal::from(1_000_000))
            * pricing.input_cost_per_1m;
        let output_cost = (Decimal::from(completion_tokens) / Decimal::from(1_000_000))
            * pricing.output_cost_per_1m;
        let total_cost_usd = input_cost + output_cost;

        // Convert to INR
        let total_cost_inr = total_cost_usd * Decimal::from_f64_retain(USD_TO_INR_RATE).unwrap();

        Ok(CostCalculation {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cost_usd: total_cost_usd,
            cost_inr: total_cost_inr,
        })
    }

    /// Calculate cost, or zero if model pricing not found (e.g. local Ollama/Exora)
    pub async fn calculate_cost_or_zero(
        &self,
        model: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> CostCalculation {
        match self.calculate_cost(model, prompt_tokens, completion_tokens).await {
            Ok(cost) => cost,
            Err(_) => CostCalculation {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                cost_usd: Decimal::from(0),
                cost_inr: Decimal::from(0),
            },
        }
    }

    // ============================================
    // TOKEN LIMIT CHECKING
    // ============================================

    /// Check if user has enough tokens available for a request
    pub async fn check_token_limit(
        &self,
        user_id: Uuid,
        requested_tokens: i32,
    ) -> Result<TokenLimitCheck, UsageError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(UsageError::UserNotFound(user_id))?;

        // Owner: unlimited tokens, always allowed
        if user.is_owner {
            return Ok(TokenLimitCheck {
                allowed: true,
                tokens_available: i64::MAX,
                tokens_used: user.tokens_used_this_month.unwrap_or(0),
                token_limit: i64::MAX,
                percentage_used: 0.0,
                warning: None,
            });
        }

        let tokens_used = user.tokens_used_this_month.unwrap_or(0);
        let token_limit = user.monthly_token_limit.unwrap_or(5000);
        let tokens_available = token_limit - tokens_used;
        let percentage_used = (tokens_used as f64 / token_limit as f64) * 100.0;

        let allowed = tokens_used + requested_tokens as i64 <= token_limit;

        let warning = if percentage_used >= 90.0 && percentage_used < 100.0 {
            Some(format!(
                "You've used {:.1}% of your monthly token limit",
                percentage_used
            ))
        } else if !allowed {
            Some("Token limit exceeded".to_string())
        } else {
            None
        };

        Ok(TokenLimitCheck {
            allowed,
            tokens_available,
            tokens_used,
            token_limit,
            percentage_used,
            warning,
        })
    }

    // ============================================
    // USAGE RECORDING
    // ============================================

    /// Record usage in a transaction (atomically updates messages, users, monthly_usage)
    pub async fn record_usage(&self, usage: UsageRecord) -> Result<Message, UsageError> {
        // Calculate cost
        let cost = self
            .calculate_cost(&usage.model, usage.prompt_tokens, usage.completion_tokens)
            .await?;

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // 1. Insert message record
        let message = self.insert_message(&mut tx, &usage, &cost).await?;

        // 2. Update user's monthly token usage
        self.update_user_tokens(&mut tx, usage.user_id, cost.total_tokens as i64)
            .await?;

        // 3. Update or create monthly usage aggregate
        self.update_monthly_usage(&mut tx, usage.user_id, &usage.model, &usage.provider, &cost)
            .await?;

        // Commit transaction
        tx.commit().await?;

        Ok(message)
    }

    /// Record usage from client (e.g. direct Ollama/Exora). Uses 0 cost if model pricing not found.
    pub async fn record_usage_from_client(&self, usage: UsageRecord) -> Result<Message, UsageError> {
        let cost = self
            .calculate_cost_or_zero(&usage.model, usage.prompt_tokens, usage.completion_tokens)
            .await;

        let mut tx = self.pool.begin().await?;

        let message = self.insert_message(&mut tx, &usage, &cost).await?;
        self.update_user_tokens(&mut tx, usage.user_id, cost.total_tokens as i64)
            .await?;
        self.update_monthly_usage(&mut tx, usage.user_id, &usage.model, &usage.provider, &cost)
            .await?;

        tx.commit().await?;

        Ok(message)
    }

    /// Insert a message record
    async fn insert_message(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        usage: &UsageRecord,
        cost: &CostCalculation,
    ) -> Result<Message, UsageError> {
        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (
                user_id, license_key, model, provider,
                prompt_tokens, completion_tokens, total_tokens,
                cost_usd, cost_inr,
                conversation_id, request_duration_ms, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'success')
            RETURNING *
            "#
        )
        .bind(usage.user_id)
        .bind(&usage.license_key)
        .bind(&usage.model)
        .bind(&usage.provider)
        .bind(cost.prompt_tokens)
        .bind(cost.completion_tokens)
        .bind(cost.total_tokens)
        .bind(cost.cost_usd)
        .bind(cost.cost_inr)
        .bind(&usage.conversation_id)
        .bind(usage.request_duration_ms)
        .fetch_one(&mut **tx)
        .await?;

        Ok(message)
    }

    /// Update user's token count
    async fn update_user_tokens(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        tokens: i64,
    ) -> Result<(), UsageError> {
        sqlx::query(
            r#"
            UPDATE users
            SET tokens_used_this_month = COALESCE(tokens_used_this_month, 0) + $1,
                updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(tokens)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Update monthly usage aggregate
    async fn update_monthly_usage(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        model: &str,
        provider: &str,
        cost: &CostCalculation,
    ) -> Result<(), UsageError> {
        let month = Utc::now().format("%Y-%m").to_string();

        // Create model usage entry for JSONB
        let model_key = format!("{}_{}", provider, model);
        
        sqlx::query(
            r#"
            INSERT INTO monthly_usage (user_id, month, total_tokens, total_cost_usd, total_cost_inr, total_requests, model_usage)
            VALUES ($1, $2, $3, $4, $5, 1, jsonb_build_object($6, jsonb_build_object('tokens', $3, 'requests', 1, 'cost_usd', $4)))
            ON CONFLICT (user_id, month) DO UPDATE SET
                total_tokens = monthly_usage.total_tokens + $3,
                total_cost_usd = monthly_usage.total_cost_usd + $4,
                total_cost_inr = monthly_usage.total_cost_inr + $5,
                total_requests = monthly_usage.total_requests + 1,
                model_usage = jsonb_set(
                    COALESCE(monthly_usage.model_usage, '{}'::jsonb),
                    ARRAY[$6],
                    COALESCE(monthly_usage.model_usage->$6, '{}'::jsonb) || 
                    jsonb_build_object(
                        'tokens', COALESCE((monthly_usage.model_usage->$6->>'tokens')::bigint, 0) + $3,
                        'requests', COALESCE((monthly_usage.model_usage->$6->>'requests')::int, 0) + 1,
                        'cost_usd', COALESCE((monthly_usage.model_usage->$6->>'cost_usd')::numeric, 0) + $4
                    ),
                    true
                ),
                updated_at = NOW()
            "#
        )
        .bind(user_id)
        .bind(&month)
        .bind(cost.total_tokens as i64)
        .bind(cost.cost_usd)
        .bind(cost.cost_inr)
        .bind(&model_key)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    // ============================================
    // USAGE STATS QUERIES
    // ============================================

    /// Get current month's usage stats for a user
    pub async fn get_user_usage(&self, user_id: Uuid) -> Result<UsageStats, UsageError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(UsageError::UserNotFound(user_id))?;

        let month = Utc::now().format("%Y-%m").to_string();
        
        let monthly_usage = sqlx::query_as::<_, MonthlyUsage>(
            "SELECT * FROM monthly_usage WHERE user_id = $1 AND month = $2"
        )
        .bind(user_id)
        .bind(&month)
        .fetch_optional(&self.pool)
        .await?;

        let (tokens_used, token_limit, percentage_used) = if user.is_owner {
            (
                user.tokens_used_this_month.unwrap_or(0),
                i64::MAX,
                0.0,
            )
        } else {
            let used = user.tokens_used_this_month.unwrap_or(0);
            let limit = user.monthly_token_limit.unwrap_or(5000);
            let pct = (used as f64 / limit as f64) * 100.0;
            (used, limit, pct)
        };

        // Parse model breakdown from JSONB
        let model_breakdown = if let Some(usage) = &monthly_usage {
            self.parse_model_breakdown(&usage.model_usage)
        } else {
            vec![]
        };

        Ok(UsageStats {
            user_id,
            plan: user.plan.unwrap_or_else(|| "free".to_string()),
            tokens_used,
            token_limit,
            percentage_used,
            total_cost_usd: monthly_usage.as_ref().map(|u| u.total_cost_usd).unwrap_or_else(|| Decimal::from(0)),
            total_cost_inr: monthly_usage.as_ref().map(|u| u.total_cost_inr).unwrap_or_else(|| Decimal::from(0)),
            total_requests: monthly_usage.as_ref().map(|u| u.total_requests).unwrap_or(0),
            monthly_reset_at: user.monthly_reset_at.unwrap_or_else(Utc::now),
            model_breakdown,
        })
    }

    /// Parse model usage breakdown from JSONB
    fn parse_model_breakdown(&self, model_usage_json: &serde_json::Value) -> Vec<ModelUsageBreakdown> {
        let mut breakdown = vec![];

        if let Some(obj) = model_usage_json.as_object() {
            for (key, value) in obj {
                if let Some(data) = value.as_object() {
                    // Split key back into provider_model
                    let parts: Vec<&str> = key.splitn(2, '_').collect();
                    let (provider, model) = if parts.len() == 2 {
                        (parts[0].to_string(), parts[1].to_string())
                    } else {
                        ("unknown".to_string(), key.clone())
                    };

                    breakdown.push(ModelUsageBreakdown {
                        model: model.clone(),
                        provider: provider.clone(),
                        tokens: data.get("tokens").and_then(|v| v.as_i64()).unwrap_or(0),
                        requests: data.get("requests").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        cost_usd: data.get("cost_usd")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<Decimal>().ok())
                            .unwrap_or_else(|| Decimal::from(0)),
                    });
                }
            }
        }

        breakdown
    }

    /// Get usage history (recent messages)
    pub async fn get_usage_history(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> Result<Vec<UsageHistoryItem>, UsageError> {
        let history = sqlx::query_as::<_, UsageHistoryItem>(
            r#"
            SELECT id, model, provider, prompt_tokens, completion_tokens, total_tokens,
                   cost_usd, cost_inr, status, created_at
            FROM messages
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }

    // ============================================
    // ADMIN / UTILITY
    // ============================================

    /// Update user's plan and token limit
    pub async fn update_user_plan(
        &self,
        user_id: Uuid,
        plan: &str,
        token_limit: i64,
    ) -> Result<(), UsageError> {
        sqlx::query(
            r#"
            UPDATE users
            SET plan = $1,
                monthly_token_limit = $2,
                updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(plan)
        .bind(token_limit)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reset monthly tokens for a user (called by monthly reset job)
    pub async fn reset_monthly_tokens(&self, user_id: Uuid) -> Result<(), UsageError> {
        sqlx::query(
            r#"
            UPDATE users
            SET tokens_used_this_month = 0,
                monthly_reset_at = DATE_TRUNC('month', NOW() + INTERVAL '1 month'),
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation_logic() {
        // Example: GPT-4o-mini
        // Input: $0.15/1M, Output: $0.60/1M
        // 400 input + 900 output tokens
        let input_tokens = Decimal::from(400);
        let output_tokens = Decimal::from(900);
        let input_rate = Decimal::from_f64_retain(0.15).unwrap();
        let output_rate = Decimal::from_f64_retain(0.60).unwrap();

        let input_cost = (input_tokens / Decimal::from(1_000_000)) * input_rate;
        let output_cost = (output_tokens / Decimal::from(1_000_000)) * output_rate;
        let total_cost = input_cost + output_cost;

        assert_eq!(input_cost.to_string(), "0.00006");
        assert_eq!(output_cost.to_string(), "0.00054");
        assert_eq!(total_cost.to_string(), "0.00060");
    }
}
