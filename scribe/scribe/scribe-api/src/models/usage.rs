use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================
// DATABASE MODELS
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub user_id: Uuid,
    pub license_key: Option<String>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: Decimal,
    pub cost_inr: Option<Decimal>,
    pub conversation_id: Option<String>,
    pub request_duration_ms: Option<i32>,
    pub status: String, // success, error, rate_limited
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MonthlyUsage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub month: String, // Format: 'YYYY-MM'
    pub total_tokens: i64,
    pub total_cost_usd: Decimal,
    pub total_cost_inr: Decimal,
    pub total_requests: i32,
    pub model_usage: serde_json::Value, // JSONB
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModelPricing {
    pub id: Uuid,
    pub model: String,
    pub provider: String,
    pub input_cost_per_1m: Decimal,
    pub output_cost_per_1m: Decimal,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================
// REQUEST / RESPONSE DTOS
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub user_id: Uuid,
    pub license_key: Option<String>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub conversation_id: Option<String>,
    pub request_duration_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCalculation {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: Decimal,
    pub cost_inr: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub user_id: Uuid,
    pub plan: String,
    pub tokens_used: i64,
    pub token_limit: i64,
    pub percentage_used: f64,
    pub total_cost_usd: Decimal,
    pub total_cost_inr: Decimal,
    pub total_requests: i32,
    pub monthly_reset_at: DateTime<Utc>,
    pub model_breakdown: Vec<ModelUsageBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageBreakdown {
    pub model: String,
    pub provider: String,
    pub tokens: i64,
    pub requests: i32,
    pub cost_usd: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageHistoryItem {
    pub id: Uuid,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: Decimal,
    pub cost_inr: Option<Decimal>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLimitCheck {
    pub allowed: bool,
    pub tokens_available: i64,
    pub tokens_used: i64,
    pub token_limit: i64,
    pub percentage_used: f64,
    pub warning: Option<String>,
}

// ============================================
// ERROR TYPES
// ============================================

#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    #[error("Token limit exceeded: {0}/{1} tokens used")]
    TokenLimitExceeded(i64, i64),
    
    #[error("Model pricing not found: {0}")]
    ModelPricingNotFound(String),
    
    #[error("User not found: {0}")]
    UserNotFound(Uuid),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
