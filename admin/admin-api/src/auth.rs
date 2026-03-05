use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
}

/// Customer JWT claims (sub = user_id)
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomerClaims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
}

pub async fn require_auth(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, Response> {
    let config = &state.config;
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header").into_response());
        }
    };

    let _token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.admin_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response())?;

    Ok(next.run(request).await)
}
