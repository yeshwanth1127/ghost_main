use crate::config::Config;
use futures::StreamExt;

#[derive(Clone)]
pub struct OpenRouterService {
    pub config: Config,
}

impl OpenRouterService {
    pub fn new() -> Self {
        // TODO: Load config from environment
        Self {
            config: Config::from_env().unwrap(),
        }
    }

    pub async fn chat(
        &self,
        user_message: &str,
        system_prompt: Option<&str>,
        images: Option<&serde_json::Value>,
        history: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<impl futures::Stream<Item = Result<String, reqwest::Error>>, reqwest::Error> {
        let client = reqwest::Client::new();
        
        // Select model: require client-provided model_id; if not provided and images present,
        // the client should have chosen a vision-capable model. We do not hardcode defaults.
        let model = model_id.unwrap_or("");

        let mut messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": prompt
            }));
        }

        // Add history if provided
        if let Some(hist) = history {
            if let Ok(hist_array) = serde_json::from_str::<Vec<serde_json::Value>>(hist) {
                messages.extend(hist_array);
            }
        }

        // Build user message content
        let mut user_content = Vec::new();
        user_content.push(serde_json::json!({"type": "text", "text": user_message}));
        
        if let Some(imgs) = images {
            // Handle image data
            if let Some(img_array) = imgs.as_array() {
                for img in img_array {
                    user_content.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": {"url": format!("data:image/png;base64,{}", img.as_str().unwrap_or(""))}
                    }));
                }
            } else if let Some(img_str) = imgs.as_str() {
                user_content.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": {"url": format!("data:image/png;base64,{}", img_str)}
                }));
            }
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": user_content
        }));

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true
        });

        let response = client
            .post(&format!("{}/chat/completions", self.config.openrouter_base_url))
            .header("Authorization", format!("Bearer {}", self.config.openrouter_api_key))
            .header("HTTP-Referer", "https://exora.solutions")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        // Return streaming response
        Ok(response.bytes_stream().map(|chunk| {
            chunk.map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        }))
    }
}
