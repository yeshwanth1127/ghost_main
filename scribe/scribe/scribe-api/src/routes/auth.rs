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

pub async fn checkout() -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "checkout_url": "https://exora.solutions/checkout"
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