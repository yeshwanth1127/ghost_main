use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::create_customer_token;
use crate::routes::auth::generate_license_key;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SendOtpRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct SendOtpResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub email: String,
    pub otp: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyOtpResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_ends_at: Option<String>,
}

fn hash_otp(otp: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(otp.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn send_otp(
    State(state): State<AppState>,
    Json(req): Json<SendOtpRequest>,
) -> Result<Json<SendOtpResponse>, (axum::http::StatusCode, Json<SendOtpResponse>)> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(SendOtpResponse {
                success: false,
                message: "Invalid email address".to_string(),
            }),
        ));
    }

    // Check if user already exists with a license
    if let Ok(Some(_)) = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM users u 
         JOIN licenses l ON l.user_id = u.id 
         WHERE u.email = $1 AND l.status = 'active'",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    {
        return Err((
            axum::http::StatusCode::CONFLICT,
            Json(SendOtpResponse {
                success: false,
                message: "An account with this email already exists. Please sign in.".to_string(),
            }),
        ));
    }

    let otp: String = (0..6).map(|_| rand::random::<u8>() % 10).map(|d| (b'0' + d) as char).collect();
    let otp_hash = hash_otp(&otp);
    let expires_at = Utc::now() + Duration::minutes(10);

    // Delete any existing OTP for this email
    let _ = sqlx::query("DELETE FROM otp_verifications WHERE email = $1")
        .bind(&email)
        .execute(&state.pool)
        .await;

    sqlx::query(
        "INSERT INTO otp_verifications (id, email, otp_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&otp_hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store OTP: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(SendOtpResponse {
                success: false,
                message: "Failed to send verification code".to_string(),
            }),
        )
    })?;

    let body = format!(
        "Your Ghost verification code is: {}\n\nThis code expires in 10 minutes.\n\nIf you didn't request this, you can ignore this email.\n\n— Ghost by Exora",
        otp
    );

    let send_err: Option<String> = if !state.config.smtp_username.is_empty() && !state.config.smtp_password.is_empty() {
        // SMTP (Hostinger, etc.)
        let message = MessageBuilder::new()
            .from(("Ghost", state.config.smtp_from_email.as_str()))
            .to(vec![(email.as_str(), email.as_str())])
            .subject("Your Ghost verification code")
            .text_body(body.as_str());
        let smtp = SmtpClientBuilder::new(
            state.config.smtp_host.as_str(),
            state.config.smtp_port,
        )
        .implicit_tls(false)
        .credentials((state.config.smtp_username.as_str(), state.config.smtp_password.as_str()));
        match smtp.connect().await {
            Ok(mut client) => client.send(message).await.err().map(|e| e.to_string()),
            Err(e) => Some(e.to_string()),
        }
    } else if !state.config.resend_api_key.is_empty() {
        // Resend API
        let payload = serde_json::json!({
            "from": format!("Ghost <{}>", state.config.smtp_from_email),
            "to": [email],
            "subject": "Your Ghost verification code",
            "text": body,
        });
        let client = reqwest::Client::new();
        let res = client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", state.config.resend_api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => None,
            Ok(r) => {
                let status = r.status();
                let err_body = r.text().await.unwrap_or_default();
                tracing::error!("Resend API error: {} - {}", status, err_body);
                Some(format!("Resend error: {}", status))
            }
            Err(e) => Some(e.to_string()),
        }
    } else {
        tracing::info!("[DEV] OTP for {}: {}", email, otp);
        None
    };

    if let Some(e) = send_err {
        tracing::error!("Failed to send OTP email: {}", e);
        let _ = sqlx::query("DELETE FROM otp_verifications WHERE email = $1")
            .bind(&email)
            .execute(&state.pool)
            .await;
        return Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(SendOtpResponse {
                success: false,
                message: "Failed to send verification code. Please try again.".to_string(),
            }),
        ));
    }

    Ok(Json(SendOtpResponse {
        success: true,
        message: "Verification code sent to your email".to_string(),
    }))
}

