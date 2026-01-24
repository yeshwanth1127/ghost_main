use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct License {
    pub id: Uuid,
    pub license_key: String,
    pub user_id: Option<Uuid>,
    pub status: String, // active, suspended, expired
    pub tier: String,   // trial, free, basic, pro, enterprise
    pub max_instances: i32,
    pub is_trial: bool,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LicenseInstance {
    pub id: Uuid,
    pub license_id: Uuid,
    pub instance_name: String,
    pub machine_id: String,
    pub app_version: Option<String>,
    pub last_validated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageLog {
    pub id: Uuid,
    pub license_id: Uuid,
    pub endpoint: Option<String>,
    pub model_used: Option<String>,
    pub tokens_used: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationRequest {
    pub license_key: String,
    pub instance_name: String,
    pub machine_id: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationResponse {
    pub activated: bool,
    pub error: Option<String>,
    pub license_key: Option<String>,
    pub instance: Option<InstanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub is_active: bool,
    pub last_validated_at: Option<String>,
}
