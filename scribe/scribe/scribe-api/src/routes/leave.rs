use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::json;
use tracing::error;

use crate::{
    models::leave::{LeaveApplicationPayload, LeaveApplicationRequest, LeaveApplicationResponse},
    services::AppState,
};

pub async fn create_leave_application(
    State(state): State<AppState>,
    Json(request): Json<LeaveApplicationRequest>,
) -> Result<(StatusCode, Json<LeaveApplicationResponse>), (StatusCode, Json<serde_json::Value>)> {
    let payload: LeaveApplicationPayload = request.into();

    tracing::info!(
        "📥 Leave application received for {} ({}) with {} attachments",
        payload.name,
        payload.usn,
        payload.attachments.len()
    );

    let record = state
        .leave_service
        .save_application(payload)
        .await
        .map_err(|error| {
            error!("Failed to save leave application: {error:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to save leave application"
                })),
            )
        })?;

    tracing::info!(
        "✅ Leave application stored with id {} at {}",
        record.id,
        record.created_at
    );

    Ok((
        StatusCode::CREATED,
        Json(LeaveApplicationResponse {
            id: record.id,
            created_at: record.created_at,
        }),
    ))
}

