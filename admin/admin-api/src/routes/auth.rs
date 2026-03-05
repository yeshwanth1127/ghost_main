use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::auth::Claims;
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
