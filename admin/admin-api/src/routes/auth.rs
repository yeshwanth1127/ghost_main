use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::{Claims, CustomerClaims};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomerLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerLoginResponse {
    pub token: String,
    pub user_id: String,
    pub email: String,
    pub license_key: String,
    pub plan: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub email: String,
    pub license_key: String,
    pub plan: String,
    pub trial_ends_at: String,
    pub message: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, &'static str)> {
    let row = sqlx::query(
        "SELECT username, password_hash FROM admin_users WHERE username = $1",
    )
    .bind(&req.username)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let row = row.ok_or((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials"))?;

    let password_hash: String = row.get("password_hash");

    let valid = bcrypt::verify(&req.password, &password_hash)
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Auth error"))?;

    if !valid {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials"));
    }

    let username: String = row.get("username");
    let exp = chrono::Utc::now() + chrono::Duration::days(7);
    let claims = Claims {
        sub: username,
        exp: exp.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.admin_secret.as_bytes()),
    )
    .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Token error"))?;

    Ok(Json(LoginResponse { token }))
}

pub async fn customer_login(
    State(state): State<AppState>,
    Json(req): Json<CustomerLoginRequest>,
) -> Result<Json<CustomerLoginResponse>, (axum::http::StatusCode, String)> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".to_string()));
    }

    let row = sqlx::query(
        "SELECT u.id, u.email, u.plan, u.password_hash, l.license_key 
         FROM users u 
         LEFT JOIN licenses l ON u.id = l.user_id 
         WHERE u.email = $1 
         LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let row = row.ok_or((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    let password_hash: Option<String> = row.get("password_hash");
    let password_hash = password_hash.ok_or((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    let valid = bcrypt::verify(&req.password, &password_hash)
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Auth error".to_string()))?;

    if !valid {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    let user_id: Uuid = row.get("id");
    let email: String = row.get("email");
    let plan: String = row.get::<Option<String>, _>("plan").unwrap_or_else(|| "free".to_string());
    let license_key: String = row.get::<Option<String>, _>("license_key").unwrap_or_default();

    let exp = chrono::Utc::now() + chrono::Duration::days(7);
    let claims = CustomerClaims {
        sub: user_id.to_string(),
        email: email.clone(),
        exp: exp.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.admin_secret.as_bytes()),
    )
    .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Token error".to_string()))?;

    Ok(Json(CustomerLoginResponse {
        token,
        user_id: user_id.to_string(),
        email,
        license_key,
        plan,
    }))
}

pub fn generate_license_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let part1: String = (0..8).map(|_| chars[rng.gen_range(0..chars.len())] as char).collect();
    let part2: String = (0..8).map(|_| chars[rng.gen_range(0..chars.len())] as char).collect();
    format!("GHOST-{}-{}", part1, part2)
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (axum::http::StatusCode, String)> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".to_string()));
    }
    if req.password.len() < 6 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Password must be at least 6 characters".to_string()));
    }

    if sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((axum::http::StatusCode::CONFLICT, "User with this email already exists".to_string()));
    }

    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user_id = Uuid::new_v4();
    let license_key = generate_license_key();
    let now = chrono::Utc::now();
    let trial_ends_at = now + chrono::Duration::days(14);
    let monthly_reset_at = now + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, plan, monthly_token_limit, tokens_used_this_month, monthly_reset_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'free', 5000, 0, $4, $5, $5)",
    )
    .bind(&user_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(monthly_reset_at)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query(
        "INSERT INTO licenses (id, license_key, user_id, status, tier, max_instances, is_trial, trial_ends_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', 'free', 1, true, $4, $5, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(&license_key)
    .bind(&user_id)
    .bind(trial_ends_at)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RegisterResponse {
        user_id: user_id.to_string(),
        email,
        license_key,
        plan: "free".to_string(),
        trial_ends_at: trial_ends_at.to_rfc3339(),
        message: "Account created with 14-day free trial".to_string(),
    }))
}
