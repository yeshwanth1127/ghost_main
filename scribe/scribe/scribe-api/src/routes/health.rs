use axum::response::{IntoResponse, Json};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    tracing::info!("💚 Health check request received");
    Json(json!({
        "status": "ok",
        "service": "ghost-api",
        "gateway": true
    }))
}

pub async fn status() -> impl IntoResponse {
    Json(json!({
        "status": "operational",
        "version": "0.1.0",
        "uptime": "online"
    }))
}