/// Send OTP for login (only when user already exists with a license)
pub async fn send_login_otp(
    State(state): State<AppState>,
    Json(req): Json<SendOtpRequest>,
) -> Result<Json<SendOtpResponse>, (axum::http::StatusCode, Json<SendOtpResponse>)> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(SendOtpResponse {
                success: false,
                message: "Invalid email address".to_string(),
            }),
        ));
    }

    // Require user to exist with a license (opposite of trial send_otp)
    if let Ok(None) = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM users u 
         JOIN licenses l ON l.user_id = u.id 
         WHERE u.email = $1 AND l.status = 'active'",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(SendOtpResponse {
                success: false,
                message: "No account found with this email. Start a free trial first.".to_string(),
            }),
        ));
    }

    // Same OTP logic as send_otp
    let otp: String = (0..6).map(|_| rand::random::<u8>() % 10).map(|d| (b'0' + d) as char).collect();
    let otp_hash = hash_otp(&otp);
    let expires_at = Utc::now() + Duration::minutes(10);

    let _ = sqlx::query("DELETE FROM otp_verifications WHERE email = $1")
        .bind(&email)
        .execute(&state.pool)
        .await;

    sqlx::query(
        "INSERT INTO otp_verifications (id, email, otp_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&otp_hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store OTP: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(SendOtpResponse {
                success: false,
                message: "Failed to send verification code".to_string(),
            }),
        )
    })?;

    let body = format!(
        "Your Ghost verification code is: {}\n\nThis code expires in 10 minutes.\n\nIf you didn't request this, you can ignore this email.\n\n— Ghost by Exora",
        otp
    );

    let send_err: Option<String> = if !state.config.smtp_username.is_empty() && !state.config.smtp_password.is_empty() {
        let message = MessageBuilder::new()
            .from(("Ghost", state.config.smtp_from_email.as_str()))
            .to(vec![(email.as_str(), email.as_str())])
            .subject("Your Ghost verification code")
            .text_body(body.as_str());
        let smtp = SmtpClientBuilder::new(
            state.config.smtp_host.as_str(),
            state.config.smtp_port,
        )
        .implicit_tls(false)
        .credentials((state.config.smtp_username.as_str(), state.config.smtp_password.as_str()));
        match smtp.connect().await {
            Ok(mut client) => client.send(message).await.err().map(|e| e.to_string()),
            Err(e) => Some(e.to_string()),
        }
    } else if !state.config.resend_api_key.is_empty() {
        let payload = serde_json::json!({
            "from": format!("Ghost <{}>", state.config.smtp_from_email),
            "to": [email],
            "subject": "Your Ghost verification code",
            "text": body,
        });
        let client = reqwest::Client::new();
        let res = client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", state.config.resend_api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => None,
            Ok(r) => {
                let status = r.status();
                let err_body = r.text().await.unwrap_or_default();
                tracing::error!("Resend API error: {} - {}", status, err_body);
                Some(format!("Resend error: {}", status))
            }
            Err(e) => Some(e.to_string()),
        }
    } else {
        tracing::info!("[DEV] Login OTP for {}: {}", email, otp);
        None
    };

    if let Some(e) = send_err {
        tracing::error!("Failed to send OTP email: {}", e);
        let _ = sqlx::query("DELETE FROM otp_verifications WHERE email = $1")
            .bind(&email)
            .execute(&state.pool)
            .await;
        return Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(SendOtpResponse {
                success: false,
                message: "Failed to send verification code. Please try again.".to_string(),
            }),
        ));
    }

    Ok(Json(SendOtpResponse {
        success: true,
        message: "Verification code sent to your email".to_string(),
    }))
}

