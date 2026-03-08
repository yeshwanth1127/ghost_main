use axum::{
    extract::{Request, State},
    http::StatusCode,
    Json,
};
use axum::body::to_bytes;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::Row;
use uuid::Uuid;

use crate::state::AppState;

const RAZORPAY_API: &str = "https://api.razorpay.com/v1";

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub plan: String, // starter, pro, power
    pub email: String,
    pub user_id: Option<String>, // if signed in
    pub license_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSubscriptionResponse {
    pub subscription_id: String,
    pub key_id: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPaymentRequest {
    pub razorpay_payment_id: String,
    pub razorpay_subscription_id: String,
    pub razorpay_signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyPaymentResponse {
    pub success: bool,
    pub license_key: Option<String>,
    pub plan: Option<String>,
    pub message: String,
}

fn get_plan_id<'a>(config: &'a crate::config::Config, plan: &str) -> Option<&'a str> {
    match plan {
        "starter" => Some(config.razorpay_plan_starter.as_str()),
        "pro" => Some(config.razorpay_plan_pro.as_str()),
        "power" => Some(config.razorpay_plan_power.as_str()),
        _ => None,
    }
}

fn get_token_limit(plan: &str) -> i64 {
    match plan {
        "starter" => 500_000,
        "pro" => 1_000_000,
        "power" => 2_000_000,
        _ => 5_000,
    }
}

pub async fn create_subscription(
    State(state): State<AppState>,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<Json<CreateSubscriptionResponse>, (axum::http::StatusCode, String)> {
    if state.config.razorpay_key_id.is_empty() {
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Razorpay not configured".to_string()));
    }

    let plan_id = get_plan_id(&state.config, &req.plan)
        .filter(|s| !s.is_empty())
        .ok_or((axum::http::StatusCode::BAD_REQUEST, "Invalid plan".to_string()))?;

    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".to_string()));
    }

    let (user_id, email_for_notes) = if let Some(uid) = &req.user_id {
        (uid.clone(), email.clone())
    } else if let Some(lk) = &req.license_key {
        let row = sqlx::query("SELECT u.id, u.email FROM users u JOIN licenses l ON l.user_id = u.id WHERE l.license_key = $1")
            .bind(lk)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if let Some(r) = row {
            let id: Uuid = r.get("id");
            let em: String = r.get("email");
            (id.to_string(), em)
        } else {
            (Uuid::new_v4().to_string(), email.clone())
        }
    } else {
        let row = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if let Some(id) = row {
            (id.to_string(), email.clone())
        } else {
            (Uuid::new_v4().to_string(), email.clone())
        }
    };

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "plan_id": plan_id,
        "total_count": 12,
        "quantity": 1,
        "customer_notify": true,
        "notes": {
            "email": email_for_notes,
            "user_id": user_id
        }
    });

    let res = client
        .post(format!("{}/subscriptions", RAZORPAY_API))
        .basic_auth(&state.config.razorpay_key_id, Some(&state.config.razorpay_key_secret))
        .json(&body)
        .send()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;

    let status = res.status();
    let text = res.text().await.map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;

    if !status.is_success() {
        return Err((axum::http::StatusCode::BAD_GATEWAY, format!("Razorpay error: {}", text)));
    }

    let sub: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let subscription_id = sub["id"]
        .as_str()
        .ok_or((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No subscription id".to_string()))?
        .to_string();

    Ok(Json(CreateSubscriptionResponse {
        subscription_id,
        key_id: state.config.razorpay_key_id.clone(),
    }))
}

fn verify_razorpay_signature(secret: &str, payment_id: &str, subscription_id: &str, signature: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;
    let payload = format!("{}|{}", payment_id, subscription_id);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());
    expected == signature
}

