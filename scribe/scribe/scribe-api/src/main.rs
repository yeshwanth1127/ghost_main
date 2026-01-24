mod config;
mod db;
mod middleware;
mod models;
mod routes;
mod services;

use axum::{
    extract::Request,
    http::Method,
    response::Response,
    routing::{get, post},
    Router,
};
use axum::middleware::{from_fn, Next};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

use config::Config;
use db::create_pool;
use services::leave::LeaveService;

fn load_env_file(path: &str) {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            for (i, raw_line) in contents.lines().enumerate() {
                let mut line = raw_line.trim();
                if line.is_empty() { continue; }
                // Strip UTF-8 BOM if present on the very first line
                if i == 0 {
                    const BOM: char = '\u{FEFF}';
                    if line.starts_with(BOM) {
                        line = &line[BOM.len_utf8()..];
                    }
                }
                if line.starts_with('#') { continue; }
                if let Some(eq_idx) = line.find('=') {
                    let key = line[..eq_idx].trim();
                    let val = line[eq_idx + 1..].trim();
                    if !key.is_empty() {
                        std::env::set_var(key, val);
                    }
                }
            }
            tracing::info!("Loaded environment variables from {}", path);
        }
        Err(e) => {
            tracing::warn!("Could not read {}: {}", path, e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "ghost_api=debug".to_string()),
        )
        .init();

    // Load environment variables explicitly from local .env (robust parser)
    load_env_file(".env");

    let config = Config::from_env()?;

    // Create database connection pool
    let pool = create_pool(&config.database_url).await?;
    let leave_pool = create_pool(&config.leave_database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    ensure_leave_schema(&leave_pool)
        .await
        .expect("Failed to ensure leave applications schema");

    // Build the application
    let leave_service = LeaveService::new(leave_pool.clone());
    let app = create_router(pool.clone(), leave_service).await;

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🚀🚀🚀 Ghost API server listening on {} 🚀🚀🚀", addr);
    tracing::info!("📍 Available endpoints:");
    tracing::info!("  - GET  http://{}:{}/health", "localhost", config.port);
    tracing::info!("  - POST http://{}:{}/api/v1/models", "localhost", config.port);
    tracing::info!("  - POST http://{}:{}/api/v1/create-trial", "localhost", config.port);
    tracing::info!("  - POST http://{}:{}/api/v1/chat", "localhost", config.port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_router(pool: PgPool, leave_service: LeaveService) -> Router {
    tracing::info!("🚀 Setting up routes...");
    
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any);

    // Create services
    let license_service = services::license::LicenseService::new(pool.clone());
    let openrouter_service = services::openrouter::OpenRouterService::new();
    let whisper_service = services::whisper::WhisperService::new();

    let app_state = services::AppState {
        pool: pool.clone(),
        license_service: license_service.clone(),
        openrouter_service: openrouter_service.clone(),
        whisper_service: whisper_service.clone(),
        leave_service: leave_service.clone(),
    };

    Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/api/v1/status", get(routes::health::status))
        .route("/api/v1/activate", post(routes::auth::activate))
        .route("/api/v1/deactivate", post(routes::auth::deactivate))
        .route("/api/v1/validate", post(routes::auth::validate))
        .route("/api/v1/checkout", get(routes::auth::checkout))
        .route("/api/v1/create-trial", post(routes::auth::create_trial))
        .route("/api/v1/chat", post(routes::chat::chat))
        .route("/api/v1/audio", post(routes::audio::transcribe))
        .route("/api/v1/models", post(routes::models::list_models))
        .route("/api/v1/prompt", post(routes::models::generate_prompt))
        // Desktop updater (Tauri) manifest
        .route(
            "/api/v1/desktop-updates/tauri",
            get(routes::updates::tauri_manifest),
        )
        .route(
            "/api/v1/leave-applications",
            post(routes::leave::create_leave_application),
        )
        .layer(from_fn(log_request))
        .layer(cors)
        .with_state(app_state)
}

// Logging middleware
async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    tracing::info!("📥 INCOMING REQUEST: {} {}", method, uri);
    tracing::info!("   Headers: {:?}", headers);
    
    let response = next.run(request).await;
    
    tracing::info!("📤 RESPONSE STATUS: {}", response.status());
    
    response
}

async fn ensure_leave_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS leave_applications (
            id UUID PRIMARY KEY,
            name TEXT NOT NULL,
            usn TEXT NOT NULL,
            department TEXT NOT NULL,
            reason TEXT NOT NULL,
            summary TEXT NOT NULL,
            attachments JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}