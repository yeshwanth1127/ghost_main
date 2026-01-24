use crate::config::Config;

#[derive(Clone)]
pub struct WhisperService {
    config: Config,
}

impl WhisperService {
    pub fn new() -> Self {
        Self {
            config: Config::from_env().unwrap(),
        }
    }

    pub async fn transcribe(&self, audio_base64: &str) -> Result<String, reqwest::Error> {
        use base64::{Engine as _, engine::general_purpose};
        let client = reqwest::Client::new();
        
        // Decode base64 audio
        let audio_bytes = general_purpose::STANDARD.decode(audio_base64).unwrap();
        
        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_bytes)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .unwrap()
            );

        let response = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.config.openai_api_key))
            .multipart(form)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        let transcription = json["text"].as_str().unwrap_or("").to_string();

        Ok(transcription)
    }
}