pub async fn verify_payment(
    State(state): State<AppState>,
    Json(req): Json<VerifyPaymentRequest>,
) -> Result<Json<VerifyPaymentResponse>, (axum::http::StatusCode, String)> {
    if state.config.razorpay_key_secret.is_empty() {
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Razorpay not configured".to_string()));
    }

    if !verify_razorpay_signature(
        &state.config.razorpay_key_secret,
        &req.razorpay_payment_id,
        &req.razorpay_subscription_id,
        &req.razorpay_signature,
    ) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid signature".to_string()));
    }

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/subscriptions/{}", RAZORPAY_API, req.razorpay_subscription_id))
        .basic_auth(&state.config.razorpay_key_id, Some(&state.config.razorpay_key_secret))
        .send()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;

    let sub: serde_json::Value = res
        .json()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;

    let plan_id = sub["plan_id"].as_str().unwrap_or("");
    let notes = sub.get("notes").and_then(|n| n.as_object());
    let email = notes
        .and_then(|n| n.get("email"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let user_id_str = notes
        .and_then(|n| n.get("user_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let plan = if state.config.razorpay_plan_starter == plan_id {
        "starter"
    } else if state.config.razorpay_plan_pro == plan_id {
        "pro"
    } else if state.config.razorpay_plan_power == plan_id {
        "power"
    } else {
        "starter"
    };

    let token_limit = get_token_limit(plan);

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (user_id, license_key) = if !user_id_str.is_empty() {
        let uid = Uuid::parse_str(user_id_str).map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "Invalid user_id".to_string()))?;
        let is_owner: bool = sqlx::query_scalar("SELECT COALESCE(is_owner, false) FROM users WHERE id = $1")
            .bind(&uid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .unwrap_or(false);
        if is_owner {
            tx.rollback().await.ok();
            return Err((axum::http::StatusCode::BAD_REQUEST, "Cannot process payment for owner account".to_string()));
        }
        let lk: Option<String> = sqlx::query_scalar("SELECT license_key FROM licenses WHERE user_id = $1 AND COALESCE(is_owner, false) = false LIMIT 1")
            .bind(&uid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query(
            "UPDATE users SET plan = $1, monthly_token_limit = $2, razorpay_subscription_id = $3, updated_at = NOW() WHERE id = $4 AND COALESCE(is_owner, false) = false",
        )
        .bind(plan)
        .bind(token_limit)
        .bind(&req.razorpay_subscription_id)
        .bind(&uid)
        .execute(&mut *tx)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query(
            "UPDATE licenses SET tier = $1, is_trial = false, trial_ends_at = NULL, updated_at = NOW() WHERE user_id = $2 AND COALESCE(is_owner, false) = false",
        )
        .bind(plan)
        .bind(&uid)
        .execute(&mut *tx)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let lk = if let Some(k) = lk {
            k
        } else {
            let new_key = crate::routes::auth::generate_license_key();
            let license_id = Uuid::new_v4();
            let now = chrono::Utc::now();
            sqlx::query(
                "INSERT INTO licenses (id, license_key, user_id, status, tier, max_instances, is_trial, created_at, updated_at)
                 VALUES ($1, $2, $3, 'active', $4, 1, false, $5, $5)",
            )
            .bind(&license_id)
            .bind(&new_key)
            .bind(&uid)
            .bind(plan)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            new_key
        };

        (uid, lk)
    } else if !email.is_empty() {
        let existing = sqlx::query(
            "SELECT u.id FROM users u WHERE u.email = $1",
        )
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(row) = existing {
            let uid: Uuid = row.get("id");
            let is_owner: bool = sqlx::query_scalar("SELECT COALESCE(is_owner, false) FROM users WHERE id = $1")
                .bind(&uid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .unwrap_or(false);
            if is_owner {
                tx.rollback().await.ok();
                return Err((axum::http::StatusCode::BAD_REQUEST, "Cannot process payment for owner account".to_string()));
            }
            let lk: Option<String> = sqlx::query_scalar("SELECT license_key FROM licenses WHERE user_id = $1 AND COALESCE(is_owner, false) = false LIMIT 1")
                .bind(&uid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            sqlx::query(
                "UPDATE users SET plan = $1, monthly_token_limit = $2, razorpay_subscription_id = $3, updated_at = NOW() WHERE id = $4 AND COALESCE(is_owner, false) = false",
            )
            .bind(plan)
            .bind(token_limit)
            .bind(&req.razorpay_subscription_id)
            .bind(&uid)
            .execute(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            sqlx::query(
                "UPDATE licenses SET tier = $1, is_trial = false, trial_ends_at = NULL, updated_at = NOW() WHERE user_id = $2 AND COALESCE(is_owner, false) = false",
            )
            .bind(plan)
            .bind(&uid)
            .execute(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let license_key = if let Some(k) = lk {
                k
            } else {
                let new_key = crate::routes::auth::generate_license_key();
                let license_id = Uuid::new_v4();
                let now = chrono::Utc::now();
                sqlx::query(
                    "INSERT INTO licenses (id, license_key, user_id, status, tier, max_instances, is_trial, created_at, updated_at)
                     VALUES ($1, $2, $3, 'active', $4, 1, false, $5, $5)",
                )
                .bind(&license_id)
                .bind(&new_key)
                .bind(&uid)
                .bind(plan)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                new_key
            };
            (uid, license_key)
        } else {
            let user_id = Uuid::new_v4();
            let license_key = crate::routes::auth::generate_license_key();
            let license_id = Uuid::new_v4();
            let now = chrono::Utc::now();

            sqlx::query(
                "INSERT INTO users (id, email, plan, monthly_token_limit, tokens_used_this_month, monthly_reset_at, razorpay_subscription_id, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $7)",
            )
            .bind(&user_id)
            .bind(&email)
            .bind(plan)
            .bind(token_limit)
            .bind(now + chrono::Duration::days(30))
            .bind(&req.razorpay_subscription_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            sqlx::query(
                "INSERT INTO licenses (id, license_key, user_id, status, tier, max_instances, is_trial, created_at, updated_at)
                 VALUES ($1, $2, $3, 'active', $4, 1, false, $5, $5)",
            )
            .bind(&license_id)
            .bind(&license_key)
            .bind(&user_id)
            .bind(plan)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            (user_id, license_key)
        }
    } else {
        tx.rollback().await.ok();
        return Err((axum::http::StatusCode::BAD_REQUEST, "No email in subscription notes".to_string()));
    };

    let license_id: Uuid = sqlx::query_scalar("SELECT id FROM licenses WHERE user_id = $1 LIMIT 1")
        .bind(&user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query(
        "INSERT INTO transactions (license_id, amount, currency, status, payment_provider, provider_transaction_id, created_at)
         VALUES ($1, 0, 'INR', 'captured', 'razorpay', $2, NOW())",
    )
    .bind(&license_id)
    .bind(&req.razorpay_payment_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit().await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(VerifyPaymentResponse {
        success: true,
        license_key: Some(license_key),
        plan: Some(plan.to_string()),
        message: "Payment verified. Your plan has been upgraded.".to_string(),
    }))
}

/// Razorpay webhook payload structure
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub entity: String,
    pub event: String,
    pub payload: WebhookPayloadData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayloadData {
    pub subscription: Option<WebhookSubscription>,
    pub payment: Option<WebhookPayment>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookSubscription {
    pub entity: WebhookSubscriptionEntity,
}

#[derive(Debug, Deserialize)]
pub struct WebhookSubscriptionEntity {
    pub id: String,
    pub plan_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayment {
    pub entity: WebhookPaymentEntity,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPaymentEntity {
    pub id: String,
    pub amount: Option<i64>,
    pub currency: Option<String>,
    pub status: Option<String>,
}

fn verify_webhook_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    if secret.is_empty() {
        return false;
    }
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());
    expected == signature
}

pub async fn webhook(
    State(state): State<AppState>,
    request: Request,
) -> Result<StatusCode, (StatusCode, String)> {
    if state.config.razorpay_webhook_secret.is_empty() {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "Webhook not configured".to_string()));
    }

    let (parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let signature = parts
        .headers
        .get("X-Razorpay-Signature")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !verify_webhook_signature(
        &state.config.razorpay_webhook_secret,
        &body_bytes,
        signature,
    ) {
        return Err((StatusCode::BAD_REQUEST, "Invalid signature".to_string()));
    }

    let payload: WebhookPayload = serde_json::from_slice(&body_bytes)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    match payload.event.as_str() {
        "subscription.charged" => {
            if let (Some(sub), Some(pay)) = (payload.payload.subscription, payload.payload.payment) {
                let sub_id = sub.entity.id;
                let payment_id = pay.entity.id;
                let amount = pay.entity.amount.unwrap_or(0) as f64 / 100.0;

                let row = sqlx::query(
                    "SELECT l.id, l.user_id FROM licenses l
                     JOIN users u ON u.id = l.user_id
                     WHERE u.razorpay_subscription_id = $1 AND COALESCE(u.is_owner, false) = false",
                )
                .bind(&sub_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                if let Some(r) = row {
                    let license_id: Uuid = r.get("id");
                    sqlx::query(
                        "INSERT INTO transactions (license_id, amount, currency, status, payment_provider, provider_transaction_id, created_at)
                         VALUES ($1, $2, 'INR', 'captured', 'razorpay', $3, NOW())",
                    )
                    .bind(&license_id)
                    .bind(amount)
                    .bind(&payment_id)
                    .execute(&state.pool)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                }
            }
        }
        "subscription.cancelled" | "subscription.completed" | "subscription.halted" => {
            if let Some(sub) = payload.payload.subscription {
                let sub_id = sub.entity.id;
                let _ = sqlx::query(
                    "UPDATE licenses SET tier = 'free', is_trial = true, trial_ends_at = NOW() + INTERVAL '14 days', updated_at = NOW()
                     WHERE user_id IN (SELECT id FROM users WHERE razorpay_subscription_id = $1 AND COALESCE(is_owner, false) = false)",
                )
                .bind(&sub_id)
                .execute(&state.pool)
                .await;
                let _ = sqlx::query(
                    "UPDATE users SET plan = 'free', monthly_token_limit = 5000, razorpay_subscription_id = NULL, updated_at = NOW()
                     WHERE razorpay_subscription_id = $1 AND COALESCE(is_owner, false) = false",
                )
                .bind(&sub_id)
                .execute(&state.pool)
                .await;
            }
        }
        "subscription.activated" => {
            // Subscription is active; no action needed (initial auth already handled by verify)
        }
        _ => {
            // Ignore other events
        }
    }

    Ok(StatusCode::OK)
}
