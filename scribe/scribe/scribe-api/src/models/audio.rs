use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRequest {
    pub audio_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioResponse {
    pub success: bool,
    pub transcription: Option<String>,
    pub error: Option<String>,
}
