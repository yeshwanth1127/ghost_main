use axum::extract::State;
use serde::Serialize;
use sqlx::FromRow;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct GlobalStats {
    pub total_users: i64,
    pub total_tokens: i64,
    pub total_cost_usd: rust_decimal::Decimal,
    pub total_revenue: rust_decimal::Decimal,
}

pub async fn global_stats(
    State(state): State<AppState>,
) -> Result<axum::Json<GlobalStats>, (axum::http::StatusCode, String)> {
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM users")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (total_tokens, total_cost_usd): (Option<i64>, Option<rust_decimal::Decimal>) =
        sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(total_tokens), 0)::bigint,
                COALESCE(SUM(cost_usd), 0)
            FROM messages
            "#,
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_revenue = rust_decimal::Decimal::from(10) * rust_decimal::Decimal::from(total_users);

    Ok(axum::Json(GlobalStats {
        total_users,
        total_tokens: total_tokens.unwrap_or(0),
        total_cost_usd: total_cost_usd.unwrap_or(rust_decimal::Decimal::ZERO),
        total_revenue,
    }))
}

#[derive(Debug, Serialize, FromRow)]
pub struct ModelBreakdownRow {
    pub model: String,
    pub provider: String,
    pub tokens: i64,
    pub cost_usd: rust_decimal::Decimal,
    pub requests: i64,
}

pub async fn model_breakdown(
    State(state): State<AppState>,
) -> Result<axum::Json<Vec<ModelBreakdownRow>>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query_as::<_, ModelBreakdownRow>(
        r#"
        SELECT
            model,
            provider,
            COALESCE(SUM(total_tokens), 0)::bigint as tokens,
            COALESCE(SUM(cost_usd), 0) as cost_usd,
            COUNT(*)::bigint as requests
        FROM messages
        GROUP BY model, provider
        ORDER BY tokens DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(axum::Json(rows))
}

#[derive(Debug, Serialize, FromRow)]
pub struct TopUserRow {
    pub email: Option<String>,
    pub tokens: i64,
    pub cost_usd: rust_decimal::Decimal,
}

pub async fn top_users(
    State(state): State<AppState>,
) -> Result<axum::Json<Vec<TopUserRow>>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query_as::<_, TopUserRow>(
        r#"
        SELECT
            u.email,
            COALESCE(SUM(m.total_tokens), 0)::bigint as tokens,
            COALESCE(SUM(m.cost_usd), 0) as cost_usd
        FROM users u
        LEFT JOIN messages m ON m.user_id = u.id
        GROUP BY u.id, u.email
        ORDER BY tokens DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(axum::Json(rows))
}

#[derive(Debug, Serialize, FromRow)]
pub struct RecentMessageRow {
    pub user_id: uuid::Uuid,
    pub email: Option<String>,
    pub model: String,
    pub provider: String,
    pub total_tokens: i32,
    pub cost_usd: rust_decimal::Decimal,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn recent_messages(
    State(state): State<AppState>,
) -> Result<axum::Json<Vec<RecentMessageRow>>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query_as::<_, RecentMessageRow>(
        r#"
        SELECT
            m.user_id,
            u.email,
            m.model,
            m.provider,
            m.total_tokens,
            m.cost_usd,
            m.created_at
        FROM messages m
        LEFT JOIN users u ON u.id = m.user_id
        ORDER BY m.created_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(axum::Json(rows))
}
