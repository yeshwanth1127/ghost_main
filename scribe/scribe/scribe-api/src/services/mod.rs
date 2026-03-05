pub mod license;
pub mod openrouter;
pub mod whisper;
pub mod payment;
pub mod usage;
pub mod model_router;

use sqlx::PgPool;
use crate::config::Config;
use crate::gateway_state::GatewayState;
use crate::services::{
    license::LicenseService,
    openrouter::OpenRouterService,
    whisper::WhisperService,
    usage::UsageService,
    model_router::ModelRouter,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub license_service: LicenseService,
    pub openrouter_service: OpenRouterService,
    pub whisper_service: WhisperService,
    pub usage_service: UsageService,
    pub model_router: ModelRouter,
    pub gateway_state: GatewayState,
}
