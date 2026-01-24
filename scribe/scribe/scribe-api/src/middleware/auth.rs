use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use tracing::{error, info};

pub struct LicenseInfo {
    pub license_key: String,
    pub machine_id: String,
    pub instance_id: String,
}

pub async fn auth_middleware(
    headers: HeaderMap,
    State(pool): State<PgPool>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract required headers
    let license_key = headers
        .get("license_key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing license_key header");
            StatusCode::UNAUTHORIZED
        })?;

    let machine_id = headers
        .get("machine_id")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing machine_id header");
            StatusCode::UNAUTHORIZED
        })?;

    let instance_id = headers
        .get("instance")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Validate license exists and is active
    let license_result = sqlx::query_as::<_, crate::models::License>(
        "SELECT * FROM licenses WHERE license_key = $1 AND status = 'active'",
    )
    .bind(license_key)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        error!("Database error during license validation: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let license = license_result.ok_or_else(|| {
        error!("Invalid or inactive license key");
        StatusCode::UNAUTHORIZED
    })?;

    // Check if license has expired
    if let Some(expires_at) = license.expires_at {
        if expires_at < chrono::Utc::now() {
            error!("License has expired");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    
    // Check if trial has expired (if it's a trial license)
    // Note: We'll need to fetch this from the license record
    // For now, this placeholder will be implemented in the full validation

    // Check instance limit
    let instance_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM license_instances WHERE license_id = $1",
    )
    .bind(license.id)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if instance_count >= license.max_instances as i64 {
        error!("Instance limit reached");
        return Err(StatusCode::FORBIDDEN);
    }

    // Insert or update license instance
    if !instance_id.is_empty() {
        let _ = sqlx::query(
            "INSERT INTO license_instances (license_id, instance_name, machine_id, app_version) 
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (license_id, machine_id) 
             DO UPDATE SET last_validated_at = NOW()",
        )
        .bind(license.id)
        .bind(instance_id)
        .bind(machine_id)
        .execute(&pool)
        .await;
    }

    info!("License validated successfully for {}", license_key);

    Ok(next.run(request).await)
}

// Auth middleware is applied manually in routes that need it
