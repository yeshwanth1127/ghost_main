use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::models::{
    ActivationRequest, ActivationResponse, InstanceInfo, ValidateResponse,
};

use crate::services::AppState;

pub async fn activate(
    State(state): State<AppState>,
    Json(request): Json<ActivationRequest>,
) -> impl IntoResponse {
    // Validate license exists
    let license = match sqlx::query_as::<_, crate::models::License>(
        "SELECT * FROM licenses WHERE license_key = $1 AND status = 'active'",
    )
    .bind(&request.license_key)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(l)) => l,
        Ok(None) => {
            return Json(ActivationResponse {
                activated: false,
                error: Some("Invalid or inactive license key".to_string()),
                license_key: None,
                instance: None,
            })
            .into_response();
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ActivationResponse {
                activated: false,
                error: Some("Database error".to_string()),
                license_key: None,
                instance: None,
            })
            .into_response();
        }
    };

    // Create license instance
    let instance_id = Uuid::new_v4();
    match sqlx::query(
        "INSERT INTO license_instances (id, license_id, instance_name, machine_id, app_version) 
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (license_id, machine_id) DO NOTHING
         RETURNING id, instance_name, created_at",
    )
    .bind(instance_id)
    .bind(license.id)
    .bind(&request.instance_name)
    .bind(&request.machine_id)
    .bind(&request.app_version)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(row)) => {
            Json(ActivationResponse {
                activated: true,
                error: None,
                license_key: Some(request.license_key.clone()),
                instance: Some(InstanceInfo {
                    id: row.get::<Uuid, _>(0).to_string(),
                    name: row.get::<String, _>(1),
                    created_at: row.get::<chrono::DateTime<Utc>, _>(2).to_rfc3339(),
                }),
            }).into_response()
        }
        Ok(None) => {
            // Already registered
            Json(ActivationResponse {
                activated: true,
                error: None,
                license_key: Some(request.license_key.clone()),
                instance: Some(InstanceInfo {
                    id: instance_id.to_string(),
                    name: request.instance_name.clone(),
                    created_at: Utc::now().to_rfc3339(),
                }),
            }).into_response()
        }
        Err(e) => {
            tracing::error!("Error creating instance: {}", e);
            Json(ActivationResponse {
                activated: false,
                error: Some("Failed to create instance".to_string()),
                license_key: None,
                instance: None,
            }).into_response()
        }
    }
}

pub async fn deactivate(
    State(state): State<AppState>,
    Json(request): Json<ActivationRequest>,
) -> impl IntoResponse {
    // Delete license instance
    let result = sqlx::query(
        "DELETE FROM license_instances WHERE license_id = (SELECT id FROM licenses WHERE license_key = $1) 
         AND machine_id = $2",
    )
    .bind(&request.license_key)
    .bind(&request.machine_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(ActivationResponse {
            activated: false,
            error: None,
            license_key: Some(request.license_key.clone()),
            instance: None,
        }),
        Err(e) => {
            tracing::error!("Error deactivating: {}", e);
            Json(ActivationResponse {
                activated: false,
                error: Some("Failed to deactivate".to_string()),
                license_key: None,
                instance: None,
            })
        }
    }
}

pub async fn validate(
    State(state): State<AppState>,
    Json(request): Json<ActivationRequest>,
) -> impl IntoResponse {
    let license = match sqlx::query_as::<_, crate::models::License>(
        "SELECT * FROM licenses WHERE license_key = $1",
    )
    .bind(&request.license_key)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(l)) => l,
        _ => {
            return Json(ValidateResponse {
                is_active: false,
                last_validated_at: None,
            })
            .into_response();
        }
    };

    // Check if license is active
    let mut is_active = license.status == "active";

    // Owner license: never expires, no trial check
    if !license.is_owner {
        // Check if trial has expired
        if is_active && license.is_trial {
            if let Some(trial_ends_at) = license.trial_ends_at {
                if trial_ends_at < Utc::now() {
                    is_active = false;
                }
            }
        }
        // Check if paid license has expired
        if is_active {
            if let Some(expires_at) = license.expires_at {
                if expires_at < Utc::now() {
                    is_active = false;
                }
            }
        }
    }

    // Update last_validated_at
    let _ = sqlx::query(
        "UPDATE license_instances SET last_validated_at = NOW() 
         WHERE license_id = (SELECT id FROM licenses WHERE license_key = $1) AND machine_id = $2",
    )
    .bind(&request.license_key)
    .bind(&request.machine_id)
    .execute(&state.pool)
    .await;

    Json(ValidateResponse {
        is_active,
        last_validated_at: Some(Utc::now().to_rfc3339()),
    }).into_response()
}

pub async fn checkout(State(state): State<AppState>) -> impl IntoResponse {
    let base = state.config.payment_base_url.trim_end_matches('/');
    let checkout_url = format!("{}/subscriptions", base);
    Json(serde_json::json!({
        "success": true,
        "checkout_url": checkout_url
    }))
}

