use axum::{
    extract::{Json, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::UsageRecord;
use crate::services::AppState;

// ============================================
// REQUEST PARAMS
// ============================================

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    #[serde(default = "default_limit")]
    limit: i32,
}

fn default_limit() -> i32 {
    50
}

#[derive(Debug, Deserialize)]
pub struct RecordUsageRequest {
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

// ============================================
// ENDPOINTS
// ============================================

/// GET /api/v1/usage/:user_id
/// Get current month's usage statistics for a user
pub async fn get_usage_stats(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    tracing::info!("📊 Fetching usage stats for user: {}", user_id);

    match state.usage_service.get_user_usage(user_id).await {
        Ok(stats) => {
            tracing::info!("✅ Usage stats retrieved: {} tokens used", stats.tokens_used);
            (StatusCode::OK, JsonResponse(stats)).into_response()
        }
        Err(e) => {
            tracing::error!("❌ Error fetching usage stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({
                    "error": format!("Failed to fetch usage stats: {}", e)
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/usage/:user_id/history?limit=50
/// Get recent message history for a user
pub async fn get_usage_history(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    tracing::info!(
        "📜 Fetching usage history for user: {} (limit: {})",
        user_id,
        params.limit
    );

    // Validate limit
    let limit = if params.limit > 100 {
        tracing::warn!("⚠️ Limiting history to 100 records (requested: {})", params.limit);
        100
    } else if params.limit < 1 {
        10
    } else {
        params.limit
    };

    match state
        .usage_service
        .get_usage_history(user_id, limit)
        .await
    {
        Ok(history) => {
            tracing::info!("✅ Retrieved {} history items", history.len());
            (StatusCode::OK, JsonResponse(history)).into_response()
        }
        Err(e) => {
            tracing::error!("❌ Error fetching usage history: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({
                    "error": format!("Failed to fetch usage history: {}", e)
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/usage/:user_id/limit-check
/// Check if user can make a request without exceeding limits
pub async fn check_token_limit(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    tracing::info!("🔍 Checking token limit for user: {}", user_id);

    match state.usage_service.check_token_limit(user_id, 0).await {
        Ok(limit_check) => {
            tracing::info!(
                "✅ Token limit check: {}/{} tokens used ({:.1}%)",
                limit_check.tokens_used,
                limit_check.token_limit,
                limit_check.percentage_used
            );
            (StatusCode::OK, JsonResponse(limit_check)).into_response()
        }
        Err(e) => {
            tracing::error!("❌ Error checking token limit: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({
                    "error": format!("Failed to check token limit: {}", e)
                })),
            )
                .into_response()
        }
    }
}

/// POST /api/v1/usage/record
/// Record usage from client (e.g. direct Ollama/Exora). Requires x-license-key header.
pub async fn record_usage_from_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RecordUsageRequest>,
) -> impl IntoResponse {
    let license_key = headers
        .get("x-license-key")
        .or_else(|| headers.get("license-key"))
        .or_else(|| headers.get("license_key"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let Some(ref key) = license_key else {
        return (
            StatusCode::UNAUTHORIZED,
            JsonResponse(serde_json::json!({"error": "Missing license key. Provide x-license-key header."})),
        )
            .into_response();
    };

    let user_id = match sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT user_id FROM licenses WHERE license_key = $1 AND status = 'active'"
    )
    .bind(key)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(Some(uid))) => uid,
        Ok(Some(None)) | Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                JsonResponse(serde_json::json!({"error": "Invalid or inactive license key."})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("❌ Error looking up license: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({"error": "Database error"})),
            )
                .into_response();
        }
    };

    let usage = UsageRecord {
        user_id,
        license_key: Some(key.clone()),
        model: body.model,
        provider: body.provider,
        prompt_tokens: body.prompt_tokens,
        completion_tokens: body.completion_tokens,
        conversation_id: None,
        request_duration_ms: None,
    };

    match state.usage_service.record_usage_from_client(usage).await {
        Ok(_) => {
            tracing::info!("✅ Usage recorded from client for user {}", user_id);
            (StatusCode::OK, JsonResponse(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => {
            tracing::error!("❌ Failed to record usage from client: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({"error": format!("Failed to record usage: {}", e)})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/usage/pricing
/// Get all active model pricing
pub async fn get_model_pricing(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("💰 Fetching model pricing");

    match state.usage_service.get_all_model_pricing().await {
        Ok(pricing) => {
            tracing::info!("✅ Retrieved {} pricing records", pricing.len());
            (StatusCode::OK, JsonResponse(pricing)).into_response()
        }
        Err(e) => {
            tracing::error!("❌ Error fetching model pricing: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({
                    "error": format!("Failed to fetch model pricing: {}", e)
                })),
            )
                .into_response()
        }
    }
}
