mod config;
mod db;
mod gateway;
mod gateway_state;
mod middleware;
mod models;
mod routes;
mod services;

use axum::{
    extract::Request,
    http::Method,
    response::Response,
    routing::{any, get, post},
    Router,
};
use axum::middleware::{from_fn, Next};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

use config::Config;
use db::create_pool;

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
    // Initialize tracing - use info level by default so chat/OpenRouter logs always show
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "ghost_api=info,info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(rust_log.clone())
        .init();

    eprintln!(">>> ghost-api starting (with GATEWAY) <<<");
    eprintln!(">>> RUST_LOG={}", rust_log);

    // Load environment variables explicitly from local .env (robust parser)
    load_env_file(".env");

    let config = Config::from_env()?;

    // Create database connection pool
    let pool = create_pool(&config.database_url).await?;

    // Run migrations (auto-repair orphaned version 3 if VersionMissing)
    let migrate_result = sqlx::migrate!("./migrations").run(&pool).await;
    if let Err(e) = migrate_result {
        if e.to_string().contains("VersionMissing(3)") {
            tracing::warn!("Repairing orphaned migration version 3...");
            if sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 3")
                .execute(&pool)
                .await
                .is_ok()
            {
                if let Err(e2) = sqlx::migrate!("./migrations").run(&pool).await {
                    panic!("Failed to run migrations after repair: {}", e2);
                }
                tracing::info!("Migration repair successful");
            } else {
                panic!("Failed to run migrations: {}. Try: DELETE FROM _sqlx_migrations WHERE version = 3;", e);
            }
        } else {
            panic!("Failed to run migrations: {}", e);
        }
    }

    // Background task: reset monthly tokens for users whose monthly_reset_at has passed
    let pool_reset = pool.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            if let Err(e) = sqlx::query(
                r#"
                UPDATE users
                SET tokens_used_this_month = 0,
                    monthly_reset_at = DATE_TRUNC('month', NOW() + INTERVAL '1 month'),
                    updated_at = NOW()
                WHERE monthly_reset_at < NOW()
                "#,
            )
            .execute(&pool_reset)
            .await
            {
                tracing::error!("Monthly token reset failed: {}", e);
            } else {
                tracing::info!("Monthly token reset task completed");
            }
        }
    });

    // Build the application
    let app = create_router(pool.clone(), config.clone()).await;

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🚀🚀🚀 Ghost API server listening on {} 🚀🚀🚀", addr);
    tracing::info!("📍 Available endpoints:");
    tracing::info!("  - GET  http://{}:{}/health", "localhost", config.port);
    tracing::info!("  - GET  http://{}:{}/gateway-ping (gateway check)", "localhost", config.port);
    tracing::info!("  - POST http://{}:{}/api/v1/models", "localhost", config.port);
    tracing::info!("  - POST http://{}:{}/api/v1/create-trial", "localhost", config.port);
    tracing::info!("  - POST http://{}:{}/api/v1/chat", "localhost", config.port);
    tracing::info!("  - WS   ws://{}:{}/gateway (agent mode)", "localhost", config.port);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(">>> FATAL: Could not bind to {}: {} <<<", addr, e);
            eprintln!(">>> Another process is probably using this port. Stop it (e.g. pm2 stop ghost-api) or use PORT=8084 <<<");
            return Err(e.into());
        }
    };
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_router(pool: PgPool, config: Config) -> Router {
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
    let usage_service = services::usage::UsageService::new(pool.clone());
    let model_router = services::model_router::ModelRouter::new(usage_service.clone());

    let gateway_state = gateway_state::GatewayState::new();
    let app_state = services::AppState {
        pool: pool.clone(),
        config: config.clone(),
        license_service: license_service.clone(),
        openrouter_service: openrouter_service.clone(),
        whisper_service: whisper_service.clone(),
        usage_service: usage_service.clone(),
        model_router: model_router.clone(),
        gateway_state: gateway_state.clone(),
    };

    Router::new()
        // Gateway: top-level /gateway-ping (same style as /health) and WS at /gateway
        .route("/gateway-ping", get(gateway::gateway_ping))
        .route("/gateway/ping", get(gateway::gateway_ping))
        .route("/gateway", any(gateway::gateway_ws_handler))
        .route("/gateway/", any(gateway::gateway_ws_handler))
        .route("/health", get(routes::health::health_check))
        .route("/api/v1/status", get(routes::health::status))
        .route("/api/v1/activate", post(routes::auth::activate))
        .route("/api/v1/deactivate", post(routes::auth::deactivate))
        .route("/api/v1/validate", post(routes::auth::validate))
        .route("/api/v1/checkout", get(routes::auth::checkout))
        .route("/api/v1/create-trial", post(routes::auth::create_trial))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/get-user", post(routes::auth::get_user_from_license))
        .route("/api/v1/chat", post(routes::chat::chat))
        .route("/api/v1/audio", post(routes::audio::transcribe))
        // Usage tracking endpoints
        .route("/api/v1/usage/:user_id", get(routes::usage::get_usage_stats))
        .route("/api/v1/usage/:user_id/history", get(routes::usage::get_usage_history))
        .route("/api/v1/usage/:user_id/limit-check", get(routes::usage::check_token_limit))
        .route("/api/v1/usage/record", post(routes::usage::record_usage_from_client))
        .route("/api/v1/usage/pricing", get(routes::usage::get_model_pricing))
        // Models and updates
        .route("/api/v1/models", post(routes::models::list_models))
        .route("/api/v1/prompt", post(routes::models::generate_prompt))
        // Desktop updater (Tauri) manifest
        .route(
            "/api/v1/desktop-updates/tauri",
            get(routes::updates::tauri_manifest),
        )
        .layer(from_fn(log_request))
        .layer(cors)
        .with_state(app_state)
}

// Logging middleware - eprintln ensures logs show even if RUST_LOG filters them
async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    eprintln!("[API] 📥 INCOMING: {} {}", method, uri);
    tracing::info!("📥 INCOMING REQUEST: {} {}", method, uri);
    tracing::info!("   Headers: {:?}", headers);
    
    let response = next.run(request).await;
    
    tracing::info!("📤 RESPONSE STATUS: {}", response.status());
    
    response
}