pub async fn create_trial(
    State(state): State<AppState>,
    Json(request): Json<ActivationRequest>,
) -> impl IntoResponse {
    tracing::info!("🎯 CREATE TRIAL REQUEST RECEIVED");
    let license_key_str = request.license_key.clone();
    let machine_id_str = request.machine_id.clone();
    tracing::info!("License Key: {}", license_key_str);
    tracing::info!("Machine ID: {}", machine_id_str);
    
    // No user linking; licenses are created standalone

    // Create trial license in database
    let license_id = Uuid::new_v4();
    let trial_ends_at = Utc::now() + chrono::Duration::days(14); // 14 days trial
    
    tracing::info!("Creating trial license with ID: {}", license_id);

    // Insert license
    match sqlx::query(
        "INSERT INTO licenses (id, license_key, status, tier, is_trial, trial_ends_at, created_at, updated_at)
         VALUES ($1, $2, 'active', 'trial', true, $3, NOW(), NOW())
         ON CONFLICT (license_key) DO NOTHING
         RETURNING id"
    )
    .bind(license_id)
    .bind(&request.license_key)
    .bind(trial_ends_at)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(_)) => {
            // License created successfully, now create instance
            let instance_id = Uuid::new_v4();
            match sqlx::query(
                "INSERT INTO license_instances (id, license_id, instance_name, machine_id, app_version)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (license_id, machine_id) DO UPDATE
                 SET last_validated_at = NOW()
                 RETURNING id, instance_name, created_at"
            )
            .bind(instance_id)
            .bind(license_id)
            .bind(&request.instance_name)
            .bind(&request.machine_id)
            .bind(&request.app_version)
            .fetch_one(&state.pool)
            .await
            {
                Ok(row) => {
                    Json(ActivationResponse {
                        activated: true,
                        error: None,
                        license_key: Some(license_key_str.clone()),
                        instance: Some(InstanceInfo {
                            id: row.get::<Uuid, _>(0).to_string(),
                            name: row.get::<String, _>(1),
                            created_at: row.get::<chrono::DateTime<Utc>, _>(2).to_rfc3339(),
                        }),
                    }).into_response()
                }
                Err(e) => {
                    tracing::error!("Error creating trial instance: {}", e);
                    Json(ActivationResponse {
                        activated: false,
                        error: Some("Failed to create trial instance".to_string()),
                        license_key: None,
                        instance: None,
                    }).into_response()
                }
            }
        }
        Ok(None) => {
            // License already exists, try to activate
            match sqlx::query_as::<_, crate::models::License>(
                "SELECT * FROM licenses WHERE license_key = $1"
            )
            .bind(&request.license_key)
            .fetch_optional(&state.pool)
            .await
            {
                Ok(Some(license)) => {
                    // Create instance for existing license
                    let instance_id = Uuid::new_v4();
                    match sqlx::query(
                        "INSERT INTO license_instances (id, license_id, instance_name, machine_id, app_version)
                         VALUES ($1, $2, $3, $4, $5)
                         ON CONFLICT (license_id, machine_id) DO UPDATE
                         SET last_validated_at = NOW()
                         RETURNING id, instance_name, created_at"
                    )
                    .bind(instance_id)
                    .bind(license.id)
                    .bind(&request.instance_name)
                    .bind(&request.machine_id)
                    .bind(&request.app_version)
                    .fetch_one(&state.pool)
                    .await
                    {
                        Ok(row) => {
                            Json(ActivationResponse {
                                activated: true,
                                error: None,
                                license_key: Some(license_key_str.clone()),
                                instance: Some(InstanceInfo {
                                    id: row.get::<Uuid, _>(0).to_string(),
                                    name: row.get::<String, _>(1),
                                    created_at: row.get::<chrono::DateTime<Utc>, _>(2).to_rfc3339(),
                                }),
                            }).into_response()
                        }
                        Err(e) => {
                            tracing::error!("Error creating instance for existing trial: {}", e);
                            Json(ActivationResponse {
                                activated: false,
                                error: Some("Failed to create instance".to_string()),
                                license_key: None,
                                instance: None,
                            }).into_response()
                        }
                    }
                }
                _ => {
                    Json(ActivationResponse {
                        activated: false,
                        error: Some("Failed to create trial license".to_string()),
                        license_key: None,
                        instance: None,
                    }).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error creating trial: {}", e);
            Json(ActivationResponse {
                activated: false,
                error: Some("Database error".to_string()),
                license_key: None,
                instance: None,
            }).into_response()
        }
    }
}

// ============================================
// USER REGISTRATION
// ============================================

#[derive(Debug, serde::Deserialize)]
pub struct RegisterRequest {
    pub email: String,
}

