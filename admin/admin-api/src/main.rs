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

use routes::{admin, auth as auth_routes, payments as payment_routes};
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env: try admin-api dir first, then scribe-api (same DB)
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_path_override("../../scribe/scribe/scribe-api/.env");

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
        .route("/api/payments/create-subscription", post(payment_routes::create_subscription))
        .route("/api/payments/verify", post(payment_routes::verify_payment))
        .route("/api/payments/webhook", post(payment_routes::webhook))
        .nest("/api/stats", stats_routes)
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Admin API listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
