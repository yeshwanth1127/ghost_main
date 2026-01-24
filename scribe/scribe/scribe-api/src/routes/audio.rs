use axum::{extract::State, response::Json};

use crate::models::{AudioRequest, AudioResponse};
use crate::services::AppState;

pub async fn transcribe(
    State(state): State<AppState>,
    Json(request): Json<AudioRequest>,
) -> Json<AudioResponse> {
    match state.whisper_service.transcribe(&request.audio_base64).await {
        Ok(transcription) => Json(AudioResponse {
            success: true,
            transcription: Some(transcription),
            error: None,
        }),
        Err(e) => Json(AudioResponse {
            success: false,
            transcription: None,
            error: Some(e.to_string()),
        }),
    }
}