pub async fn verify_otp(
    State(state): State<AppState>,
    Json(req): Json<VerifyOtpRequest>,
) -> Result<Json<VerifyOtpResponse>, (axum::http::StatusCode, Json<VerifyOtpResponse>)> {
    let email = req.email.trim().to_lowercase();
    let otp = req.otp.trim();

    if email.is_empty() || !email.contains('@') {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(VerifyOtpResponse {
                success: false,
                message: "Invalid email address".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        ));
    }

    if otp.len() != 6 || !otp.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(VerifyOtpResponse {
                success: false,
                message: "Invalid verification code".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        ));
    }

    let otp_hash = hash_otp(otp);

    let row = sqlx::query(
        "SELECT id, expires_at FROM otp_verifications WHERE email = $1 AND otp_hash = $2",
    )
    .bind(&email)
    .bind(&otp_hash)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error verifying OTP: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyOtpResponse {
                success: false,
                message: "Verification failed".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        )
    })?;

    let row = row.ok_or((
        axum::http::StatusCode::UNAUTHORIZED,
        Json(VerifyOtpResponse {
            success: false,
            message: "Invalid or expired verification code".to_string(),
            token: None,
            user_id: None,
            email: None,
            license_key: None,
            plan: None,
            trial_ends_at: None,
        }),
    ))?;

    let expires_at: chrono::DateTime<Utc> = row.get("expires_at");
    if expires_at < Utc::now() {
        let _ = sqlx::query("DELETE FROM otp_verifications WHERE email = $1")
            .bind(&email)
            .execute(&state.pool)
            .await;
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(VerifyOtpResponse {
                success: false,
                message: "Verification code has expired. Please request a new one.".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        ));
    }

    // Delete used OTP
    let _ = sqlx::query("DELETE FROM otp_verifications WHERE email = $1")
        .bind(&email)
        .execute(&state.pool)
        .await;

    // Check if user already exists (login flow)
    let existing = sqlx::query(
        "SELECT u.id, u.email, u.plan, l.license_key, l.trial_ends_at
         FROM users u
         JOIN licenses l ON l.user_id = u.id
         WHERE u.email = $1 AND l.status = 'active'
         LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyOtpResponse {
                success: false,
                message: "Verification failed".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        )
    })?;

    if let Some(row) = existing {
        let user_id: Uuid = row.get("id");
        let user_email: String = row.get("email");
        let plan: String = row.get::<Option<String>, _>("plan").unwrap_or_else(|| "free".to_string());
        let license_key: String = row.get::<Option<String>, _>("license_key").unwrap_or_default();
        let trial_ends_at: Option<chrono::DateTime<Utc>> = row.get("trial_ends_at");

        let token = create_customer_token(&user_id.to_string(), &user_email, &state.config.admin_secret)
            .map_err(|e| {
                tracing::error!("Token error: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(VerifyOtpResponse {
                        success: false,
                        message: "Login failed".to_string(),
                        token: None,
                        user_id: None,
                        email: None,
                        license_key: None,
                        plan: None,
                        trial_ends_at: None,
                    }),
                )
            })?;

        tracing::info!("User logged in via OTP: {}", user_email);

        return Ok(Json(VerifyOtpResponse {
            success: true,
            message: "Login successful".to_string(),
            token: Some(token),
            user_id: Some(user_id.to_string()),
            email: Some(user_email),
            license_key: Some(license_key),
            plan: Some(plan),
            trial_ends_at: trial_ends_at.map(|t| t.to_rfc3339()),
        }));
    }

    // New user: create user + license with trial
    let user_id = Uuid::new_v4();
    let license_key = generate_license_key();
    let now = Utc::now();
    let trial_ends_at = now + Duration::days(14);
    let monthly_reset_at = now + Duration::days(30);

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| {
            tracing::error!("Failed to start transaction: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyOtpResponse {
                    success: false,
                    message: "Failed to create account".to_string(),
                    token: None,
                    user_id: None,
                    email: None,
                    license_key: None,
                    plan: None,
                    trial_ends_at: None,
                }),
            )
        })?;

    sqlx::query(
        "INSERT INTO users (id, email, plan, monthly_token_limit, tokens_used_this_month, monthly_reset_at, created_at, updated_at)
         VALUES ($1, $2, 'free', 5000, 0, $3, $4, $4)",
    )
    .bind(&user_id)
    .bind(&email)
    .bind(monthly_reset_at)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create user: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyOtpResponse {
                success: false,
                message: "Failed to create account".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        )
    })?;

    sqlx::query(
        "INSERT INTO licenses (id, license_key, user_id, status, tier, max_instances, is_trial, trial_ends_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', 'free', 1, true, $4, $5, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(&license_key)
    .bind(&user_id)
    .bind(trial_ends_at)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create license: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyOtpResponse {
                success: false,
                message: "Failed to create account".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        )
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyOtpResponse {
                success: false,
                message: "Failed to create account".to_string(),
                token: None,
                user_id: None,
                email: None,
                license_key: None,
                plan: None,
                trial_ends_at: None,
            }),
        )
    })?;

    let token = create_customer_token(&user_id.to_string(), &email, &state.config.admin_secret)
        .map_err(|e| {
            tracing::error!("Token error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyOtpResponse {
                    success: false,
                    message: "Account created but login failed".to_string(),
                    token: None,
                    user_id: None,
                    email: None,
                    license_key: None,
                    plan: None,
                    trial_ends_at: None,
                }),
            )
        })?;

    tracing::info!("Free trial created: {} with license {}", email, license_key);

    Ok(Json(VerifyOtpResponse {
        success: true,
        message: "Free trial activated! Use your email in the Ghost app to get started.".to_string(),
        token: Some(token),
        user_id: Some(user_id.to_string()),
        email: Some(email),
        license_key: Some(license_key),
        plan: Some("free".to_string()),
        trial_ends_at: Some(trial_ends_at.to_rfc3339()),
    }))
}
