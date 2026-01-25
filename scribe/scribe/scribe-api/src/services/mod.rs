pub mod license;
pub mod openrouter;
pub mod whisper;
pub mod payment;

use sqlx::PgPool;
use crate::services::{
    license::LicenseService,
    openrouter::OpenRouterService,
    whisper::WhisperService,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub license_service: LicenseService,
    pub openrouter_service: OpenRouterService,
    pub whisper_service: WhisperService,
}
