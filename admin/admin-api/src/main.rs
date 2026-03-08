mod auth;
mod config;
mod routes;
mod state;

use axum::{routing::get, routing::post, Router};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

use routes::{admin, auth as auth_routes, payments as payment_routes, trial as trial_routes};
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Required for rustls (used by mail-send SMTP)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Load .env: admin-api dir first (works regardless of cwd), then cwd, then scribe-api
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env"));
    let _ = dotenvy::dotenv();
    let scribe_env = manifest_dir.join("../../scribe/scribe/scribe-api/.env");
    if scribe_env.exists() {
        let _ = dotenvy::from_path_override(&scribe_env);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "admin_api=info,info".to_string()),
        )
        .init();

    let config = Arc::new(config::Config::from_env()?);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    let app_state = AppState {
        pool: pool.clone(),
        config: config.clone(),
    };

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let protected = axum::middleware::from_fn_with_state(app_state.clone(), auth::require_auth);

    let stats_routes = Router::new()
        .route("/global", get(admin::global_stats))
        .route("/model-breakdown", get(admin::model_breakdown))
        .route("/top-users", get(admin::top_users))
        .route("/recent-messages", get(admin::recent_messages))
        .route_layer(protected)
        .with_state(app_state.clone());

    let app = Router::new()
        .route("/api/auth/login", post(auth_routes::login))
        .route("/api/auth/customer-login", post(auth_routes::customer_login))
        .route("/api/auth/register", post(auth_routes::register))
        .route("/api/trial/send-otp", post(trial_routes::send_otp))
        .route("/api/trial/send-login-otp", post(trial_routes::send_login_otp))
        .route("/api/trial/verify-otp", post(trial_routes::verify_otp))
        .route("/api/payments/create-subscription", post(payment_routes::create_subscription))
        .route("/api/payments/verify", post(payment_routes::verify_payment))
        .route("/api/payments/webhook", post(payment_routes::webhook))
        .nest("/api/stats", stats_routes)
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Admin API listening on {}", addr);
    if !config.smtp_username.is_empty() {
        tracing::info!("Email (SMTP) configured: {} @ {}", config.smtp_username, config.smtp_host);
    } else if !config.resend_api_key.is_empty() {
        tracing::info!("Email (Resend) configured");
    } else {
        tracing::info!("Email not configured - OTP will be logged to console only");
    }
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