#[derive(Debug, serde::Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub email: String,
    pub license_key: String,
    pub plan: String,
    pub trial_ends_at: String,
    pub message: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> impl IntoResponse {
    // Validate email
    let email = request.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid email address"
            }))
        ).into_response();
    }

    // Check if user already exists
    if let Ok(Some(_)) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE email = $1"
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    {
        return (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "User with this email already exists"
            }))
        ).into_response();
    }

    let user_id = Uuid::new_v4();
    let license_key = generate_license_key();
    let now = Utc::now();
    let trial_ends_at = now + chrono::Duration::days(14);
    let monthly_reset_at = now + chrono::Duration::days(30);

    // Create user and license in transaction
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database error"
                }))
            ).into_response();
        }
    };

    // Create user
    if let Err(e) = sqlx::query(
        "INSERT INTO users (id, email, plan, monthly_token_limit, tokens_used_this_month, monthly_reset_at, created_at, updated_at)
         VALUES ($1, $2, 'free', 5000, 0, $3, $4, $5)"
    )
    .bind(&user_id)
    .bind(&email)
    .bind(monthly_reset_at)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Failed to create user: {}", e);
        let _ = tx.rollback().await;
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create user"
            }))
        ).into_response();
    }

    // Create license with trial
    if let Err(e) = sqlx::query(
        "INSERT INTO licenses (id, license_key, user_id, status, tier, max_instances, is_trial, trial_ends_at, created_at)
         VALUES ($1, $2, $3, 'active', 'free', 1, TRUE, $4, $5)"
    )
    .bind(Uuid::new_v4())
    .bind(&license_key)
    .bind(&user_id)
    .bind(trial_ends_at)
    .bind(now)
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Failed to create license: {}", e);
        let _ = tx.rollback().await;
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create license"
            }))
        ).into_response();
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Database error"
            }))
        ).into_response();
    }

    tracing::info!(
        "✅ New user registered: {} with license: {}",
        email,
        license_key
    );

    (
        axum::http::StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user_id.to_string(),
            email,
            license_key,
            plan: "free".to_string(),
            trial_ends_at: trial_ends_at.to_rfc3339(),
            message: "User registered successfully with 14-day free trial".to_string(),
        })
    ).into_response()
}

/// Generate a random license key (format: GHOST-XXXXXXXX-XXXXXXXX)
fn generate_license_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let part1: String = (0..8)
        .map(|_| chars.chars().nth(rng.gen_range(0..chars.len())).unwrap())
        .collect();
    let part2: String = (0..8)
        .map(|_| chars.chars().nth(rng.gen_range(0..chars.len())).unwrap())
        .collect();
    format!("GHOST-{}-{}", part1, part2)
}

// ============================================
// USER LOGIN
// ============================================

#[derive(Debug, serde::Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

#[derive(Debug, serde::Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub email: String,
    pub license_key: String,
    pub plan: String,
    pub message: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    let email = request.email.trim().to_lowercase();
    
    if email.is_empty() || !email.contains('@') {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid email address"
            }))
        ).into_response();
    }

    // Get user and their license
    match sqlx::query(
        "SELECT u.id, u.email, u.plan, l.license_key 
         FROM users u 
         LEFT JOIN licenses l ON u.id = l.user_id 
         WHERE u.email = $1 
         LIMIT 1"
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(row)) => {
            let user_id: Uuid = row.get(0);
            let user_email: String = row.get(1);
            let plan: String = row.get(2);
            let license_key: Option<String> = row.get(3);

            if let Some(license_key) = license_key {
                Json(LoginResponse {
                    user_id: user_id.to_string(),
                    email: user_email,
                    license_key,
                    plan,
                    message: "Login successful".to_string(),
                }).into_response()
            } else {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": "No license found for this user"
                    }))
                ).into_response()
            }
        }
        Ok(None) => {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "User not found. Please register first."
                }))
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database error"
                }))
            ).into_response()
        }
    }
}

// ============================================
// GET USER ID FROM LICENSE KEY
// ============================================

#[derive(Debug, serde::Deserialize)]
pub struct GetUserRequest {
    pub license_key: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GetUserResponse {
    pub user_id: String,
    pub email: String,
    pub plan: String,
}

pub async fn get_user_from_license(
    State(state): State<AppState>,
    Json(request): Json<GetUserRequest>,
) -> impl IntoResponse {
    match sqlx::query(
        "SELECT u.id, u.email, u.plan 
         FROM users u 
         JOIN licenses l ON u.id = l.user_id 
         WHERE l.license_key = $1 
         LIMIT 1"
    )
    .bind(&request.license_key)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(row)) => {
            let user_id: Uuid = row.get(0);
            let email: String = row.get(1);
            let plan: String = row.get(2);

            Json(GetUserResponse {
                user_id: user_id.to_string(),
                email,
                plan,
            }).into_response()
        }
        Ok(None) => {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "License not found"
                }))
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database error"
                }))
            ).into_response()
        }
    }
}